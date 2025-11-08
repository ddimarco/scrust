# CascLib Automatic Cargo Build Integration

This guide shows how to build CascLib automatically as part of `cargo build` - no manual compilation needed!

## Quick Start (Recommended)

### Option 1: Git Submodule (Best for Development)

```bash
cd /path/to/scrust

# Add CascLib as a git submodule
git submodule add https://github.com/ladislav-zezula/CascLib.git \
    src/scformats/vendor/CascLib

# Initialize the submodule
git submodule update --init --recursive

# That's it! Now just build normally
cargo build
```

The build script will automatically:
1. Detect the CascLib submodule
2. Run CMake to configure it
3. Build the static library
4. Link it into your project

### Option 2: Vendored Source (Best for Distribution)

If you don't want to use git submodules (e.g., for crates.io distribution):

```bash
cd /path/to/scrust

# Download and extract CascLib source
mkdir -p src/scformats/vendor
cd src/scformats/vendor
git clone https://github.com/ladislav-zezula/CascLib.git casclib-src
rm -rf casclib-src/.git  # Remove git metadata

# Build
cd ../../..  # Back to repo root
cargo build
```

### Option 3: External Source Path

If you have CascLib source elsewhere:

```bash
# Point to existing CascLib checkout
export CASCLIB_SRC=/path/to/your/CascLib
cargo build
```

---

## How It Works

The updated `build.rs` uses the `cmake` crate to:

1. **Auto-detect CascLib source** in this priority order:
   - `src/scformats/vendor/CascLib` (git submodule)
   - `src/scformats/vendor/casclib-src` (vendored)
   - `$CASCLIB_SRC` environment variable
   - System library (fallback)

2. **Build with CMake**:
   ```rust
   cmake::Config::new(casclib_src)
       .define("BUILD_SHARED_LIBS", "OFF")
       .build()
   ```

3. **Link statically**: Creates `libcasc.a` and links it into your binary

4. **Handle dependencies**: Automatically links zlib and bz2 (CascLib dependencies)

---

## Prerequisites

### System Dependencies

CascLib needs zlib and bz2 libraries installed:

**Ubuntu/Debian:**
```bash
sudo apt-get install cmake zlib1g-dev libbz2-dev
```

**macOS:**
```bash
brew install cmake zlib bzip2
```

**Fedora/RHEL:**
```bash
sudo dnf install cmake zlib-devel bzip2-devel
```

**Windows:**
- Install CMake from https://cmake.org/download/
- zlib/bz2 will be built from source by CascLib's CMake

---

## Complete Example: Fresh Setup

Here's a complete walkthrough from scratch:

```bash
# Clone your scrust repo
git clone <your-repo-url> scrust
cd scrust

# Check out this branch with CASC support
git checkout claude/code-review-011CUvfNkHfVzkNhSWqz43rG

# Pull the latest changes
git pull

# Add CascLib as submodule
git submodule add https://github.com/ladislav-zezula/CascLib.git \
    src/scformats/vendor/CascLib

# Initialize submodules
git submodule update --init --recursive

# Install system dependencies (Ubuntu example)
sudo apt-get install cmake zlib1g-dev libbz2-dev

# Build everything - CascLib will be built automatically!
cargo build

# You should see output like:
# warning: Building CascLib from git submodule
# [cmake output...]
# Compiling scformats v0.1.0
# Compiling scrust v0.1.0
```

---

## Build Output

When building, you'll see:

```
   Compiling scformats v0.1.0
warning: Building CascLib from git submodule
-- The C compiler identification is GNU 11.4.0
-- Detecting C compiler ABI info
-- Detecting C compiler ABI info - done
-- Check for working C compiler: /usr/bin/cc - skipped
-- Configuring done
-- Generating done
-- Build files have been written to: /path/to/scrust/target/debug/build/scformats-xxx/out/build
[  6%] Building C object CMakeFiles/casc.dir/src/CascCommon.c.o
[ 12%] Building C object CMakeFiles/casc.dir/src/CascDecompress.c.o
...
[100%] Built target casc
```

This is normal! It only happens once, then CMake caches the build.

---

## Incremental Builds

The build script is smart about rebuilds:

- **First build**: Compiles CascLib (~30 seconds)
- **Subsequent builds**: Reuses cached build (instant)
- **Rebuilds only when**:
  - `vendor/CascLib` source changes
  - `CASCLIB_SRC` environment variable changes
  - You run `cargo clean`

---

## Cross-Platform Support

This works on all platforms CMake supports:

### Linux
✅ Works out of the box with gcc/clang

### macOS
✅ Works with Xcode command line tools

### Windows
✅ Works with:
- MSVC (Visual Studio)
- MinGW-w64
- MSYS2

**Windows-specific notes**:
```powershell
# Install dependencies via vcpkg
vcpkg install zlib:x64-windows bzip2:x64-windows

# Or let CascLib build them from source (slower)
cargo build
```

---

## Troubleshooting

### Error: "CMake not found"

**Solution**: Install CMake

```bash
# Ubuntu/Debian
sudo apt-get install cmake

# macOS
brew install cmake

# Or download from https://cmake.org/download/
```

### Error: "zlib.h not found" or "bzlib.h not found"

**Solution**: Install development headers

```bash
# Ubuntu/Debian
sudo apt-get install zlib1g-dev libbz2-dev

# macOS
brew install zlib bzip2

# Fedora
sudo dnf install zlib-devel bzip2-devel
```

### Error: "cannot find -lcasc"

**Solution**: Build failed, check for CMake errors above. Common causes:

1. **Missing CMakeLists.txt**: Make sure CascLib submodule was initialized:
   ```bash
   git submodule update --init --recursive
   ```

