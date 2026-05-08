use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sqlx::Row;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, Schema, Value, FAST, STORED, STRING, TEXT};
use tantivy::{doc, Index, ReloadPolicy};
use thiserror::Error;
use walkdir::WalkDir;

use crate::embeddings::{cosine_similarity, embed_text, vector_magnitude, EmbeddingSummary};
use crate::metadata::{current_timestamp, MetadataError, MetadataStore};

const DEFAULT_SEARCH_SCAN_LIMIT: usize = 2_000;
const MAX_SEARCH_SCAN_LIMIT: usize = 10_000;
const SEARCH_SCAN_MULTIPLIER: usize = 50;
const CHUNK_TARGET_LINES: usize = 80;
const CHUNK_OVERLAP_LINES: usize = 12;
const SEARCH_INDEX_DIR: &str = ".localbrain/search";

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchIndexSummary {
    pub root: String,
    pub documents_indexed: usize,
    pub embeddings_indexed: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub path: String,
    pub chunk_id: Option<String>,
    pub kind: String,
    pub title: String,
    pub snippet: String,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
    pub text_score: f32,
    pub vector_score: f32,
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SearchChunk {
    id: String,
    title: String,
    content: String,
    start_line: usize,
    end_line: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IndexedDocument {
    pub path: String,
    pub kind: String,
    pub title: String,
}

#[derive(Debug, Error)]
pub enum SearchError {
    #[error("metadata error: {0}")]
    Metadata(#[from] MetadataError),
    #[error("sqlite operation failed: {0}")]
    Sqlite(#[from] sqlx::Error),
    #[error("filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("walk failed: {0}")]
    Walk(#[from] walkdir::Error),
    #[error("embedding vector is invalid for {path}")]
    InvalidVector { path: String },
    #[error("search index operation failed: {0}")]
    Tantivy(#[from] tantivy::TantivyError),
    #[error("search query parse failed: {0}")]
    QueryParse(#[from] tantivy::query::QueryParserError),
}

pub async fn rebuild_search_index(
    path: impl AsRef<Path>,
    metadata_store: &MetadataStore,
) -> Result<SearchIndexSummary, SearchError> {
    let requested_path = path.as_ref();
    let root = metadata_store.resolve_path(requested_path)?;
    let normalized_root = metadata_store.normalize_path(requested_path);
    let mut summary = SearchIndexSummary {
        root: normalized_root,
        documents_indexed: 0,
        embeddings_indexed: 0,
        errors: Vec::new(),
    };

    for path in searchable_paths(&root)? {
        match index_document(&path, metadata_store).await {
            Ok(_) => {
                summary.documents_indexed += 1;
                summary.embeddings_indexed += 1;
            }
            Err(error) => summary
                .errors
                .push(format!("{}: {}", path.display(), error)),
        }
    }

    rebuild_tantivy_index(metadata_store).await?;

    Ok(summary)
}

pub async fn clear_search_index(metadata_store: &MetadataStore) -> Result<(), SearchError> {
    sqlx::query("DELETE FROM chunk_embeddings")
        .execute(metadata_store.pool())
        .await?;
    sqlx::query("DELETE FROM embeddings")
        .execute(metadata_store.pool())
        .await?;
    sqlx::query("DELETE FROM search_chunks")
        .execute(metadata_store.pool())
        .await?;
    sqlx::query("DELETE FROM search_documents")
        .execute(metadata_store.pool())
        .await?;
    clear_tantivy_index(metadata_store)?;
    Ok(())
}

pub async fn indexed_documents(
    metadata_store: &MetadataStore,
    limit: usize,
) -> Result<Vec<IndexedDocument>, SearchError> {
    let rows = sqlx::query(
        "
        SELECT path, kind, title
        FROM search_documents
        ORDER BY path
        LIMIT ?
        ",
    )
    .bind(i64::try_from(limit).unwrap_or(i64::MAX))
    .fetch_all(metadata_store.pool())
    .await?;

    let mut documents = Vec::new();
    for row in rows {
        documents.push(IndexedDocument {
            path: row.try_get("path")?,
            kind: row.try_get("kind")?,
            title: row.try_get("title")?,
        });
    }

    Ok(documents)
}

pub async fn index_document(
    path: impl AsRef<Path>,
    metadata_store: &MetadataStore,
) -> Result<EmbeddingSummary, SearchError> {
    let path = path.as_ref();
    let source_path = metadata_store.resolve_path(path)?;
    let content = fs::read_to_string(&source_path)?;
    let normalized_path = metadata_store.normalize_path(path);
    let title = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(&normalized_path)
        .to_string();
    let kind = document_kind(path);
    let updated_at = current_timestamp()?;
    let vector = embed_text(&format!("{normalized_path}\n{title}\n{content}"));
    let chunks = chunk_content(&title, &content);

    metadata_store.record_file_metadata(path).await?;
    upsert_search_document(
        metadata_store,
        &normalized_path,
        kind,
        &title,
        &content,
        &updated_at,
    )
    .await?;
    upsert_embedding(metadata_store, &normalized_path, &vector, &updated_at).await?;
    replace_search_chunks(metadata_store, &normalized_path, kind, &chunks, &updated_at).await?;

    Ok(EmbeddingSummary {
        path: normalized_path,
        dimensions: vector.len(),
        magnitude: vector_magnitude(&vector),
    })
}

pub async fn search_text(
    metadata_store: &MetadataStore,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>, SearchError> {
    let index = open_or_create_tantivy_index(metadata_store)?;
    let schema = index.schema();
    let fields = tantivy_fields(&schema)?;
    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommitWithDelay)
        .try_into()?;
    reader.reload()?;
    let searcher = reader.searcher();
    let query_parser =
        QueryParser::for_index(&index, vec![fields.path, fields.title, fields.content]);
    let parsed_query = match query_parser.parse_query(query) {
        Ok(parsed) => parsed,
        Err(_) => query_parser.parse_query(&query_terms(query).join(" "))?,
    };
    let top_docs = searcher.search(
        &parsed_query,
        &TopDocs::with_limit(i64_to_usize(scan_limit_for(limit))),
    )?;
    let terms = query_terms(query);
    let mut results = Vec::new();

    for (score, address) in top_docs {
        let retrieved = searcher.doc(address)?;
        let path = extract_text_field(&retrieved, fields.path);
        let chunk_id = extract_text_field(&retrieved, fields.chunk_id);
        let kind = extract_text_field(&retrieved, fields.kind);
        let title = extract_text_field(&retrieved, fields.title);
        let content = extract_text_field(&retrieved, fields.content);
        let start_line =
            extract_u64_field(&retrieved, fields.start_line).map(|value| value as usize);
        let end_line = extract_u64_field(&retrieved, fields.end_line).map(|value| value as usize);
        let text_score = score.max(0.0);

        results.push(SearchResult {
            path,
            chunk_id: Some(chunk_id),
            kind,
            title,
            snippet: snippet(&content, &terms),
            start_line,
            end_line,
            text_score,
            vector_score: 0.0,
            score: text_score,
        });
    }

    results.sort_by(sort_by_score);
    results.truncate(limit);
    Ok(results)
}

pub async fn hybrid_search(
    metadata_store: &MetadataStore,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>, SearchError> {
    let rows = sqlx::query(
        "
        SELECT c.path, c.chunk_id, c.kind, c.title, c.content, c.start_line, c.end_line, e.vector_json, e.vector_blob
        FROM search_chunks c
        LEFT JOIN chunk_embeddings e ON e.path = c.path AND e.chunk_id = c.chunk_id
        ORDER BY c.updated_at DESC
        LIMIT ?
        ",
    )
    .bind(scan_limit_for(limit))
    .fetch_all(metadata_store.pool())
    .await?;

    let query_terms = query_terms(query);
    let query_vector = embed_text(query);
    let mut results = Vec::new();

    for row in rows {
        let path: String = row.try_get("path")?;
        let chunk_id: String = row.try_get("chunk_id")?;
        let kind: String = row.try_get("kind")?;
        let title: String = row.try_get("title")?;
        let content: String = row.try_get("content")?;
        let start_line = row.try_get::<i64, _>("start_line")?;
        let end_line = row.try_get::<i64, _>("end_line")?;
        let vector_json: Option<String> = row.try_get("vector_json")?;
        let vector_blob: Option<Vec<u8>> = row.try_get("vector_blob")?;
        let haystack = format!("{title}\n{content}").to_lowercase();
        let text_score = score_text(&haystack, &query_terms);
        let vector_score = match (vector_json, vector_blob) {
            (Some(vector_json), _) => serde_json::from_str::<Vec<f32>>(&vector_json)
                .ok()
                .filter(|vector| vector.len() == query_vector.len())
                .map(|vector| cosine_similarity(&query_vector, &vector).max(0.0))
                .unwrap_or(0.0),
            (None, Some(vector_blob)) => vector_from_blob(&vector_blob)
                .filter(|vector| vector.len() == query_vector.len())
                .map(|vector| cosine_similarity(&query_vector, &vector).max(0.0))
                .unwrap_or(0.0),
            (None, None) => 0.0,
        };
        let score = (text_score * 0.65) + (vector_score * 0.35);

        if score > 0.0 {
            results.push(SearchResult {
                path,
                chunk_id: Some(chunk_id),
                kind,
                title,
                snippet: snippet(&content, &query_terms),
                start_line: Some(i64_to_usize(start_line)),
                end_line: Some(i64_to_usize(end_line)),
                text_score,
                vector_score,
                score,
            });
        }
    }

    results.sort_by(sort_by_score);
    results.truncate(limit);
    Ok(results)
}

pub async fn project_overview_chunks(
    metadata_store: &MetadataStore,
    limit: usize,
) -> Result<Vec<SearchResult>, SearchError> {
    let rows = sqlx::query(
        "
        SELECT path, chunk_id, kind, title, content, start_line, end_line
        FROM search_chunks
        ORDER BY path, start_line
        LIMIT ?
        ",
    )
    .bind(scan_limit_for(limit))
    .fetch_all(metadata_store.pool())
    .await?;

    let mut results = Vec::new();

    for row in rows {
        let path: String = row.try_get("path")?;
        let chunk_id: String = row.try_get("chunk_id")?;
        let kind: String = row.try_get("kind")?;
        let title: String = row.try_get("title")?;
        let content: String = row.try_get("content")?;
        let start_line = row.try_get::<i64, _>("start_line")?;
        let end_line = row.try_get::<i64, _>("end_line")?;
        let priority = overview_priority(&path, &chunk_id, i64_to_usize(start_line), &content);

        if priority > 0.0 {
            results.push(SearchResult {
                path,
                chunk_id: Some(chunk_id),
                kind,
                title,
                snippet: context_snippet(&content),
                start_line: Some(i64_to_usize(start_line)),
                end_line: Some(i64_to_usize(end_line)),
                text_score: priority,
                vector_score: 0.0,
                score: priority,
            });
        }
    }

    results.sort_by(sort_by_score);
    dedupe_results_by_path(&mut results);
    results.truncate(limit);
    Ok(results)
}

pub async fn document_for_path(
    metadata_store: &MetadataStore,
    path: &str,
    query: &str,
) -> Result<Option<SearchResult>, SearchError> {
    let row = sqlx::query(
        "
        SELECT path, kind, title, content
        FROM search_documents
        WHERE path = ?
        LIMIT 1
        ",
    )
    .bind(path)
    .fetch_optional(metadata_store.pool())
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let path: String = row.try_get("path")?;
    let kind: String = row.try_get("kind")?;
    let title: String = row.try_get("title")?;
    let content: String = row.try_get("content")?;
    let terms = query_terms(query);

    Ok(Some(SearchResult {
        path,
        chunk_id: None,
        kind,
        title,
        snippet: snippet(&content, &terms),
        start_line: None,
        end_line: None,
        text_score: 1.0,
        vector_score: 1.0,
        score: 1.0,
    }))
}

async fn upsert_search_document(
    metadata_store: &MetadataStore,
    path: &str,
    kind: &str,
    title: &str,
    content: &str,
    updated_at: &str,
) -> Result<(), SearchError> {
    sqlx::query(
        "
        INSERT INTO search_documents (path, kind, title, content, updated_at)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(path) DO UPDATE SET
          kind = excluded.kind,
          title = excluded.title,
          content = excluded.content,
          updated_at = excluded.updated_at
        ",
    )
    .bind(path)
    .bind(kind)
    .bind(title)
    .bind(content)
    .bind(updated_at)
    .execute(metadata_store.pool())
    .await?;

    Ok(())
}

async fn upsert_embedding(
    metadata_store: &MetadataStore,
    path: &str,
    vector: &[f32],
    updated_at: &str,
) -> Result<(), SearchError> {
    let vector_json = serde_json::to_string(vector).map_err(|_| SearchError::InvalidVector {
        path: path.to_string(),
    })?;
    let vector_blob = vector_to_blob(vector);

    sqlx::query(
        "
        INSERT INTO embeddings (path, dimensions, vector_json, vector_blob, updated_at)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(path) DO UPDATE SET
          dimensions = excluded.dimensions,
          vector_json = excluded.vector_json,
          vector_blob = excluded.vector_blob,
          updated_at = excluded.updated_at
        ",
    )
    .bind(path)
    .bind(i64::try_from(vector.len()).unwrap_or(i64::MAX))
    .bind(vector_json)
    .bind(vector_blob)
    .bind(updated_at)
    .execute(metadata_store.pool())
    .await?;

    Ok(())
}

async fn replace_search_chunks(
    metadata_store: &MetadataStore,
    path: &str,
    kind: &str,
    chunks: &[SearchChunk],
    updated_at: &str,
) -> Result<(), SearchError> {
    sqlx::query("DELETE FROM chunk_embeddings WHERE path = ?")
        .bind(path)
        .execute(metadata_store.pool())
        .await?;
    sqlx::query("DELETE FROM search_chunks WHERE path = ?")
        .bind(path)
        .execute(metadata_store.pool())
        .await?;

    for chunk in chunks {
        sqlx::query(
            "
            INSERT INTO search_chunks (
              path,
              chunk_id,
              kind,
              title,
              content,
              start_line,
              end_line,
              updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(path)
        .bind(&chunk.id)
        .bind(kind)
        .bind(&chunk.title)
        .bind(&chunk.content)
        .bind(usize_to_i64(chunk.start_line))
        .bind(usize_to_i64(chunk.end_line))
        .bind(updated_at)
        .execute(metadata_store.pool())
        .await?;

        upsert_chunk_embedding(metadata_store, path, chunk, updated_at).await?;
    }

    Ok(())
}

async fn upsert_chunk_embedding(
    metadata_store: &MetadataStore,
    path: &str,
    chunk: &SearchChunk,
    updated_at: &str,
) -> Result<(), SearchError> {
    let vector = embed_text(&format!("{path}\n{}\n{}", chunk.title, chunk.content));
    let vector_json = serde_json::to_string(&vector).map_err(|_| SearchError::InvalidVector {
        path: path.to_string(),
    })?;
    let vector_blob = vector_to_blob(&vector);

    sqlx::query(
        "
        INSERT INTO chunk_embeddings (path, chunk_id, dimensions, vector_json, vector_blob, updated_at)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(path, chunk_id) DO UPDATE SET
          dimensions = excluded.dimensions,
          vector_json = excluded.vector_json,
          vector_blob = excluded.vector_blob,
          updated_at = excluded.updated_at
        ",
    )
    .bind(path)
    .bind(&chunk.id)
    .bind(i64::try_from(vector.len()).unwrap_or(i64::MAX))
    .bind(vector_json)
    .bind(vector_blob)
    .bind(updated_at)
    .execute(metadata_store.pool())
    .await?;

    Ok(())
}

fn chunk_content(title: &str, content: &str) -> Vec<SearchChunk> {
    let lines = content.lines().collect::<Vec<_>>();
    if lines.is_empty() {
        return vec![SearchChunk {
            id: "chunk-0001".to_string(),
            title: format!("{title}:L1-L1"),
            content: String::new(),
            start_line: 1,
            end_line: 1,
        }];
    }

    let mut chunks = Vec::new();
    let mut start = 0usize;

    while start < lines.len() {
        let mut end = (start + CHUNK_TARGET_LINES).min(lines.len());
        if end < lines.len() {
            end = prefer_boundary(&lines, start, end);
        }

        let start_line = start + 1;
        let end_line = end.max(start + 1);
        let content = lines[start..end_line].join("\n");
        let id = format!("chunk-{start_line:04}-{end_line:04}");
        chunks.push(SearchChunk {
            id,
            title: format!("{title}:L{start_line}-L{end_line}"),
            content,
            start_line,
            end_line,
        });

        if end_line >= lines.len() {
            break;
        }
        start = end_line.saturating_sub(CHUNK_OVERLAP_LINES).max(start + 1);
    }

    chunks
}

fn prefer_boundary(lines: &[&str], start: usize, fallback_end: usize) -> usize {
    let min_end = (start + CHUNK_TARGET_LINES / 2).min(fallback_end);
    for index in (min_end..fallback_end).rev() {
        let line = lines[index].trim_start();
        if line.is_empty()
            || line.starts_with("def ")
            || line.starts_with("class ")
            || line.starts_with("function ")
            || line.starts_with("export ")
            || line.starts_with("pub ")
            || line.starts_with("fn ")
        {
            return index.max(start + 1);
        }
    }
    fallback_end
}

fn searchable_paths(root: &Path) -> Result<Vec<PathBuf>, SearchError> {
    let mut paths = Vec::new();

    if root.is_file() {
        if is_searchable_file(root) {
            paths.push(root.to_path_buf());
        }
        return Ok(paths);
    }

    for entry in WalkDir::new(root).into_iter().filter_entry(|entry| {
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(path);
        path == root || !is_ignored_path(relative)
    }) {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(path);

        if path.is_file() && is_searchable_file(path) && !is_ignored_path(relative) {
            paths.push(path.to_path_buf());
        }
    }

    paths.sort();
    Ok(paths)
}

fn document_kind(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("md") => "wiki",
        _ => "code",
    }
}

fn is_searchable_file(path: &Path) -> bool {
    let has_supported_extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension,
                "js" | "jsx"
                    | "ts"
                    | "tsx"
                    | "rs"
                    | "py"
                    | "java"
                    | "go"
                    | "c"
                    | "h"
                    | "cpp"
                    | "hpp"
                    | "cs"
                    | "php"
                    | "rb"
                    | "swift"
                    | "kt"
                    | "kts"
                    | "scala"
                    | "html"
                    | "css"
                    | "scss"
                    | "json"
                    | "yaml"
                    | "yml"
                    | "toml"
                    | "xml"
                    | "sql"
                    | "sh"
                    | "md"
                    | "txt"
            )
        });

    has_supported_extension || is_extensionless_searchable_file(path)
}

fn is_extensionless_searchable_file(path: &Path) -> bool {
    path.extension().is_none()
        && path.file_name().is_some_and(|name| {
            matches!(
                name.to_string_lossy().as_ref(),
                "Dockerfile"
                    | "dockerfile"
                    | "Containerfile"
                    | "Makefile"
                    | "makefile"
                    | "justfile"
                    | "Procfile"
                    | "Brewfile"
                    | "Vagrantfile"
                    | "Jenkinsfile"
                    | "Tiltfile"
                    | "README"
                    | "LICENSE"
            )
        })
}

fn is_ignored_path(path: &Path) -> bool {
    path.components().any(|component| {
        let value = component.as_os_str().to_string_lossy();
        matches!(
            value.as_ref(),
            ".git"
                | ".localbrain"
                | "node_modules"
                | "vendor"
                | "dist"
                | "build"
                | "target"
                | "__snapshots__"
                | "snapshots"
        ) || (value.starts_with('.') && !matches!(value.as_ref(), ".github" | ".vscode"))
    }) || path.file_name().is_some_and(|file_name| {
        let value = file_name.to_string_lossy();
        value.starts_with(".env")
            || value.ends_with("_snapshot.json")
            || value.ends_with(".snap")
            || value.ends_with(".key")
            || value.ends_with(".pem")
            || value.ends_with(".p12")
    })
}

fn query_terms(query: &str) -> Vec<String> {
    query
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .filter(|term| term.len() > 1)
        .map(|term| term.to_lowercase())
        .collect()
}

fn score_text(haystack: &str, terms: &[String]) -> f32 {
    if terms.is_empty() {
        return 0.0;
    }

    let mut score = 0.0;
    for term in terms {
        let occurrences = haystack.matches(term).count();
        if occurrences > 0 {
            score += 1.0 + (occurrences as f32).ln();
        }
    }

    score / terms.len() as f32
}

fn snippet(content: &str, terms: &[String]) -> String {
    let lower = content.to_lowercase();
    let start = terms
        .iter()
        .filter_map(|term| lower.find(term))
        .min()
        .unwrap_or(0);
    let start = previous_char_boundary(content, start.saturating_sub(80));
    let end = next_char_boundary(content, (start + 220).min(content.len()));
    content[start..end].replace('\n', " ").trim().to_string()
}

fn context_snippet(content: &str) -> String {
    const MAX_CHARS: usize = 1_200;

    content
        .chars()
        .take(MAX_CHARS)
        .collect::<String>()
        .trim()
        .to_string()
}

fn overview_priority(path: &str, chunk_id: &str, start_line: usize, content: &str) -> f32 {
    let path = path.to_lowercase();
    let file_name = path.rsplit('/').next().unwrap_or(path.as_str());
    let content = content.to_lowercase();
    let mut score = 0.0;

    if matches!(
        file_name,
        "readme.md"
            | "package.json"
            | "cargo.toml"
            | "tauri.conf.json"
            | "pyproject.toml"
            | "requirements.txt"
            | "dockerfile"
    ) {
        score += 1.0;
    }
    if matches!(
        file_name,
        "main.rs" | "main.ts" | "main.tsx" | "app.tsx" | "app.ts" | "mod.rs" | "__init__.py"
    ) {
        score += 0.75;
    }
    if path.contains("/src/")
        || path.starts_with("src/")
        || path.contains("/src-tauri/")
        || path.starts_with("src-tauri/")
    {
        score += 0.25;
    }
    if path.contains("indexer")
        || path.contains("search")
        || path.contains("llm")
        || path.contains("parser")
        || path.contains("commands")
        || path.contains("api")
    {
        score += 0.55;
    }
    if content.contains("tauri")
        || content.contains("react")
        || content.contains("index")
        || content.contains("search")
        || content.contains("parse")
        || content.contains("local brain")
    {
        score += 0.35;
    }
    if start_line == 1 || chunk_id.ends_with("-0080") {
        score += 0.25;
    }

    score
}

fn dedupe_results_by_path(results: &mut Vec<SearchResult>) {
    let mut seen = std::collections::HashSet::new();
    results.retain(|result| seen.insert(result.path.clone()));
}

fn sort_by_score(left: &SearchResult, right: &SearchResult) -> Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.path.cmp(&right.path))
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn i64_to_usize(value: i64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

fn scan_limit_for(result_limit: usize) -> i64 {
    let scan_limit = result_limit
        .saturating_mul(SEARCH_SCAN_MULTIPLIER)
        .clamp(DEFAULT_SEARCH_SCAN_LIMIT, MAX_SEARCH_SCAN_LIMIT);

    i64::try_from(scan_limit).unwrap_or(i64::MAX)
}

fn previous_char_boundary(value: &str, mut index: usize) -> usize {
    while index > 0 && !value.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn next_char_boundary(value: &str, mut index: usize) -> usize {
    while index < value.len() && !value.is_char_boundary(index) {
        index += 1;
    }
    index
}

fn vector_to_blob(vector: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(std::mem::size_of_val(vector));
    for value in vector {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn vector_from_blob(blob: &[u8]) -> Option<Vec<f32>> {
    if blob.len() % std::mem::size_of::<f32>() != 0 {
        return None;
    }
    let mut vector = Vec::with_capacity(blob.len() / std::mem::size_of::<f32>());
    for chunk in blob.chunks_exact(std::mem::size_of::<f32>()) {
        let mut bytes = [0_u8; 4];
        bytes.copy_from_slice(chunk);
        vector.push(f32::from_le_bytes(bytes));
    }
    Some(vector)
}

struct TantivyFields {
    path: Field,
    chunk_id: Field,
    kind: Field,
    title: Field,
    content: Field,
    start_line: Field,
    end_line: Field,
}

fn search_index_path(metadata_store: &MetadataStore) -> Result<PathBuf, SearchError> {
    let workspace_root = metadata_store.workspace_root_path()?;
    Ok(workspace_root.join(SEARCH_INDEX_DIR))
}

fn build_tantivy_schema() -> Schema {
    let mut builder = Schema::builder();
    builder.add_text_field("path", STRING | STORED);
    builder.add_text_field("chunk_id", STRING | STORED);
    builder.add_text_field("kind", STRING | STORED);
    builder.add_text_field("title", TEXT | STORED);
    builder.add_text_field("content", TEXT | STORED);
    builder.add_u64_field("start_line", FAST | STORED);
    builder.add_u64_field("end_line", FAST | STORED);
    builder.build()
}

fn tantivy_fields(schema: &Schema) -> Result<TantivyFields, SearchError> {
    let required = |name: &str| {
        schema
            .get_field(name)
            .map_err(|_| SearchError::InvalidVector {
                path: format!("missing tantivy field: {name}"),
            })
    };

    Ok(TantivyFields {
        path: required("path")?,
        chunk_id: required("chunk_id")?,
        kind: required("kind")?,
        title: required("title")?,
        content: required("content")?,
        start_line: required("start_line")?,
        end_line: required("end_line")?,
    })
}

fn open_or_create_tantivy_index(metadata_store: &MetadataStore) -> Result<Index, SearchError> {
    let index_path = search_index_path(metadata_store)?;
    std::fs::create_dir_all(&index_path)?;
    let schema = build_tantivy_schema();

    if index_path.join("meta.json").exists() {
        Ok(Index::open_in_dir(&index_path)?)
    } else {
        Ok(Index::create_in_dir(&index_path, schema)?)
    }
}

fn clear_tantivy_index(metadata_store: &MetadataStore) -> Result<(), SearchError> {
    let index_path = search_index_path(metadata_store)?;
    if index_path.exists() {
        std::fs::remove_dir_all(index_path)?;
    }
    Ok(())
}

async fn rebuild_tantivy_index(metadata_store: &MetadataStore) -> Result<(), SearchError> {
    clear_tantivy_index(metadata_store)?;
    let index = open_or_create_tantivy_index(metadata_store)?;
    let schema = index.schema();
    let fields = tantivy_fields(&schema)?;
    let mut writer = index.writer(30_000_000)?;

    let rows = sqlx::query(
        "
        SELECT path, chunk_id, kind, title, content, start_line, end_line
        FROM search_chunks
        ORDER BY path, start_line
        ",
    )
    .fetch_all(metadata_store.pool())
    .await?;

    for row in rows {
        let path: String = row.try_get("path")?;
        let chunk_id: String = row.try_get("chunk_id")?;
        let kind: String = row.try_get("kind")?;
        let title: String = row.try_get("title")?;
        let content: String = row.try_get("content")?;
        let start_line = row.try_get::<i64, _>("start_line")?;
        let end_line = row.try_get::<i64, _>("end_line")?;

        writer.add_document(doc!(
            fields.path => path,
            fields.chunk_id => chunk_id,
            fields.kind => kind,
            fields.title => title,
            fields.content => content,
            fields.start_line => i64_to_usize(start_line) as u64,
            fields.end_line => i64_to_usize(end_line) as u64,
        ))?;
    }

    writer.commit()?;
    Ok(())
}

fn extract_text_field(document: &tantivy::TantivyDocument, field: Field) -> String {
    document
        .get_first(field)
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string()
}

fn extract_u64_field(document: &tantivy::TantivyDocument, field: Field) -> Option<u64> {
    document.get_first(field).and_then(|value| value.as_u64())
}

#[cfg(test)]
mod tests {
    use super::{hybrid_search, project_overview_chunks, rebuild_search_index, search_text};
    use crate::metadata::MetadataStore;
    use std::fs;

    #[tokio::test]
    async fn indexes_and_searches_code_documents() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        fs::create_dir_all(temp_dir.path().join("src")).expect("src dir should be created");
        fs::write(
            temp_dir.path().join("src/App.tsx"),
            "export function App() { return <main>Local Brain Search</main>; }",
        )
        .expect("source should be written");
        let store = MetadataStore::open(temp_dir.path().join("metadata"))
            .await
            .expect("store should open");

        let summary = rebuild_search_index(temp_dir.path(), &store)
            .await
            .expect("search index should rebuild");
        let text_results = search_text(&store, "Local Brain", 5)
            .await
            .expect("text search should run");
        let hybrid_results = hybrid_search(&store, "local search", 5)
            .await
            .expect("hybrid search should run");

        assert_eq!(summary.documents_indexed, 1);
        assert_eq!(summary.embeddings_indexed, 1);
        assert_eq!(text_results[0].path, "src/App.tsx");
        assert!(text_results[0].chunk_id.is_some());
        assert_eq!(text_results[0].start_line, Some(1));
        assert!(!hybrid_results.is_empty());
        assert!(temp_dir
            .path()
            .join(".localbrain/search/meta.json")
            .exists());
    }

    #[tokio::test]
    async fn project_overview_prefers_project_spine_files() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        fs::create_dir_all(temp_dir.path().join("src-tauri/src/llm"))
            .expect("src-tauri dir should be created");
        fs::create_dir_all(temp_dir.path().join("notes")).expect("notes dir should be created");
        fs::write(
            temp_dir.path().join("package.json"),
            r#"{"name":"localbrain","scripts":{"dev":"vite"}}"#,
        )
        .expect("package should be written");
        fs::write(
            temp_dir.path().join("src-tauri/src/llm/mod.rs"),
            "pub async fn ask_local() { /* local brain search answer generation */ }",
        )
        .expect("llm source should be written");
        fs::write(
            temp_dir.path().join("notes/random.txt"),
            "small unrelated note",
        )
        .expect("note should be written");
        let store = MetadataStore::open(temp_dir.path().join("metadata"))
            .await
            .expect("store should open");

        rebuild_search_index(temp_dir.path(), &store)
            .await
            .expect("search index should rebuild");
        let overview = project_overview_chunks(&store, 4)
            .await
            .expect("overview search should run");

        assert!(overview.iter().any(|result| result.path == "package.json"));
        assert!(overview
            .iter()
            .any(|result| result.path == "src-tauri/src/llm/mod.rs"));
        assert!(overview
            .iter()
            .all(|result| result.snippet.chars().count() <= 1_200));
    }
}
