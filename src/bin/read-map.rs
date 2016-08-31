use std::env;

#[macro_use]
extern crate scrust;
use scrust::{GameContext, View, ViewAction, GameEvents};
use scrust::terrain::Map;
use scrust::scunits::{SCUnit, SCSprite, IScriptableTrait, SCImageTrait, IScriptEntityAction};
use scrust::gamedata::GRPCache;
use scrust::pal::palimg_to_texture;

use scrust::LayerTrait;
use scrust::ui::UiLayer;

extern crate sdl2;
use sdl2::pixels::Color;
use sdl2::render::Texture;
use sdl2::rect::{Rect, Point};


struct UnitsLayer {
    units: Vec<SCUnit>,

    // XXX distinguish high & low layer
    sprites: Vec<SCSprite>,
}
impl UnitsLayer {
    fn from_map(context: &mut GameContext, map: &Map) -> Self {
        // create map units
        let mut units = Vec::<SCUnit>::new();
        for mapunit in &map.data.units {
            // XXX: make use of mapunit data
            let unit = SCUnit::new(&context.gd, mapunit.unit_id as usize, mapunit.x, mapunit.y);
            units.push(unit);
        }

        let mut sprites = Vec::<SCSprite>::new();
        for mapsprite in &map.data.sprites {
            let sprite = SCSprite::new(&context.gd, mapsprite.sprite_no, mapsprite.x, mapsprite.y);
            sprites.push(sprite);
        }
        UnitsLayer {
            units: units,
            sprites: sprites,
        }
    }

    fn update(&mut self, context: &GameContext) {
        for u in &mut self.units {
            let action = u.get_scimg_mut().step(&context.gd);
            match action {
                // XXX distinguish high & low layer
                Some(IScriptEntityAction::CreateSpriteOverlay {sprite_id, x, y}) => {
                    let sprite = SCSprite::new(&context.gd, sprite_id, x, y);
                    self.sprites.push(sprite);
                },
                Some(IScriptEntityAction::CreateSpriteUnderlay {sprite_id, x, y}) => {
                    let sprite = SCSprite::new(&context.gd, sprite_id, x, y);
                    self.sprites.push(sprite);
                },
                _ => {}
            }
        }
    }

    fn generate_events(&self, gc: &GameContext) -> Vec<GameEvents> {
        let events = Vec::<GameEvents>::new();
        events
    }

    fn render(&self,
              map_x: u16,
              map_y: u16,
              grp_cache: &GRPCache,
              buffer: &mut [u8],
              screen_pitch: u32,
              screen_height: usize) {
        // units
        let right_map_x = map_x + screen_pitch as u16;
        let bottom_map_y = map_y + screen_height as u16;
        // FIXME: draw in proper order
        for u in &self.sprites {
            if u.get_iscript_state().map_pos_x > map_x &&
               u.get_iscript_state().map_pos_x < right_map_x &&
               u.get_iscript_state().map_pos_y > map_y &&
               u.get_iscript_state().map_pos_y < bottom_map_y {
                let cx = (u.get_iscript_state().map_pos_x - map_x) as u32;
                let cy = (u.get_iscript_state().map_pos_y - map_y) as u32;

                u.get_scimg().draw(grp_cache, cx, cy, buffer, screen_pitch);
            }
        }

        for u in &self.units {
            if u.get_iscript_state().map_pos_x > map_x &&
               u.get_iscript_state().map_pos_x < right_map_x &&
               u.get_iscript_state().map_pos_y > map_y &&
               u.get_iscript_state().map_pos_y < bottom_map_y {
                let cx = (u.get_iscript_state().map_pos_x - map_x) as u32;
                let cy = (u.get_iscript_state().map_pos_y - map_y) as u32;

                u.get_scimg().draw(grp_cache, cx, cy, buffer, screen_pitch);
            }
        }
    }
}


struct MapView {
    map: Map,

    units_layer: UnitsLayer,
    ui_layer: UiLayer,
}
const MAP_RENDER_W: u16 = 20;
const MAP_RENDER_H: u16 = 12;
impl MapView {
    fn new(context: &mut GameContext, mapfn: &str) -> Self {
        let map = Map::read(&context.gd, mapfn);
        println!("map name: {}", map.name());
        println!("map desc: {}", map.description());
        context.screen.set_palette(&map.terrain_info.pal.to_sdl()).ok();
        let units_layer = UnitsLayer::from_map(context, &map);
        let ui_layer = UiLayer::new(context, &map);

        MapView {
            map: map,
            units_layer: units_layer,
            ui_layer: ui_layer,
        }
    }
}
impl View for MapView {
    fn render(&mut self, context: &mut GameContext, _: f64) -> ViewAction {
        if context.events.now.quit || context.events.now.key_escape == Some(true) {
            return ViewAction::Quit;
        }

        self.units_layer.update(context);
        self.ui_layer.update(context);

        // clear the screen
        context.screen.fill_rect(None, Color::RGB(0, 0, 0)).ok();
        let screen_pitch = context.screen.pitch();
        // HACK
        let buffer_height = 480;

        let map_x = context.map_pos.x() as u16;
        let map_y = context.map_pos.y() as u16;

        {
            let grp_cache = &*context.gd.grp_cache.borrow();
                let mut screen = &mut context.screen;
                screen.with_lock_mut(|buffer: &mut [u8]| {
                    self.map.render(map_x,
                                    map_y,
                                    MAP_RENDER_W,
                                    MAP_RENDER_H,
                                    buffer,
                                    screen_pitch);

                    self.units_layer.render(map_x,
                                            map_y,
                                            grp_cache,
                                            buffer,
                                            screen_pitch,
                                            buffer_height);

                });
            }


            ViewAction::None
        }

        fn render_layers(&mut self, context: &mut GameContext) {
            self.ui_layer.render(&mut context.renderer);
        }
        fn process_layer_events(&mut self, gc: &mut GameContext) {
            for ev in &gc.game_events {
                if self.ui_layer.process_event(ev) {
                    continue;
                }

                // FIXME: create map layer
                match *ev {
                    GameEvents::MoveMap(x, y) => {
                        gc.map_pos = Point::new(x, y);
                    },
                    _ => {}
            }
        }


        gc.game_events.clear();
    }

    fn generate_layer_events(&self, context: &mut GameContext) {
        let mut vecevents = self.ui_layer.generate_events(context);

        context.game_events.extend(vecevents);
    }

}


fn main() {
    ::scrust::spawn("font rendering",
                    "/home/dm/.wine/drive_c/StarCraft/",
                    |gc| {
        let args: Vec<String> = env::args().collect();
        let mapfn = if args.len() == 2 {
            args[1].clone()
        } else {
            String::from("/home/dm/.wine/drive_c/StarCraft/Maps/(2)Space Madness.scm")
        };
        Box::new(MapView::new(gc, &mapfn))
    });

}
