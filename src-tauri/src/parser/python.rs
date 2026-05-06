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
