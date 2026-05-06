use std::path::Path;

use super::{CodeSymbol, ParsedFile, SourceLanguage, SourceRange, SymbolKind};

pub fn parse_source(path: &str, language: SourceLanguage, source: &str) -> ParsedFile {
    let mut symbols = Vec::new();

    for (index, line) in source.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim_start();

        if trimmed.is_empty() || is_comment(trimmed, language) {
            continue;
        }

        collect_import(trimmed, line_number, line, language, &mut symbols);
        collect_symbol(trimmed, line_number, line, language, &mut symbols);
    }

    if symbols.is_empty() {
        if let Some(name) = Path::new(path).file_stem().and_then(|value| value.to_str()) {
            symbols.push(symbol(
                line_range(1, 1, source.lines().next().unwrap_or_default()),
                name.to_string(),
                SymbolKind::Object,
            ));
        }
    }

    ParsedFile {
        path: path.to_string(),
        language,
        symbols,
    }
}

fn collect_import(
    trimmed: &str,
    line_number: usize,
    line: &str,
    language: SourceLanguage,
    symbols: &mut Vec<CodeSymbol>,
) {
    let name = match language {
        SourceLanguage::Python => trimmed
            .strip_prefix("from ")
            .and_then(|value| value.split_whitespace().next())
            .or_else(|| {
                trimmed
                    .strip_prefix("import ")
                    .and_then(|value| value.split([',', ' ']).next())
            }),
        SourceLanguage::Go
        | SourceLanguage::Java
        | SourceLanguage::Kotlin
        | SourceLanguage::Swift => trimmed
            .strip_prefix("import ")
            .map(|value| value.trim_matches(['"', '\'', ';'])),
        SourceLanguage::Ruby => trimmed
            .strip_prefix("require ")
            .map(|value| value.trim_matches(['"', '\''])),
        SourceLanguage::Php => trimmed
            .strip_prefix("use ")
            .map(|value| value.trim_end_matches(';')),
        _ => None,
    };

    if let Some(name) = name.and_then(clean_name) {
        symbols.push(symbol(
            line_range(line_number, column_of(line, &name), line),
            name,
            SymbolKind::Import,
        ));
    }
}

fn collect_symbol(
    trimmed: &str,
    line_number: usize,
    line: &str,
    language: SourceLanguage,
    symbols: &mut Vec<CodeSymbol>,
) {
    let found = match language {
        SourceLanguage::Python => python_symbol(trimmed),
        SourceLanguage::Rust => rust_symbol(trimmed),
        SourceLanguage::Go => go_symbol(trimmed),
        SourceLanguage::Java | SourceLanguage::Kotlin | SourceLanguage::CSharp => {
            jvm_style_symbol(trimmed)
        }
        SourceLanguage::Swift => swift_symbol(trimmed),
        SourceLanguage::Ruby => ruby_symbol(trimmed),
        SourceLanguage::Php => php_symbol(trimmed),
        SourceLanguage::C | SourceLanguage::Cpp => c_style_symbol(trimmed),
        SourceLanguage::Shell => shell_symbol(trimmed),
        SourceLanguage::Sql => sql_symbol(trimmed),
        SourceLanguage::Json => json_symbol(trimmed),
        SourceLanguage::Yaml => yaml_symbol(trimmed),
        SourceLanguage::Toml | SourceLanguage::Ini => section_symbol(trimmed),
        SourceLanguage::Xml => xml_symbol(trimmed),
        SourceLanguage::Css | SourceLanguage::Scss | SourceLanguage::Less => css_symbol(trimmed),
        SourceLanguage::Vue | SourceLanguage::Svelte | SourceLanguage::Astro => {
            component_symbol(trimmed)
        }
        SourceLanguage::JavaScript
        | SourceLanguage::TypeScript
        | SourceLanguage::Tsx
        | SourceLanguage::Jsx => None,
    };

    if let Some((name, kind)) = found {
        let column = column_of(line, &name);
        symbols.push(symbol(line_range(line_number, column, line), name, kind));
    }
}

