use crate::graph::{GraphIngestSummary, GraphStore};
use crate::indexer::{IndexFileSummary, IndexPathSummary};
use crate::metadata::{FileChangeStatus, FileMetadata, IndexRunSummary, MetadataStore};
use crate::parser::CodeSymbol;
use crate::parser::{parse_file, ParsedFile};
use crate::search::{SearchIndexSummary, SearchResult};
use crate::wiki::WikiSummary;

#[tauri::command]
pub fn get_app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[tauri::command]
pub fn parse_source_file(path: String) -> Result<ParsedFile, String> {
    parse_file(path).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn index_file_to_graph(
    path: String,
    store: tauri::State<GraphStore>,
) -> Result<GraphIngestSummary, String> {
    let parsed = parse_file(path).map_err(|error| error.to_string())?;

    store
        .upsert_parsed_file(&parsed)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_graph_symbols(
    path: String,
    store: tauri::State<GraphStore>,
) -> Result<Vec<CodeSymbol>, String> {
    store
        .get_symbols_for_file(&path)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn record_file_metadata(
    path: String,
    store: tauri::State<'_, MetadataStore>,
) -> Result<FileMetadata, String> {
    store
        .record_file_metadata(path)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_file_metadata(
    path: String,
    store: tauri::State<'_, MetadataStore>,
) -> Result<Option<FileMetadata>, String> {
    store
        .get_file(&path)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn check_file_changed(
    path: String,
    store: tauri::State<'_, MetadataStore>,
) -> Result<FileChangeStatus, String> {
    store
        .classify_file(path)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn index_file(
    path: String,
    metadata_store: tauri::State<'_, MetadataStore>,
    graph_store: tauri::State<'_, GraphStore>,
) -> Result<IndexFileSummary, String> {
    crate::indexer::index_file(path, &metadata_store, &graph_store)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn index_path(
    path: String,
    metadata_store: tauri::State<'_, MetadataStore>,
    graph_store: tauri::State<'_, GraphStore>,
) -> Result<IndexPathSummary, String> {
    crate::indexer::index_path(path, &metadata_store, &graph_store)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_index_status(
    metadata_store: tauri::State<'_, MetadataStore>,
) -> Result<Option<IndexRunSummary>, String> {
    crate::indexer::get_index_status(&metadata_store)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn generate_wiki(
    path: String,
    metadata_store: tauri::State<'_, MetadataStore>,
    graph_store: tauri::State<'_, GraphStore>,
) -> Result<WikiSummary, String> {
    crate::wiki::generate_wiki(path, &metadata_store, &graph_store)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn rebuild_search_index(
    path: String,
    metadata_store: tauri::State<'_, MetadataStore>,
) -> Result<SearchIndexSummary, String> {
    crate::search::rebuild_search_index(path, &metadata_store)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn search_code(
    query: String,
    limit: Option<usize>,
    metadata_store: tauri::State<'_, MetadataStore>,
) -> Result<Vec<SearchResult>, String> {
    crate::search::search_text(&metadata_store, &query, limit.unwrap_or(10))
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn hybrid_search(
    query: String,
    limit: Option<usize>,
    metadata_store: tauri::State<'_, MetadataStore>,
) -> Result<Vec<SearchResult>, String> {
    crate::search::hybrid_search(&metadata_store, &query, limit.unwrap_or(10))
        .await
        .map_err(|error| error.to_string())
}
