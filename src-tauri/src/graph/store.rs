use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use kuzu::{Connection, Database, Error as KuzuError, LogicalType, SystemConfig, Value};
use serde::Serialize;
use thiserror::Error;

use crate::parser::{CodeSymbol, ParsedFile, SourceLanguage, SourceRange, SymbolKind};

use super::schema::{CREATE_CONTAINS_TABLE, CREATE_FILE_TABLE, CREATE_SYMBOL_TABLE};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GraphIngestSummary {
    pub file_path: String,
    pub language: String,
    pub symbol_count: usize,
    pub contains_count: usize,
    pub symbol_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GraphContext {
    pub path: String,
    pub relation: String,
    pub symbol: CodeSymbol,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GraphViewNode {
    pub id: String,
    pub label: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GraphViewEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GraphView {
    pub nodes: Vec<GraphViewNode>,
    pub edges: Vec<GraphViewEdge>,
}

pub struct GraphStore {
    database: Database,
}

#[derive(Debug, Error)]
pub enum GraphError {
    #[error("failed to create graph database directory: {0}")]
    CreateDirectory(#[from] std::io::Error),
    #[error("kuzudb query failed: {0}")]
    Kuzu(#[from] KuzuError),
    #[error("graph query returned invalid {field}: {value}")]
    InvalidValue { field: &'static str, value: String },
    #[error("system clock is before unix epoch")]
    SystemClock,
}

impl GraphStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, GraphError> {
        let path = path.as_ref();
        fs::create_dir_all(path)?;
        let database_path = path.join("localbrain.kuzu");
        let database = Database::new(
            database_path.to_string_lossy().as_ref(),
            SystemConfig::default(),
        )?;

        let store = Self { database };
        store.init_schema()?;
        Ok(store)
    }

    pub fn open_default<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<Self, GraphError> {
        use tauri::Manager;
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|_| GraphError::InvalidValue {
                field: "app_data_dir",
                value: "failed to resolve app data directory".to_string(),
            })?;

        let path = app_data_dir.join(".localbrain").join("graph");
        Self::open(path)
    }

    pub fn init_schema(&self) -> Result<(), GraphError> {
        let conn = self.connect()?;

        conn.query(CREATE_FILE_TABLE)?;
        conn.query(CREATE_SYMBOL_TABLE)?;
        conn.query(CREATE_CONTAINS_TABLE)?;

        Ok(())
    }

    pub fn upsert_parsed_file(
        &self,
        parsed: &ParsedFile,
    ) -> Result<GraphIngestSummary, GraphError> {
        let conn = self.connect()?;
        let updated_at = current_timestamp()?;
        let language = language_label(parsed.language).to_string();

        conn.query("BEGIN TRANSACTION")?;

        let result = (|| -> Result<GraphIngestSummary, GraphError> {
            self.clear_file_with_connection(&conn, &parsed.path)?;

            let mut upsert_file = conn.prepare(
                "
                MERGE (file:File {path: $path})
                ON CREATE SET file.language = $language, file.updated_at = $updated_at
                ON MATCH SET file.language = $language, file.updated_at = $updated_at
                ",
            )?;
            conn.execute(
                &mut upsert_file,
                vec![
                    ("path", parsed.path.clone().into()),
                    ("language", language.clone().into()),
                    ("updated_at", updated_at.into()),
                ],
            )?;

            let mut upsert_symbol = conn.prepare(
                "
                MERGE (symbol:Symbol {id: $id})
                ON CREATE SET
                  symbol.file_path = $file_path,
                  symbol.name = $name,
                  symbol.kind = $kind,
                  symbol.parent = $parent,
                  symbol.source = $source,
                  symbol.start_line = $start_line,
                  symbol.start_column = $start_column,
                  symbol.end_line = $end_line,
                  symbol.end_column = $end_column
                ON MATCH SET
                  symbol.file_path = $file_path,
                  symbol.name = $name,
                  symbol.kind = $kind,
                  symbol.parent = $parent,
                  symbol.source = $source,
                  symbol.start_line = $start_line,
                  symbol.start_column = $start_column,
                  symbol.end_line = $end_line,
                  symbol.end_column = $end_column
                ",
            )?;
            let mut link_contains = conn.prepare(
                "
                MATCH (file:File), (symbol:Symbol)
                WHERE file.path = $file_path AND symbol.id = $symbol_id
                MERGE (file)-[:CONTAINS]->(symbol)
                ",
            )?;

            for symbol in &parsed.symbols {
                let symbol_id = symbol_id(&parsed.path, symbol);
                conn.execute(
                    &mut upsert_symbol,
                    vec![
                        ("id", symbol_id.clone().into()),
                        ("file_path", parsed.path.clone().into()),
                        ("name", symbol.name.clone().into()),
                        ("kind", kind_label(symbol.kind).to_string().into()),
                        ("parent", optional_string(&symbol.parent)),
                        ("source", optional_string(&symbol.source)),
                        ("start_line", usize_to_i64(symbol.range.start_line).into()),
                        (
                            "start_column",
                            usize_to_i64(symbol.range.start_column).into(),
                        ),
                        ("end_line", usize_to_i64(symbol.range.end_line).into()),
                        ("end_column", usize_to_i64(symbol.range.end_column).into()),
                    ],
                )?;
                conn.execute(
                    &mut link_contains,
                    vec![
                        ("file_path", parsed.path.clone().into()),
                        ("symbol_id", symbol_id.into()),
                    ],
                )?;
            }

            let contains_count =
                self.count_contains_for_file_with_connection(&conn, &parsed.path)?;

            Ok(GraphIngestSummary {
                file_path: parsed.path.clone(),
                language,
                symbol_count: parsed.symbols.len(),
                contains_count,
                symbol_names: parsed
                    .symbols
                    .iter()
                    .map(|symbol| symbol.name.clone())
                    .collect(),
            })
        })();

        match result {
            Ok(summary) => {
                conn.query("COMMIT")?;
                Ok(summary)
            }
            Err(e) => {
                eprintln!("KuzuDB Upsert Error for {}: {:?}", parsed.path, e);
                let _ = conn.query("ROLLBACK");
                Err(e)
            }
        }
    }

    pub fn get_symbols_for_file(&self, path: &str) -> Result<Vec<CodeSymbol>, GraphError> {
        let conn = self.connect()?;
        let mut query = conn.prepare(
            "
            MATCH (:File {path: $path})-[:CONTAINS]->(symbol:Symbol)
            RETURN
              symbol.name,
              symbol.kind,
              symbol.parent,
              symbol.source,
              symbol.start_line,
              symbol.start_column,
              symbol.end_line,
              symbol.end_column
            ORDER BY symbol.start_line, symbol.start_column, symbol.name
            ",
        )?;
        let result = conn.execute(&mut query, vec![("path", path.to_string().into())])?;
        let mut symbols = Vec::new();

        for row in result {
            symbols.push(CodeSymbol {
                name: value_to_string(&row[0], "name")?,
                kind: value_to_symbol_kind(&row[1])?,
                parent: value_to_optional_string(&row[2], "parent")?,
                source: value_to_optional_string(&row[3], "source")?,
                range: SourceRange {
                    start_line: value_to_usize(&row[4], "start_line")?,
                    start_column: value_to_usize(&row[5], "start_column")?,
                    end_line: value_to_usize(&row[6], "end_line")?,
                    end_column: value_to_usize(&row[7], "end_column")?,
                },
            });
        }

        Ok(symbols)
    }

    pub fn get_graph_context(
        &self,
        path_or_symbol: &str,
        limit: usize,
    ) -> Result<Vec<GraphContext>, GraphError> {
        let normalized_limit = limit.max(1);
        let mut contexts = Vec::new();

        for symbol in self.get_symbols_for_file(path_or_symbol)? {
            contexts.push(GraphContext {
                path: path_or_symbol.to_string(),
                relation: "contains".to_string(),
                symbol,
            });
            if contexts.len() >= normalized_limit {
                return Ok(contexts);
            }
        }

        let conn = self.connect()?;
        let mut query = conn.prepare(
            "
            MATCH (symbol:Symbol)
            WHERE symbol.name = $name
            RETURN
              symbol.file_path,
              symbol.name,
              symbol.kind,
              symbol.parent,
              symbol.source,
              symbol.start_line,
              symbol.start_column,
              symbol.end_line,
              symbol.end_column
            ORDER BY symbol.file_path, symbol.start_line
            ",
        )?;
        let result = conn.execute(
            &mut query,
            vec![("name", path_or_symbol.to_string().into())],
        )?;

        for row in result {
            if contexts.len() >= normalized_limit {
                break;
            }

            contexts.push(GraphContext {
                path: value_to_string(&row[0], "file_path")?,
                relation: "matches_symbol".to_string(),
                symbol: CodeSymbol {
                    name: value_to_string(&row[1], "name")?,
                    kind: value_to_symbol_kind(&row[2])?,
                    parent: value_to_optional_string(&row[3], "parent")?,
                    source: value_to_optional_string(&row[4], "source")?,
                    range: SourceRange {
                        start_line: value_to_usize(&row[5], "start_line")?,
                        start_column: value_to_usize(&row[6], "start_column")?,
                        end_line: value_to_usize(&row[7], "end_line")?,
                        end_column: value_to_usize(&row[8], "end_column")?,
                    },
                },
            });
        }

        contexts.truncate(normalized_limit);
        Ok(contexts)
    }

    pub fn get_graph_view(&self, path: &str, limit: usize) -> Result<GraphView, GraphError> {
        let normalized_limit = limit.max(1);
        let symbols = self.get_symbols_for_file(path)?;
        let mut nodes = vec![GraphViewNode {
            id: path.to_string(),
            label: path.to_string(),
            kind: "file".to_string(),
        }];
        let mut edges = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();
        seen_ids.insert(path.to_string());

        for symbol in symbols.into_iter().take(normalized_limit) {
            let symbol_node_id = symbol_id(path, &symbol);
            let symbol_kind = visual_kind_for_symbol(&symbol);

            if !seen_ids.contains(&symbol_node_id) {
                nodes.push(GraphViewNode {
                    id: symbol_node_id.clone(),
                    label: symbol.name.clone(),
                    kind: symbol_kind.clone(),
                });
                seen_ids.insert(symbol_node_id.clone());
            }

            edges.push(GraphViewEdge {
                id: format!("{path}->{symbol_node_id}"),
                source: path.to_string(),
                target: symbol_node_id.clone(),
                label: "contains".to_string(),
            });

            // If it's an import, try to find the matching file or symbol
            if symbol.kind == SymbolKind::Import {
                if let Some(source) = &symbol.source {
                    // Basic heuristic: if source matches a local file path exactly or with extension
                    let possible_paths = vec![
                        source.clone(),
                        format!("{source}.ts"),
                        format!("{source}.tsx"),
                        format!("{source}.js"),
                        format!("src/{source}"),
                        format!("src/{source}.ts"),
                        format!("src/{source}.tsx"),
                    ];

                    let mut linked_local_file = false;
                    for p in possible_paths {
                        if self.file_exists(&p)? {
                            if !seen_ids.contains(&p) {
                                nodes.push(GraphViewNode {
                                    id: p.clone(),
                                    label: p.clone(),
                                    kind: "file".to_string(),
                                });
                                seen_ids.insert(p.clone());
                            }
                            edges.push(GraphViewEdge {
                                id: format!("{symbol_node_id}->{p}"),
                                source: symbol_node_id.clone(),
                                target: p,
                                label: "imports".to_string(),
                            });
                            linked_local_file = true;
                            break;
                        }
                    }

                    // Unresolved import path is likely an external package/library.
                    if !linked_local_file {
                        let external_id = format!("lib::{source}");
                        if !seen_ids.contains(&external_id) {
                            nodes.push(GraphViewNode {
                                id: external_id.clone(),
                                label: source.clone(),
                                kind: "external_library".to_string(),
                            });
                            seen_ids.insert(external_id.clone());
                        }
                        edges.push(GraphViewEdge {
                            id: format!("{symbol_node_id}->{external_id}"),
                            source: symbol_node_id.clone(),
                            target: external_id,
                            label: "imports".to_string(),
                        });
                    }
                }
            }
        }

        Ok(GraphView { nodes, edges })
    }

    pub fn file_exists(&self, path: &str) -> Result<bool, GraphError> {
        let conn = self.connect()?;
        let mut query = conn.prepare("MATCH (f:File {path: $path}) RETURN COUNT(f)")?;
        let mut result = conn.execute(&mut query, vec![("path", path.to_string().into())])?;

        match result.next().and_then(|row| row.into_iter().next()) {
            Some(value) => Ok(value_to_usize(&value, "file_exists")? > 0),
            None => Ok(false),
        }
    }

    pub fn clear_file(&self, path: &str) -> Result<(), GraphError> {
        let conn = self.connect()?;
        self.clear_file_with_connection(&conn, path)
    }

    fn connect(&self) -> Result<Connection<'_>, GraphError> {
        Ok(Connection::new(&self.database)?)
    }

    fn clear_file_with_connection(
        &self,
        conn: &Connection<'_>,
        path: &str,
    ) -> Result<(), GraphError> {
        let mut delete_contains = conn.prepare(
            "
            MATCH (:File {path: $path})-[contains:CONTAINS]->(:Symbol)
            DELETE contains
            ",
        )?;
        conn.execute(
            &mut delete_contains,
            vec![("path", path.to_string().into())],
        )?;

        let mut delete_symbols = conn.prepare(
            "
            MATCH (symbol:Symbol)
            WHERE symbol.file_path = $path
            DELETE symbol
            ",
        )?;
        conn.execute(&mut delete_symbols, vec![("path", path.to_string().into())])?;

        let mut delete_file = conn.prepare(
            "
            MATCH (file:File {path: $path})
            DELETE file
            ",
        )?;
        conn.execute(&mut delete_file, vec![("path", path.to_string().into())])?;

        Ok(())
    }

    fn count_contains_for_file_with_connection(
        &self,
        conn: &Connection<'_>,
        path: &str,
    ) -> Result<usize, GraphError> {
        let mut query = conn.prepare(
            "
            MATCH (:File {path: $path})-[contains:CONTAINS]->(:Symbol)
            RETURN COUNT(contains)
            ",
        )?;
        let mut result = conn.execute(&mut query, vec![("path", path.to_string().into())])?;

        match result.next().and_then(|row| row.into_iter().next()) {
            Some(value) => value_to_usize(&value, "contains_count"),
            None => Ok(0),
        }
    }
}

fn visual_kind_for_symbol(symbol: &CodeSymbol) -> String {
    if symbol.kind == SymbolKind::Import {
        return "import".to_string();
    }

    if symbol.kind == SymbolKind::Function
        && symbol.name.starts_with("use")
        && symbol
            .name
            .chars()
            .nth(3)
            .is_some_and(|character| character.is_ascii_uppercase())
    {
        return "hook".to_string();
    }

    kind_label(symbol.kind).to_string()
}

fn current_timestamp() -> Result<String, GraphError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| GraphError::SystemClock)?
        .as_millis()
        .to_string())
}

