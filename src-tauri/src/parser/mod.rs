mod generic;
mod python;
mod types;
mod typescript;

use std::fs;
#[cfg(test)]
use std::path::Component;
use std::path::Path;
use thiserror::Error;

pub use types::{CodeSymbol, ParsedFile, SourceLanguage, SourceRange, SymbolKind};

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("unsupported source file extension: {0}")]
    UnsupportedExtension(String),
    #[error("failed to read source file: {0}")]
    ReadFile(#[from] std::io::Error),
    #[error("tree-sitter failed to parse source")]
    ParseFailed,
    #[error("tree-sitter language error: {0}")]
    Language(#[from] tree_sitter::LanguageError),
}

#[cfg(test)]
pub fn parse_file(path: impl AsRef<Path>) -> Result<ParsedFile, ParserError> {
    let requested_path = path.as_ref();
    let source_path = resolve_source_path(requested_path);
    let language = language_from_path(requested_path)?;
    let display_path = normalize_display_path(requested_path);

    parse_source_file(&source_path, &display_path, language)
}

pub fn parse_file_with_display_path(
    source_path: impl AsRef<Path>,
    display_path: &str,
) -> Result<ParsedFile, ParserError> {
    let source_path = source_path.as_ref();
    let language = language_from_path(source_path)?;

    parse_source_file(source_path, display_path, language)
}

fn parse_source_file(
    source_path: &Path,
    display_path: &str,
    language: SourceLanguage,
) -> Result<ParsedFile, ParserError> {
    let source = fs::read_to_string(source_path)?;

    match language {
        SourceLanguage::JavaScript
        | SourceLanguage::TypeScript
        | SourceLanguage::Tsx
        | SourceLanguage::Jsx => typescript::parse_source(display_path, language, &source),
        SourceLanguage::Python => python::parse_source(display_path, language, &source),
        _ => Ok(generic::parse_source(display_path, language, &source)),
    }
}

#[cfg(test)]
fn resolve_source_path(path: &Path) -> std::path::PathBuf {
    if path.exists() || path.is_absolute() {
        return path.to_path_buf();
    }

    let parent_candidate = Path::new("..").join(path);
    if parent_candidate.exists() {
        return parent_candidate;
    }

    path.to_path_buf()
}

fn language_from_path(path: &Path) -> Result<SourceLanguage, ParserError> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some(extension) => language_from_extension(extension)
            .ok_or_else(|| ParserError::UnsupportedExtension(extension.to_string())),
        extension => Err(ParserError::UnsupportedExtension(
            extension.unwrap_or("none").to_string(),
        )),
    }
}

pub(crate) fn language_from_extension(extension: &str) -> Option<SourceLanguage> {
    match extension {
        "js" | "mjs" | "cjs" => Some(SourceLanguage::JavaScript),
        "jsx" => Some(SourceLanguage::Jsx),
        "ts" | "mts" | "cts" => Some(SourceLanguage::TypeScript),
        "tsx" => Some(SourceLanguage::Tsx),
        "rs" => Some(SourceLanguage::Rust),
        "go" => Some(SourceLanguage::Go),
        "py" => Some(SourceLanguage::Python),
        "java" => Some(SourceLanguage::Java),
        "kt" | "kts" => Some(SourceLanguage::Kotlin),
        "swift" => Some(SourceLanguage::Swift),
        "rb" => Some(SourceLanguage::Ruby),
        "php" => Some(SourceLanguage::Php),
        "c" | "h" => Some(SourceLanguage::C),
        "cpp" | "hpp" => Some(SourceLanguage::Cpp),
        "cs" => Some(SourceLanguage::CSharp),
        "sh" | "bash" | "zsh" | "fish" => Some(SourceLanguage::Shell),
        "sql" => Some(SourceLanguage::Sql),
        "json" | "jsonc" => Some(SourceLanguage::Json),
        "yaml" | "yml" => Some(SourceLanguage::Yaml),
        "toml" => Some(SourceLanguage::Toml),
        "ini" | "cfg" | "conf" => Some(SourceLanguage::Ini),
        "xml" => Some(SourceLanguage::Xml),
        "css" => Some(SourceLanguage::Css),
        "scss" => Some(SourceLanguage::Scss),
        "less" => Some(SourceLanguage::Less),
        "vue" => Some(SourceLanguage::Vue),
        "svelte" => Some(SourceLanguage::Svelte),
        "astro" => Some(SourceLanguage::Astro),
        _ => None,
    }
}