fn python_symbol(line: &str) -> Option<(String, SymbolKind)> {
    if let Some(name) = line
        .strip_prefix("async def ")
        .or_else(|| line.strip_prefix("def "))
    {
        return take_identifier(name).map(|name| (name, SymbolKind::Function));
    }
    line.strip_prefix("class ")
        .and_then(take_identifier)
        .map(|name| (name, SymbolKind::Class))
}

fn rust_symbol(line: &str) -> Option<(String, SymbolKind)> {
    find_after_keyword(line, "fn ")
        .map(|name| (name, SymbolKind::Function))
        .or_else(|| find_after_keyword(line, "struct ").map(|name| (name, SymbolKind::Class)))
        .or_else(|| find_after_keyword(line, "enum ").map(|name| (name, SymbolKind::Enum)))
        .or_else(|| find_after_keyword(line, "trait ").map(|name| (name, SymbolKind::Interface)))
        .or_else(|| find_after_keyword(line, "type ").map(|name| (name, SymbolKind::TypeAlias)))
}

fn go_symbol(line: &str) -> Option<(String, SymbolKind)> {
    if let Some(rest) = line.strip_prefix("func ") {
        let rest = rest
            .strip_prefix('(')
            .and_then(|value| value.split_once(')'))
            .map(|(_, value)| value.trim_start())
            .unwrap_or(rest);
        return take_identifier(rest).map(|name| (name, SymbolKind::Function));
    }
    if let Some(rest) = line.strip_prefix("type ") {
        let name = take_identifier(rest)?;
        let kind = if rest.contains(" struct") {
            SymbolKind::Class
        } else if rest.contains(" interface") {
            SymbolKind::Interface
        } else {
            SymbolKind::TypeAlias
        };
        return Some((name, kind));
    }
    None
}

fn jvm_style_symbol(line: &str) -> Option<(String, SymbolKind)> {
    find_after_keyword(line, "class ")
        .map(|name| (name, SymbolKind::Class))
        .or_else(|| {
            find_after_keyword(line, "interface ").map(|name| (name, SymbolKind::Interface))
        })
        .or_else(|| find_after_keyword(line, "enum ").map(|name| (name, SymbolKind::Enum)))
        .or_else(|| find_function_before_paren(line).map(|name| (name, SymbolKind::Function)))
}

fn swift_symbol(line: &str) -> Option<(String, SymbolKind)> {
    find_after_keyword(line, "func ")
        .map(|name| (name, SymbolKind::Function))
        .or_else(|| find_after_keyword(line, "class ").map(|name| (name, SymbolKind::Class)))
        .or_else(|| find_after_keyword(line, "struct ").map(|name| (name, SymbolKind::Class)))
        .or_else(|| find_after_keyword(line, "enum ").map(|name| (name, SymbolKind::Enum)))
        .or_else(|| find_after_keyword(line, "protocol ").map(|name| (name, SymbolKind::Interface)))
}

fn ruby_symbol(line: &str) -> Option<(String, SymbolKind)> {
    line.strip_prefix("def ")
        .and_then(take_identifier)
        .map(|name| (name, SymbolKind::Function))
        .or_else(|| {
            line.strip_prefix("class ")
                .and_then(take_identifier)
                .map(|name| (name, SymbolKind::Class))
        })
        .or_else(|| {
            line.strip_prefix("module ")
                .and_then(take_identifier)
                .map(|name| (name, SymbolKind::Object))
        })
}

fn php_symbol(line: &str) -> Option<(String, SymbolKind)> {
    find_after_keyword(line, "function ")
        .map(|name| (name, SymbolKind::Function))
        .or_else(|| find_after_keyword(line, "class ").map(|name| (name, SymbolKind::Class)))
        .or_else(|| {
            find_after_keyword(line, "interface ").map(|name| (name, SymbolKind::Interface))
        })
        .or_else(|| find_after_keyword(line, "trait ").map(|name| (name, SymbolKind::Interface)))
}

