use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseSchema {
    pub provider: String,
    pub source: String,
    pub sources: Vec<String>,
    pub tables: Vec<DatabaseTable>,
    pub relationships: Vec<DatabaseRelationship>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseTable {
    pub name: String,
    pub columns: Vec<DatabaseColumn>,
    pub primary_keys: Vec<String>,
    pub indexes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseColumn {
    pub name: String,
    pub data_type: String,
    pub is_primary_key: bool,
    pub is_unique: bool,
    pub is_nullable: bool,
    pub default_value: Option<String>,
    pub references_table: Option<String>,
    pub references_column: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseRelationship {
    pub from_table: String,
    pub from_column: String,
    pub to_table: String,
    pub to_column: String,
    pub kind: String,
}

pub fn detect_and_parse(workspace_root: &Path) -> Result<Option<DatabaseSchema>, String> {
    let direct_prisma_candidates = [
        workspace_root.join("prisma").join("schema.prisma"),
        workspace_root.join("schema.prisma"),
    ];
    for path in direct_prisma_candidates {
        if path.is_file() {
            return parse_prisma_schema(&path, workspace_root).map(Some);
        }
    }

    let mut first_prisma: Option<PathBuf> = None;
    let mut sql_candidates = Vec::new();
    let mut model_candidates = Vec::new();
    let mut inferred_provider = None;

    for entry in WalkDir::new(workspace_root)
        .follow_links(false)
        .max_depth(6)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if should_skip_path(path) || !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let file_name_lower = file_name.to_ascii_lowercase();

        if first_prisma.is_none()
            && (file_name_lower == "schema.prisma" || file_name_lower.ends_with(".prisma"))
        {
            first_prisma = Some(path.to_path_buf());
            continue;
        }

        if file_name_lower.ends_with(".sql") && is_sql_schema_candidate(path, &file_name_lower) {
            sql_candidates.push(path.to_path_buf());
            continue;
        }

        if file_name_lower.ends_with(".model.ts")
            || file_name_lower.ends_with(".model.js")
            || (path
                .to_string_lossy()
                .to_ascii_lowercase()
                .contains("/database/")
                && (file_name_lower.ends_with(".ts") || file_name_lower.ends_with(".js")))
        {
            model_candidates.push(path.to_path_buf());
            continue;
        }

        if inferred_provider.is_none()
            && (file_name_lower == ".env"
                || file_name_lower.starts_with(".env.")
                || file_name_lower == "application.yml"
                || file_name_lower == "application.yaml"
                || file_name_lower == "application.properties"
                || file_name_lower == "docker-compose.yml"
                || file_name_lower == "docker-compose.yaml")
        {
            if let Ok(content) = fs::read_to_string(path) {
                inferred_provider = infer_provider_from_text(&content);
            }
        }
    }

    if let Some(path) = first_prisma {
        return parse_prisma_schema(&path, workspace_root).map(Some);
    }

    if !sql_candidates.is_empty() {
        let mut parsed_schemas = Vec::new();
        for path in sql_candidates {
            let parsed = parse_sql_schema(&path, workspace_root)?;
            if !parsed.tables.is_empty() {
                parsed_schemas.push(parsed);
            }
        }
        if !parsed_schemas.is_empty() {
            return Ok(Some(merge_sql_schemas(workspace_root, parsed_schemas)));
        }
    }

    if !model_candidates.is_empty() {
        let mut tables = Vec::new();
        let mut relationships = Vec::new();
        let mut sources = Vec::new();
        for path in model_candidates {
            if let Some((table, rels)) = parse_mongoose_model(&path)? {
                sources.push(relative_path_str(workspace_root, &path));
                tables.push(table);
                relationships.extend(rels);
            }
        }
        if !tables.is_empty() {
            return Ok(Some(DatabaseSchema {
                provider: "mongodb".to_string(),
                source: relative_path_str(workspace_root, workspace_root),
                sources,
                tables,
                relationships,
            }));
        }
    }

    if let Some(provider) = inferred_provider {
        return Ok(Some(DatabaseSchema {
            provider,
            source: relative_path_str(workspace_root, workspace_root),
            sources: Vec::new(),
            tables: Vec::new(),
            relationships: Vec::new(),
        }));
    }

    Ok(None)
}

fn relative_path_str(workspace_root: &Path, path: &Path) -> String {
    path.strip_prefix(workspace_root)
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}

fn should_skip_path(path: &Path) -> bool {
    const BLOCKED: &[&str] = &[
        ".git",
        ".hg",
        ".svn",
        "node_modules",
        "target",
        "dist",
        "build",
        ".next",
        ".nuxt",
        ".cache",
        ".idea",
        ".vscode",
        "vendor",
        "coverage",
    ];
    path.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        BLOCKED
            .iter()
            .any(|blocked| name.eq_ignore_ascii_case(blocked))
    })
}

