mod commands;
mod graph;
mod parser;
mod watcher;

use commands::{get_app_version, get_graph_symbols, index_file_to_graph, parse_source_file};
use watcher::{start_watcher, WatcherState};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(WatcherState::new())
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
