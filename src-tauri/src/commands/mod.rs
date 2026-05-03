use crate::parser::{parse_file, ParsedFile};

#[tauri::command]
pub fn get_app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[tauri::command]
pub fn parse_source_file(path: String) -> Result<ParsedFile, String> {
    parse_file(path).map_err(|error| error.to_string())
}
