# CASC Setup Guide

This guide explains how to add CASC (Content Addressable Storage Container) support to SCRust for StarCraft: Remastered and modern StarCraft installations.

## Background

- **MPQ format**: Used by original StarCraft (pre-1.18)
- **CASC format**: Used by StarCraft 1.18+ and StarCraft: Remastered (2017+)

If you downloaded StarCraft from Battle.net in 2017 or later, you have CASC format.

## Quick Check: Which Format Do You Have?

```bash
cd /path/to/starcraft

# MPQ format has:
ls *.MPQ  # STARDAT.MPQ, BROODAT.MPQ, etc.

# CASC format has:
ls .build.info .build.db  # CASC metadata files
ls -d Data/  # CASC data directory
```

---

## Part 1: Build CascLib C Library

### Prerequisites

```bash
# Ubuntu/Debian
sudo apt-get install build-essential cmake git

# macOS
brew install cmake

# Fedora/RHEL
sudo dnf install gcc-c++ cmake git
```

### Clone and Build CascLib

```bash
# Clone the official CascLib repository
git clone https://github.com/ladislav-zezula/CascLib.git
cd CascLib

# Create build directory
mkdir build
cd build

# Configure with CMake
cmake .. -DCMAKE_BUILD_TYPE=Release

# Build the library
make

# Verify the library was built
ls libcasc.a  # Should show the static library
```

### Option A: Install System-Wide (Recommended)

```bash
# Still in CascLib/build directory
sudo make install

# Verify installation
ldconfig -p | grep casc  # Linux
# Or check /usr/local/lib for libcasc.a
```

### Option B: Use Local Build (Development)

```bash
# Export the path to your local build
export CASCLIB_PATH=/path/to/CascLib/build

# Add to your ~/.bashrc or ~/.zshrc to persist:
echo 'export CASCLIB_PATH=/path/to/CascLib/build' >> ~/.bashrc
```

---

## Part 2: Update Your SCRust Build

The `casclib.rs` module and build script are already created in this branch.

### Test the Build

```bash
cd /path/to/scrust

# If using system-wide install:
cargo build

# If using local build:
CASCLIB_PATH=/path/to/CascLib/build cargo build
```

You should see warnings from the build script:
```
warning: CascLib found at: /path/to/CascLib/build
```

---

## Part 3: Update GameData to Use CASC

Here's how to modify your `src/gamedata.rs` to support both MPQ and CASC:

### Option 1: Auto-Detect Format

```rust
// src/gamedata.rs
use scformats::stormlib::{MPQArchive, MPQArchiveFile};
use scformats::casclib::{CascArchive, CascArchiveFile};
use std::path::Path;

pub enum ArchiveFile {
    MPQ(MPQArchiveFile),
    CASC(CascArchiveFile),
}

impl std::io::Read for ArchiveFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            ArchiveFile::MPQ(f) => f.read(buf),
            ArchiveFile::CASC(f) => f.read(buf),
        }
    }
}

pub enum Archive {
    MPQ(MPQArchive),
    CASC(CascArchive),
}

impl Archive {
    pub fn open_file(&self, filename: &str) -> ArchiveFile {
        match self {
            Archive::MPQ(mpq) => ArchiveFile::MPQ(mpq.open_file(filename)),
            Archive::CASC(casc) => ArchiveFile::CASC(casc.open_file(filename)),
        }
    }

    pub fn has_file(&self, filename: &str) -> bool {
        match self {
            Archive::MPQ(mpq) => mpq.has_file(filename),
            Archive::CASC(casc) => casc.has_file(filename),
        }
    }
}

pub fn detect_format(scdata_path: &Path) -> &'static str {
    if scdata_path.join("STARDAT.MPQ").exists() {
        "MPQ"
    } else if scdata_path.join(".build.info").exists() {
        "CASC"
    } else {
        panic!("Unknown StarCraft data format at {:?}", scdata_path);
    }
}

pub struct GameData {
    archives: Vec<Archive>,
    // ... rest of your fields
}

impl GameData {
    pub fn init(scdata_path: &Path) -> GameData {
        let format = detect_format(scdata_path);
        println!("Detected {} format", format);

        let mut archives = Vec::new();

        match format {
            "MPQ" => {
                // Load MPQ archives
                archives.push(Archive::MPQ(MPQArchive::open(
                    scdata_path.join("STARDAT.MPQ").to_str().unwrap()
                )));
                archives.push(Archive::MPQ(MPQArchive::open(
                    scdata_path.join("BROODAT.MPQ").to_str().unwrap()
                )));
                // Add other MPQ files as needed
            }
            "CASC" => {
                // Open CASC storage (single handle for entire game)
                archives.push(Archive::CASC(CascArchive::open(
                    scdata_path.to_str().unwrap()
                )));
            }
            _ => panic!("Unsupported format"),
        }

        // Rest of GameData initialization...
        GameData {
            archives,
            // ...
        }
    }

    fn open_(&self, filename: &str) -> Option<ArchiveFile> {
        for archive in &self.archives {
            if archive.has_file(filename) {
                return Some(archive.open_file(filename));
            }
        }
        None
    }
}
```

