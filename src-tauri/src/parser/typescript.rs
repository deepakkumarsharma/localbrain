use super::{CodeSymbol, ParsedFile, ParserError, SourceLanguage, SourceRange, SymbolKind};
use tree_sitter::{Node, Parser};

pub fn parse_source(
    path: &str,
    language: SourceLanguage,
    source: &str,
) -> Result<ParsedFile, ParserError> {
    let mut parser = Parser::new();
    set_parser_language(&mut parser, language)?;

    let tree = parser.parse(source, None).ok_or(ParserError::ParseFailed)?;
    let mut symbols = Vec::new();
    collect_symbols(tree.root_node(), source, &mut symbols);

    Ok(ParsedFile {
        path: path.to_string(),
        language,
        symbols,
    })
}

fn set_parser_language(
    parser: &mut Parser,
    language: SourceLanguage,
) -> Result<(), tree_sitter::LanguageError> {
    match language {
        SourceLanguage::JavaScript | SourceLanguage::Jsx => {
            parser.set_language(&tree_sitter_javascript::LANGUAGE.into())
        }
        SourceLanguage::TypeScript => {
            parser.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
        }
        SourceLanguage::Tsx => parser.set_language(&tree_sitter_typescript::LANGUAGE_TSX.into()),
        _ => Ok(()),
    }
}

