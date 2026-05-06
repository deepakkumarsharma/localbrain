use crate::graph::{GraphContext, GraphIngestSummary, GraphStore, GraphView};
use crate::indexer::{IndexFileSummary, IndexPathSummary};
use crate::llm::ChatAnswer;
use crate::metadata::{FileChangeStatus, FileMetadata, IndexRunSummary, MetadataStore};
use crate::parser::CodeSymbol;
use crate::parser::{parse_file_with_display_path, ParsedFile};
use crate::search::{SearchIndexSummary, SearchResult};
use crate::settings::{LlmProvider, ProviderSettings, SettingsStore};
use crate::wiki::WikiSummary;
use std::path::{Path, PathBuf};

#[tauri::command]
pub fn get_app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
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

    let selected_score = score_project_root_candidate(&canonical);
    let mut best = canonical.clone();
    let mut best_score = selected_score;

    for ancestor in canonical.ancestors().take(7) {
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

    if should_promote {
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

    for marker in [
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
    ] {
        if path.join(marker).exists() {
            score += 30;
        }
    }

    if path.join("README.md").exists() {
        score += 5;
    }

    score
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

#[cfg(test)]
mod tests {
    use super::{is_suspicious_leaf, is_suspicious_name, resolve_project_root, score_project_root_candidate};
    use std::fs;
    use std::path::Path;

    // --- is_suspicious_name ---

    #[test]
    fn suspicious_names_are_detected() {
        for name in ["output", "outputs", "dist", "build", "target", "coverage", "tmp", "temp", "node_modules", "report", "reports"] {
            assert!(is_suspicious_name(name), "{name} should be suspicious");
        }
    }

    #[test]
    fn non_suspicious_names_are_not_detected() {
        for name in ["src", "lib", "app", "core", "main", "api", "backend", "frontend"] {
            assert!(!is_suspicious_name(name), "{name} should not be suspicious");
        }
    }

    #[test]
    fn suspicious_name_check_is_case_sensitive() {
        // is_suspicious_name takes already-lowercased input from score_project_root_candidate
        assert!(is_suspicious_name("dist"));
        // uppercase is NOT suspicious since the caller lowercases first
        assert!(!is_suspicious_name("Dist"));
    }

    // --- is_suspicious_leaf ---

    #[test]
    fn suspicious_leaf_detects_build_dir() {
        let temp = tempfile::tempdir().expect("temp dir");
        let build_dir = temp.path().join("build");
        fs::create_dir(&build_dir).expect("create build dir");
        assert!(is_suspicious_leaf(&build_dir));
    }

    #[test]
    fn non_suspicious_leaf_returns_false() {
        let temp = tempfile::tempdir().expect("temp dir");
        let src_dir = temp.path().join("src");
        fs::create_dir(&src_dir).expect("create src dir");
        assert!(!is_suspicious_leaf(&src_dir));
    }

    #[test]
    fn suspicious_leaf_with_path_having_no_filename_returns_false() {
        assert!(!is_suspicious_leaf(Path::new("/")));
    }

    // --- score_project_root_candidate ---

    #[test]
    fn score_increases_with_git_dir() {
        let temp = tempfile::tempdir().expect("temp dir");
        let score_without_git = score_project_root_candidate(temp.path());
        fs::create_dir(temp.path().join(".git")).expect("create .git");
        let score_with_git = score_project_root_candidate(temp.path());
        assert!(score_with_git > score_without_git, "git dir should increase score");
    }

    #[test]
    fn score_increases_with_package_json() {
        let temp = tempfile::tempdir().expect("temp dir");
        let baseline = score_project_root_candidate(temp.path());
        fs::write(temp.path().join("package.json"), "{}").expect("write package.json");
        let with_marker = score_project_root_candidate(temp.path());
        assert!(with_marker > baseline, "package.json should increase score");
    }

    #[test]
    fn score_increases_with_cargo_toml() {
        let temp = tempfile::tempdir().expect("temp dir");
        let baseline = score_project_root_candidate(temp.path());
        fs::write(temp.path().join("Cargo.toml"), "[package]").expect("write Cargo.toml");
        let with_marker = score_project_root_candidate(temp.path());
        assert!(with_marker > baseline);
    }

    #[test]
    fn score_decreases_for_suspicious_dir_name() {
        let temp = tempfile::tempdir().expect("temp dir");
        let dist_dir = temp.path().join("dist");
        fs::create_dir(&dist_dir).expect("create dist dir");
        let suspicious_score = score_project_root_candidate(&dist_dir);
        // dist is suspicious so score should be negative or lower than base
        assert!(suspicious_score < 0, "dist should get negative score");
    }

    #[test]
    fn score_decreases_heavily_for_node_modules() {
        let temp = tempfile::tempdir().expect("temp dir");
        let nm_dir = temp.path().join("node_modules");
        fs::create_dir(&nm_dir).expect("create node_modules dir");
        let nm_score = score_project_root_candidate(&nm_dir);
        // node_modules gets -25 (suspicious) + -100 (explicit) = -125
        assert!(nm_score <= -100, "node_modules should have very negative score");
    }

    #[test]
    fn score_includes_readme_bonus() {
        let temp = tempfile::tempdir().expect("temp dir");
        let baseline = score_project_root_candidate(temp.path());
        fs::write(temp.path().join("README.md"), "# Project").expect("write README");
        let with_readme = score_project_root_candidate(temp.path());
        assert_eq!(with_readme - baseline, 5, "README.md should add exactly 5 points");
    }

    // --- resolve_project_root ---

    #[test]
    fn resolve_project_root_returns_canonical_path_for_simple_dir() {
        let temp = tempfile::tempdir().expect("temp dir");
        let result = resolve_project_root(temp.path().to_string_lossy().to_string())
            .expect("should resolve successfully");
        // The returned path should be a real canonical path
        assert!(std::path::Path::new(&result).is_dir(), "result should be a directory");
    }

    #[test]
    fn resolve_project_root_fails_for_nonexistent_path() {
        let result = resolve_project_root("/nonexistent/path/xyz_abc".to_string());
        assert!(result.is_err(), "nonexistent path should fail");
    }

    #[test]
    fn resolve_project_root_prefers_ancestor_with_git_over_suspicious_leaf() {
        let temp = tempfile::tempdir().expect("temp dir");
        // Create project root with .git
        fs::create_dir(temp.path().join(".git")).expect("create .git");
        // Create a suspicious subdirectory
        let dist_dir = temp.path().join("dist");
        fs::create_dir(&dist_dir).expect("create dist dir");

        let result = resolve_project_root(dist_dir.to_string_lossy().to_string())
            .expect("should resolve");
        // The canonical parent (with .git) should be preferred over dist/
        let result_path = std::path::PathBuf::from(&result);
        // Should not end in "dist" - should have been promoted to the git root
        assert_ne!(
            result_path.file_name().and_then(|n| n.to_str()),
            Some("dist"),
            "dist leaf should be promoted to ancestor with .git"
        );
    }

    #[test]
    fn resolve_project_root_keeps_non_suspicious_dir_unchanged() {
        let temp = tempfile::tempdir().expect("temp dir");
        let src_dir = temp.path().join("src");
        fs::create_dir(&src_dir).expect("create src dir");

        let result = resolve_project_root(src_dir.to_string_lossy().to_string())
            .expect("should resolve");
        let result_path = std::path::PathBuf::from(&result);
        // src is not suspicious and parent has no markers that would score 40+ more
        assert_eq!(
            result_path.file_name().and_then(|n| n.to_str()),
            Some("src"),
            "non-suspicious src dir should not be promoted"
        );
    }
}
