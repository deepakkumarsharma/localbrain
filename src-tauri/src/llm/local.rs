use crate::settings::SettingsStore;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use tokio::sync::mpsc::Receiver;
use tokio::sync::watch;

pub struct LocalLlmState {
    pub server_port: u16,
    /// Holds the child process handle. Use std::sync::Mutex for sync is_running() checks.
    pub child: Mutex<Option<CommandChild>>,
    /// Async mutex used to serialize concurrent start_llama_server calls.
    /// Held for the entire spawn + health-wait window so only one instance
    /// ever tries to bind the port.
    startup_lock: tokio::sync::Mutex<()>,
}

impl LocalLlmState {
    pub fn new() -> Self {
        Self {
            server_port: 8080,
            child: Mutex::new(None),
            startup_lock: tokio::sync::Mutex::new(()),
        }
    }

    /// Returns true if a child handle exists in the mutex.
    /// NOTE: this does NOT guarantee the process is alive — use is_server_alive() for that.
    pub fn is_running(&self) -> bool {
        self.child.lock().unwrap().is_some()
    }

    pub fn kill_child_if_running(&self) -> Result<(), String> {
        let mut child_guard = self.child.lock().map_err(|error| error.to_string())?;
        if let Some(child) = child_guard.take() {
            child
                .kill()
                .map_err(|error| format!("Failed to kill llama-server: {error}"))?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LlamaCompletionRequest {
    pub prompt: String,
    pub n_predict: i32,
    pub stream: bool,
    pub stop: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LlamaCompletionResponse {
    pub content: String,
}

// Expand `~` in paths — the shell won't do this for sidecar args
fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        let home = std::env::var("HOME").unwrap_or_default();
        path.replacen("~", &home, 1)
    } else {
        path.to_string()
    }
}

// Quick liveness check — is the server actually responding right now?
async fn is_server_alive(port: u16) -> bool {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };

    for endpoint in ["/health", "/v1/models"] {
        let url = format!("http://127.0.0.1:{}{}", port, endpoint);
        if let Ok(resp) = client.get(url).send().await {
            if resp.status().is_success() {
                return true;
            }
        }
    }
    false
}

// Poll /health every 500ms until ready or timeout
async fn wait_for_server_ready(
    port: u16,
    mut terminated_rx: watch::Receiver<bool>,
) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .map_err(|error| format!("Failed to build health client: {error}"))?;
    let endpoints = ["/health", "/v1/models"];
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(70);

    println!("[llama] Waiting for server on port {}...", port);

    while std::time::Instant::now() < deadline {
        if *terminated_rx.borrow() {
            return Err("llama-server terminated before becoming ready".to_string());
        }

        let mut ready = false;
        for endpoint in endpoints {
            let url = format!("http://127.0.0.1:{}{}", port, endpoint);
            match client.get(url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    ready = true;
                    break;
                }
                Ok(resp) => {
                    println!(
                        "[llama] Still loading model... endpoint={} status={}",
                        endpoint,
                        resp.status()
                    );
                }
                Err(_) => {
                    // Port not yet bound, keep waiting
                }
            }
        }
        if ready {
            println!("[llama] Server is ready ✓");
            return Ok(());
        }
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {}
            changed = terminated_rx.changed() => {
                if changed.is_ok() && *terminated_rx.borrow() {
                    return Err("llama-server terminated before becoming ready".to_string());
                }
            }
        }
    }

    Err("llama-server did not become ready within 70 seconds".to_string())
}

// Drain stdout/stderr in a background task so the process never gets SIGPIPE
fn spawn_log_drain(mut rx: Receiver<CommandEvent>, terminated_tx: watch::Sender<bool>) {
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    println!("[llama stdout] {}", String::from_utf8_lossy(&line));
                }
                CommandEvent::Stderr(line) => {
                    eprintln!("[llama stderr] {}", String::from_utf8_lossy(&line));
                }
                CommandEvent::Error(e) => {
                    eprintln!("[llama error] {}", e);
                }
                CommandEvent::Terminated(payload) => {
                    let _ = terminated_tx.send(true);
                    eprintln!(
                        "[llama TERMINATED] code={:?} signal={:?}",
                        payload.code, payload.signal
                    );
                    break;
                }
                _ => {}
            }
        }
    });
}

