extern crate scrust;
use scrust::pcx::{PCX};


use scrust::{GameContext, View, ViewAction};

struct PCXView {
    pcx: PCX,
}
impl PCXView {
    fn new(context: &mut GameContext, pcx_filename: &str) -> PCXView {
        let pcx = PCX::read(&mut context.gd.open(pcx_filename).unwrap());
        context.screen.set_palette(&pcx.palette.to_sdl()).expect("could not set palette");
        PCXView {
            pcx: pcx,
        }
    }
}
impl View for PCXView {
    fn render(&mut self, context: &mut GameContext, _: f64) -> ViewAction {
        if context.events.now.quit || context.events.now.key_escape == Some(true) {
            return ViewAction::Quit;
        }

        context.screen.with_lock_mut(|buffer: &mut [u8]| {
            buffer.clone_from_slice(&self.pcx.data.as_slice());
        });

        ViewAction::None
    }
}

fn main() {
    ::scrust::spawn("pcx loading", "/home/dm/.wine/drive_c/StarCraft/", |gc| {
        Box::new(PCXView::new(gc, "glue\\title\\title.pcx"))
    });
}
