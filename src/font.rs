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
    /// 	Index of the last letter in file
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
        for ofs in letter_offsets {
            file.seek(SeekFrom::Start(ofs as u64)).ok();

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

                i += skipped as usize;
                if i < datasize {
                    data[i] = color_idx;
                }
                i += 1;
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

    pub fn get_letter(&self, c: char) -> &FontLetter {
        assert!(c != ' ');
        &self.letters[(c as usize) - 33]
    }

    pub fn letter_width(&self, c: char) -> u32 {
        if c == ' ' {
            min((self.header.max_width as u32) / 3, 6)
        } else {
            let letter = &self.get_letter(c);
            (letter.xoffset as u32) + (letter.width as u32)
        }
    }

    pub fn line_height(&self) -> u32 {
        let letter = &self.get_letter('y');
        (letter.yoffset as u32) + (letter.height as u32)
    }
}

// render into 8bit screen buffer

extern crate sdl2;
use self::sdl2::rect::Rect;

#[derive(Clone)]
pub enum HorizontalAlignment {
    Left,
    Center,
    Right
}
#[derive(Clone)]
pub enum VerticalAlignment {
    Top,
    Center,
    Bottom
}

pub struct TextLayout {
    width: u32,
    height: u32,
}

pub trait RenderText {
    fn layout(&self, text: &str) -> TextLayout;

    fn render_text_aligned(&self,
                           text: &str,
                           color_idx: usize,
                           reindexing_table: &[u8],
                           trg_buf: &mut [u8],
                           trg_pitch: u32,
                           trg_rect: &Rect,
                           halign: HorizontalAlignment,
                           valign: VerticalAlignment);
    fn render_textbox(&self,
                      text: &str,
                      color_idx: usize,
                      reindexing_table: &[u8],
                      trg_buf: &mut [u8],
                      trg_pitch: u32,
                      trg_rect: &Rect);
    }
impl RenderText for Font {
    fn layout(&self, text: &str) -> TextLayout {
        let mut w = 0;
        let h = self.line_height();
        for c in text.chars() {
            if c == ' ' {
                w = w + 1 + self.letter_width(' ');
            }
            if (c as u8) > 32 {
                w += 1 + self.letter_width(c);
            }
        }
        TextLayout {
            width: w,
            height: h,
        }
    }

    fn render_text_aligned(&self,
                           text: &str,
                           color_idx: usize,
                           reindexing_table: &[u8],
                           trg_buf: &mut [u8],
                           trg_pitch: u32,
                           trg_rect: &Rect,
                           halign: HorizontalAlignment,
                           valign: VerticalAlignment,
    ) {
        let layout = self.layout(text);
        let x = match halign {
            HorizontalAlignment::Left => {
                trg_rect.x()
            },
            HorizontalAlignment::Center => {
                let cx = trg_rect.left() + (trg_rect.width() as i32) / 2;
                cx - (layout.width as i32) / 2
            },
            HorizontalAlignment::Right => {
                trg_rect.right() - (layout.width as i32)
            }
        };
        let y = match valign {
            VerticalAlignment::Top => {
                trg_rect.y()
            },
            VerticalAlignment::Center => {
                let cy = trg_rect.top() + (trg_rect.height() as i32) / 2;
                cy - (layout.height as i32) / 2
            },
            VerticalAlignment::Bottom => {
                trg_rect.bottom() - (layout.height as i32)
            }
        };
        let r = Rect::new(x, y, layout.width, layout.height);
        self.render_textbox(text, color_idx, reindexing_table,
                            trg_buf, trg_pitch, &r);
    }

    fn render_textbox(&self,
                      text: &str,
                      color_idx_initial: usize,
                      reindexing_table: &[u8],
                      trg_buf: &mut [u8],
                      trg_pitch: u32,
                      trg_rect: &Rect) {
        // for now, assume only single lines
        let y = trg_rect.y() as usize;
        let mut x = trg_rect.x() as usize;
        let mut color_idx = color_idx_initial;
        // TODO: proper reindexing?
        for c in text.chars() {
            let c_int = c as u8;
            if c_int < 32 {
                // color code
                color_idx =
                    if c_int == 1 {
                        color_idx_initial
                    } else {
                        (c_int - 2) as usize
                    };
            } else if c == ' ' {
                x = x + 1 + self.letter_width(' ') as usize;
            } else {
                let letter = &self.get_letter(c);
                for yl in (letter.yoffset as u32)..(letter.yoffset as u32 + letter.height as u32) {
                    for xl in letter.xoffset as u32..(letter.xoffset as u32 + letter.width as u32) {
                        let col = letter.data[(((yl - letter.yoffset as u32) * letter.width as u32) +
                                               (xl - letter.xoffset as u32)) as usize];
                        if col != 0 {
                            let outpos = ((y + yl as usize) * trg_pitch as usize) + (x + xl as usize);
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
}

// render into rgb24 texture
//
// use ::pal::Palette;
//
// extern crate sdl2;
// use self::sdl2::pixels::PixelFormatEnum;
// use self::sdl2::rect::Rect;
// use self::sdl2::render::{Renderer, Texture};
//
// pub trait RenderText {
// fn render_textbox(&self,
// text: &str,
// color_idx: usize,
// renderer: &mut Renderer,
// pal: &Palette,
// reindexing_table: &[u8],
// width: u32,
// height: u32)
// -> sdl2::render::Texture;
// }
// impl RenderText for Font {
// fn render_textbox(&self,
// text: &str,
// color_idx: usize,
// renderer: &mut Renderer,
// pal: &Palette,
// reindexing_table: &[u8],
// width: u32,
// height: u32)
// -> sdl2::render::Texture {
// let mut texture =
// renderer.create_texture_streaming(PixelFormatEnum::RGB24, width, height).unwrap();
//
// for now, assume only single lines
// texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
// let y = 0;
// let mut x: usize = 0;
// for c in text.chars() {
// if c != ' ' {
// let ref letter = self.get_letter(c);
// for yl in (letter.yoffset as u32)..(letter.yoffset as u32 + letter.height as u32) {
// for xl in letter.xoffset as u32..(letter.xoffset as u32 + letter.width as u32) {
// let col = letter.data[(((yl - letter.yoffset as u32) * letter.width as u32) +
// (xl - letter.xoffset as u32)) as usize];
//
//
// let outpos = ((y + yl as usize)*width as usize) + (x + xl as usize);
// let offset = outpos * 3;
//
// let col_mapped = reindexing_table[col as usize + (color_idx * 8)] as usize;
// buffer[offset + 0] = pal.data[col_mapped*3 + 0];
// buffer[offset + 1] = pal.data[col_mapped*3 + 1];
// buffer[offset + 2] = pal.data[col_mapped*3 + 2];
// }
// }
// }
// let letterwidth = self.letter_width(c);
// x = x + 1 + letterwidth as usize;
// }
// })
// .ok();
// return texture;
// }
// }