### Option 2: CASC-Only (Simplest)

If you only want to support modern StarCraft:

```rust
// src/gamedata.rs
use scformats::casclib::{CascArchive, CascArchiveFile};

pub struct GameData {
    casc: CascArchive,
    // ... rest of your fields
}

impl GameData {
    pub fn init(scdata_path: &Path) -> GameData {
        let casc = CascArchive::open(scdata_path.to_str().unwrap());

        // Test that we can read files
        assert!(casc.has_file("arr\\units.dat"), "Failed to find units.dat");

        GameData {
            casc,
            // ... initialize rest
        }
    }

    pub fn open_file(&self, filename: &str) -> Option<CascArchiveFile> {
        if self.casc.has_file(filename) {
            Some(self.casc.open_file(filename))
        } else {
            None
        }
    }
}
```

---

## Part 4: Common Issues & Troubleshooting

### Build Fails: "cannot find -lcasc"

**Solution**: CascLib not found by linker

```bash
# Check if library exists
find /usr -name "libcasc*" 2>/dev/null
find ~/CascLib -name "libcasc*" 2>/dev/null

# If found locally, export path:
export CASCLIB_PATH=/path/to/CascLib/build

# If not found at all, rebuild CascLib (see Part 1)
```

### Runtime Error: "Failed to open CASC storage"

**Possible causes**:

1. **Wrong path**: CASC expects the game directory, not a file:
   ```rust
   // Wrong
   CascArchive::open("/path/to/starcraft/STARDAT.MPQ")

   // Correct
   CascArchive::open("/path/to/starcraft")
   ```

2. **Missing CASC files**: Verify your StarCraft installation:
   ```bash
   ls /path/to/starcraft/.build.info
   ls /path/to/starcraft/Data/
   ```

3. **Permissions**: Ensure you have read access to the StarCraft directory

### File Not Found in CASC

CASC uses different file paths than MPQ. Try:

```rust
// MPQ might use
"arr\\units.dat"

// CASC might need
"SC/arr/units.dat"  // Forward slashes
"arr\\units.dat"    // Or backslashes still work
```

Enable debug output to see what CascLib is doing:
```bash
RUST_LOG=debug cargo run
```

---

## Part 5: Testing

Create a simple test binary to verify CASC works:

```rust
// src/bin/test-casc.rs
use scformats::casclib::CascArchive;

fn main() {
    let scdata_path = std::env::args().nth(1)
        .expect("Usage: test-casc <path-to-starcraft>");

    println!("Opening CASC storage at: {}", scdata_path);
    let casc = CascArchive::open(&scdata_path);

    // Test some common files
    let test_files = vec![
        "arr\\units.dat",
        "arr\\weapons.dat",
        "unit\\terran\\marine.grp",
        "tileset\\badlands\\badlands.cv5",
    ];

    for file in test_files {
        print!("Checking {}... ", file);
        if casc.has_file(file) {
            let data = casc.open_file(file);
            println!("✓ Found ({} bytes)", data.get_ref().len());
        } else {
            println!("✗ Not found");
        }
    }
}
```

Run it:
```bash
cargo run --bin test-casc -- /path/to/starcraft
```

---

## Part 6: Performance Considerations

### CASC vs MPQ

**CASC advantages**:
- Faster file lookup (content-addressed)
- Better compression
- Deduplication
- Streaming support

**MPQ advantages**:
- Single-file archives (easier distribution)
- More tooling support

### Optimization Tips

1. **Cache file handles**: Opening files repeatedly is expensive
   ```rust
   // Bad
   for _ in 0..1000 {
       let file = casc.open_file("arr\\units.dat");
   }

   // Good
   let file = casc.open_file("arr\\units.dat");
   // Use file multiple times
   ```

2. **Batch operations**: Read multiple files at once if possible

3. **Preload common files**: Load frequently-used files at startup

---

## Next Steps

1. ✅ Build CascLib
2. ✅ Test with `cargo build`
3. ✅ Create test binary to verify file access
4. ✅ Update `GameData::init()` to use CASC
5. ✅ Test your game with CASC data
6. 🎯 Consider abstracting Archive trait (see Option 1 above) for MPQ compatibility

---

## Reference Links

- [CascLib GitHub](https://github.com/ladislav-zezula/CascLib)
- [CASC Format Docs](http://www.zezula.net/en/casc/main.html)
- [StarCraft File Formats](https://staredit-network.fandom.com/wiki/Modding_Files_Overview)

---

## Summary

You now have:
- ✅ `casclib.rs` - Rust FFI bindings to CascLib
- ✅ `build.rs` - Build script to link CascLib
- ✅ API compatible with your existing MPQ code

The API is intentionally similar to `stormlib.rs`:
```rust
// Both work the same way
let mpq = MPQArchive::open("STARDAT.MPQ");
let casc = CascArchive::open("/path/to/starcraft");

let file1 = mpq.open_file("arr\\units.dat");
let file2 = casc.open_file("arr\\units.dat");
// Both return Cursor<Vec<u8>>
```

This makes migration easy - just swap the archive type!
