use std::env;

#[macro_use]
extern crate read_pcx;
use read_pcx::{GameContext, View, ViewAction};
use read_pcx::terrain::{Map};

extern crate sdl2;
use sdl2::pixels::Color;

struct MapView {
    map: Map,
    map_x: u16,
    map_y: u16,
}
impl MapView {
    fn new(context: &mut GameContext, mapfn: &str) -> MapView {
        let map = Map::read(&context.gd, mapfn);
        println!("map name: {}", map.name());
        println!("map desc: {}", map.description());
        context.screen.set_palette(&map.terrain_info.pal.to_sdl()).ok();
        MapView {
            map: map,
            map_x: 0,
            map_y: 0,
        }
    }
}
impl View for MapView {
    fn render(&mut self, context: &mut GameContext, elapsed: f64) -> ViewAction {
        const MAP_RENDER_W: u16 = 20;
        const MAP_RENDER_H: u16 = 12;
        const SCROLLING_SPEED: u16 = 4;
        if context.events.now.quit || context.events.now.key_escape == Some(true) {
            return ViewAction::Quit;
        }
        if context.events.key_left {
            if self.map_x > 0 {
                self.map_x -= SCROLLING_SPEED;
            }
        } else if context.events.key_right {
            if self.map_x/32 + MAP_RENDER_W < self.map.data.width {
                self.map_x += SCROLLING_SPEED;
            }
        }
        if context.events.key_up {
            if self.map_y > 0 {
                self.map_y -= SCROLLING_SPEED;
            }
        } else if context.events.key_down {
            if self.map_y/32 + MAP_RENDER_H < self.map.data.height {
                self.map_y += SCROLLING_SPEED;
            }
        }

        // clear the screen
        context.screen.fill_rect(None, Color::RGB(0,0,0)).ok();
        let screen_pitch = context.screen.pitch();
        context.screen.with_lock_mut(|buffer: &mut [u8]| {
            self.map.render(self.map_x, self.map_y, MAP_RENDER_W, MAP_RENDER_H,
                            buffer, screen_pitch);
        });

        ViewAction::None
    }
}


fn main() {
    ::read_pcx::spawn("font rendering", "/home/dm/.wine/drive_c/StarCraft/", |gc| {
        let args: Vec<String> = env::args().collect();
        let mapfn = if args.len() == 2 {args[1].clone()} else { String::from("/home/dm/.wine/drive_c/StarCraft/Maps/(2)Space Madness.scm")};
        Box::new(MapView::new(gc, &mapfn))
    });

}
