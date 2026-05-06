mod api;
mod commands;
mod embeddings;
mod graph;
mod indexer;
mod llm;
mod logging;
mod metadata;
mod parser;
mod search;
mod settings;
mod watcher;
mod wiki;

use api::{get_agent_api_status, start_agent_api, stop_agent_api, AgentApiState};
use commands::{
    ask_local, check_file_changed, clear_search_index, generate_wiki, get_app_version,
    get_file_metadata, get_graph_context, get_graph_symbols, get_graph_view, get_index_status,
    get_local_llm_status, get_provider_settings, get_wiki_content, hybrid_search, index_file,
    index_file_to_graph, index_path, parse_source_file, rebuild_search_index, record_file_metadata,
    resolve_project_root, search_code, set_local_model_path, set_provider, set_workspace_root,
    start_local_llm, stop_local_llm,
};
use settings::SettingsStore;
use tauri::{Manager, RunEvent};
use watcher::{start_watcher, WatcherState};

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(WatcherState::new())
        .manage(AgentApiState::new())
        .manage(SettingsStore::new())
        .manage(llm::local::LocalLlmState::new())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "failed to resolve app data dir",
                )
            })?;
            let log_dir = app_data_dir.join(".localbrain").join("logs");
            logging::init_local_logging(log_dir)?;
            let store = graph::GraphStore::open_default(app.handle())?;
            app.manage(store);
            let metadata_store = tauri::async_runtime::block_on(
                metadata::MetadataStore::open_default(app.handle()),
            )?;
            app.manage(metadata_store);
            let settings_store = app.state::<SettingsStore>();
            settings_store.load_from_disk(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            check_file_changed,
            ask_local,
            get_agent_api_status,
            get_app_version,
            get_file_metadata,
            get_graph_context,
            get_graph_symbols,
            get_graph_view,
            get_index_status,
            get_provider_settings,
            get_wiki_content,
            generate_wiki,
            hybrid_search,
            index_file,
            index_file_to_graph,
            index_path,
            parse_source_file,
            clear_search_index,
            rebuild_search_index,
            record_file_metadata,
            resolve_project_root,
            search_code,
            get_local_llm_status,
            set_local_model_path,
            set_provider,
            set_workspace_root,
            start_agent_api,
            start_local_llm,
            stop_local_llm,
            stop_agent_api,
            start_watcher
        ])
        .build(tauri::generate_context!())
        .expect("failed to build Local Brain");

    app.run(|app_handle, event| {
        if let RunEvent::Exit = event {
            let llm_state = app_handle.state::<llm::local::LocalLlmState>();
            if let Err(error) = llm_state.kill_child_if_running() {
                eprintln!("Failed to stop llama-server on app exit: {error}");
            }
            let settings_store = app_handle.state::<SettingsStore>();
            if let Err(error) = settings_store.set_local_model_path(app_handle, None) {
                eprintln!("Failed to clear local model path on app exit: {error}");
            }
        }
    });
}