fn validate_llama_runtime_files(app: &AppHandle) -> Result<(), String> {
    let binaries_dir = app
        .path()
        .resolve("binaries", BaseDirectory::Resource)
        .map_err(|e| format!("Failed to resolve local AI runtime directory: {e}"))?;
    if !binaries_dir.exists() {
        return Err(format!(
            "Missing local AI runtime directory: {}. Reinstall the app or run the documented installer.",
            binaries_dir.display()
        ));
    }

    let required = [
        "libllama-common.0.0.9025.dylib",
        "libllama.0.0.9025.dylib",
        "libggml.0.10.2.dylib",
        "libggml-base.0.10.2.dylib",
        "libggml-cpu.0.10.2.dylib",
        "libggml-metal.0.10.2.dylib",
        "libggml-blas.0.10.2.dylib",
        "libggml-rpc.0.10.2.dylib",
        "libmtmd.0.0.9025.dylib",
    ];

    for file in required {
        let path = binaries_dir.join(file);
        let meta = std::fs::metadata(&path).map_err(|_| {
            format!(
                "Missing local AI runtime file: {}. Run `npm run llm:repair`.",
                path.display()
            )
        })?;
        if meta.len() == 0 {
            return Err(format!(
                "Corrupted local AI runtime file (0 bytes): {}. Run `npm run llm:repair`.",
                path.display()
            ));
        }
    }

    Ok(())
}

fn repair_llama_runtime_files(app: &AppHandle) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| format!("Failed to read current dir: {e}"))?;
    let src_dir = cwd.join("src-tauri").join("target").join("debug");
    let dst_dir = app
        .path()
        .resolve("binaries", BaseDirectory::Resource)
        .map_err(|e| format!("Failed to resolve runtime destination directory: {e}"))?;

    if !src_dir.exists() || !dst_dir.exists() {
        return Err(format!(
            "Local AI runtime repair directories are missing. source={} destination={}",
            src_dir.display(),
            dst_dir.display()
        ));
    }

    let entries = std::fs::read_dir(&src_dir)
        .map_err(|e| format!("Failed to read runtime source dir: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read runtime source entry: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("dylib") {
            continue;
        }
        if path.file_name().is_none() {
            continue;
        }
        let dest_path = dst_dir.join(path.file_name().unwrap());
        std::fs::copy(&path, &dest_path).map_err(|e| {
            format!(
                "Failed to repair runtime file {} -> {}: {e}",
                path.display(),
                dest_path.display()
            )
        })?;
    }

    Ok(())
}

