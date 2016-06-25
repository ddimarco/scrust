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

use read_pcx::font::RenderText;


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
