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
