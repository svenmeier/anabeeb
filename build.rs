use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    if cfg!(target_os = "windows") {
        setup_fluidsynth();
    }
}

fn setup_fluidsynth() {
    let lib_dir = Path::new("target/fluidsynth/lib");
    let bin_dir = Path::new("target/fluidsynth/bin");
    let out_dir = match env::var("PROFILE").as_deref() {
        Ok("release") => PathBuf::from("target/release"),
        _ => PathBuf::from("target/debug"),
    };

    // Rename libfluidsynth-3.lib to fluidsynth.lib,
    // so it matches the link name in rust-fluidsynth's ffi.rs 
    let old_lib = lib_dir.join("libfluidsynth-3.lib");
    let new_lib = lib_dir.join("fluidsynth.lib");
    if old_lib.exists() && !new_lib.exists() {
        fs::rename(&old_lib, &new_lib)
            .unwrap_or_else(|e| panic!("Failed to rename {:?} to {:?}: {}", old_lib, new_lib, e));
    }

    // instruct cargo
    // ... where to find the lib
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    // ... which library to link
    println!("cargo:rustc-link-lib=dylib=fluidsynth");
    // ... to ignore symbols no longer present in fluidsynth-2
    println!("cargo:rustc-link-arg=/FORCE:UNRESOLVED");


    // Copy all .dll files to build output
    if let Ok(entries) = fs::read_dir(bin_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "dll" {
                    let file_name = path.file_name().unwrap();
                    let dest_path = out_dir.join(file_name);
                    fs::copy(&path, &dest_path)
                        .unwrap_or_else(|e| panic!("Failed to copy {:?} to {:?}: {}", path, dest_path, e));
                }
            }
        }
    } else {
        panic!("bin folder {:?} not found", bin_dir);
    }
}
