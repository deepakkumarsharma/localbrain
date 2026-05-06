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

    let project_name = if normalized_root.is_empty() {
        root.file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("workspace"))
            .to_string_lossy()
            .to_string()
    } else {
        normalized_root.clone()
    };

    let mut index_lines = vec![
        "# Local Brain Wiki".to_string(),
        String::new(),
        format!("Generated for `{project_name}`."),
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

pub async fn get_wiki_content(
    path: impl AsRef<Path>,
    metadata_store: &MetadataStore,
) -> Result<Option<String>, WikiError> {
    let requested_path = path.as_ref();
    let root = metadata_store.resolve_path(".")?;
    let output_dir = wiki_output_dir(&root);
    let display_path = metadata_store.normalize_path(requested_path);
    let file_name = wiki_file_name(&display_path);
    let page_path = output_dir.join(file_name);

    if !page_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(page_path)?;
    Ok(Some(content))
}

fn render_file_page(file_path: &str, symbols: &[CodeSymbol]) -> String {
    let components = symbols_of_kind(symbols, SymbolKind::Component);
    let functions = symbols_of_kind(symbols, SymbolKind::Function);
    let methods = symbols_of_kind(symbols, SymbolKind::Method);
    let classes = symbols_of_kind(symbols, SymbolKind::Class);
    let interfaces = symbols_of_kind(symbols, SymbolKind::Interface);
    let type_aliases = symbols_of_kind(symbols, SymbolKind::TypeAlias);
    let enums = symbols_of_kind(symbols, SymbolKind::Enum);
    let imports = symbols_of_kind(symbols, SymbolKind::Import);
    let exports = symbols_of_kind(symbols, SymbolKind::Export);

    let (local_imports, external_imports): (Vec<&CodeSymbol>, Vec<&CodeSymbol>) =
        imports.iter().copied().partition(|symbol| {
            symbol
                .source
                .as_deref()
                .is_some_and(|source| source.starts_with('.') || source.starts_with('/'))
        });

    let hooks: Vec<&CodeSymbol> = functions
        .iter()
        .copied()
        .filter(|symbol| {
            symbol.name.starts_with("use")
                && symbol
                    .name
                    .chars()
                    .nth(3)
                    .is_some_and(|character| character.is_ascii_uppercase())
        })
        .collect();

    let module_profile = infer_module_profile(
        file_path,
        components.len(),
        hooks.len(),
        classes.len(),
        interfaces.len() + type_aliases.len() + enums.len(),
    );

    let mut lines = vec![
        format!("# `{file_path}`"),
        String::new(),
        format!(
            "> Generated from local parser symbols ({} found). This page summarizes structure and likely responsibilities.",
            symbols.len()
        ),
        String::new(),
        "## Quick Summary".to_string(),
        String::new(),
        format!("- Role: **{}**", module_profile),
        format!(
            "- Structure: **{} components**, **{} functions**, **{} methods**, **{} types**, **{} imports**",
            components.len(),
            functions.len(),
            methods.len(),
            interfaces.len() + type_aliases.len() + enums.len() + classes.len(),
            imports.len()
        ),
        String::new(),
        "## Exposed API and Entry Points".to_string(),
        String::new(),
    ];

    if exports.is_empty() && components.is_empty() {
        lines.push(
            "- No explicit exports were detected in parser output for this file.".to_string(),
        );
    } else {
        for symbol in exports.iter().chain(components.iter()).take(12) {
            lines.push(format!(
                "- `{}` ({}) at `L{}:{}`",
                symbol.name,
                human_kind_label(symbol.kind),
                symbol.range.start_line,
                symbol.range.start_column
            ));
        }
    }

    lines.push(String::new());
    lines.push("## Dependencies".to_string());
    lines.push(String::new());
    lines.push("### External Libraries".to_string());
    if external_imports.is_empty() {
        lines.push("- No external package imports detected.".to_string());
    } else {
        for symbol in external_imports.iter().take(20) {
            lines.push(format!(
                "- `{}` from `{}`",
                symbol.name,
                symbol.source.as_deref().unwrap_or("unknown")
            ));
        }
    }

    lines.push(String::new());
    lines.push("### Local Module Links".to_string());
    if local_imports.is_empty() {
        lines.push("- No relative/local imports detected.".to_string());
    } else {
        for symbol in local_imports.iter().take(20) {
            lines.push(format!(
                "- `{}` from `{}`",
                symbol.name,
                symbol.source.as_deref().unwrap_or("unknown")
            ));
        }
    }

    lines.push(String::new());
    lines.push("## Internal Implementation".to_string());
    lines.push(String::new());

    if !hooks.is_empty() {
        lines.push("### Hooks".to_string());
        for hook in hooks.iter().take(12) {
            lines.push(format!(
                "- `{}` at `L{}:{}`",
                hook.name, hook.range.start_line, hook.range.start_column
            ));
        }
        lines.push(String::new());
    }

    let internal_functions: Vec<&CodeSymbol> = functions
        .iter()
        .copied()
        .filter(|symbol| !hooks.iter().any(|hook| hook.name == symbol.name))
        .collect();
    if !internal_functions.is_empty() {
        lines.push("### Functions".to_string());
        for symbol in internal_functions.iter().take(16) {
            lines.push(format!(
                "- `{}` at `L{}:{}`",
                symbol.name, symbol.range.start_line, symbol.range.start_column
            ));
        }
        lines.push(String::new());
    }

    if !methods.is_empty() {
        lines.push("### Methods".to_string());
        for symbol in methods.iter().take(16) {
            lines.push(format!(
                "- `{}` at `L{}:{}`",
                symbol.name, symbol.range.start_line, symbol.range.start_column
            ));
        }
        lines.push(String::new());
    }

    let type_symbols = classes
        .iter()
        .chain(interfaces.iter())
        .chain(type_aliases.iter())
        .chain(enums.iter())
        .copied()
        .collect::<Vec<_>>();
    if !type_symbols.is_empty() {
        lines.push("### Types and Models".to_string());
        for symbol in type_symbols.iter().take(16) {
            lines.push(format!(
                "- `{}` ({}) at `L{}:{}`",
                symbol.name,
                human_kind_label(symbol.kind),
                symbol.range.start_line,
                symbol.range.start_column
            ));
        }
        lines.push(String::new());
    }

    format!("{}\n", lines.join("\n"))
}

fn symbols_of_kind(symbols: &[CodeSymbol], kind: SymbolKind) -> Vec<&CodeSymbol> {
    symbols
        .iter()
        .filter(|symbol| symbol.kind == kind)
        .collect()
}

fn human_kind_label(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Function => "function",
        SymbolKind::Component => "component",
        SymbolKind::Class => "class",
        SymbolKind::Method => "method",
        SymbolKind::Object => "object",
        SymbolKind::Enum => "enum",
        SymbolKind::Interface => "interface",
        SymbolKind::TypeAlias => "type alias",
        SymbolKind::Import => "import",
        SymbolKind::Export => "export",
    }
}

fn infer_module_profile(
    file_path: &str,
    component_count: usize,
    hook_count: usize,
    class_count: usize,
    type_count: usize,
) -> &'static str {
    if component_count > 0 {
        return "UI component module";
    }
    if hook_count > 0 {
        return "React hook module";
    }
    if class_count > 0 {
        return "Object-oriented service/module";
    }
    if type_count > 0 {
        return "Type/model definition module";
    }
    if file_path.ends_with(".json")
        || file_path.ends_with(".toml")
        || file_path.ends_with(".yaml")
        || file_path.ends_with(".yml")
    {
        return "Configuration module";
    }
    "General utility/module"
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
