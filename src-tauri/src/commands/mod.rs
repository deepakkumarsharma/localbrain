use crate::graph::{GraphContext, GraphIngestSummary, GraphStore, GraphView};
use crate::indexer::{IndexFileSummary, IndexPathSummary, IndexProgress};
use crate::llm::ChatAnswer;
use crate::metadata::{FileChangeStatus, FileMetadata, IndexRunSummary, MetadataStore};
use crate::parser::CodeSymbol;
use crate::parser::{parse_file_with_display_path, ParsedFile};
use crate::search::{SearchIndexSummary, SearchResult};
use crate::settings::{LlmProvider, ProviderSettings, SettingsStore};
use crate::wiki::WikiSummary;
use std::path::{Path, PathBuf};
use tauri::Emitter;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexProgressEvent {
    pub run_id: Option<u64>,
    #[serde(flatten)]
    pub progress: IndexProgress,
}

#[tauri::command]
pub fn get_app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[tauri::command]
pub fn detect_database_structure(
    path: String,
) -> Result<Option<crate::database::DatabaseSchema>, String> {
    let workspace_path = PathBuf::from(path);
    let canonical = workspace_path
        .canonicalize()
        .map_err(|error| format!("failed to resolve workspace path: {error}"))?;
    if !canonical.is_dir() {
        return Err("workspace path is not a directory".to_string());
    }
    crate::database::detect_and_parse(&canonical)
}