fn is_sql_schema_candidate(path: &Path, file_name_lower: &str) -> bool {
    if file_name_lower.contains("schema")
        || file_name_lower.contains("migration")
        || file_name_lower.contains("init")
    {
        return true;
    }

    let path_lower = path.to_string_lossy().to_ascii_lowercase();
    path_lower.contains("/migrations/")
        || path_lower.contains("\\migrations\\")
        || path_lower.contains("/database/")
        || path_lower.contains("\\database\\")
        || path_lower.contains("/db/")
        || path_lower.contains("\\db\\")
}

fn parse_prisma_schema(path: &Path, workspace_root: &Path) -> Result<DatabaseSchema, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read Prisma schema '{}': {error}", path.display()))?;

    let provider = parse_provider(&content).unwrap_or_else(|| "postgresql".to_string());
    let mut tables = Vec::new();
    let mut relationships = Vec::new();

    let lines: Vec<&str> = content.lines().collect();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index].trim();
        if !line.starts_with("model ") || !line.ends_with('{') {
            index += 1;
            continue;
        }

        let model_name = line
            .trim_start_matches("model ")
            .trim_end_matches('{')
            .trim()
            .to_string();

        index += 1;
        let mut columns = Vec::new();
        let mut primary_keys = Vec::new();
        let mut indexes = Vec::new();

        while index < lines.len() {
            let field_line = lines[index].trim();
            if field_line == "}" {
                break;
            }
            if field_line.is_empty() || field_line.starts_with("//") {
                index += 1;
                continue;
            }

            if field_line.starts_with("@@") {
                indexes.push(field_line.to_string());
                index += 1;
                continue;
            }

            if let Some(column) = parse_field_line(field_line, &model_name, &mut relationships) {
                if column.is_primary_key {
                    primary_keys.push(column.name.clone());
                }
                columns.push(column);
            }

            index += 1;
        }

        tables.push(DatabaseTable {
            name: model_name,
            columns,
            primary_keys,
            indexes,
        });
        index += 1;
    }

    infer_missing_relationship_columns(&mut tables, &mut relationships);

    Ok(DatabaseSchema {
        provider,
        source: relative_path_str(workspace_root, path),
        sources: vec![relative_path_str(workspace_root, path)],
        tables,
        relationships,
    })
}

fn parse_sql_schema(path: &Path, workspace_root: &Path) -> Result<DatabaseSchema, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read SQL schema '{}': {error}", path.display()))?;

    let provider = infer_provider_from_text(&content).unwrap_or_else(|| "sql".to_string());
    let mut tables = Vec::new();
    let mut relationships = Vec::new();

    let mut lines = content.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if !trimmed.to_ascii_lowercase().starts_with("create table") {
            continue;
        }

        let table_name = extract_sql_table_name(trimmed);
        if table_name.is_empty() {
            continue;
        }

        let mut columns = Vec::new();
        let mut primary_keys = Vec::new();
        let mut indexes = Vec::new();

        while let Some(def_line) = lines.peek().copied() {
            let definition = def_line.trim().trim_end_matches(',');
            let definition_lower = definition.to_ascii_lowercase();
            if definition.starts_with(");") || definition == ")" {
                lines.next();
                break;
            }

            if definition_lower.starts_with("primary key") {
                primary_keys.extend(extract_sql_constraint_columns(definition));
                lines.next();
                continue;
            }

            if is_table_constraint_definition(&definition_lower)
            {
                if let Some((local_col, ref_table, ref_col)) =
                    extract_sql_foreign_key(definition, &table_name)
                {
                    relationships.push(DatabaseRelationship {
                        from_table: table_name.clone(),
                        from_column: local_col,
                        to_table: ref_table,
                        to_column: ref_col,
                        kind: "many-to-one".to_string(),
                    });
                }
                indexes.push(definition.to_string());
                lines.next();
                continue;
            }

            if let Some(column) = parse_sql_column(definition) {
                if column.is_primary_key {
                    primary_keys.push(column.name.clone());
                }
                columns.push(column);
            }
            lines.next();
        }

        tables.push(DatabaseTable {
            name: table_name,
            columns,
            primary_keys,
            indexes,
        });
    }

    Ok(DatabaseSchema {
        provider,
        source: relative_path_str(workspace_root, path),
        sources: vec![relative_path_str(workspace_root, path)],
        tables,
        relationships,
    })
}

