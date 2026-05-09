use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use tokio::sync::mpsc::Receiver;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunbookCommand {
    pub id: String,
    pub name: String,
    pub command: String,
    pub cwd: String,
    pub source: String,
    pub kind: String,
    pub requires_confirmation: bool,
    pub risk: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunbookChecklist {
    pub project_selected: bool,
    pub dependencies_detected: bool,
    pub runtime_ready: bool,
    pub local_model_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunbookDiscovery {
    pub workspace_root: String,
    pub commands: Vec<RunbookCommand>,
    pub checklist: RunbookChecklist,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunbookProcessView {
    pub process_id: String,
    pub command_id: String,
    pub kind: String,
    pub workspace_root: String,
    pub name: String,
    pub command: String,
    pub cwd: String,
    pub source: String,
    pub status: String,
    pub started_at: String,
    pub exited_at: Option<String>,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunbookLogEvent {
    pub process_id: String,
    pub stream: String,
    pub line: String,
    pub ts: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartRunbookProcessRequest {
    pub workspace_root: String,
    pub command: RunbookCommand,
    pub confirmed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunbookProcessStartedEvent {
    pub process: RunbookProcessView,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunbookProcessExitedEvent {
    pub process: RunbookProcessView,
}

struct RunbookProcessRuntime {
    pub view: RunbookProcessView,
    pub child: CommandChild,
}

pub struct RunbookState {
    processes: Mutex<HashMap<String, RunbookProcessRuntime>>,
}

impl RunbookState {
    pub fn new() -> Self {
        Self {
            processes: Mutex::new(HashMap::new()),
        }
    }

    pub fn get_process_views(&self) -> Vec<RunbookProcessView> {
        let guard = self.processes.lock().unwrap();
        guard.values().map(|process| process.view.clone()).collect()
    }

    fn insert_process(&self, process_id: String, process: RunbookProcessRuntime) {
        let mut guard = self.processes.lock().unwrap();
        guard.insert(process_id, process);
    }

    pub fn stop_process(&self, process_id: &str) -> Result<(), String> {
        let mut guard = self.processes.lock().map_err(|error| error.to_string())?;
        let Some(runtime) = guard.remove(process_id) else {
            // Idempotent stop: process may have already terminated and been removed.
            return Ok(());
        };
        runtime
            .child
            .kill()
            .map_err(|error| format!("failed to stop process: {error}"))?;
        Ok(())
    }

    pub fn mark_exited(
        &self,
        process_id: &str,
        exit_code: Option<i32>,
    ) -> Option<RunbookProcessView> {
        let mut guard = self.processes.lock().unwrap();
        let runtime = guard.get_mut(process_id)?;
        runtime.view.status = "stopped".to_string();
        runtime.view.exited_at = Some(now_iso_like());
        runtime.view.exit_code = exit_code;
        Some(runtime.view.clone())
    }

    pub fn get_process(&self, process_id: &str) -> Option<RunbookProcessView> {
        let guard = self.processes.lock().unwrap();
        guard.get(process_id).map(|process| process.view.clone())
    }

    pub fn stop_all(&self) {
        let mut guard = match self.processes.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        for (_, runtime) in guard.drain() {
            let _ = runtime.child.kill();
        }
    }
}

pub fn discover_commands(workspace_root: &Path) -> Result<RunbookDiscovery, String> {
    let mut commands = Vec::new();
    let package_json = workspace_root.join("package.json");
    let compose_yaml = workspace_root.join("docker-compose.yml");
    let compose_yaml_alt = workspace_root.join("docker-compose.yaml");
    let makefile = workspace_root.join("Makefile");

    if package_json.is_file() {
        commands.extend(parse_package_scripts(workspace_root, &package_json)?);
    }
    if compose_yaml.is_file() {
        commands.extend(parse_compose_services(workspace_root, &compose_yaml)?);
    }
    if compose_yaml_alt.is_file() {
        commands.extend(parse_compose_services(workspace_root, &compose_yaml_alt)?);
    }
    if makefile.is_file() {
        commands.extend(parse_make_targets(workspace_root, &makefile)?);
    }

    commands.sort_by(|left, right| left.name.cmp(&right.name));
    commands.dedup_by(|left, right| left.id == right.id);

    let discovery = RunbookDiscovery {
        workspace_root: workspace_root.to_string_lossy().to_string(),
        checklist: RunbookChecklist {
            project_selected: true,
            dependencies_detected: !commands.is_empty(),
            runtime_ready: true,
            local_model_ready: true,
        },
        commands,
    };
    Ok(discovery)
}

pub fn start_process(
    app: &AppHandle,
    state: &RunbookState,
    workspace_root: &Path,
    request: StartRunbookProcessRequest,
) -> Result<RunbookProcessView, String> {
    let confirmed = request.confirmed.unwrap_or(false);
    let command = request.command;
    if !is_discovered_command(workspace_root, &command)? {
        return Err(
            "runbook command is not in the discovered workspace command catalog".to_string(),
        );
    }
    if command.requires_confirmation && !confirmed {
        return Err("runbook command requires explicit confirmation".to_string());
    }
    let safe_cwd = safe_cwd(workspace_root, &command.cwd)?;
    let (program, args) = split_command(&command.command)?;
    let shell_command = app
        .shell()
        .command(&program)
        .args(args)
        .current_dir(&safe_cwd);

    let (rx, child) = shell_command
        .spawn()
        .map_err(|error| format!("failed to spawn runbook command: {error}"))?;

    let process_id = format!(
        "runbook-{}",
        short_hash(&(command.id.clone() + &now_iso_like()))
    );
    let view = RunbookProcessView {
        process_id: process_id.clone(),
        command_id: command.id.clone(),
        kind: command.kind.clone(),
        workspace_root: workspace_root.to_string_lossy().to_string(),
        name: command.name.clone(),
        command: command.command.clone(),
        cwd: command.cwd.clone(),
        source: command.source.clone(),
        status: "running".to_string(),
        started_at: now_iso_like(),
        exited_at: None,
        exit_code: None,
    };
    let runtime = RunbookProcessRuntime {
        view: view.clone(),
        child,
    };
    state.insert_process(process_id.clone(), runtime);

    let _ = app.emit(
        "runbook-process-started",
        RunbookProcessStartedEvent {
            process: view.clone(),
        },
    );
    spawn_log_drain(app.clone(), process_id, rx);
    Ok(view)
}

fn is_discovered_command(
    workspace_root: &Path,
    requested: &RunbookCommand,
) -> Result<bool, String> {
    let discovery = discover_commands(workspace_root)?;
    Ok(discovery.commands.iter().any(|candidate| {
        candidate.id == requested.id
            && candidate.name == requested.name
            && candidate.command == requested.command
            && candidate.cwd == requested.cwd
            && candidate.source == requested.source
            && candidate.kind == requested.kind
    }))
}

fn spawn_log_drain(app: AppHandle, process_id: String, mut rx: Receiver<CommandEvent>) {
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => emit_log(&app, &process_id, "stdout", line),
                CommandEvent::Stderr(line) => emit_log(&app, &process_id, "stderr", line),
                CommandEvent::Terminated(payload) => {
                    let state = app.state::<RunbookState>();
                    let updated = state.mark_exited(&process_id, payload.code);
                    if let Some(process) = updated {
                        let _ = app.emit(
                            "runbook-process-exited",
                            RunbookProcessExitedEvent { process },
                        );
                    }
                    break;
                }
                CommandEvent::Error(message) => {
                    emit_log(&app, &process_id, "stderr", message.into_bytes())
                }
                _ => {}
            }
        }
    });
}

fn emit_log(app: &AppHandle, process_id: &str, stream: &str, bytes: Vec<u8>) {
    let line = redact_secret_like_values(&String::from_utf8_lossy(&bytes));
    if line.trim().is_empty() {
        return;
    }
    let _ = app.emit(
        "runbook-log",
        RunbookLogEvent {
            process_id: process_id.to_string(),
            stream: stream.to_string(),
            line,
            ts: now_iso_like(),
        },
    );
}

fn safe_cwd(workspace_root: &Path, requested_cwd: &str) -> Result<PathBuf, String> {
    let root = workspace_root
        .canonicalize()
        .map_err(|error| format!("failed to resolve workspace root: {error}"))?;
    let candidate = if requested_cwd.trim().is_empty() || requested_cwd == "." {
        root.clone()
    } else {
        root.join(requested_cwd)
    };
    let resolved = candidate
        .canonicalize()
        .map_err(|error| format!("failed to resolve command cwd: {error}"))?;
    if !resolved.starts_with(&root) {
        return Err("command cwd escapes workspace root".to_string());
    }
    Ok(resolved)
}

fn split_command(value: &str) -> Result<(String, Vec<String>), String> {
    let parts = value
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect::<Vec<String>>();
    if parts.is_empty() {
        return Err("empty command is not allowed".to_string());
    }
    let program = parts[0].clone();
    let args = parts[1..].to_vec();
    Ok((program, args))
}

fn parse_package_scripts(
    workspace_root: &Path,
    path: &Path,
) -> Result<Vec<RunbookCommand>, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    let value = serde_json::from_str::<serde_json::Value>(&content)
        .map_err(|error| format!("failed to parse package.json: {error}"))?;
    let scripts = value
        .get("scripts")
        .and_then(|scripts| scripts.as_object())
        .cloned()
        .unwrap_or_default();
    let source = relative_path(workspace_root, path);
    let commands = scripts
        .iter()
        .map(|(name, _)| {
            let command = format!("npm run {name}");
            new_command(&source, name, &command, ".", "package-script", false, "low")
        })
        .collect::<Vec<_>>();
    Ok(commands)
}

fn parse_compose_services(
    workspace_root: &Path,
    path: &Path,
) -> Result<Vec<RunbookCommand>, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    let source = relative_path(workspace_root, path);
    let mut in_services = false;
    let mut commands = Vec::new();
    for raw_line in content.lines() {
        let line = raw_line.trim_end();
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        if line.trim_start() == "services:" {
            in_services = true;
            continue;
        }
        if !in_services {
            continue;
        }
        if !line.starts_with(' ') && !line.starts_with('\t') {
            in_services = false;
            continue;
        }
        let trimmed = line.trim_start();
        if let Some(service) = trimmed.strip_suffix(':') {
            if service.contains(' ') {
                continue;
            }
            let command = format!("docker compose up {service}");
            commands.push(new_command(
                &source,
                &format!("compose:{service}"),
                &command,
                ".",
                "compose-service",
                true,
                "medium",
            ));
        }
    }
    Ok(commands)
}

fn parse_make_targets(workspace_root: &Path, path: &Path) -> Result<Vec<RunbookCommand>, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    let source = relative_path(workspace_root, path);
    let mut commands = Vec::new();
    for raw_line in content.lines() {
        let line = raw_line.trim_end();
        if line.trim().is_empty() || line.starts_with('\t') || line.starts_with('#') {
            continue;
        }
        if let Some((target, _)) = line.split_once(':') {
            let target_name = target.trim();
            if target_name.is_empty() || target_name.starts_with('.') || target_name.contains(' ') {
                continue;
            }
            let command = format!("make {target_name}");
            commands.push(new_command(
                &source,
                &format!("make:{target_name}"),
                &command,
                ".",
                "make-target",
                false,
                "low",
            ));
        }
    }
    Ok(commands)
}