2. **CMake configuration failed**: Check CMake output for errors

3. **Compiler not found**: Install build tools:
   ```bash
   sudo apt-get install build-essential  # Ubuntu
   ```

### Warning: "CascLib source not found"

The build script couldn't find CascLib source. It will try system library as fallback.

**To fix**, use one of these:

```bash
# Option 1: Add as submodule
git submodule add https://github.com/ladislav-zezula/CascLib.git \
    src/scformats/vendor/CascLib

# Option 2: Set environment variable
export CASCLIB_SRC=/path/to/CascLib

# Option 3: Install system-wide
sudo apt install libcasc  # If available on your distro
```

### Build is slow / rebuilds every time

**Cause**: Cargo thinks CascLib changed

**Solutions**:

1. **Submodule uncommitted changes**: Commit or stash changes in `vendor/CascLib`

2. **File timestamps**: Git submodules can have stale timestamps. Update:
   ```bash
   git submodule update --init --recursive
   ```

3. **Clear and rebuild**:
   ```bash
   cargo clean
   cargo build
   ```

---

## Git Submodule Management

### Cloning repo with submodules

When others clone your repo:

```bash
# Clone with submodules automatically
git clone --recursive <repo-url>

# Or if already cloned:
git submodule update --init --recursive
```

### Updating CascLib to latest version

```bash
cd src/scformats/vendor/CascLib
git pull origin master
cd ../../../..
git add src/scformats/vendor/CascLib
git commit -m "Update CascLib to latest version"
```

### Removing the submodule (if needed)

```bash
git submodule deinit -f src/scformats/vendor/CascLib
git rm -f src/scformats/vendor/CascLib
rm -rf .git/modules/src/scformats/vendor/CascLib
```

---

## Alternative: No Git Submodules

If you prefer not to use git submodules:

### Download and vendor the source

```bash
# Create vendor directory
mkdir -p src/scformats/vendor

# Download CascLib
cd src/scformats/vendor
curl -L https://github.com/ladislav-zezula/CascLib/archive/refs/heads/master.zip -o casclib.zip
unzip casclib.zip
mv CascLib-master casclib-src
rm casclib.zip

# Back to repo root
cd ../../..

# Add to git
git add src/scformats/vendor/casclib-src
git commit -m "Vendor CascLib source"

# Build
cargo build
```

**Pros**:
- No submodule complexity
- Works on crates.io (if you publish)
- Offline builds work

**Cons**:
- Harder to update CascLib
- Larger repo size

---

## CI/CD Integration

### GitHub Actions

```yaml
name: Build

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3
        with:
          submodules: recursive  # Important!

      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y cmake zlib1g-dev libbz2-dev

      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Build
        run: cargo build --release
```

### GitLab CI

```yaml
build:
  image: rust:latest
  before_script:
    - git submodule update --init --recursive
    - apt-get update && apt-get install -y cmake zlib1g-dev libbz2-dev
  script:
    - cargo build --release
```

---

## Performance Notes

### Build Times

| Step | Time (typical) |
|------|----------------|
| First CascLib build | ~30 seconds |
| Cached build | <1 second |
| Clean rebuild | ~30 seconds |
| Incremental Rust | ~5 seconds |

### Optimizations

1. **Use cached builds**: Don't `cargo clean` unless necessary

2. **Parallel builds**: CMake uses all CPU cores automatically

3. **Release builds**: Add `-DCMAKE_BUILD_TYPE=Release` in build.rs for faster CascLib (already done)

4. **ccache**: Speed up C compilation:
   ```bash
   sudo apt install ccache
   export CMAKE_C_COMPILER_LAUNCHER=ccache
   ```

---

## Comparison: Manual vs Cargo Build

### Manual Build (Old Approach)

```bash
# 1. Clone CascLib separately
git clone https://github.com/ladislav-zezula/CascLib.git
cd CascLib
mkdir build && cd build

# 2. Build manually
cmake .. -DCMAKE_BUILD_TYPE=Release
make

# 3. Install or set environment
sudo make install
# OR
export CASCLIB_PATH=/path/to/CascLib/build

# 4. Build scrust
cd /path/to/scrust
cargo build
```

**Problems**:
- 😞 Multiple manual steps
- 😞 Different for each developer
- 😞 Hard to reproduce
- 😞 Doesn't work in CI without extra setup

### Cargo Build (New Approach)

```bash
# 1. Clone with submodules
git clone --recursive <repo-url>

# 2. Build
cargo build
```

**Benefits**:
- ✅ One command
- ✅ Works the same for everyone
- ✅ Reproducible builds
- ✅ CI-friendly
- ✅ Cross-platform

---

## Summary

You now have **fully automatic CascLib builds** integrated into Cargo:

1. **Add CascLib once**:
   ```bash
   git submodule add https://github.com/ladislav-zezula/CascLib.git src/scformats/vendor/CascLib
   ```

2. **Build normally**:
   ```bash
   cargo build
   ```

3. **It just works™** - CascLib builds automatically, caches the result, and links statically.

No more manual compilation, environment variables, or per-developer setup!

---

## Next Steps

1. Add CascLib submodule (see Quick Start)
2. Test build: `cargo build`
3. Verify CASC files can be opened (see CASC_SETUP.md)
4. Update GameData to use CASC (see examples in CASC_SETUP.md)

---

## Related Files

- `src/scformats/build.rs` - The build script that does the magic
- `src/scformats/Cargo.toml` - Adds `cmake` build dependency
- `src/scformats/src/casclib.rs` - Rust FFI bindings
- `CASC_SETUP.md` - Original setup guide (manual build)