fn is_table_constraint_definition(definition_lower: &str) -> bool {
    definition_lower.starts_with("constraint ")
        || definition_lower.starts_with("foreign key")
        || definition_lower.starts_with("unique")
        || definition_lower.starts_with("check")
        || definition_lower.starts_with("exclude")
        || definition_lower.starts_with("index")
        || definition_lower.starts_with("key ")
}

fn merge_sql_schemas(workspace_root: &Path, schemas: Vec<DatabaseSchema>) -> DatabaseSchema {
    let mut provider = "sql".to_string();
    let mut sources = Vec::new();
    let mut table_map: BTreeMap<String, DatabaseTable> = BTreeMap::new();
    let mut relationships = Vec::new();
    let mut relationship_keys = BTreeMap::new();

    for schema in schemas {
        if provider == "sql" && schema.provider != "sql" {
            provider = schema.provider.clone();
        }
        for source in schema.sources {
            if !sources.contains(&source) {
                sources.push(source);
            }
        }
        for table in schema.tables {
            table_map
                .entry(table.name.clone())
                .and_modify(|existing| {
                    for column in &table.columns {
                        if !existing.columns.iter().any(|col| col.name == column.name) {
                            existing.columns.push(column.clone());
                        }
                    }
                    for key in &table.primary_keys {
                        if !existing.primary_keys.contains(key) {
                            existing.primary_keys.push(key.clone());
                        }
                    }
                    for index in &table.indexes {
                        if !existing.indexes.contains(index) {
                            existing.indexes.push(index.clone());
                        }
                    }
                })
                .or_insert(table);
        }
        for rel in schema.relationships {
            let key = format!(
                "{}:{}:{}:{}:{}",
                rel.from_table, rel.from_column, rel.to_table, rel.to_column, rel.kind
            );
            if relationship_keys.insert(key, ()).is_none() {
                relationships.push(rel);
            }
        }
    }

    DatabaseSchema {
        provider,
        source: relative_path_str(workspace_root, workspace_root),
        sources,
        tables: table_map.into_values().collect(),
        relationships,
    }
}

fn extract_sql_table_name(line: &str) -> String {
    let lowered = line.to_ascii_lowercase();
    let Some(idx) = lowered.find("create table") else {
        return String::new();
    };
    let mut candidate = line[idx + "create table".len()..].trim();
    if candidate.to_ascii_lowercase().starts_with("if not exists") {
        candidate = candidate["if not exists".len()..].trim();
    }
    candidate
        .trim_start_matches('"')
        .trim_start_matches('`')
        .trim_start_matches('[')
        .split([' ', '('])
        .next()
        .unwrap_or_default()
        .trim_end_matches('"')
        .trim_end_matches('`')
        .trim_end_matches(']')
        .to_string()
}

fn parse_sql_column(definition: &str) -> Option<DatabaseColumn> {
    let mut parts = definition.split_whitespace();
    let name = parts
        .next()?
        .trim_matches('"')
        .trim_matches('`')
        .trim_matches('[')
        .trim_matches(']')
        .to_string();
    let data_type = parts.next()?.to_string();
    if name.is_empty() || name.eq_ignore_ascii_case("constraint") {
        return None;
    }

    let lowered = definition.to_ascii_lowercase();
    Some(DatabaseColumn {
        name,
        data_type,
        is_primary_key: lowered.contains("primary key"),
        is_unique: lowered.contains(" unique"),
        is_nullable: !lowered.contains("not null"),
        default_value: extract_sql_default(definition),
        references_table: None,
        references_column: None,
    })
}

fn extract_sql_default(definition: &str) -> Option<String> {
    let lowered = definition.to_ascii_lowercase();
    let idx = lowered.find(" default ")?;
    Some(definition[idx + " default ".len()..].trim().to_string())
}

