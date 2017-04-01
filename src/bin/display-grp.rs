use std::env;

extern crate sdl2;
use sdl2::rect::Rect;
use sdl2::pixels::Color;

extern crate scrust;
use scrust::{GameContext, GameState, View, ViewAction};
use scrust::gamedata::GameData;
use scrust::render::{render_buffer_solid, render_buffer_with_solid_reindexing};

extern crate scformats;
use scformats::grp::GRP;
use scformats::pcx::PCX;
use scformats::font::{RenderText, FontSize};
use scformats::terrain::GameDataTrait;

struct GRPView {
    grpfile: String,
    grp: GRP,
    frame: usize,
    reindex: bool,
    reindexing_table: Vec<u8>,
}
impl GRPView {
    fn new(gd: &GameData, context: &mut GameContext, grpfile: &str, use_reindex: bool, reindexfile: &str) -> Self {
        let pal = gd.install_pal.to_sdl();
        context.screen.set_palette(&pal).ok();
        let grp = GRP::read(&mut gd.open(grpfile).unwrap());
        let reindex = PCX::read(&mut gd.open(reindexfile).unwrap());
        GRPView {
            grpfile: grpfile.to_owned(),
            grp: grp,
            frame: 0,
            reindex: use_reindex,
            reindexing_table: reindex.data,
        }
    }
}
use sdl2::keyboard::Keycode;
impl View for GRPView {
    fn render(&mut self, gd: &GameData, context: &mut GameContext, _: &GameState, _: f64) -> ViewAction {
        if context.events.now.quit || context.events.now.is_key_pressed(&Keycode::Escape) {
            return ViewAction::Quit;
        }

        if context.events.now.is_key_pressed(&Keycode::Space) {
            self.frame = (self.frame + 1) % self.grp.header.frame_count;
        }
        if context.events.now.is_key_pressed(&Keycode::R) {
            self.reindex = !self.reindex;
        }

        context.screen.fill_rect(None, Color::RGB(0, 0, 0)).ok();

        let title_rect = Rect::new(50, 10, 100, 100);
        let fnt = &gd.font(FontSize::Font14);
        let screen_pitch = context.screen.pitch();
        //let reindex = &gd.font_reindex.data;
        let reindex = &gd.font_reindexing_store.get_game_reindex().data;
        let title = format!("frame {}/{} of {}",
                            self.frame,
                            self.grp.header.frame_count,
                            self.grpfile);
        context.screen.with_lock_mut(|buffer: &mut [u8]| {
            fnt.render_textbox(title.as_ref(),
                               0,
                               reindex,
                               buffer,
                               screen_pitch,
                               &title_rect);

            if self.reindex {
                render_buffer_with_solid_reindexing(&self.grp.frames[self.frame],
                                                    self.grp.header.width as u32,
                                                    self.grp.header.height as u32,
                                                    false,
                                                    100,
                                                    100,
                                                    buffer,
                                                    screen_pitch,
                                                    &self.reindexing_table);
            } else {
                render_buffer_solid(&self.grp.frames[self.frame],
                                    self.grp.header.width as u32,
                                    self.grp.header.height as u32,
                                    false,
                                    100,
                                    100,
                                    buffer,
                                    screen_pitch);
            }
        });



        ViewAction::None
    }
}


fn main() {
    let args: Vec<String> = env::args().collect();
    let grpfile = if args.len() < 2 {
        "unit/cmdbtns/cmdicons.grp"
    } else {
        args[1].as_ref()
    };
    let reindexfile = if args.len() < 3 {
        "unit\\cmdbtns\\ticon.pcx"
    } else {
        args[2].as_ref()
    };
    let use_reindex = args.len() >= 3;

    ::scrust::spawn("grp viewer",
                    |gd, gc, _| Box::new(GRPView::new(gd, gc, grpfile, use_reindex, reindexfile)));

}
