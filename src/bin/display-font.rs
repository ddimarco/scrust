use std::path::Path;

extern crate sdl2;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;
use sdl2::render::{Renderer, Texture};

extern crate read_pcx;
use read_pcx::font::{Font, FontSize};
use read_pcx::pcx::PCX;
use read_pcx::pal::Palette;

use read_pcx::gamedata::GameData;

use std::fs::File;
use std::io::Write;

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
            let mut y = 0;
            let mut x: usize = 0;

            let mut outfile = File::create("/tmp/font.ppm").unwrap();
            outfile.write_fmt(format_args!("P3\n")).ok();
            outfile.write_fmt(format_args!("{0} {1}\n", width, height)).ok();
            outfile.write_fmt(format_args!("255\n")).ok();

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

            let pixelsize = 3;
            for i in 0..buffer.len()/pixelsize {
                outfile.write_fmt(format_args!("{0} {1} {2}\n",
                                               buffer[i * pixelsize + 0],
                                               buffer[i * pixelsize + 1],
                                               buffer[i * pixelsize + 2],
                                               )).ok();
            }


            })
            .ok();
        return texture;
    }
}

fn main() {
    println!("opening mpq...");
    let gd = GameData::init(&Path::new("/home/dm/.wine/drive_c/StarCraft/"));

    let fnt = gd.font(FontSize::Font16);
    println!("low-id: {}, high-idx: {}, max-width: {}, max-height: {}",
             fnt.header.low_idx,
             fnt.header.high_idx,
             fnt.header.max_width,
             fnt.header.max_height);

    // sdl
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("sc font display", 640, 480)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let mut renderer = window.renderer().build().unwrap();

    let (w, h) = (320, fnt.line_height());
    let texture = fnt.render_textbox("Na, wie isses?", 0, &mut renderer,
                                     &gd.fontmm_reindex.palette, &gd.fontmm_reindex.data, w, h);
    println!("w: {}, h: {}", w, h);

    renderer.set_draw_color(Color::RGBA(0, 0, 0, 0));
    renderer.clear();
    renderer.copy(&texture, None, Some(sdl2::rect::Rect::new(0, 0, w, h)));
    renderer.present();
    let mut event_pump = sdl_context.event_pump().unwrap();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'running,
                _ => {}
            }
        }
    }
}
