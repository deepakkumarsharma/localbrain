use crate::metadata::MetadataStore;
use crate::parser::language_from_extension;
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
    metadata_store: tauri::State<'_, MetadataStore>,
) -> Result<(), String> {
    let path_to_watch = metadata_store
        .resolve_path(&path)
        .map_err(|error| error.to_string())?;
    if !path_to_watch.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    let app_clone = app.clone();
    let metadata_store_clone = metadata_store.inner().clone();
    let watch_root = path_to_watch.canonicalize().map_err(|e| e.to_string())?;
    println!("Watcher starting for root: {:?}", watch_root);

    let mut debouncer_guard = state.debouncer.lock().map_err(|e| e.to_string())?;

    let mut debouncer = new_debouncer(
        Duration::from_millis(100),
        move |res: DebounceEventResult| match res {
            Ok(events) => {
                for event in events {
                    println!("Watcher event: {:?}", event.path);
                    if !should_emit_path(&event.path) {
                        println!("Watcher: skipping path {:?}", event.path);
                        continue;
                    }

                    let path_str = metadata_store_clone.normalize_path(&event.path);
                    println!("Watcher: emitting file-changed for {}", path_str);
                    let _ = app_clone.emit("file-changed", path_str);
                }
            }
            Err(e) => eprintln!("Watcher error: {:?}", e),
        },
    )
    .map_err(|e| e.to_string())?;

    debouncer
        .watcher()
        .watch(&watch_root, RecursiveMode::Recursive)
        .map_err(|e| e.to_string())?;

    *debouncer_guard = Some(debouncer);

    Ok(())
}

fn should_emit_path(path: &Path) -> bool {
    if is_generated_wiki_path(path) {
        return false;
    }

    if path.components().any(|component| {
        let value = component.as_os_str().to_string_lossy();
        matches!(
            value.as_ref(),
            ".git"
                | "node_modules"
                | "target"
                | "dist"
                | ".ssh"
                | ".localbrain"
                | "__snapshots__"
                | "snapshots"
        )
    }) {
        return false;
    }

    if path.file_name().is_some_and(|file_name| {
        let value = file_name.to_string_lossy();
        value == ".DS_Store"
            || value.starts_with(".env")
            || value.ends_with("_snapshot.json")
            || value.ends_with(".snap")
    }) {
        return false;
    }

    let has_supported_extension = path.extension().is_some_and(|extension| {
        let extension = extension.to_string_lossy();
        language_from_extension(extension.as_ref()).is_some()
            || matches!(extension.as_ref(), "md" | "html" | "txt")
    });

    has_supported_extension || is_extensionless_watch_file(path)
}

fn is_generated_wiki_path(path: &Path) -> bool {
    let parts = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>();

    parts
        .windows(2)
        .any(|window| window[0] == "docs" && window[1] == "wiki")
}

fn is_extensionless_watch_file(path: &Path) -> bool {
    path.extension().is_none()
        && path.file_name().is_some_and(|name| {
            matches!(
                name.to_string_lossy().as_ref(),
                "Dockerfile"
                    | "dockerfile"
                    | "Containerfile"
                    | "Makefile"
                    | "makefile"
                    | "justfile"
                    | "Procfile"
                    | "Brewfile"
                    | "Vagrantfile"
                    | "Jenkinsfile"
                    | "Tiltfile"
                    | "README"
                    | "LICENSE"
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
        assert!(should_emit_path(Path::new("docs/architecture/auth.md")));
    }

    #[test]
    fn ignores_noise_and_secret_paths() {
        assert!(!should_emit_path(Path::new("docs/wiki/auth.md")));
        assert!(!should_emit_path(Path::new(
            "/workspace/project/docs/wiki/auth.md"
        )));
        assert!(!should_emit_path(Path::new("node_modules/pkg/index.ts")));
        assert!(!should_emit_path(Path::new(".git/config")));
        assert!(!should_emit_path(Path::new(".env.local")));
        assert!(!should_emit_path(Path::new(".ssh/config")));
        assert!(!should_emit_path(Path::new("dist/assets/index.js")));
        assert!(!should_emit_path(Path::new(
            "packages/db/src/migrations/meta/0040_snapshot.json"
        )));
        assert!(!should_emit_path(Path::new(
            "src/components/__snapshots__/App.test.tsx.snap"
        )));
    }

    #[test]
    fn ignores_unsupported_extensions() {
        assert!(should_emit_path(Path::new("src/styles.css")));
        assert!(should_emit_path(Path::new("src/config.yaml")));
        assert!(should_emit_path(Path::new("README")));
        assert!(should_emit_path(Path::new("Dockerfile")));
    }
}
