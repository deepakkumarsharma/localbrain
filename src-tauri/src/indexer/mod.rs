use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use serde::Serialize;
use thiserror::Error;
use walkdir::WalkDir;

use crate::graph::{GraphError, GraphIngestSummary, GraphStore};
use crate::metadata::{FileChangeStatus, FileMetadata, IndexRunSummary, MetadataStore};
use crate::parser::{language_from_extension, parse_file_with_display_path, ParserError};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IndexFileSummary {
    pub path: String,
    pub status: FileChangeStatus,
    pub skipped: bool,
    pub metadata: Option<FileMetadata>,
    pub graph: Option<GraphIngestSummary>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IndexPathSummary {
    pub path: String,
    pub files_seen: usize,
    pub files_changed: usize,
    pub files_skipped: usize,
    pub files_deleted: usize,
    pub errors: Vec<String>,
    pub run: Option<IndexRunSummary>,
    pub files: Vec<IndexFileSummary>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IndexProgress {
    pub phase: String,
    pub files_seen: usize,
    pub files_total: usize,
    pub files_changed: usize,
    pub files_skipped: usize,
    pub files_deleted: usize,
    pub errors: usize,
    pub current_path: Option<String>,
}

#[derive(Debug, Error)]
pub enum IndexerError {
    #[error("metadata error: {0}")]
    Metadata(#[from] crate::metadata::MetadataError),
    #[error("parser error: {0}")]
    Parser(#[from] ParserError),
    #[error("graph error: {0}")]
    Graph(#[from] GraphError),
    #[error("path is not a supported source file: {0}")]
    UnsupportedPath(String),
    #[error("failed to walk path: {0}")]
    Walk(#[from] walkdir::Error),
}

pub async fn index_file(
    path: impl AsRef<Path>,
    metadata_store: &MetadataStore,
    graph_store: &GraphStore,
) -> Result<IndexFileSummary, IndexerError> {
    let requested_path = path.as_ref();
    let normalized_path = metadata_store.normalize_path(requested_path);
    let supports_graph_parse = is_graph_source_file(requested_path);

    let status = metadata_store.classify_file(requested_path).await?;

    if !supports_graph_parse {
        return match status {
            FileChangeStatus::Unchanged => {
                let metadata = metadata_store.get_file(&normalized_path).await?;
                Ok(IndexFileSummary {
                    path: normalized_path,
                    status: FileChangeStatus::Unchanged,
                    skipped: true,
                    metadata,
                    graph: None,
                })
            }
            FileChangeStatus::Deleted => {
                graph_store.clear_file(&normalized_path)?;
                metadata_store.mark_file_deleted(&normalized_path).await?;
                Ok(IndexFileSummary {
                    path: normalized_path.clone(),
                    status: FileChangeStatus::Deleted,
                    skipped: false,
                    metadata: metadata_store.get_file(&normalized_path).await?,
                    graph: None,
                })
            }
            FileChangeStatus::New | FileChangeStatus::Changed => {
                let metadata = metadata_store.mark_file_indexed(requested_path).await?;
                Ok(IndexFileSummary {
                    path: normalized_path,
                    status,
                    skipped: true,
                    metadata: Some(metadata),
                    graph: None,
                })
            }
            FileChangeStatus::Error => Err(IndexerError::UnsupportedPath(normalized_path)),
        };
    }

    match status {
        FileChangeStatus::Unchanged => {
            let metadata = metadata_store.get_file(&normalized_path).await?;
            if graph_store
                .get_symbols_for_file(&normalized_path)?
                .is_empty()
            {
                let source_path = metadata_store.resolve_path(requested_path)?;
                let parsed = parse_file_with_display_path(source_path, &normalized_path)?;
                let graph_summary = upsert_parsed_file_with_retries(graph_store, &parsed).await?;
                let metadata = metadata_store.mark_file_indexed(requested_path).await?;

                return Ok(IndexFileSummary {
                    path: parsed.path,
                    status: FileChangeStatus::Unchanged,
                    skipped: false,
                    metadata: Some(metadata),
                    graph: Some(graph_summary),
                });
            }

            Ok(IndexFileSummary {
                path: normalized_path,
                status: FileChangeStatus::Unchanged,
                skipped: true,
                metadata,
                graph: None,
            })
        }
        FileChangeStatus::Deleted => {
            graph_store.clear_file(&normalized_path)?;
            metadata_store.mark_file_deleted(&normalized_path).await?;
            Ok(IndexFileSummary {
                path: normalized_path.clone(),
                status: FileChangeStatus::Deleted,
                skipped: false,
                metadata: metadata_store.get_file(&normalized_path).await?,
                graph: None,
            })
        }
        FileChangeStatus::New | FileChangeStatus::Changed => {
            let source_path = metadata_store.resolve_path(requested_path)?;
            let parsed = parse_file_with_display_path(source_path, &normalized_path)?;
            let graph_summary = upsert_parsed_file_with_retries(graph_store, &parsed).await?;
            let metadata = metadata_store.mark_file_indexed(requested_path).await?;
            Ok(IndexFileSummary {
                path: parsed.path,
                status,
                skipped: false,
                metadata: Some(metadata),
                graph: Some(graph_summary),
            })
        }
        FileChangeStatus::Error => Err(IndexerError::UnsupportedPath(normalized_path)),
    }
}

async fn upsert_parsed_file_with_retries(
    graph_store: &GraphStore,
    parsed: &crate::parser::ParsedFile,
) -> Result<GraphIngestSummary, IndexerError> {
    let mut last_error = None;

    for i in 0..3 {
        match graph_store.upsert_parsed_file(parsed) {
            Ok(summary) => return Ok(summary),
            Err(e) => {
                last_error = Some(IndexerError::Graph(e));
                if i < 2 {
                    tokio::time::sleep(std::time::Duration::from_millis(50 * (i + 1))).await;
                }
            }
        }
    }

    Err(last_error.expect("retry loop should capture the final graph error"))
}

pub async fn index_path(
    path: impl AsRef<Path>,
    metadata_store: &MetadataStore,
    graph_store: &GraphStore,
) -> Result<IndexPathSummary, IndexerError> {
    index_path_with_progress(path, metadata_store, graph_store, |_| {}).await
}

pub async fn index_path_with_progress(
    path: impl AsRef<Path>,
    metadata_store: &MetadataStore,
    graph_store: &GraphStore,
    mut on_progress: impl FnMut(IndexProgress),
) -> Result<IndexPathSummary, IndexerError> {
    let requested_path = path.as_ref();
    let paths = indexable_paths(requested_path, metadata_store)?;
    let run_id = metadata_store.begin_index_run().await?;

    let normalized_root = metadata_store.normalize_path(requested_path);
    let mut summary = IndexPathSummary {
        path: normalized_root.clone(),
        files_seen: 0,
        files_changed: 0,
        files_skipped: 0,
        files_deleted: 0,
        errors: Vec::new(),
        run: None,
        files: Vec::new(),
    };
    let files_total = paths.len();

    on_progress(IndexProgress {
        phase: "discovered".to_string(),
        files_seen: 0,
        files_total,
        files_changed: 0,
        files_skipped: 0,
        files_deleted: 0,
        errors: 0,
        current_path: None,
    });

    // Reconciliation phase: identify and mark deleted files
    if let Ok(previously_tracked) = metadata_store.get_tracked_files(&normalized_root).await {
        let discovered_set: HashSet<String> = paths
            .iter()
            .map(|p| metadata_store.normalize_path(p))
            .collect();

        for path_str in previously_tracked {
            if !discovered_set.contains(&path_str) {
                summary.files_deleted += 1;
                summary.files_changed += 1;

                if let Err(e) = graph_store.clear_file(&path_str) {
                    summary
                        .errors
                        .push(format!("Failed to clear graph for {}: {}", path_str, e));
                }

                match metadata_store.get_file(&path_str).await {
                    Ok(Some(metadata)) => {
                        if let Err(e) = metadata_store.mark_file_deleted(&path_str).await {
                            summary.errors.push(format!(
                                "Failed to mark metadata deleted for {}: {}",
                                path_str, e
                            ));
                        }

                        summary.files.push(IndexFileSummary {
                            path: path_str,
                            status: FileChangeStatus::Deleted,
                            skipped: false,
                            metadata: Some(metadata),
                            graph: None,
                        });
                    }
                    _ => {
                        if let Err(e) = metadata_store.mark_file_deleted(&path_str).await {
                            summary.errors.push(format!(
                                "Failed to mark metadata deleted for {}: {}",
                                path_str, e
                            ));
                        }
                    }
                }
            }
        }
    }

    for path in paths {
        let current_path = metadata_store.normalize_path(&path);
        summary.files_seen += 1;

        match index_file(&path, metadata_store, graph_store).await {
            Ok(file_summary) => {
                if file_summary.skipped {
                    summary.files_skipped += 1;
                } else if file_summary.status == FileChangeStatus::Deleted {
                    summary.files_deleted += 1;
                    summary.files_changed += 1;
                } else {
                    summary.files_changed += 1;
                }

                summary.files.push(file_summary);
            }
            Err(error) => {
                summary
                    .errors
                    .push(format!("{}: {}", path.display(), error));
            }
        }

        on_progress(IndexProgress {
            phase: "indexing".to_string(),
            files_seen: summary.files_seen,
            files_total,
            files_changed: summary.files_changed,
            files_skipped: summary.files_skipped,
            files_deleted: summary.files_deleted,
            errors: summary.errors.len(),
            current_path: Some(current_path),
        });
    }

    let status = if summary.errors.is_empty() {
        "complete"
    } else {
        "error"
    };
    metadata_store
        .finish_index_run(
            run_id,
            usize_to_i64(summary.files_seen),
            usize_to_i64(summary.files_changed),
            status,
        )
        .await?;
    summary.run = metadata_store.latest_index_run().await?;

    on_progress(IndexProgress {
        phase: "complete".to_string(),
        files_seen: summary.files_seen,
        files_total,
        files_changed: summary.files_changed,
        files_skipped: summary.files_skipped,
        files_deleted: summary.files_deleted,
        errors: summary.errors.len(),
        current_path: None,
    });

    Ok(summary)
}

pub async fn get_index_status(
    metadata_store: &MetadataStore,
) -> Result<Option<IndexRunSummary>, IndexerError> {
    Ok(metadata_store.latest_index_run().await?)
}

fn indexable_paths(
    path: &Path,
    metadata_store: &MetadataStore,
) -> Result<Vec<PathBuf>, IndexerError> {
    let source_path = metadata_store.resolve_path(path)?;

    if source_path.is_file() {
        return Ok(if is_indexable_file(&source_path) {
            vec![source_path]
        } else {
            Vec::new()
        });
    }

    let mut paths = Vec::new();
    for entry in WalkDir::new(&source_path)
        .into_iter()
        .filter_entry(|entry| {
            let path = entry.path();
            let relative_path = path.strip_prefix(&source_path).unwrap_or(path);
            path == source_path || !is_ignored_path(relative_path)
        })
    {
        let entry = entry?;
        let path = entry.path();
        let relative_path = path.strip_prefix(&source_path).unwrap_or(path);

        if path.is_file() && is_indexable_file(path) && !is_ignored_path(relative_path) {
            paths.push(path.to_path_buf());
        }
    }

    paths.sort();
    Ok(paths)
}

fn is_graph_source_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| language_from_extension(extension).is_some())
}

fn is_indexable_file(path: &Path) -> bool {
    let has_supported_extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension,
                "js" | "mjs"
                    | "cjs"
                    | "jsx"
                    | "ts"
                    | "mts"
                    | "cts"
                    | "tsx"
                    | "rs"
                    | "go"
                    | "py"
                    | "java"
                    | "kt"
                    | "kts"
                    | "swift"
                    | "rb"
                    | "php"
                    | "c"
                    | "h"
                    | "cpp"
                    | "hpp"
                    | "cs"
                    | "sh"
                    | "bash"
                    | "zsh"
                    | "fish"
                    | "sql"
                    | "json"
                    | "jsonc"
                    | "yaml"
                    | "yml"
                    | "toml"
                    | "ini"
                    | "cfg"
                    | "conf"
                    | "xml"
                    | "css"
                    | "scss"
                    | "less"
                    | "vue"
                    | "svelte"
                    | "astro"
                    | "md"
                    | "txt"
            )
        });

    has_supported_extension || is_extensionless_indexable_file(path)
}

fn is_extensionless_indexable_file(path: &Path) -> bool {
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
    if is_generated_wiki_path(path) {
        return true;
    }

    path.components().any(|component| {
        let Component::Normal(value) = component else {
            return false;
        };
        let value = value.to_string_lossy();

        matches!(
            value.as_ref(),
            ".git"
                | ".localbrain"
                | "node_modules"
                | "vendor"
                | ".venv"
                | "__pycache__"
                | "dist"
                | "build"
                | ".next"
                | "target"
                | ".ssh"
                | ".aws"
                | "__snapshots__"
        ) || (value.starts_with('.') && !matches!(value.as_ref(), ".github" | ".vscode"))
    }) || path.file_name().is_some_and(|file_name| {
        let value = file_name.to_string_lossy();
        matches!(value.as_ref(), ".DS_Store" | "Thumbs.db" | ".npmrc")
            || value.starts_with(".env")
            || value.ends_with("_snapshot.json")
            || value.ends_with(".snap")
            || value.ends_with(".key")
            || value.ends_with(".pem")
            || value.ends_with(".p12")
            || value.ends_with(".o")
            || value.ends_with(".so")
    })
}

fn is_generated_wiki_path(path: &Path) -> bool {
    let parts = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>();

    parts
        .windows(2)
        .any(|window| window[0] == "docs" && window[1] == "wiki")
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{index_file, index_path, is_graph_source_file, is_ignored_path, is_indexable_file};
    use crate::graph::GraphStore;
    use crate::metadata::{FileChangeStatus, MetadataStore};
    use std::fs;
    use std::path::Path;

    #[tokio::test]
    async fn indexes_changed_file_and_skips_unchanged_file() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let source_path = temp_dir.path().join("App.tsx");
        fs::write(&source_path, "export function App() { return null; }")
            .expect("source file should be written");
        let metadata_store = MetadataStore::open(temp_dir.path().join("metadata"))
            .await
            .expect("metadata store should open");
        metadata_store
            .set_workspace_root(temp_dir.path())
            .expect("workspace root should be set");
        let graph_store =
            GraphStore::open(temp_dir.path().join("graph")).expect("graph store should open");

        let first = index_file(&source_path, &metadata_store, &graph_store)
            .await
            .expect("file should index");
        let second = index_file(&source_path, &metadata_store, &graph_store)
            .await
            .expect("file should skip");

        assert!(!first.skipped);
        assert!(second.skipped);
    }

    #[tokio::test]
    async fn indexes_supported_files_in_directory() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        fs::create_dir_all(temp_dir.path().join("src")).expect("src dir should be created");
        fs::create_dir_all(temp_dir.path().join("node_modules/pkg"))
            .expect("ignored dir should be created");
        fs::write(
            temp_dir.path().join("src/App.tsx"),
            "export function App() { return null; }",
        )
        .expect("source file should be written");
        fs::write(
            temp_dir.path().join("node_modules/pkg/index.ts"),
            "export const ignored = true;",
        )
        .expect("ignored file should be written");
        let metadata_store = MetadataStore::open(temp_dir.path().join("metadata"))
            .await
            .expect("metadata store should open");
        metadata_store
            .set_workspace_root(temp_dir.path())
            .expect("workspace root should be set");
        let graph_store =
            GraphStore::open(temp_dir.path().join("graph")).expect("graph store should open");

        let summary = index_path(temp_dir.path(), &metadata_store, &graph_store)
            .await
            .expect("path should index");

        assert!(summary.files_seen >= 1);
        assert!(summary
            .files
            .iter()
            .any(|file| file.path.ends_with("src/App.tsx")));
        assert!(summary
            .files
            .iter()
            .all(|file| !file.path.contains("node_modules")));
        assert!(summary.errors.is_empty());
    }

    #[tokio::test]
    async fn indexes_supported_graph_files_from_directory() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        fs::write(temp_dir.path().join("tsconfig.json"), "{}").expect("json should be written");
        fs::write(
            temp_dir.path().join("theme.go"),
            "package theme\n\nfunc RenderTheme() {}\n",
        )
        .expect("go file should be written");
        fs::write(temp_dir.path().join("README.md"), "# Hello")
            .expect("markdown should be written");

        let metadata_store = MetadataStore::open(temp_dir.path().join("metadata"))
            .await
            .expect("metadata store should open");
        let graph_store =
            GraphStore::open(temp_dir.path().join("graph")).expect("graph store should open");

        let summary = index_path(temp_dir.path(), &metadata_store, &graph_store)
            .await
            .expect("path should index");

        assert!(summary
            .files
            .iter()
            .any(|file| file.path.ends_with("tsconfig.json")));
        let go_file = summary
            .files
            .iter()
            .find(|file| file.path.ends_with("theme.go"))
            .expect("go file should be indexed");
        assert!(!go_file.skipped);
        assert_eq!(
            go_file.graph.as_ref().map(|graph| graph.symbol_count),
            Some(1)
        );
        assert!(summary
            .files
            .iter()
            .any(|file| file.path.ends_with("README.md")));
        assert!(summary.errors.is_empty());
    }

    #[tokio::test]
    async fn reports_indexing_progress_for_each_file() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        fs::create_dir_all(temp_dir.path().join("src")).expect("src dir should be created");
        fs::write(
            temp_dir.path().join("src/App.tsx"),
            "export function App() { return null; }",
        )
        .expect("tsx file should be written");
        fs::write(temp_dir.path().join("README.md"), "# Hello")
            .expect("markdown should be written");
        let metadata_store = MetadataStore::open(temp_dir.path().join("metadata"))
            .await
            .expect("metadata store should open");
        metadata_store
            .set_workspace_root(temp_dir.path())
            .expect("workspace root should be set");
        let graph_store =
            GraphStore::open(temp_dir.path().join("graph")).expect("graph store should open");
        let mut progress = Vec::new();

        let summary = super::index_path_with_progress(
            temp_dir.path(),
            &metadata_store,
            &graph_store,
            |event| {
                progress.push(event);
            },
        )
        .await
        .expect("path should index");

        assert_eq!(summary.files_seen, 2);
        assert!(progress.iter().any(|event| event.phase == "discovered"));
        assert!(progress.iter().any(|event| event.phase == "complete"));
        assert_eq!(
            progress
                .iter()
                .filter(|event| event.phase == "indexing")
                .count(),
            summary.files_seen
        );
        assert!(progress
            .iter()
            .any(|event| event.current_path.as_deref() == Some("src/App.tsx")));
    }

    #[tokio::test]
    async fn backfills_missing_graph_for_unchanged_supported_files() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let source_path = temp_dir.path().join("page_index.py");
        fs::write(
            &source_path,
            "import os\n\nclass PageIndex:\n    pass\n\ndef check_title():\n    return True\n",
        )
        .expect("python file should be written");

        let metadata_store = MetadataStore::open(temp_dir.path().join("metadata"))
            .await
            .expect("metadata store should open");
        metadata_store
            .set_workspace_root(temp_dir.path())
            .expect("workspace root should be set");
        let graph_store =
            GraphStore::open(temp_dir.path().join("graph")).expect("graph store should open");

        metadata_store
            .mark_file_indexed("page_index.py")
            .await
            .expect("metadata should be pre-marked as indexed");

        let summary = index_path(temp_dir.path(), &metadata_store, &graph_store)
            .await
            .expect("path should index");
        let file = summary
            .files
            .iter()
            .find(|file| file.path.ends_with("page_index.py"))
            .expect("python file should be present");

        assert_eq!(file.status, FileChangeStatus::Unchanged);
        assert!(!file.skipped);
        assert_eq!(file.graph.as_ref().map(|graph| graph.symbol_count), Some(3));
        assert!(graph_store
            .get_symbols_for_file(&file.path)
            .expect("symbols should be readable")
            .iter()
            .any(|symbol| symbol.name == "check_title"));
    }

    #[test]
    fn filters_supported_and_ignored_paths() {
        assert!(is_graph_source_file(Path::new("src/App.tsx")));
        assert!(is_graph_source_file(Path::new("src/main.rs")));
        assert!(is_indexable_file(Path::new("src/main.rs")));
        assert!(is_indexable_file(Path::new("README.md")));
        assert!(is_indexable_file(Path::new("Dockerfile")));
        assert!(!is_indexable_file(Path::new("docs/design.pdf")));
        assert!(is_ignored_path(Path::new("node_modules/pkg/index.ts")));
        assert!(is_ignored_path(Path::new(".localbrain/metadata.db")));
        assert!(is_ignored_path(Path::new("docs/wiki/index.md")));
        assert!(is_ignored_path(Path::new(
            "/workspace/project/docs/wiki/auth.md"
        )));
        assert!(is_ignored_path(Path::new(
            "packages/db/src/migrations/meta/0040_snapshot.json"
        )));
        assert!(is_ignored_path(Path::new(
            "src/components/__snapshots__/App.test.tsx.snap"
        )));
        assert!(!is_ignored_path(Path::new("docs/architecture/auth.md")));
        assert!(!is_ignored_path(Path::new("src/snapshots/example.ts")));
        assert!(!is_ignored_path(Path::new("src/App.tsx")));
    }
}
