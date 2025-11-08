// Build script for scformats
//
// This links against StormLib (for MPQ archives) and CascLib (for CASC archives)
//
// Setup instructions:
//
// For CascLib:
// 1. Clone and build CascLib:
//    git clone https://github.com/ladislav-zezula/CascLib.git
//    cd CascLib
//    mkdir build && cd build
//    cmake .. -DCMAKE_BUILD_TYPE=Release
//    make
//
// 2. Either:
//    a) Install system-wide: sudo make install
//    b) Copy libcasc.a to a lib directory and set CASCLIB_PATH
//       export CASCLIB_PATH=/path/to/CascLib/build
//
// For StormLib:
// 1. Ensure StormLib is installed (your existing setup)

fn main() {
    // Try to find CascLib
    // Priority:
    // 1. CASCLIB_PATH environment variable
    // 2. System library path
    // 3. Skip if not found (optional dependency)

    if let Ok(casclib_path) = std::env::var("CASCLIB_PATH") {
        println!("cargo:rustc-link-search=native={}", casclib_path);
        println!("cargo:rustc-link-lib=static=casc");
        println!("cargo:warning=CascLib found at: {}", casclib_path);
    } else {
        // Try system library
        println!("cargo:rustc-link-lib=dylib=casc");
        println!("cargo:warning=CascLib not found in CASCLIB_PATH, trying system libraries");
        println!("cargo:warning=If build fails, set CASCLIB_PATH or install CascLib");
    }

    // StormLib should already be linked from your existing setup
    // If not, you can add it here:
    // println!("cargo:rustc-link-lib=dylib=storm");

    println!("cargo:rerun-if-changed=src/casclib.rs");
    println!("cargo:rerun-if-changed=src/stormlib.rs");
    println!("cargo:rerun-if-env-changed=CASCLIB_PATH");
}
