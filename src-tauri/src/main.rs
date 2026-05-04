mod commands;
mod embeddings;
mod graph;
mod indexer;
mod metadata;
mod parser;
mod search;
mod watcher;
mod wiki;

use commands::{
    check_file_changed, generate_wiki, get_app_version, get_file_metadata, get_graph_symbols,
    get_index_status, hybrid_search, index_file, index_file_to_graph, index_path,
    parse_source_file, rebuild_search_index, record_file_metadata, search_code,
};
use tauri::Manager;
use watcher::{start_watcher, WatcherState};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(WatcherState::new())
        .setup(|app| {
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
            get_app_version,
            get_file_metadata,
            get_graph_symbols,
            get_index_status,
            generate_wiki,
            hybrid_search,
            index_file,
            index_file_to_graph,
            index_path,
            parse_source_file,
            rebuild_search_index,
            record_file_metadata,
            search_code,
            start_watcher
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Local Brain");
}
