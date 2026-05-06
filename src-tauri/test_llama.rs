use std::process::Command;
fn main() {
    let binary = std::env::var("LLAMA_BINARY")
        .expect("LLAMA_BINARY is required (example: ./binaries/llama-server-aarch64-apple-darwin)");
    let model =
        std::env::var("LLAMA_MODEL").expect("LLAMA_MODEL is required (path to a .gguf model)");

    let mut child = Command::new(&binary)
        .args(["--model", &model, "--port", "11434", "--host", "127.0.0.1"])
        .spawn()
        .expect("failed to execute process");
    std::thread::sleep(std::time::Duration::from_secs(5));
    println!("killing child...");
    child.kill().unwrap();
    let status = child.wait().expect("failed to wait on child");
    println!("process exited with: {}", status);
}