fn new_command(
    source: &str,
    name: &str,
    command: &str,
    cwd: &str,
    kind: &str,
    requires_confirmation: bool,
    risk: &str,
) -> RunbookCommand {
    let id = short_hash(&format!("{source}:{name}:{command}:{cwd}:{kind}"));
    RunbookCommand {
        id,
        name: name.to_string(),
        command: command.to_string(),
        cwd: cwd.to_string(),
        source: source.to_string(),
        kind: kind.to_string(),
        requires_confirmation,
        risk: risk.to_string(),
    }
}

fn relative_path(workspace_root: &Path, path: &Path) -> String {
    path.strip_prefix(workspace_root)
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}

fn short_hash(value: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn now_iso_like() -> String {
    let now = SystemTime::now();
    let since_epoch = now.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    since_epoch.to_string()
}

fn redact_secret_like_values(line: &str) -> String {
    let upper = line.to_ascii_uppercase();
    if let Some(index) = upper.find("AUTHORIZATION:") {
        let prefix = &line[..index];
        return format!("{prefix}Authorization: ***");
    }
    if let Some(index) = upper.find("BEARER ") {
        let prefix = &line[..index];
        return format!("{prefix}Bearer ***");
    }

    let mut cleaned = line.to_string();
    for marker in [
        "API_KEY",
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "PASSWD",
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "GEMINI_API_KEY",
    ] {
        if let Some(index) = cleaned.to_ascii_uppercase().find(marker) {
            let prefix = &cleaned[..index];
            cleaned = format!("{prefix}{marker}=***");
            break;
        }
    }
    cleaned
}
