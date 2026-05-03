use notify_debouncer_mini::{
    new_debouncer,
    notify::{RecommendedWatcher, RecursiveMode},
    DebounceEventResult, Debouncer,
};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub struct WatcherState {
    pub debouncer: Arc<Mutex<Option<Debouncer<RecommendedWatcher>>>>,
}

impl WatcherState {
    pub fn new() -> Self {
        Self {
            debouncer: Arc::new(Mutex::new(None)),
        }
    }
}

#[tauri::command]
pub fn start_watcher(
    path: String,
    app: AppHandle,
    state: tauri::State<'_, WatcherState>,
) -> Result<(), String> {
    let path_to_watch = Path::new(&path);
    if !path_to_watch.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    let app_clone = app.clone();
    let watch_root = path_to_watch.canonicalize().map_err(|e| e.to_string())?;
    let mut debouncer_guard = state.debouncer.lock().map_err(|e| e.to_string())?;

    let mut debouncer = new_debouncer(
        Duration::from_millis(50),
        move |res: DebounceEventResult| match res {
            Ok(events) => {
                for event in events {
                    if !should_emit_path(&event.path) {
                        continue;
                    }

                    let relative_path = event.path.strip_prefix(&watch_root).unwrap_or(&event.path);
                    let path_str = relative_path.to_string_lossy().to_string();

                    let _ = app_clone.emit("file-changed", path_str);
                }
            }
            Err(e) => eprintln!("Watcher error: {:?}", e),
        },
    )
    .map_err(|e| e.to_string())?;

    debouncer
        .watcher()
        .watch(path_to_watch, RecursiveMode::Recursive)
        .map_err(|e| e.to_string())?;

    *debouncer_guard = Some(debouncer);

    Ok(())
}

fn should_emit_path(path: &Path) -> bool {
    if path.components().any(|component| {
        let value = component.as_os_str().to_string_lossy();
        matches!(
            value.as_ref(),
            ".git" | "node_modules" | "target" | "dist" | ".ssh"
        )
    }) {
        return false;
    }

    if path.file_name().is_some_and(|file_name| {
        let value = file_name.to_string_lossy();
        value == ".DS_Store" || value.starts_with(".env")
    }) {
        return false;
    }

    path.extension().is_some_and(|extension| {
        matches!(
            extension.to_string_lossy().as_ref(),
            "ts" | "tsx" | "rs" | "py" | "md" | "java" | "html"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::should_emit_path;
    use std::path::Path;

    #[test]
    fn allows_supported_source_files() {
        assert!(should_emit_path(Path::new("src/App.tsx")));
        assert!(should_emit_path(Path::new("src-tauri/src/main.rs")));
        assert!(should_emit_path(Path::new("docs/wiki/auth.md")));
    }

    #[test]
    fn ignores_noise_and_secret_paths() {
        assert!(!should_emit_path(Path::new("node_modules/pkg/index.ts")));
        assert!(!should_emit_path(Path::new(".git/config")));
        assert!(!should_emit_path(Path::new(".env.local")));
        assert!(!should_emit_path(Path::new(".ssh/config")));
        assert!(!should_emit_path(Path::new("dist/assets/index.js")));
    }

    #[test]
    fn ignores_unsupported_extensions() {
        assert!(!should_emit_path(Path::new("src/styles.css")));
        assert!(!should_emit_path(Path::new("README")));
    }
}
