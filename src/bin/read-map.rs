use std::env;

#[macro_use]
extern crate scrust;
use scrust::{GameContext, View, ViewAction};
use scrust::terrain::{Map};

use scrust::scunits::{SCUnit, IScriptableTrait, SCImageTrait};

extern crate sdl2;
use sdl2::pixels::Color;

struct MapView {
    map: Map,
    map_x: u16,
    map_y: u16,

    units: Vec<SCUnit>,
}
impl MapView {
    fn new(context: &mut GameContext, mapfn: &str) -> MapView {
        let map = Map::read(&context.gd, mapfn);
        println!("map name: {}", map.name());
        println!("map desc: {}", map.description());
        context.screen.set_palette(&map.terrain_info.pal.to_sdl()).ok();

        // create map units
        let mut units = Vec::<SCUnit>::new();
        for mapunit in &map.data.units {
            // XXX: make use of mapunit data
            let unit = SCUnit::new(&context.gd, mapunit.unit_id as usize,
                                   mapunit.x, mapunit.y);
            units.push(unit);
        }

        MapView {
            map: map,
            map_x: 0,
            map_y: 0,
            units: units,
        }
    }
}
impl View for MapView {
    fn render(&mut self, context: &mut GameContext, _: f64) -> ViewAction {
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
        // HACK
        let buffer_height = 480;
        {
            let grp_cache = &*context.gd.grp_cache.borrow();
            let mut screen = &mut context.screen;
            screen.with_lock_mut(|buffer: &mut [u8]| {
                self.map.render(self.map_x, self.map_y, MAP_RENDER_W, MAP_RENDER_H,
                                buffer, screen_pitch);

                // units
                let right_map_x = self.map_x + screen_pitch as u16;
                let bottom_map_y = self.map_y + buffer_height as u16;
                for u in &self.units {
                    if u.get_iscript_state().map_pos_x > self.map_x &&
                        u.get_iscript_state().map_pos_x < right_map_x &&
                        u.get_iscript_state().map_pos_y > self.map_y &&
                        u.get_iscript_state().map_pos_y < bottom_map_y {
                            let cx = (u.get_iscript_state().map_pos_x - self.map_x) as u32;
                            let cy = (u.get_iscript_state().map_pos_y - self.map_y) as u32;

                            u.get_scimg().draw(grp_cache, cx, cy, buffer, screen_pitch);
                        }
                }
            });

        }

        for u in &mut self.units {
            u.get_scimg_mut().step(&context.gd);
        }


        ViewAction::None
        }
}


fn main() {
    ::scrust::spawn("font rendering", "/home/dm/.wine/drive_c/StarCraft/", |gc| {
        let args: Vec<String> = env::args().collect();
        let mapfn = if args.len() == 2 {args[1].clone()} else { String::from("/home/dm/.wine/drive_c/StarCraft/Maps/(2)Space Madness.scm")};
        Box::new(MapView::new(gc, &mapfn))
    });

}
