// Build script for scformats
//
// Automatically builds CascLib as part of the Cargo build process
//
// Options for including CascLib source:
// 1. Git submodule (recommended for development)
// 2. Vendored source (copy CascLib into vendor/)
// 3. System library (fallback)

use std::path::Path;

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