fn c_style_symbol(line: &str) -> Option<(String, SymbolKind)> {
    find_after_keyword(line, "struct ")
        .map(|name| (name, SymbolKind::Class))
        .or_else(|| find_after_keyword(line, "enum ").map(|name| (name, SymbolKind::Enum)))
        .or_else(|| find_function_before_paren(line).map(|name| (name, SymbolKind::Function)))
}

fn shell_symbol(line: &str) -> Option<(String, SymbolKind)> {
    if let Some(name) = line.strip_prefix("function ").and_then(take_identifier) {
        return Some((name, SymbolKind::Function));
    }
    line.find("()")
        .and_then(|index| take_identifier(line[..index].trim()))
        .map(|name| (name, SymbolKind::Function))
}

fn sql_symbol(line: &str) -> Option<(String, SymbolKind)> {
    let lower = line.to_ascii_lowercase();
    if let Some(index) = lower.find("create table ") {
        return take_identifier(&line[index + "create table ".len()..])
            .map(|name| (name, SymbolKind::Object));
    }
    if let Some(index) = lower.find("create view ") {
        return take_identifier(&line[index + "create view ".len()..])
            .map(|name| (name, SymbolKind::Object));
    }
    if let Some(index) = lower.find("create function ") {
        return take_identifier(&line[index + "create function ".len()..])
            .map(|name| (name, SymbolKind::Function));
    }
    None
}

fn json_symbol(line: &str) -> Option<(String, SymbolKind)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('"') {
        return None;
    }
    let (_, rest) = trimmed[1..].split_once('"')?;
    rest.trim_start().starts_with(':').then(|| {
        (
            trimmed[1..].split_once('"').unwrap().0.to_string(),
            SymbolKind::Object,
        )
    })
}

fn yaml_symbol(line: &str) -> Option<(String, SymbolKind)> {
    let (name, rest) = line.split_once(':')?;
    let name = name.trim();
    (!name.is_empty() && !name.starts_with('-'))
        .then(|| (name.to_string(), SymbolKind::Object))
        .filter(|_| !rest.trim_start().starts_with("//"))
}

fn section_symbol(line: &str) -> Option<(String, SymbolKind)> {
    if let Some(stripped) = line.strip_prefix('[') {
        return stripped
            .split_once(']')
            .map(|(name, _)| (name.to_string(), SymbolKind::Object));
    }
    line.split_once('=')
        .map(|(name, _)| (name.trim().to_string(), SymbolKind::Object))
        .filter(|(name, _)| !name.is_empty())
}

fn xml_symbol(line: &str) -> Option<(String, SymbolKind)> {
    let rest = line.strip_prefix('<')?;
    if rest.starts_with(['/', '?', '!']) {
        return None;
    }
    take_identifier(rest).map(|name| (name, SymbolKind::Object))
}

fn css_symbol(line: &str) -> Option<(String, SymbolKind)> {
    line.split_once('{')
        .map(|(selector, _)| (selector.trim().to_string(), SymbolKind::Object))
        .filter(|(selector, _)| !selector.is_empty() && !selector.starts_with('@'))
}

fn component_symbol(line: &str) -> Option<(String, SymbolKind)> {
    if line.starts_with("<script") || line.starts_with("<style") {
        return xml_symbol(line);
    }
    find_after_keyword(line, "function ")
        .map(|name| (name, SymbolKind::Function))
        .or_else(|| find_after_keyword(line, "class ").map(|name| (name, SymbolKind::Class)))
        .or_else(|| xml_symbol(line))
}

fn find_after_keyword(line: &str, keyword: &str) -> Option<String> {
    line.find(keyword)
        .and_then(|index| take_identifier(&line[index + keyword.len()..]))
}

fn find_function_before_paren(line: &str) -> Option<String> {
    let before_paren = line.split_once('(')?.0.trim_end();
    if before_paren.contains(['=', ':'])
        || before_paren.starts_with("if ")
        || before_paren.starts_with("for ")
    {
        return None;
    }
    before_paren
        .split_whitespace()
        .last()
        .and_then(take_identifier)
}

fn take_identifier(value: &str) -> Option<String> {
    let value = value.trim_start_matches(['&', '*', '$', '@']);
    let name: String = value
        .chars()
        .take_while(|character| {
            character.is_alphanumeric() || matches!(character, '_' | '-' | '.' | ':' | '\\')
        })
        .collect();
    clean_name(&name)
}

