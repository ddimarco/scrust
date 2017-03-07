extern crate sdl2;
use sdl2::rect::Rect;

extern crate scrust;

extern crate scformats;
use scformats::font::FontSize;
use scformats::font::RenderText;
use scrust::{GameContext, GameState, View, ViewAction};
use scrust::gamedata::GameData;

struct FontView {
    text: String,
    font_size: FontSize,
    color_idx: usize,
    trg_rect: Rect,
}
impl FontView {
    fn new(gd: &GameData,
           context: &mut GameContext,
           text: &str,
           font_size: FontSize,
           color_idx: usize)
           -> FontView {
        let pal = gd.font_reindexing_store.get_menu_reindex("mm").palette.to_sdl();
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
    fn render(&mut self, gd: &GameData, context: &mut GameContext, _: &GameState, _: f64) -> ViewAction {
        if context.events.now.quit || context.events.now.is_key_pressed(&sdl2::keyboard::Keycode::Escape) {
            return ViewAction::Quit;
        }

        let fnt = &gd.font(self.font_size);
        let screen_pitch = context.screen.pitch();
        //let reindex = &gd.fontmm_reindex.data;
        let reindex = &gd.font_reindexing_store.get_menu_reindex("mm").data;
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
    ::scrust::spawn("font rendering",
                    "/home/dm/.wine/drive_c/StarCraft/",
                    |gd, gc, _| Box::new(FontView::new(gd, gc, "Na wie isses?", FontSize::Font16, 0)));

}