fn symbol_id(file_path: &str, symbol: &CodeSymbol) -> String {
    format!(
        "{}::{}::{}::{}::{}",
        file_path,
        kind_label(symbol.kind),
        symbol.name,
        symbol.range.start_line,
        symbol.range.start_column
    )
}

fn language_label(language: SourceLanguage) -> &'static str {
    match language {
        SourceLanguage::JavaScript => "javascript",
        SourceLanguage::TypeScript => "typescript",
        SourceLanguage::Tsx => "tsx",
        SourceLanguage::Jsx => "jsx",
        SourceLanguage::Rust => "rust",
        SourceLanguage::Go => "go",
        SourceLanguage::Python => "python",
        SourceLanguage::Java => "java",
        SourceLanguage::Kotlin => "kotlin",
        SourceLanguage::Swift => "swift",
        SourceLanguage::Ruby => "ruby",
        SourceLanguage::Php => "php",
        SourceLanguage::C => "c",
        SourceLanguage::Cpp => "cpp",
        SourceLanguage::CSharp => "csharp",
        SourceLanguage::Shell => "shell",
        SourceLanguage::Sql => "sql",
        SourceLanguage::Json => "json",
        SourceLanguage::Yaml => "yaml",
        SourceLanguage::Toml => "toml",
        SourceLanguage::Ini => "ini",
        SourceLanguage::Xml => "xml",
        SourceLanguage::Css => "css",
        SourceLanguage::Scss => "scss",
        SourceLanguage::Less => "less",
        SourceLanguage::Vue => "vue",
        SourceLanguage::Svelte => "svelte",
        SourceLanguage::Astro => "astro",
    }
}