#[tauri::command]
pub fn parse_source_file(
    path: String,
    metadata_store: tauri::State<'_, MetadataStore>,
) -> Result<ParsedFile, String> {
    let source_path = metadata_store
        .resolve_path(&path)
        .map_err(|error| error.to_string())?;
    let display_path = metadata_store.normalize_path(&path);
    parse_file_with_display_path(source_path, &display_path).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn index_file_to_graph(
    path: String,
    store: tauri::State<GraphStore>,
    metadata_store: tauri::State<'_, MetadataStore>,
) -> Result<GraphIngestSummary, String> {
    let source_path = metadata_store
        .resolve_path(&path)
        .map_err(|error| error.to_string())?;
    let display_path = metadata_store.normalize_path(&path);
    let parsed = parse_file_with_display_path(source_path, &display_path)
        .map_err(|error| error.to_string())?;

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
pub fn get_graph_context(
    target: String,
    limit: Option<usize>,
    store: tauri::State<GraphStore>,
) -> Result<Vec<GraphContext>, String> {
    store
        .get_graph_context(&target, limit.unwrap_or(24))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_graph_view(
    path: String,
    limit: Option<usize>,
    store: tauri::State<GraphStore>,
    metadata_store: tauri::State<'_, MetadataStore>,
) -> Result<GraphView, String> {
    let display_path = metadata_store.normalize_path(&path);
    store
        .get_graph_view(&display_path, limit.unwrap_or(40))
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
    run_id: Option<u64>,
    window: tauri::Window,
    metadata_store: tauri::State<'_, MetadataStore>,
    graph_store: tauri::State<'_, GraphStore>,
) -> Result<IndexPathSummary, String> {
    crate::indexer::index_path_with_progress(path, &metadata_store, &graph_store, |progress| {
        let _ = window.emit("index-progress", IndexProgressEvent { run_id, progress });
    })
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
pub async fn clear_search_index(
    metadata_store: tauri::State<'_, MetadataStore>,
) -> Result<(), String> {
    crate::search::clear_search_index(&metadata_store)
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

#[tauri::command]
pub async fn ask_local(
    query: String,
    active_path: Option<String>,
    app: tauri::AppHandle,
    metadata_store: tauri::State<'_, MetadataStore>,
    graph_store: tauri::State<'_, GraphStore>,
) -> Result<ChatAnswer, String> {
    crate::llm::ask_local(
        &query,
        active_path.as_deref(),
        &metadata_store,
        &graph_store,
        &app,
    )
    .await
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn start_local_llm(app: tauri::AppHandle) -> Result<(), String> {
    crate::llm::local::start_llama_server(&app).await
}

#[tauri::command]
pub async fn stop_local_llm(app: tauri::AppHandle) -> Result<(), String> {
    crate::llm::local::stop_llama_server(&app).await
}

#[tauri::command]
pub async fn get_local_llm_status(app: tauri::AppHandle) -> bool {
    crate::llm::local::get_llm_running_status(&app).await
}

#[tauri::command]
pub async fn get_wiki_content(
    path: String,
    metadata_store: tauri::State<'_, MetadataStore>,
) -> Result<Option<String>, String> {
    crate::wiki::get_wiki_content(path, &metadata_store)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_provider_settings(
    settings_store: tauri::State<SettingsStore>,
) -> Result<ProviderSettings, String> {
    settings_store.get()
}

#[tauri::command]
pub fn set_provider(
    app: tauri::AppHandle,
    provider: LlmProvider,
    cloud_enabled: bool,
    settings_store: tauri::State<SettingsStore>,
) -> Result<ProviderSettings, String> {
    settings_store.set_provider(&app, provider, cloud_enabled)
}

#[tauri::command]
pub fn set_local_model_path(
    app: tauri::AppHandle,
    path: Option<String>,
    settings_store: tauri::State<SettingsStore>,
) -> Result<ProviderSettings, String> {
    settings_store.set_local_model_path(&app, path)
}

#[tauri::command]
pub fn set_embedding_model_path(
    app: tauri::AppHandle,
    path: Option<String>,
    settings_store: tauri::State<SettingsStore>,
) -> Result<ProviderSettings, String> {
    settings_store.set_embedding_model_path(&app, path)
}

#[tauri::command]
pub fn set_last_project_path(
    app: tauri::AppHandle,
    path: Option<String>,
    settings_store: tauri::State<SettingsStore>,
) -> Result<ProviderSettings, String> {
    settings_store.set_last_project_path(&app, path)
}

#[tauri::command]
pub fn set_workspace_root(
    path: String,
    metadata_store: tauri::State<'_, MetadataStore>,
) -> Result<String, String> {
    metadata_store
        .set_workspace_root(path)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn resolve_project_root(path: String) -> Result<String, String> {
    let selected = PathBuf::from(path);
    let canonical = selected
        .canonicalize()
        .map_err(|error| format!("failed to resolve selected folder: {error}"))?;
    if !canonical.is_dir() {
        return Err("selected path is not a directory".to_string());
    }

    let home_dir = std::env::var_os("HOME").map(PathBuf::from);
    let selected_score = score_project_root_candidate(&canonical);
    let mut best = canonical.clone();
    let mut best_score = selected_score;

    for ancestor in canonical.ancestors().take(MAX_ANCESTOR_DEPTH) {
        if home_dir.as_deref() == Some(ancestor) {
            break;
        }
        if !ancestor.is_dir() {
            continue;
        }
        let score = score_project_root_candidate(ancestor);
        if score > best_score {
            best = ancestor.to_path_buf();
            best_score = score;
        }
    }

    let should_promote =
        best != canonical && (best_score >= selected_score + 40 || is_suspicious_leaf(&canonical));

    if should_promote
        && !is_git_only_candidate(&best)
        && home_dir.as_deref() != Some(best.as_path())
    {
        Ok(best.to_string_lossy().to_string())
    } else {
        Ok(canonical.to_string_lossy().to_string())
    }
}

fn score_project_root_candidate(path: &Path) -> i32 {
    let mut score = 0;
    let leaf = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_lowercase();

    if is_suspicious_name(&leaf) {
        score -= 25;
    }
    if leaf == "node_modules" {
        score -= 100;
    }

    if path.join(".git").exists() {
        score += 100;
    }

    for marker in PROJECT_MARKERS {
        if path.join(marker).exists() {
            score += 30;
        }
    }

    if path.join("README.md").exists() {
        score += 5;
    }

    score
}

fn is_git_only_candidate(path: &Path) -> bool {
    path.join(".git").exists()
        && !PROJECT_MARKERS
            .iter()
            .any(|marker| path.join(marker).exists())
        && !path.join("README.md").exists()
}

fn is_suspicious_leaf(path: &Path) -> bool {
    let leaf = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_lowercase();
    is_suspicious_name(&leaf)
}

fn is_suspicious_name(name: &str) -> bool {
    matches!(
        name,
        "output"
            | "outputs"
            | "report"
            | "reports"
            | "dist"
            | "build"
            | "target"
            | "coverage"
            | "tmp"
            | "temp"
            | "node_modules"
    )
}
const MAX_ANCESTOR_DEPTH: usize = 7;
const PROJECT_MARKERS: [&str; 10] = [
    "package.json",
    "go.mod",
    "Cargo.toml",
    "pyproject.toml",
    "requirements.txt",
    "setup.py",
    "Gemfile",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
];