fn extract_sql_constraint_columns(definition: &str) -> Vec<String> {
    let Some(start) = definition.find('(') else {
        return Vec::new();
    };
    let Some(end) = definition[start + 1..].find(')') else {
        return Vec::new();
    };
    definition[start + 1..start + 1 + end]
        .split(',')
        .map(str::trim)
        .map(|column| {
            column
                .trim_matches('"')
                .trim_matches('`')
                .trim_matches('[')
                .trim_matches(']')
                .to_string()
        })
        .filter(|column| !column.is_empty())
        .collect()
}

fn extract_sql_foreign_key(definition: &str, table_name: &str) -> Option<(String, String, String)> {
    let lowered = definition.to_ascii_lowercase();
    let fk_idx = lowered.find("foreign key")?;
    let fk_part = &definition[fk_idx..];
    let columns = extract_sql_constraint_columns(fk_part);
    let local_col = columns.first()?.clone();

    let references_idx = lowered.find("references ")?;
    let references_part = definition[references_idx + "references ".len()..].trim();
    let target_table = references_part
        .split([' ', '('])
        .next()
        .unwrap_or(table_name)
        .trim_matches('"')
        .trim_matches('`')
        .trim_matches('[')
        .trim_matches(']')
        .to_string();
    let target_columns = extract_sql_constraint_columns(references_part);
    let target_col = target_columns
        .first()
        .cloned()
        .unwrap_or_else(|| "id".to_string());

    Some((local_col, target_table, target_col))
}

fn infer_provider_from_text(content: &str) -> Option<String> {
    let lowered = content.to_ascii_lowercase();
    if lowered.contains("postgres://") || lowered.contains("postgresql://") {
        return Some("postgresql".to_string());
    }
    if lowered.contains("mysql://") {
        return Some("mysql".to_string());
    }
    if lowered.contains("mariadb://") {
        return Some("mariadb".to_string());
    }
    if lowered.contains("mongodb://") || lowered.contains("mongodb+srv://") {
        return Some("mongodb".to_string());
    }
    if lowered.contains("sqlite://") || lowered.contains("jdbc:sqlite:") || lowered.contains(".db")
    {
        return Some("sqlite".to_string());
    }
    if lowered.contains("sqlserver://") || lowered.contains("mssql://") {
        return Some("sqlserver".to_string());
    }
    None
}

fn parse_provider(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("provider") {
            continue;
        }
        let provider = trimmed
            .split('=')
            .nth(1)
            .map(|value| value.trim().trim_matches('"').to_string());
        if provider.is_some() {
            return provider;
        }
    }
    None
}

fn parse_field_line(
    field_line: &str,
    current_model: &str,
    relationships: &mut Vec<DatabaseRelationship>,
) -> Option<DatabaseColumn> {
    let mut parts = field_line.split_whitespace();
    let name = parts.next()?.to_string();
    let raw_type = parts.next()?.to_string();
    if name.starts_with("@@") {
        return None;
    }

    let attributes = field_line
        .split('@')
        .skip(1)
        .map(|chunk| format!("@{}", chunk.trim()))
        .collect::<Vec<_>>();

    let is_nullable = raw_type.ends_with('?');
    let data_type = raw_type.trim_end_matches('?').to_string();

    let is_primary_key = attributes
        .iter()
        .any(|attribute| attribute.starts_with("@id"));
    let is_unique = attributes
        .iter()
        .any(|attribute| attribute.starts_with("@unique") || attribute.starts_with("@@unique"));

    let default_value = attributes
        .iter()
        .find(|attribute| attribute.starts_with("@default("))
        .map(|attribute| {
            attribute
                .trim_start_matches("@default(")
                .trim_end_matches(')')
                .to_string()
        });

    let mut references_table = None;
    let mut references_column = None;

    if let Some(relation_attribute) = attributes
        .iter()
        .find(|attribute| attribute.starts_with("@relation("))
    {
        let relation_body = relation_attribute
            .trim_start_matches("@relation(")
            .trim_end_matches(')');

        let mut relation_fields = Vec::new();
        let mut relation_references = Vec::new();

        for segment in relation_body.split(',') {
            let trimmed = segment.trim();
            if trimmed.starts_with("fields:") {
                relation_fields = parse_bracket_list(trimmed.trim_start_matches("fields:").trim());
            }
            if trimmed.starts_with("references:") {
                relation_references =
                    parse_bracket_list(trimmed.trim_start_matches("references:").trim());
            }
        }

        if !relation_fields.is_empty() && !relation_references.is_empty() {
            let target_table = data_type.clone();
            let target_column = relation_references
                .first()
                .cloned()
                .unwrap_or_else(|| "id".to_string());
            for local_column in relation_fields {
                relationships.push(DatabaseRelationship {
                    from_table: current_model.to_string(),
                    from_column: local_column.clone(),
                    to_table: target_table.clone(),
                    to_column: target_column.clone(),
                    kind: "many-to-one".to_string(),
                });
            }
            references_table = Some(target_table);
            references_column = Some(target_column);
        }
    }

    Some(DatabaseColumn {
        name,
        data_type,
        is_primary_key,
        is_unique,
        is_nullable,
        default_value,
        references_table,
        references_column,
    })
}

