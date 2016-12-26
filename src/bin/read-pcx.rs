extern crate sdl2;
extern crate scrust;
use scrust::pcx::PCX;

use scrust::gamedata::GameData;
use scrust::{GameContext, GameState, View, ViewAction};

struct PCXView {
    pcx: PCX,
}
impl PCXView {
    fn new(gd: &GameData, context: &mut GameContext, pcx_filename: &str) -> PCXView {
        let pcx = PCX::read(&mut gd.open(pcx_filename).unwrap());
        context.screen.set_palette(&pcx.palette.to_sdl()).expect("could not set palette");
        PCXView { pcx: pcx }
    }
}
impl View for PCXView {
    fn render(&mut self, _: &GameData, context: &mut GameContext, _: &GameState, _: f64) -> ViewAction {
        if context.events.now.quit || context.events.now.is_key_pressed(&sdl2::keyboard::Keycode::Escape) {
            return ViewAction::Quit;
        }

        context.screen.with_lock_mut(|buffer: &mut [u8]| {
            buffer.clone_from_slice(&self.pcx.data.as_slice());
        });

        ViewAction::None
    }
}

fn main() {
    ::scrust::spawn("pcx loading",
                    "/home/dm/.wine/drive_c/StarCraft/",
                    |gd, gc, _| Box::new(PCXView::new(gd, gc, "glue\\title\\title.pcx")));
}