fn kind_label(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Function => "function",
        SymbolKind::Component => "component",
        SymbolKind::Class => "class",
        SymbolKind::Method => "method",
        SymbolKind::Object => "object",
        SymbolKind::Enum => "enum",
        SymbolKind::Interface => "interface",
        SymbolKind::TypeAlias => "type_alias",
        SymbolKind::Import => "import",
        SymbolKind::Export => "export",
    }
}

fn kind_from_label(value: &str) -> Option<SymbolKind> {
    match value {
        "function" => Some(SymbolKind::Function),
        "component" => Some(SymbolKind::Component),
        "class" => Some(SymbolKind::Class),
        "method" => Some(SymbolKind::Method),
        "object" => Some(SymbolKind::Object),
        "enum" => Some(SymbolKind::Enum),
        "interface" => Some(SymbolKind::Interface),
        "type_alias" => Some(SymbolKind::TypeAlias),
        "import" => Some(SymbolKind::Import),
        "export" => Some(SymbolKind::Export),
        _ => None,
    }
}

fn optional_string(value: &Option<String>) -> Value {
    value
        .clone()
        .map(Value::String)
        .unwrap_or(Value::Null(LogicalType::String))
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn value_to_string(value: &Value, field: &'static str) -> Result<String, GraphError> {
    match value {
        Value::String(value) => Ok(value.clone()),
        other => Err(GraphError::InvalidValue {
            field,
            value: other.to_string(),
        }),
    }
}

fn value_to_optional_string(
    value: &Value,
    field: &'static str,
) -> Result<Option<String>, GraphError> {
    match value {
        Value::String(value) => Ok(Some(value.clone())),
        Value::Null(_) => Ok(None),
        other => Err(GraphError::InvalidValue {
            field,
            value: other.to_string(),
        }),
    }
}

fn value_to_symbol_kind(value: &Value) -> Result<SymbolKind, GraphError> {
    let label = value_to_string(value, "kind")?;

    kind_from_label(&label).ok_or(GraphError::InvalidValue {
        field: "kind",
        value: label,
    })
}

fn value_to_usize(value: &Value, field: &'static str) -> Result<usize, GraphError> {
    match value {
        Value::Int64(value) => usize::try_from(*value).map_err(|_| GraphError::InvalidValue {
            field,
            value: value.to_string(),
        }),
        other => Err(GraphError::InvalidValue {
            field,
            value: other.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::GraphStore;
    use crate::parser::{CodeSymbol, ParsedFile, SourceLanguage, SourceRange, SymbolKind};

    #[test]
    fn initializes_schema_without_error() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let store = GraphStore::open(temp_dir.path()).expect("graph store should open");

        store.init_schema().expect("schema should initialize");
        store.init_schema().expect("schema should initialize twice");
    }

    #[test]
    fn upsert_is_idempotent_and_reads_symbols_back() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let store = GraphStore::open(temp_dir.path()).expect("graph store should open");
        let parsed = parsed_file();

        store
            .upsert_parsed_file(&parsed)
            .expect("first ingest should work");
        let summary = store
            .upsert_parsed_file(&parsed)
            .expect("second ingest should work");
        let symbols = store
            .get_symbols_for_file("src/App.tsx")
            .expect("symbols should read back");

        assert_eq!(summary.symbol_count, 2);
        assert_eq!(summary.contains_count, 2);
        assert_eq!(symbols.len(), 2);
        assert!(symbols
            .iter()
            .any(|symbol| symbol.name == "App" && symbol.kind == SymbolKind::Component));
    }

    #[test]
    fn clear_file_removes_symbols_and_contains_relationships() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let store = GraphStore::open(temp_dir.path()).expect("graph store should open");
        let parsed = parsed_file();

        store
            .upsert_parsed_file(&parsed)
            .expect("ingest should work");
        store.clear_file("src/App.tsx").expect("clear should work");

        let symbols = store
            .get_symbols_for_file("src/App.tsx")
            .expect("query should work");

        assert!(symbols.is_empty());
    }

    #[test]
    fn symbol_id_uses_consistent_separators() {
        let symbol = CodeSymbol {
            name: "App:Shell".to_string(),
            kind: SymbolKind::Component,
            parent: None,
            source: None,
            range: SourceRange {
                start_line: 3,
                start_column: 16,
                end_line: 5,
                end_column: 1,
            },
        };

        assert_eq!(
            super::symbol_id("src/App.tsx", &symbol),
            "src/App.tsx::component::App:Shell::3::16"
        );
    }

    fn parsed_file() -> ParsedFile {
        ParsedFile {
            path: "src/App.tsx".to_string(),
            language: SourceLanguage::Tsx,
            symbols: vec![
                CodeSymbol {
                    name: "react".to_string(),
                    kind: SymbolKind::Import,
                    parent: None,
                    source: Some("react".to_string()),
                    range: SourceRange {
                        start_line: 1,
                        start_column: 0,
                        end_line: 1,
                        end_column: 26,
                    },
                },
                CodeSymbol {
                    name: "App".to_string(),
                    kind: SymbolKind::Component,
                    parent: None,
                    source: None,
                    range: SourceRange {
                        start_line: 3,
                        start_column: 16,
                        end_line: 5,
                        end_column: 1,
                    },
                },
            ],
        }
    }
}
