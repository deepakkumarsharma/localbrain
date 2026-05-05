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
    ask_local, check_file_changed, generate_wiki, get_app_version, get_file_metadata,
    get_graph_context, get_graph_symbols, get_graph_view, get_index_status, get_provider_settings,
    hybrid_search, index_file, index_file_to_graph, index_path, parse_source_file,
    rebuild_search_index, record_file_metadata, search_code, set_provider,
};
use settings::SettingsStore;
use tauri::Manager;
use watcher::{start_watcher, WatcherState};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(WatcherState::new())
        .manage(AgentApiState::new())
        .manage(SettingsStore::new())
        .setup(|app| {
            let log_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir")
                .join(".localbrain")
                .join("logs");
            logging::init_local_logging(log_dir).expect("failed to initialize local logging");
            let store =
                graph::GraphStore::open_default(app.handle()).expect("failed to open graph store");
            app.manage(store);
            let metadata_store =
                tauri::async_runtime::block_on(metadata::MetadataStore::open_default(app.handle()))
                    .expect("failed to open metadata store");
            app.manage(metadata_store);
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
            generate_wiki,
            hybrid_search,
            index_file,
            index_file_to_graph,
            index_path,
            parse_source_file,
            rebuild_search_index,
            record_file_metadata,
            search_code,
            set_provider,
            start_agent_api,
            stop_agent_api,
            start_watcher
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Local Brain");
}
