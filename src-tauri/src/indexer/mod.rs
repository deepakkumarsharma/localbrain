use std::path::{Component, Path, PathBuf};

use serde::Serialize;
use thiserror::Error;
use walkdir::WalkDir;

use crate::graph::{GraphError, GraphIngestSummary, GraphStore};
use crate::metadata::{FileChangeStatus, FileMetadata, IndexRunSummary, MetadataStore};
use crate::parser::{parse_file, ParserError};

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

    if !is_supported_source_file(requested_path) {
        return Err(IndexerError::UnsupportedPath(normalized_path));
    }

    let status = metadata_store.classify_file(requested_path).await?;

    match status {
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
            let parsed = parse_file(requested_path)?;

            // Retry logic for KuzuDB contention
            let mut last_error = None;
            let mut graph = None;

            for i in 0..3 {
                match graph_store.upsert_parsed_file(&parsed) {
                    Ok(summary) => {
                        graph = Some(summary);
                        break;
                    }
                    Err(e) => {
                        last_error = Some(IndexerError::Graph(e));
                        if i < 2 {
                            tokio::time::sleep(std::time::Duration::from_millis(50 * (i + 1)))
                                .await;
                        }
                    }
                }
            }

            if let Some(graph_summary) = graph {
                let metadata = metadata_store.mark_file_indexed(requested_path).await?;
                Ok(IndexFileSummary {
                    path: parsed.path,
                    status,
                    skipped: false,
                    metadata: Some(metadata),
                    graph: Some(graph_summary),
                })
            } else {
                Err(last_error.unwrap())
            }
        }
        FileChangeStatus::Error => Err(IndexerError::UnsupportedPath(normalized_path)),
    }
}

pub async fn index_path(
    path: impl AsRef<Path>,
    metadata_store: &MetadataStore,
    graph_store: &GraphStore,
) -> Result<IndexPathSummary, IndexerError> {
    let requested_path = path.as_ref();
    let run_id = metadata_store.begin_index_run().await?;
    let mut summary = IndexPathSummary {
        path: metadata_store.normalize_path(requested_path),
        files_seen: 0,
        files_changed: 0,
        files_skipped: 0,
        files_deleted: 0,
        errors: Vec::new(),
        run: None,
        files: Vec::new(),
    };

    let paths = indexable_paths(requested_path, metadata_store)?;

    for path in paths {
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
    let source_path = metadata_store.resolve_path(path);

    if source_path.is_file() {
        return Ok(if is_supported_source_file(&source_path) {
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

        if path.is_file() && is_supported_source_file(path) && !is_ignored_path(relative_path) {
            paths.push(path.to_path_buf());
        }
    }

    paths.sort();
    Ok(paths)
}

fn is_supported_source_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension, "js" | "jsx" | "ts" | "tsx"))
}

fn is_ignored_path(path: &Path) -> bool {
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
        ) || (value.starts_with('.') && !matches!(value.as_ref(), ".github" | ".vscode"))
    }) || path.file_name().is_some_and(|file_name| {
        let value = file_name.to_string_lossy();
        matches!(value.as_ref(), ".DS_Store" | "Thumbs.db" | ".npmrc")
            || value.starts_with(".env")
            || value.ends_with(".key")
            || value.ends_with(".pem")
            || value.ends_with(".p12")
            || value.ends_with(".o")
            || value.ends_with(".so")
    })
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{index_file, index_path, is_ignored_path, is_supported_source_file};
    use crate::graph::GraphStore;
    use crate::metadata::MetadataStore;
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
        let graph_store =
            GraphStore::open(temp_dir.path().join("graph")).expect("graph store should open");

        let summary = index_path(temp_dir.path(), &metadata_store, &graph_store)
            .await
            .expect("path should index");

        assert_eq!(summary.files_seen, 1);
        assert_eq!(summary.files_changed, 1);
        assert!(summary.errors.is_empty());
    }

    #[test]
    fn filters_supported_and_ignored_paths() {
        assert!(is_supported_source_file(Path::new("src/App.tsx")));
        assert!(!is_supported_source_file(Path::new("src/main.rs")));
        assert!(is_ignored_path(Path::new("node_modules/pkg/index.ts")));
        assert!(is_ignored_path(Path::new(".localbrain/metadata.db")));
        assert!(!is_ignored_path(Path::new("src/App.tsx")));
    }
}
