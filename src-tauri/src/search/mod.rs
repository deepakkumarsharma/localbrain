use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sqlx::Row;
use thiserror::Error;
use walkdir::WalkDir;

use crate::embeddings::{cosine_similarity, embed_text, vector_magnitude, EmbeddingSummary};
use crate::metadata::{current_timestamp, MetadataError, MetadataStore};

const DEFAULT_SEARCH_SCAN_LIMIT: usize = 2_000;
const MAX_SEARCH_SCAN_LIMIT: usize = 10_000;
const SEARCH_SCAN_MULTIPLIER: usize = 50;

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
    pub kind: String,
    pub title: String,
    pub snippet: String,
    pub text_score: f32,
    pub vector_score: f32,
    pub score: f32,
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

    Ok(summary)
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
    let vector = embed_text(&format!("{title}\n{content}"));

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
    let rows = sqlx::query(
        "
        SELECT path, kind, title, content
        FROM search_documents
        ORDER BY updated_at DESC
        LIMIT ?
        ",
    )
    .bind(scan_limit_for(limit))
    .fetch_all(metadata_store.pool())
    .await?;

    let mut results = Vec::new();
    let query_terms = query_terms(query);

    for row in rows {
        let path: String = row.try_get("path")?;
        let kind: String = row.try_get("kind")?;
        let title: String = row.try_get("title")?;
        let content: String = row.try_get("content")?;
        let haystack = format!("{title}\n{content}").to_lowercase();
        let text_score = score_text(&haystack, &query_terms);

        if text_score > 0.0 {
            results.push(SearchResult {
                path,
                kind,
                title,
                snippet: snippet(&content, &query_terms),
                text_score,
                vector_score: 0.0,
                score: text_score,
            });
        }
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
        SELECT d.path, d.kind, d.title, d.content, e.vector_json
        FROM search_documents d
        LEFT JOIN embeddings e ON e.path = d.path
        ORDER BY d.updated_at DESC
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
        let kind: String = row.try_get("kind")?;
        let title: String = row.try_get("title")?;
        let content: String = row.try_get("content")?;
        let vector_json: Option<String> = row.try_get("vector_json")?;
        let haystack = format!("{title}\n{content}").to_lowercase();
        let text_score = score_text(&haystack, &query_terms);
        let vector_score = match vector_json {
            Some(vector_json) => {
                let vector: Vec<f32> = serde_json::from_str(&vector_json)
                    .map_err(|_| SearchError::InvalidVector { path: path.clone() })?;
                cosine_similarity(&query_vector, &vector).max(0.0)
            }
            None => 0.0,
        };
        let score = (text_score * 0.65) + (vector_score * 0.35);

        if score > 0.0 {
            results.push(SearchResult {
                path,
                kind,
                title,
                snippet: snippet(&content, &query_terms),
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
        kind,
        title,
        snippet: snippet(&content, &terms),
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

    sqlx::query(
        "
        INSERT INTO embeddings (path, dimensions, vector_json, updated_at)
        VALUES (?, ?, ?, ?)
        ON CONFLICT(path) DO UPDATE SET
          dimensions = excluded.dimensions,
          vector_json = excluded.vector_json,
          updated_at = excluded.updated_at
        ",
    )
    .bind(path)
    .bind(i64::try_from(vector.len()).unwrap_or(i64::MAX))
    .bind(vector_json)
    .bind(updated_at)
    .execute(metadata_store.pool())
    .await?;

    Ok(())
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
    path.extension()
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
        })
}

fn is_ignored_path(path: &Path) -> bool {
    path.components().any(|component| {
        let value = component.as_os_str().to_string_lossy();
        matches!(
            value.as_ref(),
            ".git" | ".localbrain" | "node_modules" | "vendor" | "dist" | "build" | "target"
        ) || (value.starts_with('.') && !matches!(value.as_ref(), ".github" | ".vscode"))
    }) || path.file_name().is_some_and(|file_name| {
        let value = file_name.to_string_lossy();
        value.starts_with(".env")
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

fn sort_by_score(left: &SearchResult, right: &SearchResult) -> Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.path.cmp(&right.path))
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

#[cfg(test)]
mod tests {
    use super::{hybrid_search, rebuild_search_index, search_text};
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
        assert!(!hybrid_results.is_empty());
    }
}
