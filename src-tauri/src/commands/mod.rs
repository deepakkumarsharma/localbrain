use crate::graph::{GraphIngestSummary, GraphStore};
use crate::parser::CodeSymbol;
use crate::parser::{parse_file, ParsedFile};

#[tauri::command]
pub fn get_app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[tauri::command]
pub fn parse_source_file(path: String) -> Result<ParsedFile, String> {
    parse_file(path).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn index_file_to_graph(path: String) -> Result<GraphIngestSummary, String> {
    let parsed = parse_file(path).map_err(|error| error.to_string())?;
    let store = GraphStore::open_default().map_err(|error| error.to_string())?;

    store
        .upsert_parsed_file(&parsed)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_graph_symbols(path: String) -> Result<Vec<CodeSymbol>, String> {
    let store = GraphStore::open_default().map_err(|error| error.to_string())?;

    store
        .get_symbols_for_file(&path)
        .map_err(|error| error.to_string())
}
