use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;
// We can't easily mock Tauri app handle in a script. Let's just use std::process::Command.
use std::process::Command;
fn main() {
    let mut child = Command::new("./binaries/llama-server-aarch64-apple-darwin")
        .args(["--model", "/Users/ih8sum3r/Downloads/Phi-3-mini-4k-instruct-Q4_K_M.gguf", "--port", "11434", "--host", "127.0.0.1"])
        .spawn()
        .expect("spawn failed");
    std::thread::sleep(std::time::Duration::from_secs(5));
    child.kill().unwrap();
}