#[cfg(test)]
fn normalize_display_path(path: &Path) -> String {
    if path.is_absolute() {
        if let Ok(current_dir) = std::env::current_dir() {
            if let Ok(relative) = path.strip_prefix(&current_dir) {
                return normalize_relative_path(relative);
            }

            if let Some(parent_dir) = current_dir.parent() {
                if let Ok(relative) = path.strip_prefix(parent_dir) {
                    return normalize_relative_path(relative);
                }
            }
        }
    }

    normalize_relative_path(path)
}

#[cfg(test)]
fn normalize_relative_path(path: &Path) -> String {
    let mut parts = Vec::new();

    for component in path.components() {
        match component {
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
            Component::ParentDir => {
                parts.pop();
            }
            Component::Normal(value) => {
                parts.push(value.to_string_lossy().to_string());
            }
        }
    }

    parts.join("/")
}

#[cfg(test)]
mod tests {
    use super::{
        language_from_extension, normalize_display_path, normalize_relative_path, parse_file,
        SourceLanguage, SymbolKind,
    };
    use std::path::{Path, PathBuf};

    #[test]
    fn parses_current_app_tsx() {
        let parsed = parse_file("src/App.tsx").expect("App.tsx should parse");

        assert_eq!(parsed.path, "src/App.tsx");
        assert!(parsed
            .symbols
            .iter()
            .any(|symbol| symbol.name == "App" && symbol.kind == SymbolKind::Function));
        assert!(parsed
            .symbols
            .iter()
            .any(|symbol| symbol.name == "App" && symbol.kind == SymbolKind::Export));
    }

    #[test]
    fn normalizes_paths_for_citations() {
        assert_eq!(
            normalize_relative_path(Path::new("./src/../src/App.tsx")),
            "src/App.tsx"
        );
        assert_eq!(
            normalize_relative_path(Path::new("../src/App.tsx")),
            "src/App.tsx"
        );
    }

    #[test]
    fn parses_python_symbols() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let path = temp_dir.path().join("page_index.py");
        std::fs::write(
            &path,
            "import os\nfrom client import PageClient\n\nclass PageIndex:\n    pass\n\ndef check_title():\n    return True\n",
        )
        .expect("python file should be written");

        let parsed = parse_file(&path).expect("python file should parse");

        assert_eq!(parsed.language, SourceLanguage::Python);
        assert!(parsed
            .symbols
            .iter()
            .any(|symbol| symbol.name == "PageIndex" && symbol.kind == SymbolKind::Class));
        assert!(parsed
            .symbols
            .iter()
            .any(|symbol| symbol.name == "check_title" && symbol.kind == SymbolKind::Function));
        assert!(parsed
            .symbols
            .iter()
            .any(|symbol| symbol.name == "os" && symbol.kind == SymbolKind::Import));
    }

    #[test]
    fn maps_all_indexable_extensions_to_parser_languages() {
        for extension in [
            "js", "mjs", "cjs", "jsx", "ts", "mts", "cts", "tsx", "rs", "go", "py", "java", "kt",
            "kts", "swift", "rb", "php", "c", "h", "cpp", "hpp", "cs", "sh", "bash", "zsh", "fish",
            "sql", "json", "jsonc", "yaml", "yml", "toml", "ini", "cfg", "conf", "xml", "css",
            "scss", "less", "vue", "svelte", "astro",
        ] {
            assert!(
                language_from_extension(extension).is_some(),
                "{extension} should map to a parser language"
            );
        }
    }

    #[test]
    fn strips_workspace_prefix_from_absolute_paths() {
        let current_dir = std::env::current_dir().expect("current dir should exist");
        let app_path = PathBuf::from(&current_dir).join("../src/App.tsx");

        assert_eq!(normalize_display_path(&app_path), "src/App.tsx");
    }

    #[test]
    #[ignore = "manual parser inspection helper"]
    fn print_current_app_symbols() {
        let parsed = parse_file("src/App.tsx").expect("App.tsx should parse");
        let output = serde_json::to_string_pretty(&parsed).expect("parsed file should serialize");

        println!("{output}");
    }
}
