fn main() {
    let binaries_dir = std::path::Path::new("binaries");

    if binaries_dir.exists() {
        println!("cargo:rerun-if-changed={}", binaries_dir.display());
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
            let dest = binaries_dir.join(path.file_name().unwrap());
            if dest.exists() {
                match std::fs::canonicalize(&dest) {
                    Ok(existing) if existing == real_path => {
                        println!(
                            "cargo:warning=Skipped copy for {:?}; destination already points to source",
                            path.file_name().unwrap()
                        );
                        println!("cargo:rerun-if-changed={}", path.display());
                        continue;
                    }
                    _ => {}
                }
            }

            if let Err(e) = std::fs::copy(&real_path, &dest) {
                println!(
                    "cargo:warning=Failed to copy {:?} → {:?}: {}",
                    real_path, dest, e
                );
            } else {
                println!(
                    "cargo:warning=Copied {:?} to {}",
                    path.file_name().unwrap(),
                    binaries_dir.display()
                );
            }

            println!("cargo:rerun-if-changed={}", path.display());
        }
    }

    tauri_build::build()
}
