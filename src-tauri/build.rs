fn main() {
    // Copy dylibs next to the sidecar binary in the target directory
    // so dyld can find them at @executable_path when Tauri spawns the sidecar
    let out_dir = std::env::var("OUT_DIR").unwrap();

    // OUT_DIR = target/debug/build/<crate>/out — go up 3 levels to get target/debug/
    let target_dir = std::path::Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .unwrap()
        .to_path_buf();

    let binaries_dir = std::path::Path::new("binaries");

    if binaries_dir.exists() {
        for entry in std::fs::read_dir(binaries_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            // Skip anything that isn't a .dylib
            if path.extension().and_then(|e| e.to_str()) != Some("dylib") {
                continue;
            }

            // Resolve symlinks — some dylibs are symlinks to versioned files
            let real_path = match std::fs::canonicalize(&path) {
                Ok(p) => p,
                Err(_) => continue, // broken symlink, skip it
            };

            // Skip if the resolved path is a directory
            if !real_path.is_file() {
                continue;
            }

            // Copy the real file using the original symlink's filename
            let dest = target_dir.join(path.file_name().unwrap());
            if let Err(e) = std::fs::copy(&real_path, &dest) {
                println!(
                    "cargo:warning=Failed to copy {:?} → {:?}: {}",
                    real_path, dest, e
                );
            } else {
                println!(
                    "cargo:warning=Copied {:?} to target/debug/",
                    path.file_name().unwrap()
                );
            }

            println!("cargo:rerun-if-changed={}", path.display());
        }
    }

    tauri_build::build()
}
