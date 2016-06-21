use std::io::{Read, Seek, SeekFrom};
use std::cmp::min;

extern crate byteorder;
use byteorder::{LittleEndian, ReadBytesExt};

pub enum FontSize {
    Font10 = 0,
    Font14 = 1,
    Font16 = 2,
    Font16X = 3,
}

pub struct FontHeader {
    /// index of first letter in file
    pub low_idx: u8,
    ///	Index of the last letter in file
    pub high_idx: u8,
    pub max_width: u8,
    pub max_height: u8,
}

pub struct FontLetter {
    pub width: u8,
    pub height: u8,
    pub xoffset: i8,
    pub yoffset: i8,
    pub data: Vec<u8>,
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
            let xoff = file.read_i8().unwrap();
            let yoff = file.read_i8().unwrap();
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

    pub fn get_letter<'a>(&'a self, c: char) -> &'a FontLetter {
        assert!(c != ' ');
        &self.letters[(c as usize) - 33]
    }

    pub fn letter_width(&self, c: char) -> u32 {
        if c == ' ' {
            min((self.header.max_width as u32)/3,
                          6)
        } else {
            let ref letter = self.get_letter(c);
            (letter.xoffset as u32) + (letter.width as u32)
        }
    }

    pub fn line_height(&self) -> u32 {
        let ref letter = self.get_letter('y');
        (letter.yoffset as u32) + (letter.height as u32)
    }
}
