use std::process::Command;
fn main() {
    let mut child = Command::new("./src-tauri/binaries/llama-server-aarch64-apple-darwin")
        .args(["--model", "/Users/ih8sum3r/Downloads/Phi-3-mini-4k-instruct-Q4_K_M.gguf", "--port", "11434", "--host", "127.0.0.1"])
        .spawn()
        .expect("failed to execute process");
    std::thread::sleep(std::time::Duration::from_secs(5));
    println!("killing child...");
    child.kill().unwrap();
    let status = child.wait().expect("failed to wait on child");
    println!("process exited with: {}", status);
}
