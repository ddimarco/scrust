use std::ffi::CString;
use libc::c_char;

#[allow(unused_imports)]
use byteorder::{LittleEndian, ReadBytesExt};

// FFI bindings to CascLib C library
// Based on https://github.com/ladislav-zezula/CascLib
//
// Build instructions:
// 1. Clone CascLib: git clone https://github.com/ladislav-zezula/CascLib.git
// 2. Build with cmake:
//    cd CascLib
//    mkdir build && cd build
//    cmake ..
//    make
// 3. Copy libcasc.a to your lib directory or add to linker path
// 4. Update build.rs to link against casc library

#[link(name = "casc")]
extern "C" {
    // Storage operations
    fn CascOpenStorage(
        szDataPath: *const c_char,
        dwLocaleMask: u32,
        phStorage: *mut u64
    ) -> bool;

    fn CascCloseStorage(hStorage: u64) -> bool;

    // File operations
    fn CascOpenFile(
        hStorage: u64,
        szFileName: *const c_char,
        dwLocaleFlags: u32,
        dwOpenFlags: u32,
        phFile: *mut u64
    ) -> bool;

    fn CascGetFileSize64(
        hFile: u64,
        pFileSizeHigh: *mut u64
    ) -> bool;

    fn CascReadFile(
        hFile: u64,
        lpBuffer: *mut u8,
        dwToRead: u32,
        pdwRead: *mut u32
    ) -> bool;

    fn CascCloseFile(hFile: u64) -> bool;

    // Utility - check if file exists
    fn CascGetFileInfo(
        hStorage: u64,
        szFileName: *const c_char,
        InfoClass: u32,
        pvFileInfo: *mut u8,
        cbFileInfo: u32,
        pcbLengthNeeded: *mut u32
    ) -> bool;
}

use std::io::Cursor;

pub type CascArchiveFile = Cursor<Vec<u8>>;

pub struct CascArchive {
    pub path: String,
    handle: u64,
}

impl Drop for CascArchive {
    fn drop(&mut self) {
        self.close();
    }
}

impl CascArchive {
    /// Open a CASC storage
    ///
    /// # Arguments
    /// * `path` - Path to the game directory containing CASC files
    ///            For StarCraft, this should be the directory containing
    ///            .build.info and .build.db files
    ///
    /// # Example
    /// ```
    /// let casc = CascArchive::open("/path/to/starcraft");
    /// ```
    pub fn open(path: &str) -> CascArchive {
        let filepath = CString::new(path).unwrap();
        unsafe {
            let mut handle: u64 = 0;
            // 0 for dwLocaleMask means all locales
            let succ = CascOpenStorage(filepath.as_ptr(), 0, &mut handle);
            assert!(succ, "Opening CASC storage at {} failed!", path);
            CascArchive {
                path: path.to_string(),
                handle: handle,
            }
        }
    }

    /// Check if a file exists in the CASC storage
    ///
    /// Note: This is a simple implementation that tries to open the file
    /// A more efficient version would use CascGetFileInfo
    pub fn has_file(&self, filename: &str) -> bool {
        let filepath = CString::new(filename).unwrap();
        unsafe {
            let mut file_handle: u64 = 0;
            let result = CascOpenFile(
                self.handle,
                filepath.as_ptr(),
                0,  // locale flags
                0,  // open flags
                &mut file_handle
            );

            if result {
                CascCloseFile(file_handle);
                true
            } else {
                false
            }
        }
    }

    /// Open and read a file from the CASC storage
    ///
    /// Returns a Cursor over the file data for compatibility with MPQ API
    ///
    /// # Arguments
    /// * `filename` - File path within the archive (use backslashes for StarCraft files)
    ///
    /// # Example
    /// ```
    /// let file = casc.open_file("arr\\units.dat");
    /// ```
    pub fn open_file(&self, filename: &str) -> CascArchiveFile {
        let filepath = CString::new(filename).unwrap();
        unsafe {
            // Open the file
            let mut file_handle: u64 = 0;
            let succ = CascOpenFile(
                self.handle,
                filepath.as_ptr(),
                0,  // locale flags
                0,  // open flags
                &mut file_handle
            );
            assert!(succ, "Failed to open file: {}", filename);

            // Get file size
            let mut file_size: u64 = 0;
            let succ2 = CascGetFileSize64(file_handle, &mut file_size);
            assert!(succ2, "Failed to get file size for: {}", filename);

            // Read the entire file
            let mut buf = vec![0u8; file_size as usize];
            let mut read_bytes: u32 = 0;
            let succ3 = CascReadFile(
                file_handle,
                buf.as_mut_ptr(),
                file_size as u32,
                &mut read_bytes
            );
            assert!(succ3, "Failed to read file: {}", filename);
            assert_eq!(read_bytes as u64, file_size, "Read incomplete for: {}", filename);

            // Close the file handle
            CascCloseFile(file_handle);

            // Return as Cursor for compatibility with MPQ API
            Cursor::new(buf)
        }
    }

    fn close(&mut self) {
        unsafe {
            CascCloseStorage(self.handle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires actual StarCraft installation
    fn test_casc_open() {
        // Update this path to your StarCraft installation
        let casc = CascArchive::open("/path/to/starcraft");
        assert!(casc.has_file("arr\\units.dat"));
    }
}
