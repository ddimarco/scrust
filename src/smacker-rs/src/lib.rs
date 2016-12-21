use std::ffi::CString;
use std::ptr;
use std::fs::File;
use std::io::{Write, Read};

extern crate libc;
use libc::{c_char, c_ulong, c_uchar, c_void, c_double};

extern "C" {
    fn smk_open_file(filename: *const c_char, mode: c_uchar) -> *mut c_void;
    fn smk_open_memory(buffer: *const u8, size: c_ulong) -> *mut c_void;

    fn smk_enable_video(handle: *mut c_void, enable: c_uchar) -> c_char;


    fn smk_info_all(handle: *const c_void,
                    frame: *mut c_ulong,
                    frame_count: *mut c_ulong,
                    usf: *mut c_double)
                    -> c_char;
    fn smk_info_video(handle: *const c_void,
                      w: *mut c_ulong,
                      h: *mut c_ulong,
                      y_scale_mode: *mut c_uchar)
                      -> c_char;

    fn smk_get_palette(handle: *const c_void) -> *mut c_uchar;
    fn smk_get_video(handle: *const c_void) -> *mut c_uchar;

    fn smk_first(handle: *mut c_void) -> c_char;
    fn smk_next(handle: *mut c_void) -> c_char;

    fn smk_close(handle: *mut c_void);
}

pub struct SMK {
    handle: *mut c_void,
    pub frame: usize,
    pub frame_count: usize,
    pub usf: f32,
    pub width: usize,
    pub height: usize,
    pub y_scale_mode: u8,
}

// return results of goto_first(), goto_next()
pub enum FrameIterationStatus {
    Done = 0x0,
    More,
    Last,
}

impl SMK {
    pub fn read(infile: &mut Read, size: usize) -> Self {
        let mut inbuf = vec![0u8; size];
        infile.read(&mut inbuf).ok();
        unsafe {
            let h = smk_open_memory(inbuf.as_mut_ptr(), size as c_ulong);
            SMK::construct_smk_helper(h)
        }
    }

    pub fn from_file(filename: &str) -> Self {
        let filepath = CString::new(filename).unwrap();
        unsafe {
            // mode 0x01: memory
            let h = smk_open_file(filepath.as_ptr(), 0x01);
            SMK::construct_smk_helper(h)
        }
    }

    fn construct_smk_helper(h: *mut c_void) -> Self {
        unsafe {
            {
                let c = smk_enable_video(h, 1);
                assert!(c == 0, "could not enable smk video!");
            }

            let mut frame: c_ulong = 0;
            let mut frame_count: c_ulong = 0;
            let mut usf: c_double = 0.0;
            {
                let c = smk_info_all(h, &mut frame, &mut frame_count, &mut usf);
                assert!(c == 0, "could not get smk infos!");
            }

            let mut width: c_ulong = 0;
            let mut height: c_ulong = 0;
            let mut y_scale_mode: c_uchar = 0;
            {
                let c = smk_info_video(h, &mut width, &mut height, &mut y_scale_mode);
                assert!(c == 0, "could not get smk video infos!");
            }

            SMK {
                handle: h,
                frame: frame as usize,
                frame_count: frame_count as usize,
                usf: usf as f32,
                width: width as usize,
                height: height as usize,
                y_scale_mode: y_scale_mode,
            }
        }
    }

    fn iteration_char_to_enum(c: c_char) -> FrameIterationStatus {
        match c {
            0 => FrameIterationStatus::Done,
            1 => FrameIterationStatus::More,
            2 => FrameIterationStatus::Last,
            _ => unreachable!(),
        }
    }

    pub fn go_first_frame(&mut self) -> FrameIterationStatus {
        self.frame = 0;
        unsafe {
            let c = smk_first(self.handle);
            assert!(c >= 0, "could not jump to first frame: {}", c);
            SMK::iteration_char_to_enum(c)
        }
    }

    pub fn go_next_frame(&mut self) -> FrameIterationStatus {
        self.frame += 1;
        unsafe {
            let c = smk_next(self.handle);
            assert!(c >= 0, "could not jump to next frame!");
            SMK::iteration_char_to_enum(c)
        }
    }

    pub fn copy_palette(&self) -> Vec<u8> {
        const PAL_SIZE: usize = 256 * 3;
        let mut res_vec = Vec::<u8>::with_capacity(PAL_SIZE);
        unsafe {
            res_vec.set_len(PAL_SIZE);
            let pal_ptr = smk_get_palette(self.handle);
            ptr::copy(pal_ptr, res_vec.as_mut_ptr(), PAL_SIZE);
        }
        res_vec
    }

    pub fn copy_frame(&self) -> Vec<u8> {
        let frame_size = self.width * self.height;
        let mut res_vec = Vec::<u8>::with_capacity(frame_size);
        unsafe {
            res_vec.set_len(frame_size);
            let frame_ptr = smk_get_video(self.handle);
            ptr::copy(frame_ptr, res_vec.as_mut_ptr(), frame_size);
        }
        res_vec
    }

    pub fn get_frame(&self) -> SMKFrame {
        SMKFrame {
            frame_idx: self.frame,
            width: self.width,
            height: self.height,
            data: self.copy_frame(),
            palette: self.copy_palette(),
        }
    }

}

pub struct SMKFrame {
    pub frame_idx: usize,
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
    pub palette: Vec<u8>,
}
impl SMKFrame {
    pub fn to_ppm(&self, outfile: &str) {
        let mut outfile = File::create(outfile).unwrap();
        outfile.write_fmt(format_args!("P3\n")).ok();
        outfile.write_fmt(format_args!("{} {}\n", self.width, self.height)).ok();
        outfile.write_fmt(format_args!("255\n")).ok();

        for i in 0..self.width * self.height {
            let pal_idx: usize = 3 * (self.data[i as usize] as usize);
            outfile.write_fmt(format_args!("{0} {1} {2}\n",
                                           self.palette[pal_idx + 0],
                                           self.palette[pal_idx + 1],
                                           self.palette[pal_idx + 2]))
                .ok();
        }
    }
}

impl Iterator for SMK {
    type Item = SMKFrame;

    /// note: this depends on the state of the SMK instance
    fn next(&mut self) -> Option<SMKFrame> {
        let iterstatus = self.go_next_frame();
        match iterstatus {
            FrameIterationStatus::Last   => {
                // point to beginning
                self.go_first_frame();
                None
            },
            FrameIterationStatus::Done => {
                unreachable!()
            },
            FrameIterationStatus::More => {
                Some(self.get_frame())
            }
        }
    }
}

impl Drop for SMK {
    fn drop(&mut self) {
        unsafe {
            smk_close(self.handle);
        }
    }
}
