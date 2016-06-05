use std::io;

use std::string;
use std::mem;
use std::ptr;
use std::ffi::CString;
extern crate libc;
use libc::{c_void, c_char};

#[link(name = "storm")]
extern {
    fn SFileOpenArchive(mpqname: *const c_char, priority: u32, flags: u32, handle: &mut c_void) -> bool;
    fn SFileCloseArchive(handle: &mut c_void) -> bool;
    fn SFileHasFile(handle: *mut c_void, filename: *const c_char) -> bool;
    fn SFileOpenFileEx(handle: *mut c_void, filename: *const c_char, searchscope: u32,
                       filehandle: *mut c_void) -> bool;
}

struct MPQArchive {
    pub filename: String,
    handle: c_void
}

impl MPQArchive {
    pub fn open(filename: &str) -> MPQArchive {
        let filepath = CString::new(filename).unwrap();
        unsafe {
            let mut handle: c_void = unsafe { mem::zeroed()};
            let succ = SFileOpenArchive(filepath.as_ptr(), 0, 0x100, &mut handle);
            println!("succ: {}", succ);
            return MPQArchive {
                filename: filename.to_string(),
                handle: handle
            }
        }
    }
    pub fn has_file(&mut self, filename: &str) -> bool {
        let filepath = CString::new(filename).unwrap();
        unsafe {
            let succ = SFileHasFile(&mut self.handle, filepath.as_ptr());
            succ
        }
    }

    pub fn close(&mut self) {
        unsafe {
            SFileCloseArchive(&mut self.handle);
        }
    }
}

fn main() {
    println!("opening file");

    let mut mpq = MPQArchive::open("/home/dm/code/mysc-scheme/data/Starcraft.mpq");

    println!("opened!");
    println!("has file: {}", mpq.has_file("arr\\images.tbl"));

    mpq.close();
    // let mut handle: c_void = unsafe { mem::zeroed() };
    // let filepath = CString::new("/home/dm/code/mysc-scheme/data/Starcraft.mpq").unwrap();
    // unsafe {
    //     SFileOpenArchive(filepath.as_ptr(), 0, 0x100, &mut handle);
    // };
    //println!("res: {0}", res);

    // unsafe {
    //     let exists = SFileHasFile(&mut handle, CString::new("file000029.xxx").unwrap().as_ptr());
    //     println!("exists: {0}", exists);
    // };

    // unsafe {
    //     SFileCloseArchive(&mut handle);
    // }
}
