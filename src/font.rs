use std::fs::File;
use std::io::{Read, Write, Seek, SeekFrom};

extern crate byteorder;
use byteorder::{LittleEndian, ReadBytesExt};

pub struct FontHeader {
    /// index of first letter in file
    pub low_idx: u8,
    ///	Index of the last letter in file
    pub high_idx: u8,
    pub max_width: u8,
    pub max_height: u8,
}

pub struct FontLetter {
    width: u8,
    height: u8,
    xoffset: u8,
    yoffset: u8,
    data: Vec<u8>,
}

pub struct Font {
    pub header: FontHeader,
    pub letters: Vec<FontLetter>,
}

impl Font {
    pub fn read<T: Read + Seek>(file: &mut T) -> Font {
        // read header
        // always "FONT"
        let name = file.read_u32::<LittleEndian>().unwrap();
        assert_eq!(name, 1414418246);
        let low_idx = file.read_u8().unwrap();
        let high_idx = file.read_u8().unwrap();
        let max_width = file.read_u8().unwrap();
        let max_height = file.read_u8().unwrap();
        // skip 4 bytes
        let _ = file.read_u32::<LittleEndian>().unwrap();

        // read letter offsets
        let num_letters = (high_idx - low_idx) as usize;
        let mut letter_offsets = Vec::with_capacity(num_letters);
        for _ in 0..num_letters {
            letter_offsets.push(file.read_u32::<LittleEndian>().unwrap());
        }

        let mut letters = Vec::with_capacity(num_letters);
        // read letters
        for i in 0..num_letters {
            let ofs = letter_offsets[i] as u64;
            file.seek(SeekFrom::Start(ofs)).ok();

            // read letter header
            let w = file.read_u8().unwrap();
            let h = file.read_u8().unwrap();
            let xoff = file.read_u8().unwrap();
            let yoff = file.read_u8().unwrap();
            let datasize = w as usize * h as usize;
            let mut data = vec![0 as u8; datasize];
            let mut i: usize = 0;
            while i < datasize {
                let val = file.read_u8().unwrap();
                let color_idx = val & 0x7;
                let skipped = val >> 3;

                i = i + skipped as usize;
                if i < datasize {
                    data[i] = color_idx;
                }
                i = i + 1;
            }
            letters.push(FontLetter {
                width: w,
                height: h,
                xoffset: xoff,
                yoffset: yoff,
                data: data,
            });
        }


        Font {
            header: FontHeader {
                low_idx: low_idx,
                high_idx: high_idx,
                max_width: max_width,
                max_height: max_height,
            },
            letters: letters,
        }


    }
}