fn parse_bracket_list(value: &str) -> Vec<String> {
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_mongoose_model(
    path: &Path,
) -> Result<Option<(DatabaseTable, Vec<DatabaseRelationship>)>, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read model file '{}': {error}", path.display()))?;
    if !(content.contains("mongoose") || content.contains("Schema")) {
        return Ok(None);
    }

    let table_name = extract_model_name(&content, path);
    let mut columns = Vec::new();
    let mut relationships = Vec::new();

    if let Some((start, end)) = find_interface_block(&content) {
        let block = &content[start..end];
        for raw_line in block.lines() {
            let line = raw_line.trim().trim_end_matches(';');
            if line.is_empty() || line.starts_with("export interface") || line == "}" {
                continue;
            }
            let Some((name_raw, type_raw)) = line.split_once(':') else {
                continue;
            };
            let name = name_raw.trim().trim_end_matches('?').to_string();
            if name.is_empty() {
                continue;
            }
            let is_nullable = name_raw.contains('?');
            let data_type = type_raw.trim().to_string();
            columns.push(DatabaseColumn {
                name: name.clone(),
                data_type: data_type.clone(),
                is_primary_key: name.eq_ignore_ascii_case("id") || name.eq_ignore_ascii_case("_id"),
                is_unique: false,
                is_nullable,
                default_value: None,
                references_table: None,
                references_column: None,
            });
        }
    }

    if let Some((start, end)) = find_schema_block(&content) {
        let block = &content[start..end];
        for rel in extract_mongoose_relationships(block, &table_name) {
            if let Some(column) = columns
                .iter_mut()
                .find(|column| column.name == rel.from_column)
            {
                column.references_table = Some(rel.to_table.clone());
                column.references_column = Some(rel.to_column.clone());
            }
            relationships.push(rel);
        }
    }

    if columns.is_empty() {
        return Ok(None);
    }

    Ok(Some((
        DatabaseTable {
            name: table_name,
            columns,
            primary_keys: Vec::new(),
            indexes: Vec::new(),
        },
        relationships,
    )))
}

fn extract_model_name(content: &str, path: &Path) -> String {
    if let Some(model_pos) = content.find("model(") {
        let rest = &content[model_pos + "model(".len()..];
        let trimmed = rest.trim_start();
        if let Some(quote) = trimmed
            .chars()
            .next()
            .filter(|ch| *ch == '\'' || *ch == '"')
        {
            let after_quote = &trimmed[1..];
            if let Some(end) = after_quote.find(quote) {
                let candidate = after_quote[..end].trim();
                if !candidate.is_empty() {
                    return candidate.to_string();
                }
            }
        }
    }

    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.replace(".model.ts", "").replace(".model.js", ""))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Model".to_string())
}

fn find_interface_block(content: &str) -> Option<(usize, usize)> {
    let start = content.find("export interface ")?;
    let open = content[start..].find('{')? + start;
    find_brace_block(content, open).map(|(block_start, block_end)| (block_start + 1, block_end))
}

fn find_schema_block(content: &str) -> Option<(usize, usize)> {
    let schema_pos = content.find("new Schema(")?;
    let open = content[schema_pos..].find('{')? + schema_pos;
    find_brace_block(content, open).map(|(block_start, block_end)| (block_start + 1, block_end))
}

