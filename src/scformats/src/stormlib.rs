use std::ffi::CString;
use libc::c_char;

#[allow(unused_imports)]
use byteorder::{LittleEndian, ReadBytesExt};

// FIXME: don't treat void* as u64

#[link(name = "storm")]
extern "C" {
    fn SFileOpenArchive(mpqname: *const c_char,
                        priority: u32,
                        flags: u32,
                        handle: &mut u64)
                        -> bool;
    fn SFileCloseArchive(handle: u64) -> bool;
    fn SFileHasFile(handle: u64, filename: *const c_char) -> bool;

    fn SFileGetFileSize(handle: u64, fsizehigh: &mut u32) -> u32;
    fn SFileOpenFileEx(handle: u64,
                       filename: *const c_char,
                       searchscope: u32,
                       filehandle: &mut u64)
                       -> bool;
    fn SFileCloseFile(handle: u64) -> bool;
    fn SFileReadFile(handle: u64,
                     buffer: *mut u8,
                     toread: u32,
                     read: *mut u32,
                     lpoverlapped: u64)
                     -> bool;

    // fn SFileSetFilePointer(handle: u64,
    //                        lFilePos: u32,
    //                        plFilePos: *mut u32,
    //                        moveMethod: u32)
    //                        -> u32;

    fn SFileExtractFile(handle: u64,
                        in_filename: *const c_char,
                        out_filename: *const c_char,
                        flags: u32)
                        -> bool;
}

use std::io::Cursor;
// FIXME: use buffered read
pub type MPQArchiveFile = Cursor<Vec<u8>>;

pub struct MPQArchive {
    pub filename: String,
    handle: u64,
}

impl Drop for MPQArchive {
    fn drop(&mut self) {
        // println!("dropping archive!");
        self.close();
    }
}

impl MPQArchive {
    pub fn open(filename: &str) -> MPQArchive {
        let filepath = CString::new(filename).unwrap();
        unsafe {
            let mut handle: u64 = 0;
            let succ = SFileOpenArchive(filepath.as_ptr(), 0, 0x100, &mut handle);
            assert!(succ, "opening {} failed!", filename);
            MPQArchive {
                filename: filename.to_string(),
                handle: handle,
            }
        }
    }
    pub fn has_file(&self, filename: &str) -> bool {
        let filepath = CString::new(filename).unwrap();
        unsafe { SFileHasFile(self.handle, filepath.as_ptr()) }
    }

    pub fn extract(&self, infilename: &str, outfilename: &str) {
        let in_filepath = CString::new(infilename).unwrap();
        let out_filepath = CString::new(outfilename).unwrap();
        unsafe {
            let res = SFileExtractFile(self.handle, in_filepath.as_ptr(), out_filepath.as_ptr(), 0);
            assert!(res);
        }
    }

    pub fn open_file(&self, filename: &str) -> MPQArchiveFile {
        // XXX: might be more efficient to read the full file at once
        let filepath = CString::new(filename).unwrap();
        unsafe {
            let mut reshandle: u64 = 0;
            let succ = SFileOpenFileEx(self.handle, filepath.as_ptr(), 0, &mut reshandle);
            assert!(succ);

            let mut fshigh: u32 = 0;
            let fs = SFileGetFileSize(reshandle, &mut fshigh);

            let mut read_bytes: u32 = 0;
            let mut buf = vec![0u8; fs as usize];
            let succ2 = SFileReadFile(reshandle, buf.as_mut_ptr(), fs, &mut read_bytes, 0);
            assert!(succ2);
            SFileCloseFile(reshandle);

            Cursor::new(buf)
            // MPQArchiveFile {
            //     handle: reshandle,
            //     //archive: self,
            // }
        }
    }

    fn close(&mut self) {
        unsafe {
            SFileCloseArchive(self.handle);
        }
    }
}

// fn main() {
// println!("opening file");
//
// let mpq = MPQArchive::open("/home/dm/code/mysc/data/STARDAT.MPQ");
//
// println!("opened!");
// println!("has file: {}", mpq.has_file("arr\\images.tbl"));
// let mut infile = mpq.open_file("arr\\images.tbl");
// let mut infile2 = mpq.open_file("glue\\title\\title.pcx");
// let fs = mpq.get_filesize(res);
// let fs = infile.get_filesize();
// println!("filesize: {}", fs);
//
// let mut buf = vec![0; fs];
// infile.read(&mut buf).ok();
//
//
// let byte = infile2.read_u8().unwrap();
// println!("read single byte: {}", byte);
//
// mpq.close_file(res);
//
// println!("closed");
// }
//
