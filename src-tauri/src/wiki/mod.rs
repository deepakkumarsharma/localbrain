use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::graph::{GraphError, GraphStore};
use crate::indexer::{index_path, IndexerError};
use crate::metadata::{MetadataError, MetadataStore};
use crate::parser::{CodeSymbol, SymbolKind};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WikiSummary {
    pub root: String,
    pub output_dir: String,
    pub pages_written: usize,
    pub index_path: String,
    pub errors: Vec<String>,
}

#[derive(Debug, Error)]
pub enum WikiError {
    #[error("metadata error: {0}")]
    Metadata(#[from] MetadataError),
    #[error("indexer error: {0}")]
    Indexer(#[from] IndexerError),
    #[error("graph error: {0}")]
    Graph(#[from] GraphError),
    #[error("filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
}

pub async fn generate_wiki(
    path: impl AsRef<Path>,
    metadata_store: &MetadataStore,
    graph_store: &GraphStore,
) -> Result<WikiSummary, WikiError> {
    let requested_path = path.as_ref();
    let root = metadata_store.resolve_path(requested_path)?;
    let normalized_root = metadata_store.normalize_path(requested_path);
    let output_dir = wiki_output_dir(&root);
    fs::create_dir_all(&output_dir)?;

    let index_summary = index_path(requested_path, metadata_store, graph_store).await?;
    let mut pages = BTreeMap::new();
    let mut errors = index_summary.errors;

    for file in index_summary.files {
        if file.status.as_str() == "deleted" {
            continue;
        }

        match graph_store.get_symbols_for_file(&file.path) {
            Ok(symbols) => {
                if symbols.is_empty() {
                    continue;
                }

                pages.insert(file.path.clone(), symbols);
            }
            Err(error) => errors.push(format!("{}: {}", file.path, error)),
        }
    }

    let mut index_lines = vec![
        "# Local Brain Wiki".to_string(),
        String::new(),
        format!("Generated for `{normalized_root}`."),
        String::new(),
        "## Files".to_string(),
    ];
    let mut pages_written = 0;

    for (file_path, symbols) in pages {
        let file_name = wiki_file_name(&file_path);
        let page_path = output_dir.join(&file_name);
        let markdown = render_file_page(&file_path, &symbols);
        fs::write(&page_path, markdown)?;
        pages_written += 1;
        index_lines.push(format!("- [{}]({})", file_path, file_name));
    }

    let index_path = output_dir.join("index.md");
    fs::write(&index_path, format!("{}\n", index_lines.join("\n")))?;

    Ok(WikiSummary {
        root: normalized_root,
        output_dir: output_dir.display().to_string(),
        pages_written,
        index_path: index_path.display().to_string(),
        errors,
    })
}

fn render_file_page(file_path: &str, symbols: &[CodeSymbol]) -> String {
    let mut lines = vec![
        format!("# `{file_path}`"),
        String::new(),
        format!("{} symbols indexed.", symbols.len()),
        String::new(),
    ];

    for kind in [
        SymbolKind::Component,
        SymbolKind::Function,
        SymbolKind::Class,
        SymbolKind::Method,
        SymbolKind::Interface,
        SymbolKind::TypeAlias,
        SymbolKind::Enum,
        SymbolKind::Object,
        SymbolKind::Import,
        SymbolKind::Export,
    ] {
        let grouped: Vec<_> = symbols
            .iter()
            .filter(|symbol| symbol.kind == kind)
            .collect();
        if grouped.is_empty() {
            continue;
        }

        lines.push(format!("## {}", kind_label(kind)));
        lines.push(String::new());

        for symbol in grouped {
            let mut detail = format!(
                "- `{}` at L{}:{}",
                symbol.name, symbol.range.start_line, symbol.range.start_column
            );
            if let Some(parent) = &symbol.parent {
                detail.push_str(&format!(" parent `{parent}`"));
            }
            if let Some(source) = &symbol.source {
                detail.push_str(&format!(" from `{source}`"));
            }
            lines.push(detail);
        }

        lines.push(String::new());
    }

    format!("{}\n", lines.join("\n"))
}

fn wiki_output_dir(root: &Path) -> PathBuf {
    if root.is_file() {
        return root
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("docs")
            .join("wiki");
    }

    root.join("docs").join("wiki")
}

fn wiki_file_name(path: &str) -> String {
    let sanitized = path
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    let digest = Sha256::digest(path.as_bytes());
    let hash = format!("{:x}", digest);
    format!("{sanitized}_{}.md", &hash[..8])
}

fn kind_label(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Function => "Functions",
        SymbolKind::Component => "Components",
        SymbolKind::Class => "Classes",
        SymbolKind::Method => "Methods",
        SymbolKind::Object => "Objects",
        SymbolKind::Enum => "Enums",
        SymbolKind::Interface => "Interfaces",
        SymbolKind::TypeAlias => "Type Aliases",
        SymbolKind::Import => "Imports",
        SymbolKind::Export => "Exports",
    }
}

#[cfg(test)]
mod tests {
    use super::{generate_wiki, wiki_file_name};
    use crate::graph::GraphStore;
    use crate::metadata::MetadataStore;
    use std::fs;

    #[tokio::test]
    async fn generates_markdown_wiki_pages() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        fs::create_dir_all(temp_dir.path().join("src")).expect("src dir should be created");
        fs::write(
            temp_dir.path().join("src/App.tsx"),
            "export function App() { return null; }",
        )
        .expect("source should be written");
        let metadata_store = MetadataStore::open(temp_dir.path().join("metadata"))
            .await
            .expect("metadata store should open");
        let graph_store =
            GraphStore::open(temp_dir.path().join("graph")).expect("graph store should open");

        let summary = generate_wiki(temp_dir.path(), &metadata_store, &graph_store)
            .await
            .expect("wiki should generate");

        assert_eq!(summary.pages_written, 1);
        assert!(temp_dir.path().join("docs/wiki/index.md").exists());
    }

    #[test]
    fn wiki_file_names_are_readable_and_collision_resistant() {
        let slash_path = wiki_file_name("src/foo.ts");
        let underscore_path = wiki_file_name("src_foo.ts");

        assert_ne!(slash_path, underscore_path);
        assert!(slash_path.starts_with("src_foo.ts_"));
        assert!(slash_path.ends_with(".md"));
    }
}