pub async fn start_llama_server(app: &AppHandle) -> Result<(), String> {
    if let Err(initial_error) = validate_llama_runtime_files(app) {
        if cfg!(debug_assertions) {
            eprintln!(
                "[llama] Runtime validation failed: {initial_error}. Attempting auto-repair..."
            );
            repair_llama_runtime_files(app)?;
            validate_llama_runtime_files(app).map_err(|post_repair_error| {
                format!(
                    "{post_repair_error} Auto-repair attempted but failed verification. Run `npm run llm:repair`."
                )
            })?;
        } else {
            return Err(format!(
                "{initial_error} Please reinstall Localbrain or run the documented runtime installer."
            ));
        }
    }

    let state = app.state::<LocalLlmState>();
    let port = state.server_port;

    // Acquire the startup lock — this serializes concurrent calls.
    // If three callers arrive at once, only one proceeds through the spawn
    // path; the others wait here and then find the server already alive.
    let _startup_guard = state.startup_lock.lock().await;

    // Re-check after acquiring the lock — a concurrent caller may have
    // already started the server while we were waiting.
    if state.child.lock().unwrap().is_some() {
        if is_server_alive(port).await {
            println!("[llama] Server already running and healthy.");
            return Ok(());
        }

        // Stale handle: process died, clear it and fall through to respawn
        println!("[llama] Stale child detected — clearing and respawning...");
        let mut child_guard = state.child.lock().unwrap();
        *child_guard = None;
    }

    // Resolve and validate the model path
    let settings = app.state::<SettingsStore>().get()?;
    let raw_path = match settings.local_model_path {
        Some(p) => p,
        None => return Err("No local model path configured in settings.".to_string()),
    };

    let model_path = expand_tilde(&raw_path);

    if !std::path::Path::new(&model_path).exists() {
        return Err(format!(
            "Model file not found: '{}' — check your settings.",
            model_path
        ));
    }

    println!("[llama] Spawning llama-server with model: {}", model_path);

    let sidecar = app
        .shell()
        .sidecar("llama-server")
        .map_err(|e| format!("Failed to find llama-server sidecar: {}", e))?;

    let (rx, child) = sidecar
        .args([
            "--model",
            &model_path,
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--ctx-size",
            "4096",
            "--parallel",
            "1",
        ])
        .spawn()
        .map_err(|e| format!("Failed to spawn llama-server: {}", e))?;

    let (terminated_tx, terminated_rx) = watch::channel(false);

    // Drain stdout/stderr in the background so the process never gets SIGPIPE
    spawn_log_drain(rx, terminated_tx);

    // Store the child handle
    {
        let mut child_guard = state.child.lock().unwrap();
        *child_guard = Some(child);
    }

    // Block until /health returns 200 before releasing the startup lock.
    // This means any concurrent callers that are waiting on startup_lock
    // will find a fully ready server when they eventually acquire it.
    if let Err(error) = wait_for_server_ready(port, terminated_rx).await {
        {
            let mut child_guard = state.child.lock().unwrap();
            if let Some(child) = child_guard.take() {
                let _ = child.kill();
            }
        }
        return Err(error);
    }

    // _startup_guard drops here, releasing the lock
    Ok(())
}

pub async fn stop_llama_server(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<LocalLlmState>();
    state.kill_child_if_running()?;
    println!("[llama] Server stopped.");

    Ok(())
}

pub async fn get_llm_running_status(app: &AppHandle) -> bool {
    let state = app.state::<LocalLlmState>();
    if !state.is_running() {
        return false;
    }

    is_server_alive(state.server_port).await
}

pub async fn generate_with_llama(prompt: &str, port: u16) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let url = format!("http://127.0.0.1:{}/completion", port);

    let request = LlamaCompletionRequest {
        prompt: prompt.to_string(),
        n_predict: 512,
        stream: false,
        stop: vec![
            "</s>".to_string(),
            "Llama:".to_string(),
            "User:".to_string(),
        ],
    };

    // start_llama_server guarantees readiness, so 3 retries covers transient hiccups only
    let mut retries = 3;
    let mut last_err = String::new();

    while retries > 0 {
        match client.post(&url).json(&request).send().await {
            Ok(response) => {
                let status = response.status();

                if !status.is_success() {
                    let body = response.text().await.unwrap_or_default();
                    return Err(format!("llama-server returned {}: {}", status, body));
                }

                let completion: LlamaCompletionResponse = response
                    .json()
                    .await
                    .map_err(|e| format!("Failed to parse llama-server response: {}", e))?;

                return Ok(completion.content);
            }
            Err(e) => {
                eprintln!(
                    "[llama] Completion request failed (retries left={}): {}",
                    retries - 1,
                    e
                );
                last_err = e.to_string();
                retries -= 1;
                if retries > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }
    }

    Err(format!(
        "Failed to get completion from llama-server: {}",
        last_err
    ))
}
