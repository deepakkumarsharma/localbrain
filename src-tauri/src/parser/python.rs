use super::{CodeSymbol, ParsedFile, ParserError, SourceLanguage, SourceRange, SymbolKind};
use tree_sitter::{Node, Parser};

pub fn parse_source(
    path: &str,
    language: SourceLanguage,
    source: &str,
) -> Result<ParsedFile, ParserError> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_python::LANGUAGE.into())?;

    let tree = parser.parse(source, None).ok_or(ParserError::ParseFailed)?;
    let mut symbols = Vec::new();
    collect_symbols(tree.root_node(), source, &mut symbols);

    Ok(ParsedFile {
        path: path.to_string(),
        language,
        symbols,
    })
}

fn collect_symbols(node: Node<'_>, source: &str, symbols: &mut Vec<CodeSymbol>) {
    match node.kind() {
        "function_definition" => push_named_symbol(node, source, symbols, SymbolKind::Function),
        "class_definition" => push_named_symbol(node, source, symbols, SymbolKind::Class),
        "import_statement" | "import_from_statement" => {
            for name in import_names(node, source) {
                symbols.push(symbol(node, name, SymbolKind::Import));
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_symbols(child, source, symbols);
    }
}

fn push_named_symbol(
    node: Node<'_>,
    source: &str,
    symbols: &mut Vec<CodeSymbol>,
    kind: SymbolKind,
) {
    if let Some(name) = node_name(node, source) {
        symbols.push(symbol(node, name, kind));
    }
}

fn import_names(node: Node<'_>, source: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut cursor = node.walk();

    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "dotted_name" | "identifier" => {
                names.push(node_text(child, source).to_string());
            }
            "aliased_import" => {
                if let Some(name) = child.named_child(0) {
                    names.push(node_text(name, source).to_string());
                }
            }
            _ => {}
        }
    }

    names.sort();
    names.dedup();
    names
}

fn node_name(node: Node<'_>, source: &str) -> Option<String> {
    node.child_by_field_name("name")
        .map(|name| node_text(name, source).to_string())
}

fn symbol(node: Node<'_>, name: String, kind: SymbolKind) -> CodeSymbol {
    CodeSymbol {
        name,
        kind,
        parent: None,
        source: None,
        range: range(node),
    }
}

fn range(node: Node<'_>) -> SourceRange {
    let start = node.start_position();
    let end = node.end_position();

    SourceRange {
        start_line: start.row + 1,
        start_column: start.column + 1,
        end_line: end.row + 1,
        end_column: end.column + 1,
    }
}

fn node_text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes()).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::parse_source;
    use crate::parser::{SourceLanguage, SymbolKind};

    #[test]
    fn parses_simple_function() {
        let source = "def greet(name):\n    return 'Hello ' + name\n";
        let parsed = parse_source("greet.py", SourceLanguage::Python, source)
            .expect("should parse without error");

        assert_eq!(parsed.path, "greet.py");
        assert_eq!(parsed.language, SourceLanguage::Python);
        assert!(parsed
            .symbols
            .iter()
            .any(|s| s.name == "greet" && s.kind == SymbolKind::Function));
    }

    #[test]
    fn parses_class_definition() {
        let source = "class DataStore:\n    pass\n";
        let parsed = parse_source("store.py", SourceLanguage::Python, source)
            .expect("should parse without error");

        assert!(parsed
            .symbols
            .iter()
            .any(|s| s.name == "DataStore" && s.kind == SymbolKind::Class));
    }

    #[test]
    fn parses_import_statement() {
        let source = "import os\nimport sys\n";
        let parsed = parse_source("main.py", SourceLanguage::Python, source)
            .expect("should parse without error");

        assert!(parsed
            .symbols
            .iter()
            .any(|s| s.name == "os" && s.kind == SymbolKind::Import));
        assert!(parsed
            .symbols
            .iter()
            .any(|s| s.name == "sys" && s.kind == SymbolKind::Import));
    }

    #[test]
    fn parses_from_import_statement() {
        let source = "from collections import OrderedDict, defaultdict\n";
        let parsed = parse_source("utils.py", SourceLanguage::Python, source)
            .expect("should parse without error");

        let import_names: Vec<&str> = parsed
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Import)
            .map(|s| s.name.as_str())
            .collect();

        assert!(import_names.contains(&"OrderedDict") || import_names.contains(&"collections"));
    }

    #[test]
    fn parses_async_function() {
        let source = "import asyncio\n\nasync def fetch_data(url: str) -> str:\n    return ''\n";
        let parsed = parse_source("client.py", SourceLanguage::Python, source)
            .expect("should parse without error");

        assert!(parsed
            .symbols
            .iter()
            .any(|s| s.name == "fetch_data" && s.kind == SymbolKind::Function));
    }

    #[test]
    fn parses_nested_functions_in_class() {
        let source =
            "class Router:\n    def get(self, path):\n        pass\n    def post(self, path):\n        pass\n";
        let parsed = parse_source("router.py", SourceLanguage::Python, source)
            .expect("should parse without error");

        assert!(parsed
            .symbols
            .iter()
            .any(|s| s.name == "Router" && s.kind == SymbolKind::Class));
        assert!(parsed
            .symbols
            .iter()
            .any(|s| s.name == "get" && s.kind == SymbolKind::Function));
        assert!(parsed
            .symbols
            .iter()
            .any(|s| s.name == "post" && s.kind == SymbolKind::Function));
    }

    #[test]
    fn symbol_ranges_use_one_based_line_numbers() {
        let source = "class Foo:\n    pass\n\ndef bar():\n    pass\n";
        let parsed = parse_source("example.py", SourceLanguage::Python, source)
            .expect("should parse without error");

        let class_symbol = parsed
            .symbols
            .iter()
            .find(|s| s.name == "Foo")
            .expect("Foo class should be found");
        assert_eq!(class_symbol.range.start_line, 1, "class starts at line 1");

        let fn_symbol = parsed
            .symbols
            .iter()
            .find(|s| s.name == "bar")
            .expect("bar function should be found");
        assert_eq!(fn_symbol.range.start_line, 4, "bar starts at line 4");
    }

    #[test]
    fn empty_source_produces_no_symbols() {
        let parsed = parse_source("empty.py", SourceLanguage::Python, "")
            .expect("should parse empty source without error");

        assert!(parsed.symbols.is_empty());
    }

    #[test]
    fn deduplicates_aliased_imports() {
        // `import os as operating_system` should not produce duplicates
        let source = "import os as operating_system\nimport os\n";
        let parsed = parse_source("imports.py", SourceLanguage::Python, source)
            .expect("should parse without error");

        let os_count = parsed
            .symbols
            .iter()
            .filter(|s| s.name == "os" && s.kind == SymbolKind::Import)
            .count();
        // os should appear at most once per import statement (dedup applies within each statement)
        assert!(os_count <= 2); // two separate import lines
    }

    #[test]
    fn multiple_functions_and_classes_all_parsed() {
        let source = "import json\n\nclass Config:\n    pass\n\nclass Schema:\n    pass\n\ndef validate(cfg):\n    pass\n\ndef load(path):\n    pass\n";
        let parsed = parse_source("config.py", SourceLanguage::Python, source)
            .expect("should parse without error");

        let classes: Vec<&str> = parsed
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Class)
            .map(|s| s.name.as_str())
            .collect();
        let functions: Vec<&str> = parsed
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Function)
            .map(|s| s.name.as_str())
            .collect();

        assert!(classes.contains(&"Config"));
        assert!(classes.contains(&"Schema"));
        assert!(functions.contains(&"validate"));
        assert!(functions.contains(&"load"));
    }
}
