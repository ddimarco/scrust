extern crate sdl2;
use sdl2::rect::Rect;
use sdl2::pixels::Color;
use sdl2::render::{Renderer, Texture};

extern crate scrust;

use scrust::{GameContext, View, ViewAction, LayerTrait};
use scrust::grp::GRP;
use scrust::pal::Palette;
use scrust::pcx::PCX;

use scrust::ui::UiLayer;


struct LayersExampleView {
    ui_layer: UiLayer,
}
impl LayersExampleView {
    fn new(context: &mut GameContext) -> LayersExampleView {
        let pal = context.gd.fontmm_reindex.palette.to_sdl();
        context.screen.set_palette(&pal).ok();
        LayersExampleView { ui_layer: UiLayer::new(context) }
    }
}
impl View for LayersExampleView {
    fn render(&mut self, context: &mut GameContext, _: f64) -> ViewAction {
        if context.events.now.quit || context.events.now.key_escape == Some(true) {
            return ViewAction::Quit;
        }

        // clear the screen
        context.screen.fill_rect(None, Color::RGB(0, 0, 0)).ok();
        self.ui_layer.update(context);

        ViewAction::None
    }

    fn render_layers(&mut self, context: &mut GameContext) {
        self.ui_layer.render(&mut context.renderer);
    }
}

fn main() {
    ::scrust::spawn("layers",
                      "/home/dm/.wine/drive_c/StarCraft/",
                      |gc| Box::new(LayersExampleView::new(gc)));

}