fn collect_symbols(node: Node<'_>, source: &str, symbols: &mut Vec<CodeSymbol>) {
    match node.kind() {
        "function_declaration" | "generator_function_declaration" => {
            push_named_symbol(node, source, symbols, SymbolKind::Function);
        }
        "class_declaration" => {
            push_named_symbol(node, source, symbols, SymbolKind::Class);
        }
        "interface_declaration" => {
            push_named_symbol(node, source, symbols, SymbolKind::Interface);
        }
        "type_alias_declaration" => {
            push_named_symbol(node, source, symbols, SymbolKind::TypeAlias);
        }
        "enum_declaration" => {
            push_named_symbol(node, source, symbols, SymbolKind::Enum);
        }
        "import_statement" => {
            if let Some(import_source) = module_source(node, source) {
                symbols.push(symbol_with_source(
                    node,
                    import_source.clone(),
                    SymbolKind::Import,
                    Some(import_source.clone()),
                ));

                for import_name in import_specifier_names(node, source) {
                    symbols.push(symbol_with_source(
                        node,
                        import_name,
                        SymbolKind::Import,
                        Some(import_source.clone()),
                    ));
                }
            }
        }
        "export_statement" => {
            let export_source = module_source(node, source);
            let mut exported_names = export_specifier_names(node, source);

            if exported_names.is_empty() {
                if let Some(name) = export_name(node, source) {
                    exported_names.push(name);
                } else if export_source.is_some() {
                    exported_names.push("*".to_string());
                }
            }

            for name in exported_names {
                symbols.push(symbol_with_source(
                    node,
                    name,
                    SymbolKind::Export,
                    export_source.clone(),
                ));
            }
        }
        "variable_declarator" if has_function_value(node) => {
            if let Some(name) = node_name(node, source) {
                let kind = if starts_with_uppercase(&name) {
                    SymbolKind::Component
                } else {
                    SymbolKind::Function
                };
                symbols.push(symbol(node, name, kind));
            }
        }
        "variable_declarator" if has_object_value(node) => {
            push_named_symbol(node, source, symbols, SymbolKind::Object);
        }
        "method_definition" => {
            if let Some(name) = node_name(node, source) {
                let parent =
                    enclosing_class_name(node, source).or_else(|| object_parent_name(node, source));
                symbols.push(symbol_with_parent(
                    node,
                    scoped_name(&name, parent.as_deref()),
                    SymbolKind::Method,
                    parent,
                ));
            }
        }
        "pair" if has_function_value(node) => {
            if let Some(name) = pair_key_name(node, source) {
                let parent = object_parent_name(node, source);
                symbols.push(symbol_with_parent(
                    node,
                    scoped_name(&name, parent.as_deref()),
                    SymbolKind::Method,
                    parent,
                ));
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

fn symbol(node: Node<'_>, name: String, kind: SymbolKind) -> CodeSymbol {
    symbol_with_details(node, name, kind, None, None)
}

fn symbol_with_parent(
    node: Node<'_>,
    name: String,
    kind: SymbolKind,
    parent: Option<String>,
) -> CodeSymbol {
    symbol_with_details(node, name, kind, parent, None)
}

fn symbol_with_source(
    node: Node<'_>,
    name: String,
    kind: SymbolKind,
    source: Option<String>,
) -> CodeSymbol {
    symbol_with_details(node, name, kind, None, source)
}

fn symbol_with_details(
    node: Node<'_>,
    name: String,
    kind: SymbolKind,
    parent: Option<String>,
    source: Option<String>,
) -> CodeSymbol {
    CodeSymbol {
        name,
        kind,
        parent,
        source,
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

fn node_name(node: Node<'_>, source: &str) -> Option<String> {
    node.child_by_field_name("name")
        .map(|name| node_text(name, source).to_string())
}

fn has_function_value(node: Node<'_>) -> bool {
    node.child_by_field_name("value")
        .is_some_and(|value| matches!(value.kind(), "arrow_function" | "function_expression"))
}

fn has_object_value(node: Node<'_>) -> bool {
    node.child_by_field_name("value")
        .is_some_and(|value| value.kind() == "object")
}

fn module_source(node: Node<'_>, source: &str) -> Option<String> {
    node.child_by_field_name("source")
        .or_else(|| direct_named_child_of_kind(node, "string"))
        .map(|source_node| trim_quotes(node_text(source_node, source)).to_string())
}

fn import_specifier_names(node: Node<'_>, source: &str) -> Vec<String> {
    let mut names = Vec::new();
    collect_names_from_descendants(
        node,
        source,
        &["import_specifier", "namespace_import"],
        &mut names,
    );
    names
}

fn export_specifier_names(node: Node<'_>, source: &str) -> Vec<String> {
    let mut names = Vec::new();
    collect_names_from_descendants(node, source, &["export_specifier"], &mut names);
    names
}

fn export_name(node: Node<'_>, source: &str) -> Option<String> {
    if node.child_by_field_name("value").is_some() {
        return Some("default".to_string());
    }

    node.child_by_field_name("declaration")
        .and_then(|declaration| declaration_name(declaration, source))
        .or_else(|| first_descendant_name(node, source))
}

fn declaration_name(node: Node<'_>, source: &str) -> Option<String> {
    node_name(node, source).or_else(|| first_descendant_name(node, source))
}

fn first_descendant_name(node: Node<'_>, source: &str) -> Option<String> {
    if let Some(name) = node_name(node, source) {
        return Some(name);
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(name) = first_descendant_name(child, source) {
            return Some(name);
        }
    }

    None
}

fn collect_names_from_descendants(
    node: Node<'_>,
    source: &str,
    kinds: &[&str],
    names: &mut Vec<String>,
) {
    if kinds.contains(&node.kind()) {
        if let Some(name) = specifier_name(node, source) {
            names.push(name);
        }
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_names_from_descendants(child, source, kinds, names);
    }
}

fn specifier_name(node: Node<'_>, source: &str) -> Option<String> {
    node.child_by_field_name("alias")
        .map(|alias| node_text(alias, source).to_string())
        .or_else(|| node_name(node, source))
        .or_else(|| first_identifier_name(node, source))
}

fn enclosing_class_name(node: Node<'_>, source: &str) -> Option<String> {
    let mut current = node.parent();

    while let Some(parent) = current {
        if parent.kind() == "class_declaration" {
            return node_name(parent, source);
        }
        current = parent.parent();
    }

    None
}

fn object_parent_name(node: Node<'_>, source: &str) -> Option<String> {
    let mut current = node.parent();

    while let Some(parent) = current {
        if parent.kind() == "variable_declarator" {
            return node_name(parent, source);
        }
        current = parent.parent();
    }

    None
}

fn pair_key_name(node: Node<'_>, source: &str) -> Option<String> {
    node.child_by_field_name("key")
        .map(|key| trim_quotes(node_text(key, source)).to_string())
}

fn scoped_name(name: &str, parent: Option<&str>) -> String {
    parent
        .map(|parent| format!("{parent}.{name}"))
        .unwrap_or_else(|| name.to_string())
}

fn first_identifier_name(node: Node<'_>, source: &str) -> Option<String> {
    first_descendant_of_kinds(
        node,
        &[
            "identifier",
            "property_identifier",
            "shorthand_property_identifier",
            "type_identifier",
        ],
    )
    .map(|identifier| node_text(identifier, source).to_string())
}

fn direct_named_child_of_kind<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .find(|child| child.kind() == kind);
    found
}

fn first_descendant_of_kinds<'tree>(node: Node<'tree>, kinds: &[&str]) -> Option<Node<'tree>> {
    if kinds.contains(&node.kind()) {
        return Some(node);
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(found) = first_descendant_of_kinds(child, kinds) {
            return Some(found);
        }
    }

    None
}

fn node_text<'source>(node: Node<'_>, source: &'source str) -> &'source str {
    &source[node.byte_range()]
}

fn trim_quotes(value: &str) -> &str {
    value.trim_matches(|character| character == '"' || character == '\'' || character == '`')
}

fn starts_with_uppercase(value: &str) -> bool {
    value
        .chars()
        .next()
        .is_some_and(|character| character.is_uppercase())
}

#[cfg(test)]
mod tests {
    use super::parse_source;
    use crate::parser::{SourceLanguage, SymbolKind};

    #[test]
    fn extracts_typescript_symbols() {
        let parsed = parse_source(
            "src/example.ts",
            SourceLanguage::TypeScript,
            r#"
import { helper } from './helper';

export interface User {
  id: string;
}

type UserId = User['id'];

enum LoadState {
  Idle,
  Loading,
  Loaded,
}

export function loadUser(id: UserId) {
  return helper(id);
}

class UserRepository {}

const formatUser = function (user: User) {
  return user.id;
};
"#,
        )
        .expect("source should parse");

        assert_symbol(&parsed.symbols, "./helper", SymbolKind::Import);
        assert_symbol(&parsed.symbols, "User", SymbolKind::Interface);
        assert_symbol(&parsed.symbols, "UserId", SymbolKind::TypeAlias);
        assert_symbol(&parsed.symbols, "LoadState", SymbolKind::Enum);
        assert_symbol(&parsed.symbols, "loadUser", SymbolKind::Function);
        assert_symbol(&parsed.symbols, "UserRepository", SymbolKind::Class);
        assert_symbol(&parsed.symbols, "formatUser", SymbolKind::Function);
    }

    #[test]
    fn detects_react_component_arrow_functions_in_tsx() {
        let parsed = parse_source(
            "src/App.tsx",
            SourceLanguage::Tsx,
            r#"
export const App = () => {
  return <main>Hello</main>;
};
"#,
        )
        .expect("source should parse");

        assert_symbol(&parsed.symbols, "App", SymbolKind::Component);
        assert_symbol(&parsed.symbols, "App", SymbolKind::Export);
    }

    #[test]
    fn extracts_javascript_exports() {
        let parsed = parse_source(
            "src/math.js",
            SourceLanguage::JavaScript,
            r#"
export default function add(left, right) {
  return left + right;
}
"#,
        )
        .expect("source should parse");

        assert_symbol(&parsed.symbols, "add", SymbolKind::Export);
        assert_symbol_without_source(&parsed.symbols, "add", SymbolKind::Export);
        assert_symbol(&parsed.symbols, "add", SymbolKind::Function);
    }

    #[test]
    fn extracts_class_and_object_methods() {
        let parsed = parse_source(
            "src/services.ts",
            SourceLanguage::TypeScript,
            r#"
class UserService {
  loadUser() {}
  async saveUser() {}
}

export const api = {
  fetchUser() {},
  updateUser: () => {},
};
"#,
        )
        .expect("source should parse");

        assert_symbol(&parsed.symbols, "UserService", SymbolKind::Class);
        assert_symbol_with_parent(
            &parsed.symbols,
            "UserService.loadUser",
            SymbolKind::Method,
            "UserService",
        );
        assert_symbol_with_parent(
            &parsed.symbols,
            "UserService.saveUser",
            SymbolKind::Method,
            "UserService",
        );
        assert_symbol(&parsed.symbols, "api", SymbolKind::Object);
        assert_symbol_with_parent(&parsed.symbols, "api.fetchUser", SymbolKind::Method, "api");
        assert_symbol_with_parent(&parsed.symbols, "api.updateUser", SymbolKind::Method, "api");
    }

    #[test]
    fn extracts_named_exports_and_re_exports() {
        let parsed = parse_source(
            "src/index.ts",
            SourceLanguage::TypeScript,
            r#"
const loadUser = () => {};
const saveUser = () => {};

export { loadUser, saveUser };
export { Button, Card as LocalCard } from './components';
export * from './utils';
"#,
        )
        .expect("source should parse");

        assert_symbol(&parsed.symbols, "loadUser", SymbolKind::Export);
        assert_symbol(&parsed.symbols, "saveUser", SymbolKind::Export);
        assert_symbol_with_source(
            &parsed.symbols,
            "Button",
            SymbolKind::Export,
            "./components",
        );
        assert_symbol_with_source(
            &parsed.symbols,
            "LocalCard",
            SymbolKind::Export,
            "./components",
        );
        assert_symbol_with_source(&parsed.symbols, "*", SymbolKind::Export, "./utils");
    }

    #[test]
    fn extracts_import_source_and_named_imports() {
        let parsed = parse_source(
            "src/imports.ts",
            SourceLanguage::TypeScript,
            r#"
import React from 'react';
import { helper, loadUser as load } from './helper';
import * as utils from './utils';
"#,
        )
        .expect("source should parse");

        assert_symbol_with_source(&parsed.symbols, "react", SymbolKind::Import, "react");
        assert_symbol_with_source(&parsed.symbols, "helper", SymbolKind::Import, "./helper");
        assert_symbol_with_source(&parsed.symbols, "load", SymbolKind::Import, "./helper");
        assert_symbol_with_source(&parsed.symbols, "utils", SymbolKind::Import, "./utils");
    }

    fn assert_symbol(symbols: &[crate::parser::CodeSymbol], name: &str, kind: SymbolKind) {
        assert!(
            symbols
                .iter()
                .any(|symbol| symbol.name == name && symbol.kind == kind),
            "missing {kind:?} symbol named {name}; symbols: {symbols:?}"
        );
    }

    fn assert_symbol_with_parent(
        symbols: &[crate::parser::CodeSymbol],
        name: &str,
        kind: SymbolKind,
        parent: &str,
    ) {
        assert!(
            symbols.iter().any(|symbol| {
                symbol.name == name
                    && symbol.kind == kind
                    && symbol.parent.as_deref() == Some(parent)
            }),
            "missing {kind:?} symbol named {name} with parent {parent}; symbols: {symbols:?}"
        );
    }

    fn assert_symbol_with_source(
        symbols: &[crate::parser::CodeSymbol],
        name: &str,
        kind: SymbolKind,
        source: &str,
    ) {
        assert!(
            symbols.iter().any(|symbol| {
                symbol.name == name
                    && symbol.kind == kind
                    && symbol.source.as_deref() == Some(source)
            }),
            "missing {kind:?} symbol named {name} with source {source}; symbols: {symbols:?}"
        );
    }

    fn assert_symbol_without_source(
        symbols: &[crate::parser::CodeSymbol],
        name: &str,
        kind: SymbolKind,
    ) {
        assert!(
            symbols.iter().any(|symbol| symbol.name == name
                && symbol.kind == kind
                && symbol.source.is_none()),
            "missing {kind:?} symbol named {name} without source; symbols: {symbols:?}"
        );
    }
}
