use std::io::{Read, Seek, SeekFrom};

extern crate byteorder;
use byteorder::{ReadBytesExt, LittleEndian};

use ::utils::{read_vec_u32};

pub struct LOX {
    pub frames: Vec<LOXFrame>,
}
pub struct LOXFrame {
    pub offsets: Vec<(i8, i8)>,
}

impl LOX {
    /// for each frame, a vec of x,y offsets
    pub fn read<T: Read + Seek>(f: &mut T) -> Self {
        let frame_count = f.read_u32::<LittleEndian>().unwrap() as usize;
        let overlays_per_frame = f.read_u32::<LittleEndian>().unwrap() as usize;
        // 1 offset per frame
        let offsets = read_vec_u32(f, frame_count);
        let mut frames = Vec::<LOXFrame>::with_capacity(frame_count);
        for offset in offsets {
            f.seek(SeekFrom::Start(offset as u64)).ok();
            let mut overlay_offsets = Vec::<(i8, i8)>::with_capacity(overlays_per_frame);
            for _ in 0..overlays_per_frame {
                let x = f.read_i8().unwrap();
                let y = f.read_i8().unwrap();
                overlay_offsets.push((x, y));
            }
            frames.push(LOXFrame {
                offsets: overlay_offsets,
            });
        }
        LOX {
            frames: frames,
        }
    }

}
