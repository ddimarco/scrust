use std::io::{Read, Seek, SeekFrom};
use std::cmp::min;

extern crate byteorder;
use byteorder::{LittleEndian, ReadBytesExt};

#[derive(Copy, Clone)]
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

///////////////////////////////////////////////////////////////

// render into 8bit screen buffer
use ::GameContext;

extern crate sdl2;
use self::sdl2::rect::Rect;

pub trait RenderText {
    fn render_textbox(&self,
                      text: &str,
                      color_idx: usize,
                      reindexing_table: &[u8],
                      trg_buf: &mut [u8],
                      trg_pitch: u32,
                      trg_rect: &Rect);
}
impl RenderText for Font {
    fn render_textbox(&self,
                      text: &str,
                      color_idx: usize,
                      reindexing_table: &[u8],
                      trg_buf: &mut [u8],
                      trg_pitch: u32,
                      trg_rect: &Rect) {
        // for now, assume only single lines
        let y = trg_rect.y() as usize;
        let mut x = trg_rect.x() as usize;
        // TODO: proper reindexing?
        for c in text.chars() {
            if c != ' ' {
                let ref letter = self.get_letter(c);
                for yl in (letter.yoffset as u32)..(letter.yoffset as u32 + letter.height as u32) {
                    for xl in letter.xoffset as u32..(letter.xoffset as u32 + letter.width as u32) {
                        let col = letter.data[(((yl - letter.yoffset as u32) * letter.width as u32) +
                                               (xl - letter.xoffset as u32)) as usize];


                        let outpos = ((y + yl as usize) * trg_pitch as usize) +
                            (x + xl as usize);

                        let col_mapped = reindexing_table[col as usize + (color_idx * 8)];
                        trg_buf[outpos] = col_mapped;
                    }
                }
            }
            let letterwidth = self.letter_width(c);
            x = x + 1 + letterwidth as usize;
        }
    }
}

///////////////////////////////////////////////////////////////
// render into rgb24 texture
/*
use ::pal::Palette;

extern crate sdl2;
use self::sdl2::pixels::PixelFormatEnum;
use self::sdl2::rect::Rect;
use self::sdl2::render::{Renderer, Texture};

pub trait RenderText {
    fn render_textbox(&self,
                      text: &str,
                      color_idx: usize,
                      renderer: &mut Renderer,
                      pal: &Palette,
                      reindexing_table: &[u8],
                      width: u32,
                      height: u32)
                      -> sdl2::render::Texture;
}
impl RenderText for Font {
    fn render_textbox(&self,
                      text: &str,
                      color_idx: usize,
                      renderer: &mut Renderer,
                      pal: &Palette,
                      reindexing_table: &[u8],
                      width: u32,
                      height: u32)
                      -> sdl2::render::Texture {
        let mut texture =
            renderer.create_texture_streaming(PixelFormatEnum::RGB24, width, height).unwrap();

        // for now, assume only single lines
        texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
            let y = 0;
            let mut x: usize = 0;
            for c in text.chars() {
                if c != ' ' {
                    let ref letter = self.get_letter(c);
                    for yl in (letter.yoffset as u32)..(letter.yoffset as u32 + letter.height as u32) {
                        for xl in letter.xoffset as u32..(letter.xoffset as u32 + letter.width as u32) {
                            let col = letter.data[(((yl - letter.yoffset as u32) * letter.width as u32) +
                                                   (xl - letter.xoffset as u32)) as usize];


                            let outpos = ((y + yl as usize)*width as usize) + (x + xl as usize);
                            let offset = outpos * 3;

                            let col_mapped = reindexing_table[col as usize + (color_idx * 8)] as usize;
                            buffer[offset + 0] = pal.data[col_mapped*3 + 0];
                            buffer[offset + 1] = pal.data[col_mapped*3 + 1];
                            buffer[offset + 2] = pal.data[col_mapped*3 + 2];
                        }
                    }
                }
                let letterwidth = self.letter_width(c);
                x = x + 1 + letterwidth as usize;
            }
            })
            .ok();
        return texture;
    }
}
*/
