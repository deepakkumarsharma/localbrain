mod commands;
mod graph;
mod parser;
mod watcher;

use commands::{get_app_version, get_graph_symbols, index_file_to_graph, parse_source_file};
use tauri::Manager;
use watcher::{start_watcher, WatcherState};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(WatcherState::new())
        .setup(|app| {
            let store = graph::GraphStore::open_default(&app.handle())
                .expect("failed to open graph store");
            app.manage(store);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_version,
            get_graph_symbols,
            index_file_to_graph,
            parse_source_file,
            start_watcher
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Localbrain");
}
