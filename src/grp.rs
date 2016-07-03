use std::fs::File;
use std::io::{Read, Write, Seek, SeekFrom};

extern crate byteorder;
use byteorder::{LittleEndian, ReadBytesExt};

pub struct GRPHeader {
    pub frame_count: usize,
    pub width: u16,
    pub height: u16,
}
impl GRPHeader {
    pub fn read<T: Read>(file: &mut T) -> GRPHeader {
        let frame_count = file.read_u16::<LittleEndian>().unwrap();
        let width = file.read_u16::<LittleEndian>().unwrap();
        let height = file.read_u16::<LittleEndian>().unwrap();
        GRPHeader {
            frame_count: frame_count as usize,
            width: width,
            height: height,
        }
    }
}

// all the frames in the GRP file must be the exact same size a lot of the
// time, frames come in matching sets of 17. In the first frame, the unit
// faces north and in the seventeenth frame it faces south, and all the frames
// in between are of the unit rotating clockwise through the 180 degrees
// between north and south. Since there are only frames of the unit pointing
// north, south and east, you probably have already worked out that StarCraft
// flips the graphics to display graphics pointing west. Not all units are
// like this though. Only if the images.dat entry for the GRP file has the
// "GFX Turns" property set to 1 in Arsenal Zero will StarCraft expect sets of
// 17 rotated frames.
//
// no color palette is saved in the GRP file
pub struct GRP {
    pub header: GRPHeader,
    pub frames: Vec<Vec<u8>>,
}
impl GRP {
    fn read_line_offsets<T: Read + Seek>(file: &mut T, offset: u32, line_count: usize) -> Vec<u16> {
        file.seek(SeekFrom::Start(offset as u64)).ok();
        let mut offsets = Vec::with_capacity(line_count);
        for _ in 0..line_count {
            let val = file.read_u16::<LittleEndian>().unwrap();
            offsets.push(val);
        }

        offsets
    }

    fn read_frames<T: Read + Seek>(header: &GRPHeader, file: &mut T) -> Vec<Vec<u8>> {
        let frame_count = header.frame_count;
        let mut frames = Vec::with_capacity(frame_count);

        struct GRPFrame {
            x_offset: u8,
            y_offset: u8,
            framewidth: u8,
            frameheight: u8,
            // offset to frame data from file begin
            frameoffset: u32,
        }
        let mut frames_int = Vec::with_capacity(frame_count);
        for _ in 0..frame_count {
            let x_offset = file.read_u8().unwrap();
            let y_offset = file.read_u8().unwrap();
            let framewidth = file.read_u8().unwrap();
            let frameheight = file.read_u8().unwrap();
            let frameoffset = file.read_u32::<LittleEndian>().unwrap();

            frames_int.push(GRPFrame {
                x_offset: x_offset,
                y_offset: y_offset,
                framewidth: framewidth,
                frameheight: frameheight,
                frameoffset: frameoffset,
            });
        }

        // read the actual frame data
        // i.e. offsets to rle encoded line data beginnings
        for frame_int in &frames_int {
            let line_offsets =
                GRP::read_line_offsets(file, frame_int.frameoffset, frame_int.frameheight as usize);
            let fl = (header.width as usize) * (header.height as usize);
            let mut frame_data = vec![0 as u8; fl];
            for i in 0..line_offsets.len() {
                GRP::read_line_data(file,
                                    frame_int.y_offset as usize + i,
                                    frame_int.frameoffset as u64 + line_offsets[i] as u64,
                                    frame_int.x_offset as usize,
                                    frame_int.framewidth,
                                    &mut frame_data,
                                    header.width);
            }
            frames.push(frame_data);
        }
        frames
    }

    fn read_line_data<T: Read + Seek>(file: &mut T,
                                      line_idx: usize,
                                      line_offset: u64,
                                      xoffset: usize,
                                      framewidth: u8,
                                      data: &mut Vec<u8>,
                                      real_framewidth: u16) {
        file.seek(SeekFrom::Start(line_offset)).ok();
        let data_start = line_idx * real_framewidth as usize;
        let mut x = xoffset;
        while x - xoffset < framewidth as usize {
            let val = file.read_u8().unwrap();
            if val >= 128 {
                // skip val - 128 bytes
                let to_skip = val - 128;
                x = x + to_skip as usize;
            } else if val >= 64 {
                // repeat the next byte val - 64 times
                let next_val = file.read_u8().unwrap();
                for _ in 0..val - 64 {
                    data[data_start + x] = next_val;
                    x = x + 1;
                }
            } else {
                // just copy the next val bytes as they are
                for _ in 0..val {
                    data[data_start + x] = file.read_u8().unwrap();
                    x = x + 1;
                }
            }
        }
    }

    pub fn read<T: Read + Seek>(file: &mut T) -> GRP {
        let header = GRPHeader::read(file);
        let frames = GRP::read_frames(&header, file);

        GRP {
            header: header,
            frames: frames,
        }
    }

    pub fn frame_to_ppm(self: &GRP, frame: usize, outfile: &str) {
        let mut outfile = File::create(outfile).unwrap();
        outfile.write_fmt(format_args!("P3\n")).ok();
        outfile.write_fmt(format_args!("{0} {1}\n", self.header.width, self.header.height)).ok();
        outfile.write_fmt(format_args!("255\n")).ok();
        let frame = &self.frames[frame];
        for i in 0..(self.header.width as usize) * (self.header.height as usize) {
            let pal_idx: usize = 3 * (frame[i] as usize);
            outfile.write_fmt(format_args!("{0} {1} {2}\n", pal_idx, pal_idx, pal_idx))
                .ok();
        }
    }
}