fn find_brace_block(content: &str, open_brace_index: usize) -> Option<(usize, usize)> {
    let mut depth = 0usize;
    let mut index = open_brace_index;
    let bytes = content.as_bytes();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_backtick = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    while index < bytes.len() {
        let ch = bytes[index] as char;
        let next = bytes.get(index + 1).copied().map(char::from);
        let prev = if index > 0 {
            Some(bytes[index - 1] as char)
        } else {
            None
        };

        if in_line_comment {
            if ch == '\n' {
                in_line_comment = false;
            }
            index += 1;
            continue;
        }
        if in_block_comment {
            if ch == '*' && next == Some('/') {
                in_block_comment = false;
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }
        if !in_single_quote && !in_double_quote && !in_backtick {
            if ch == '/' && next == Some('/') {
                in_line_comment = true;
                index += 2;
                continue;
            }
            if ch == '/' && next == Some('*') {
                in_block_comment = true;
                index += 2;
                continue;
            }
        }

        if !in_double_quote && !in_backtick && ch == '\'' && prev != Some('\\') {
            in_single_quote = !in_single_quote;
            index += 1;
            continue;
        }
        if !in_single_quote && !in_backtick && ch == '"' && prev != Some('\\') {
            in_double_quote = !in_double_quote;
            index += 1;
            continue;
        }
        if !in_single_quote && !in_double_quote && ch == '`' && prev != Some('\\') {
            in_backtick = !in_backtick;
            index += 1;
            continue;
        }

        if !in_single_quote && !in_double_quote && !in_backtick {
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some((open_brace_index, index));
                }
            }
        }
        index += 1;
    }
    None
}

fn extract_mongoose_relationships(block: &str, table_name: &str) -> Vec<DatabaseRelationship> {
    let mut relationships = Vec::new();
    let mut current_field: Option<String> = None;
    for raw_line in block.lines() {
        let line = raw_line.trim();
        if let Some((field, _)) = line.split_once(':') {
            let field_name = field.trim().trim_matches('"').trim_matches('\'');
            if !field_name.is_empty()
                && field_name != "type"
                && field_name != "ref"
                && field_name != "required"
                && field_name != "default"
            {
                current_field = Some(field_name.to_string());
            }
        }
        if line.contains("ref:") {
            let target = line.split("ref:").nth(1).map(str::trim).and_then(|value| {
                let value = value.trim_start_matches('"').trim_start_matches('\'');
                let end = value.find(['"', '\'', ',', '}']).unwrap_or(value.len());
                let ref_name = value[..end].trim();
                if ref_name.is_empty() {
                    None
                } else {
                    Some(ref_name.to_string())
                }
            });
            if let (Some(from_column), Some(to_table)) = (current_field.clone(), target) {
                relationships.push(DatabaseRelationship {
                    from_table: table_name.to_string(),
                    from_column,
                    to_table,
                    to_column: "_id".to_string(),
                    kind: "many-to-one".to_string(),
                });
            }
        }
    }
    relationships
}

fn infer_missing_relationship_columns(
    tables: &mut [DatabaseTable],
    relationships: &mut Vec<DatabaseRelationship>,
) {
    let mut table_columns: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for table in tables.iter() {
        table_columns.insert(
            table.name.clone(),
            table
                .columns
                .iter()
                .map(|column| column.name.clone())
                .collect(),
        );
    }

    relationships.retain(|relationship| {
        table_columns
            .get(&relationship.from_table)
            .is_some_and(|columns| columns.contains(&relationship.from_column))
    });
}

#[cfg(test)]
mod tests {
    use super::detect_and_parse;
    use std::fs;

    #[test]
    fn detects_and_parses_prisma_schema() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let prisma_dir = temp_dir.path().join("prisma");
        fs::create_dir_all(&prisma_dir).expect("prisma dir should be created");
        fs::write(
            prisma_dir.join("schema.prisma"),
            r#"
            datasource db {
              provider = "postgresql"
              url      = env("DATABASE_URL")
            }

            model User {
              id    Int    @id
              email String @unique
              posts Post[]
            }

            model Post {
              id      Int  @id
              userId  Int
              user    User @relation(fields: [userId], references: [id])
              title   String
            }
            "#,
        )
        .expect("schema should be written");

        let detected = detect_and_parse(temp_dir.path())
            .expect("detection should succeed")
            .expect("schema should be detected");

