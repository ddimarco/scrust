use std::env;

extern crate sdl2;
use sdl2::rect::Rect;
use sdl2::pixels::Color;

extern crate scrust;
use scrust::font::FontSize;

use scrust::font::RenderText;
use scrust::{GameContext, GameState, View, ViewAction};
use scrust::grp::GRP;
use scrust::pcx::PCX;
use scrust::scunits::{render_buffer_solid, render_buffer_with_solid_reindexing};

struct GRPView {
    grpfile: String,
    grp: GRP,
    frame: usize,
    reindex: bool,
    reindexing_table: Vec<u8>,
}
impl GRPView {
    fn new(context: &mut GameContext, grpfile: &str, use_reindex: bool, reindexfile: &str) -> Self {
        let pal = context.gd.install_pal.to_sdl();
        context.screen.set_palette(&pal).ok();
        let grp = GRP::read(&mut context.gd.open(grpfile).unwrap());
        let reindex = PCX::read(&mut context.gd.open(reindexfile).unwrap());
        GRPView {
            grpfile: grpfile.to_owned(),
            grp: grp,
            frame: 0,
            reindex: use_reindex,
            reindexing_table: reindex.data,
        }
    }
}
impl View for GRPView {
    fn render(&mut self, context: &mut GameContext, _: &GameState, _: f64) -> ViewAction {
        if context.events.now.quit || context.events.now.key_escape == Some(true) {
            return ViewAction::Quit;
        }

        if context.events.now.key_space == Some(true) {
            self.frame = (self.frame + 1) % self.grp.header.frame_count;
        }
        if context.events.now.key_r == Some(true) {
            self.reindex = !self.reindex;
        }

        context.screen.fill_rect(None, Color::RGB(0, 0, 0)).ok();

        let title_rect = Rect::new(50, 10, 100, 100);
        let fnt = &context.gd.font(FontSize::Font14);
        let screen_pitch = context.screen.pitch();
        let reindex = &context.gd.font_reindex.data;
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
                    "/home/dm/.wine/drive_c/StarCraft/",
                    |gc, _| Box::new(GRPView::new(gc, grpfile, use_reindex, reindexfile)));

}
