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
use read_pcx::font::RenderText;

use read_pcx::{GameContext, View, ViewAction};

use std::fs::File;
use std::io::Write;

struct FontView {
    text: String,
    font_size: FontSize,
    color_idx: usize,
    trg_rect: Rect,
}
impl FontView {
    fn new(context: &mut GameContext, text: &str, font_size: FontSize, color_idx: usize) -> FontView {
        let pal = context.gd.fontmm_reindex.palette.to_sdl();
        context.screen.set_palette(&pal).ok();
        FontView {
            text: text.to_owned(),
            font_size: font_size,
            color_idx: color_idx,
            trg_rect: Rect::new(50, 50, 100, 100),
        }
    }
}
impl View for FontView {
    fn render(&mut self, context: &mut GameContext, elapsed: f64) -> ViewAction {
        if context.events.now.quit || context.events.now.key_escape == Some(true) {
            return ViewAction::Quit;
        }

        let fnt = context.gd.font(FontSize::Font16);
        let screen_pitch = context.screen.pitch();
        let reindex = &context.gd.fontmm_reindex.data;
        context.screen.with_lock_mut(|buffer: &mut [u8]| {
            fnt.render_textbox(self.text.as_ref(),
                               self.color_idx,
                               reindex,
                               buffer,
                               screen_pitch,
                               &self.trg_rect);
        });

        ViewAction::None
    }
}


fn main() {
    ::read_pcx::spawn("font rendering", "/home/dm/.wine/drive_c/StarCraft/", |gc| {
        Box::new(FontView::new(gc, "Na wie isses?", FontSize::Font16, 0))
    });

}
