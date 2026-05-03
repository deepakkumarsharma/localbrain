mod commands;
mod watcher;

use commands::get_app_version;
use watcher::{start_watcher, WatcherState};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(WatcherState::new())
        .invoke_handler(tauri::generate_handler![get_app_version, start_watcher])
        .run(tauri::generate_context!())
        .expect("failed to run Localbrain");
}