        assert_eq!(detected.provider, "postgresql");
        assert_eq!(detected.sources.len(), 1);
        assert_eq!(detected.tables.len(), 2);
        assert!(detected.relationships.iter().any(|relationship| {
            relationship.from_table == "Post" && relationship.from_column == "userId"
        }));
    }

    #[test]
    fn detects_and_parses_sql_schema() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let migrations_dir = temp_dir.path().join("db").join("migrations");
        fs::create_dir_all(&migrations_dir).expect("migrations dir should be created");
        fs::write(
            migrations_dir.join("001_init.sql"),
            r#"
            CREATE TABLE users (
              id INTEGER PRIMARY KEY,
              email TEXT NOT NULL UNIQUE
            );

            CREATE TABLE posts (
              id INTEGER PRIMARY KEY,
              user_id INTEGER NOT NULL,
              title TEXT,
              FOREIGN KEY (user_id) REFERENCES users(id)
            );
            "#,
        )
        .expect("sql schema should be written");

        let detected = detect_and_parse(temp_dir.path())
            .expect("detection should succeed")
            .expect("schema should be detected");

        assert_eq!(detected.provider, "sql");
        assert_eq!(detected.sources.len(), 1);
        assert_eq!(detected.tables.len(), 2);
        assert_eq!(detected.relationships.len(), 1);
        assert_eq!(detected.relationships[0].from_table, "posts");
        assert_eq!(detected.relationships[0].to_table, "users");
    }

    #[test]
    fn detects_provider_from_env_when_schema_is_missing() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        fs::write(
            temp_dir.path().join(".env"),
            "DATABASE_URL=postgresql://user:pass@localhost:5432/app\n",
        )
        .expect("env should be written");

        let detected = detect_and_parse(temp_dir.path())
            .expect("detection should succeed")
            .expect("provider should be inferred");

        assert_eq!(detected.provider, "postgresql");
        assert_eq!(detected.sources.len(), 0);
        assert!(detected.tables.is_empty());
    }

    #[test]
    fn merges_multiple_sql_migrations() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let migrations_dir = temp_dir.path().join("db").join("migrations");
        fs::create_dir_all(&migrations_dir).expect("migrations dir should be created");
        fs::write(
            migrations_dir.join("0001_users.sql"),
            r#"
            CREATE TABLE users (
              id INTEGER PRIMARY KEY,
              email TEXT NOT NULL UNIQUE
            );
            "#,
        )
        .expect("users migration should be written");
        fs::write(
            migrations_dir.join("0002_posts.sql"),
            r#"
            CREATE TABLE posts (
              id INTEGER PRIMARY KEY,
              user_id INTEGER NOT NULL,
              FOREIGN KEY (user_id) REFERENCES users(id)
            );
            "#,
        )
        .expect("posts migration should be written");

        let detected = detect_and_parse(temp_dir.path())
            .expect("detection should succeed")
            .expect("schema should be detected");

        assert_eq!(detected.sources.len(), 2);
        assert_eq!(detected.tables.len(), 2);
        assert_eq!(detected.relationships.len(), 1);
    }

    #[test]
    fn detects_sql_schema_in_database_folder_without_special_filename() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let database_dir = temp_dir.path().join("database");
        fs::create_dir_all(&database_dir).expect("database dir should be created");
        fs::write(
            database_dir.join("tables.sql"),
            r#"
            CREATE TABLE users (
              id INTEGER PRIMARY KEY,
              email TEXT NOT NULL UNIQUE
            );
            "#,
        )
        .expect("sql schema should be written");

        let detected = detect_and_parse(temp_dir.path())
            .expect("detection should succeed")
            .expect("schema should be detected");

        assert_eq!(detected.tables.len(), 1);
        assert_eq!(detected.tables[0].name, "users");
    }

    #[test]
    fn detects_mongoose_models_in_database_folder() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let database_dir = temp_dir.path().join("database");
        fs::create_dir_all(&database_dir).expect("database dir should be created");
        fs::write(
            database_dir.join("answer.model.ts"),
            r#"
            import { Schema, models, model, Document } from 'mongoose';
            export interface IAnswer extends Document {
              author: Schema.Types.ObjectId;
              content: string;
            }
            const AnswerSchema = new Schema({
              author: { type: Schema.Types.ObjectId, ref: 'User', required: true },
              content: { type: String, required: true },
            });
            const Answer = models.Answer || model('Answer', AnswerSchema);
            export default Answer;
            "#,
        )
        .expect("model should be written");

        let detected = detect_and_parse(temp_dir.path())
            .expect("detection should succeed")
            .expect("schema should be detected");

        assert_eq!(detected.provider, "mongodb");
        assert_eq!(detected.tables.len(), 1);
        assert_eq!(detected.tables[0].name, "Answer");
        assert!(detected.relationships.iter().any(|relationship| {
            relationship.from_table == "Answer"
                && relationship.from_column == "author"
                && relationship.to_table == "User"
        }));
    }
}