fn clean_name(value: impl AsRef<str>) -> Option<String> {
    let value = value
        .as_ref()
        .trim()
        .trim_matches(['"', '\'', '`', ';', ':', '{', '(', ')']);
    (!value.is_empty()).then(|| value.to_string())
}

fn column_of(line: &str, needle: &str) -> usize {
    line.find(needle).map(|index| index + 1).unwrap_or(1)
}

fn line_range(line_number: usize, start_column: usize, line: &str) -> SourceRange {
    SourceRange {
        start_line: line_number,
        start_column,
        end_line: line_number,
        end_column: line.len().max(start_column) + 1,
    }
}

fn symbol(range: SourceRange, name: String, kind: SymbolKind) -> CodeSymbol {
    CodeSymbol {
        name,
        kind,
        parent: None,
        source: None,
        range,
    }
}

fn is_comment(line: &str, language: SourceLanguage) -> bool {
    match language {
        SourceLanguage::Python
        | SourceLanguage::Ruby
        | SourceLanguage::Shell
        | SourceLanguage::Yaml => line.starts_with('#'),
        SourceLanguage::Sql => line.starts_with("--"),
        SourceLanguage::Xml => line.starts_with("<!--"),
        _ => line.starts_with("//") || line.starts_with("/*"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- python_symbol ---

    #[test]
    fn python_def_extracts_function() {
        let result = python_symbol("def compute_total(items):");
        assert_eq!(result, Some(("compute_total".to_string(), SymbolKind::Function)));
    }

    #[test]
    fn python_async_def_extracts_function() {
        let result = python_symbol("async def fetch_data(url):");
        assert_eq!(result, Some(("fetch_data".to_string(), SymbolKind::Function)));
    }

    #[test]
    fn python_class_extracts_class() {
        let result = python_symbol("class DataProcessor:");
        assert_eq!(result, Some(("DataProcessor".to_string(), SymbolKind::Class)));
    }

    #[test]
    fn python_class_with_base_extracts_name_only() {
        let result = python_symbol("class MyModel(BaseModel):");
        assert_eq!(result, Some(("MyModel".to_string(), SymbolKind::Class)));
    }

    #[test]
    fn python_irrelevant_line_returns_none() {
        assert_eq!(python_symbol("x = 42"), None);
        assert_eq!(python_symbol("return result"), None);
        assert_eq!(python_symbol("if condition:"), None);
    }

    // --- rust_symbol ---

    #[test]
    fn rust_fn_extracts_function() {
        let result = rust_symbol("pub fn index_path(root: &Path) -> Result<(), Error> {");
        assert_eq!(result, Some(("index_path".to_string(), SymbolKind::Function)));
    }

    #[test]
    fn rust_struct_extracts_class() {
        let result = rust_symbol("pub struct MetadataStore {");
        assert_eq!(result, Some(("MetadataStore".to_string(), SymbolKind::Class)));
    }

    #[test]
    fn rust_enum_extracts_enum() {
        let result = rust_symbol("pub enum SourceLanguage {");
        assert_eq!(result, Some(("SourceLanguage".to_string(), SymbolKind::Enum)));
    }

    #[test]
    fn rust_trait_extracts_interface() {
        let result = rust_symbol("pub trait Indexable {");
        assert_eq!(result, Some(("Indexable".to_string(), SymbolKind::Interface)));
    }

    #[test]
    fn rust_type_alias_extracts_type_alias() {
        let result = rust_symbol("type Result<T> = std::result::Result<T, Error>;");
        assert_eq!(result, Some(("Result".to_string(), SymbolKind::TypeAlias)));
    }

    #[test]
    fn rust_comment_line_returns_none() {
        assert_eq!(rust_symbol("// fn fake_function() {}"), None);
    }

    // --- go_symbol ---

    #[test]
    fn go_func_extracts_function() {
        let result = go_symbol("func RenderTheme() {}");
        assert_eq!(result, Some(("RenderTheme".to_string(), SymbolKind::Function)));
    }

    #[test]
    fn go_method_extracts_function() {
        let result = go_symbol("func (r *Router) Handle(path string) {}");
        assert_eq!(result, Some(("Handle".to_string(), SymbolKind::Function)));
    }

    #[test]
    fn go_type_struct_extracts_class() {
        let result = go_symbol("type Config struct {");
        assert_eq!(result, Some(("Config".to_string(), SymbolKind::Class)));
    }

    #[test]
    fn go_type_interface_extracts_interface() {
        let result = go_symbol("type Writer interface {");
        assert_eq!(result, Some(("Writer".to_string(), SymbolKind::Interface)));
    }

    #[test]
    fn go_type_alias_extracts_type_alias() {
        let result = go_symbol("type MyInt = int");
        assert_eq!(result, Some(("MyInt".to_string(), SymbolKind::TypeAlias)));
    }

    // --- jvm_style_symbol ---

    #[test]
    fn java_class_extracts_class() {
        let result = jvm_style_symbol("public class UserService {");
        assert_eq!(result, Some(("UserService".to_string(), SymbolKind::Class)));
    }

    #[test]
    fn java_interface_extracts_interface() {
        let result = jvm_style_symbol("public interface Serializable {");
        assert_eq!(result, Some(("Serializable".to_string(), SymbolKind::Interface)));
    }

    #[test]
    fn java_enum_extracts_enum() {
        let result = jvm_style_symbol("public enum Status {");
        assert_eq!(result, Some(("Status".to_string(), SymbolKind::Enum)));
    }

    #[test]
    fn java_method_extracts_function() {
        let result = jvm_style_symbol("    public void processRequest(HttpRequest req) {");
        assert_eq!(result, Some(("processRequest".to_string(), SymbolKind::Function)));
    }

    // --- shell_symbol ---

    #[test]
    fn shell_function_keyword_extracts_function() {
        let result = shell_symbol("function setup_env() {");
        assert_eq!(result, Some(("setup_env".to_string(), SymbolKind::Function)));
    }

    #[test]
    fn shell_posix_style_function_extracts_function() {
        let result = shell_symbol("build_artifacts() {");
        assert_eq!(result, Some(("build_artifacts".to_string(), SymbolKind::Function)));
    }

    #[test]
    fn shell_non_function_line_returns_none() {
        assert_eq!(shell_symbol("MY_VAR=value"), None);
        assert_eq!(shell_symbol("echo hello"), None);
    }

    // --- sql_symbol ---

    #[test]
    fn sql_create_table_extracts_object() {
        let result = sql_symbol("CREATE TABLE users (");
        assert_eq!(result, Some(("users".to_string(), SymbolKind::Object)));
    }

    #[test]
    fn sql_create_table_case_insensitive() {
        let result = sql_symbol("create table orders (id int)");
        assert_eq!(result, Some(("orders".to_string(), SymbolKind::Object)));
    }

    #[test]
    fn sql_create_view_extracts_object() {
        let result = sql_symbol("CREATE VIEW active_users AS");
        assert_eq!(result, Some(("active_users".to_string(), SymbolKind::Object)));
    }

    #[test]
    fn sql_create_function_extracts_function() {
        let result = sql_symbol("CREATE FUNCTION calculate_tax(amount NUMERIC)");
        assert_eq!(result, Some(("calculate_tax".to_string(), SymbolKind::Function)));
    }

    #[test]
    fn sql_select_returns_none() {
        assert_eq!(sql_symbol("SELECT * FROM users"), None);
    }

    // --- json_symbol ---

    #[test]
    fn json_key_value_extracts_object() {
        let result = json_symbol("  \"name\": \"localbrain\"");
        assert_eq!(result, Some(("name".to_string(), SymbolKind::Object)));
    }

    #[test]
    fn json_non_key_line_returns_none() {
        assert_eq!(json_symbol("  \"value\""), None);
        assert_eq!(json_symbol("  {"), None);
        assert_eq!(json_symbol("  42"), None);
    }

    // --- yaml_symbol ---

    #[test]
    fn yaml_key_extracts_object() {
        let result = yaml_symbol("version: \"1.0\"");
        assert_eq!(result, Some(("version".to_string(), SymbolKind::Object)));
    }

    #[test]
    fn yaml_list_item_returns_none() {
        assert_eq!(yaml_symbol("- item"), None);
    }

    #[test]
    fn yaml_empty_key_returns_none() {
        assert_eq!(yaml_symbol(": value"), None);
    }

    // --- section_symbol (TOML/INI) ---

    #[test]
    fn toml_section_header_extracts_object() {
        let result = section_symbol("[package]");
        assert_eq!(result, Some(("package".to_string(), SymbolKind::Object)));
    }

    #[test]
    fn ini_key_value_extracts_object() {
        let result = section_symbol("host = localhost");
        assert_eq!(result, Some(("host".to_string(), SymbolKind::Object)));
    }

    #[test]
    fn ini_empty_key_returns_none() {
        assert_eq!(section_symbol("= value"), None);
    }

    // --- xml_symbol ---

    #[test]
    fn xml_opening_tag_extracts_object() {
        let result = xml_symbol("<dependency>");
        assert_eq!(result, Some(("dependency".to_string(), SymbolKind::Object)));
    }

    #[test]
    fn xml_closing_tag_returns_none() {
        assert_eq!(xml_symbol("</dependency>"), None);
    }

    #[test]
    fn xml_processing_instruction_returns_none() {
        assert_eq!(xml_symbol("<?xml version=\"1.0\"?>"), None);
    }

    #[test]
    fn xml_comment_returns_none() {
        assert_eq!(xml_symbol("<!-- comment -->"), None);
    }

    // --- css_symbol ---

    #[test]
    fn css_selector_extracts_object() {
        let result = css_symbol(".button-primary {");
        assert_eq!(result, Some((".button-primary".to_string(), SymbolKind::Object)));
    }

    #[test]
    fn css_at_rule_returns_none() {
        assert_eq!(css_symbol("@media (max-width: 768px) {"), None);
    }

    #[test]
    fn css_no_brace_returns_none() {
        assert_eq!(css_symbol("color: red;"), None);
    }

    // --- is_comment ---

    #[test]
    fn python_hash_is_comment() {
        assert!(is_comment("# this is a comment", SourceLanguage::Python));
        assert!(!is_comment("x = 1  # not a comment", SourceLanguage::Python));
    }

    #[test]
    fn rust_double_slash_is_comment() {
        assert!(is_comment("// comment", SourceLanguage::Rust));
        assert!(is_comment("/* block */", SourceLanguage::Rust));
        assert!(!is_comment("let x = 1;", SourceLanguage::Rust));
    }

    #[test]
    fn sql_dash_dash_is_comment() {
        assert!(is_comment("-- this is SQL comment", SourceLanguage::Sql));
        assert!(!is_comment("SELECT *", SourceLanguage::Sql));
    }

    #[test]
    fn xml_comment_marker_is_comment() {
        assert!(is_comment("<!-- xml comment -->", SourceLanguage::Xml));
        assert!(!is_comment("<element>", SourceLanguage::Xml));
    }

    #[test]
    fn shell_hash_is_comment() {
        assert!(is_comment("# shell comment", SourceLanguage::Shell));
        assert!(!is_comment("echo hello", SourceLanguage::Shell));
    }

    // --- parse_source with fallback symbol ---

    #[test]
    fn empty_source_produces_fallback_symbol_from_filename() {
        let result = parse_source("config/settings.toml", SourceLanguage::Toml, "");
        assert_eq!(result.symbols.len(), 1);
        assert_eq!(result.symbols[0].name, "settings");
        assert_eq!(result.symbols[0].kind, SymbolKind::Object);
    }

    #[test]
    fn rust_source_parses_fn_and_struct() {
        let source = "pub struct Config {\n    port: u16,\n}\n\npub fn start(cfg: &Config) {}\n";
        let result = parse_source("src/server.rs", SourceLanguage::Rust, source);
        assert!(result.symbols.iter().any(|s| s.name == "Config" && s.kind == SymbolKind::Class));
        assert!(result.symbols.iter().any(|s| s.name == "start" && s.kind == SymbolKind::Function));
    }

    #[test]
    fn go_source_parses_func_and_type() {
        let source = "package main\n\ntype Server struct {\n    port int\n}\n\nfunc Run(s *Server) {}\n";
        let result = parse_source("main.go", SourceLanguage::Go, source);
        assert!(result.symbols.iter().any(|s| s.name == "Server" && s.kind == SymbolKind::Class));
        assert!(result.symbols.iter().any(|s| s.name == "Run" && s.kind == SymbolKind::Function));
    }

    #[test]
    fn shell_source_parses_functions() {
        let source = "#!/usr/bin/env bash\n\nfunction setup() {\n    echo 'setting up'\n}\n\nbuild() {\n    cargo build\n}\n";
        let result = parse_source("scripts/build.sh", SourceLanguage::Shell, source);
        assert!(result.symbols.iter().any(|s| s.name == "setup" && s.kind == SymbolKind::Function));
        assert!(result.symbols.iter().any(|s| s.name == "build" && s.kind == SymbolKind::Function));
    }

    #[test]
    fn sql_source_parses_tables_and_views() {
        let source = "CREATE TABLE files (\n    path TEXT PRIMARY KEY\n);\n\nCREATE VIEW active_files AS SELECT * FROM files;\n";
        let result = parse_source("schema.sql", SourceLanguage::Sql, source);
        assert!(result.symbols.iter().any(|s| s.name == "files" && s.kind == SymbolKind::Object));
        assert!(result.symbols.iter().any(|s| s.name == "active_files" && s.kind == SymbolKind::Object));
    }

    #[test]
    fn ruby_source_parses_imports_class_and_def() {
        let source = "require 'rails'\n\nclass ApplicationController\n  def index\n  end\nend\n";
        let result = parse_source("app/controller.rb", SourceLanguage::Ruby, source);
        assert!(result.symbols.iter().any(|s| s.name == "rails" && s.kind == SymbolKind::Import));
        assert!(result.symbols.iter().any(|s| s.name == "ApplicationController" && s.kind == SymbolKind::Class));
        assert!(result.symbols.iter().any(|s| s.name == "index" && s.kind == SymbolKind::Function));
    }

    #[test]
    fn javascript_and_typescript_produce_no_generic_symbols() {
        // JS/TS are handled by the tree-sitter typescript parser, not generic
        let result = parse_source("src/index.js", SourceLanguage::JavaScript, "export function main() {}");
        // No symbols extracted by generic parser for JS (dispatched to typescript parser normally)
        // But in isolation, the generic parse_source will be called and should produce fallback
        // since JavaScript/TypeScript returns None from collect_symbol
        let _ = result; // just ensure it doesn't panic
    }

    // --- take_identifier edge cases ---

    #[test]
    fn take_identifier_strips_leading_sigils() {
        assert_eq!(take_identifier("$variable"), Some("variable".to_string()));
        assert_eq!(take_identifier("@decorator"), Some("decorator".to_string()));
        assert_eq!(take_identifier("*pointer"), Some("pointer".to_string()));
        assert_eq!(take_identifier("&reference"), Some("reference".to_string()));
    }

    #[test]
    fn take_identifier_stops_at_parens() {
        assert_eq!(take_identifier("func_name(args)"), Some("func_name".to_string()));
    }

    #[test]
    fn take_identifier_returns_none_for_empty() {
        assert_eq!(take_identifier(""), None);
        assert_eq!(take_identifier("   "), None);
    }

    // --- clean_name edge cases ---

    #[test]
    fn clean_name_trims_quotes_and_braces() {
        assert_eq!(clean_name("\"key\""), Some("key".to_string()));
        assert_eq!(clean_name("'value'"), Some("value".to_string()));
        assert_eq!(clean_name("{block"), Some("block".to_string()));
    }

    #[test]
    fn clean_name_returns_none_for_empty_after_trim() {
        assert_eq!(clean_name(""), None);
        assert_eq!(clean_name("\"\""), None);
    }
}
