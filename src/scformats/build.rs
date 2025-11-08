// Build script for scformats
//
// Automatically builds CascLib as part of the Cargo build process
//
// Options for including CascLib source:
// 1. Git submodule (recommended for development)
// 2. Vendored source (copy CascLib into vendor/)
// 3. System library (fallback)

use std::path::{Path, PathBuf};
use std::fs;

fn main() {
    // Strategy 1: Try git submodule at vendor/CascLib
    let submodule_path = Path::new("vendor/CascLib");

    // Strategy 2: Try vendored source at vendor/casclib-src
    let vendored_path = Path::new("vendor/casclib-src");

    // Strategy 3: Try CASCLIB_SRC environment variable
    let env_src_path = std::env::var("CASCLIB_SRC").ok();

    let casclib_src = if submodule_path.join("CMakeLists.txt").exists() {
        println!("cargo:warning=Building CascLib from git submodule");
        submodule_path
    } else if vendored_path.join("CMakeLists.txt").exists() {
        println!("cargo:warning=Building CascLib from vendored source");
        vendored_path
    } else if let Some(ref path) = env_src_path {
        println!("cargo:warning=Building CascLib from CASCLIB_SRC: {}", path);
        Path::new(path)
    } else {
        // Strategy 4: Fallback to system library
        println!("cargo:warning=CascLib source not found, trying system library");
        println!("cargo:warning=");
        println!("cargo:warning=To build CascLib from source, either:");
        println!("cargo:warning=  1. Add as git submodule:");
        println!("cargo:warning=     git submodule add https://github.com/ladislav-zezula/CascLib.git src/scformats/vendor/CascLib");
        println!("cargo:warning=  2. Set CASCLIB_SRC environment variable");
        println!("cargo:warning=  3. Install system-wide: sudo apt install libcasc-dev (if available)");
        println!("cargo:warning=");

        // Try to link against system library
        println!("cargo:rustc-link-lib=dylib=casc");
        return;
    };

    // Patch CMakeLists.txt to fix compatibility with newer CMake versions
    // CascLib uses cmake_minimum_required(VERSION 2.6) which is too old
    patch_cmake_minimum_version(casclib_src);

    // Build CascLib using cmake crate
    let dst = cmake::Config::new(casclib_src)
        .define("BUILD_SHARED_LIBS", "OFF")  // Build static library
        .build();

    // Link against the built library
    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-search=native={}/lib64", dst.display());
    println!("cargo:rustc-link-lib=static=casc");

    // Also link zlib and bz2 which CascLib depends on
    println!("cargo:rustc-link-lib=dylib=z");
    println!("cargo:rustc-link-lib=dylib=bz2");

    println!("cargo:rerun-if-changed=vendor/CascLib");
    println!("cargo:rerun-if-changed=vendor/casclib-src");
    println!("cargo:rerun-if-env-changed=CASCLIB_SRC");
}

/// Patch CascLib's CMakeLists.txt to use a newer cmake_minimum_required version
///
/// CascLib uses VERSION 2.6 which is too old for CMake 3.27+
/// We temporarily patch it to VERSION 3.5 which works with all modern CMake
fn patch_cmake_minimum_version(casclib_src: &Path) {
    let cmake_file = casclib_src.join("CMakeLists.txt");

    // Read the file
    let content = match fs::read_to_string(&cmake_file) {
        Ok(c) => c,
        Err(e) => {
            println!("cargo:warning=Failed to read CMakeLists.txt: {}", e);
            return;
        }
    };

    // Check if already patched or if it needs patching
    if content.contains("cmake_minimum_required(VERSION 3.5)") ||
       content.contains("cmake_minimum_required(VERSION 3.") {
        // Already has a good version
        return;
    }

    // Patch: replace old cmake_minimum_required with VERSION 3.5
    let patched = content.replace(
        "cmake_minimum_required(VERSION 2.6)",
        "cmake_minimum_required(VERSION 3.5)"
    ).replace(
        "cmake_minimum_required(VERSION 2.8)",
        "cmake_minimum_required(VERSION 3.5)"
    );

    // Only write if something changed
    if patched != content {
        if let Err(e) = fs::write(&cmake_file, patched) {
            println!("cargo:warning=Failed to patch CMakeLists.txt: {}", e);
            println!("cargo:warning=You may need to manually edit {:?}", cmake_file);
            println!("cargo:warning=Change 'cmake_minimum_required(VERSION 2.x)' to 'VERSION 3.5'");
        } else {
            println!("cargo:warning=Patched CMakeLists.txt to use cmake_minimum_required(VERSION 3.5)");
        }
    }
}
