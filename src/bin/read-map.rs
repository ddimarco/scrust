use std::env;

#[macro_use]
extern crate scrust;
use scrust::{GameContext, View, ViewAction};
use scrust::terrain::Map;
use scrust::scunits::{SCUnit, SCSprite, IScriptableTrait, SCImageTrait, IScriptEntityAction};
use scrust::gamedata::GRPCache;
use scrust::pal::palimg_to_texture;

use scrust::LayerTrait;
use scrust::ui::UiLayer;

extern crate sdl2;
use sdl2::pixels::Color;
use sdl2::render::Texture;
use sdl2::rect::Rect;


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
    map_x: u16,
    map_y: u16,

    units_layer: UnitsLayer,
    ui_layer: UiLayer,
    minimap: Texture,
    mmapwratio: f32,
    mmaphratio: f32,
    mmap_cur_rect: Rect,
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

        let mmap_bmp = map.render_minimap();
        let mmap = palimg_to_texture(&mut context.renderer,
                                     map.data.width as u32, map.data.height as u32,
                                     &mmap_bmp, &map.terrain_info.pal);

        let mapw2mmapw_ratio: f32 = 128. / (map.data.width as f32);
        let maph2mmaph_ratio: f32 = 128. / (map.data.height as f32);
        let mmap_cur_rect = Rect::new(0, 0,
                                      (MAP_RENDER_W as f32 * mapw2mmapw_ratio) as u32,
                                      (MAP_RENDER_H as f32 * maph2mmaph_ratio) as u32);

        MapView {
            map: map,
            map_x: 0,
            map_y: 0,
            units_layer: units_layer,
            ui_layer: UiLayer::new(context),
            minimap: mmap,
            mmap_cur_rect: mmap_cur_rect,
            mmapwratio: mapw2mmapw_ratio,
            mmaphratio: maph2mmaph_ratio,
        }
    }
}
impl View for MapView {
    fn render(&mut self, context: &mut GameContext, _: f64) -> ViewAction {
        const SCROLLING_SPEED: u16 = 4;
        if context.events.now.quit || context.events.now.key_escape == Some(true) {
            return ViewAction::Quit;
        }
        if context.events.key_left {
            if self.map_x > 0 {
                self.map_x -= SCROLLING_SPEED;
            }
        } else if context.events.key_right {
            if self.map_x / 32 + MAP_RENDER_W < self.map.data.width {
                self.map_x += SCROLLING_SPEED;
            }
        }
        if context.events.key_up {
            if self.map_y > 0 {
                self.map_y -= SCROLLING_SPEED;
            }
        } else if context.events.key_down {
            if self.map_y / 32 + MAP_RENDER_H < self.map.data.height {
                self.map_y += SCROLLING_SPEED;
            }
        }

        self.units_layer.update(context);
        self.ui_layer.update(context);

        // clear the screen
        context.screen.fill_rect(None, Color::RGB(0, 0, 0)).ok();
        let screen_pitch = context.screen.pitch();
        // HACK
        let buffer_height = 480;

        {
            let grp_cache = &*context.gd.grp_cache.borrow();
            let mut screen = &mut context.screen;
            screen.with_lock_mut(|buffer: &mut [u8]| {
                self.map.render(self.map_x,
                                self.map_y,
                                MAP_RENDER_W,
                                MAP_RENDER_H,
                                buffer,
                                screen_pitch);

                self.units_layer.render(self.map_x,
                                        self.map_y,
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

        context.renderer.copy(&self.minimap, None, Some(self.ui_layer.mmap_rect));

        let new_x = 6 + (self.map_x as f32 * self.mmapwratio / 32.) as i32;
        let new_y = 348 + (self.map_y as f32 * self.mmaphratio / 32.) as i32;
        self.mmap_cur_rect.set_x(new_x);
        self.mmap_cur_rect.set_y(new_y);

        context.renderer.set_draw_color(Color::RGB(255, 255, 255));
        context.renderer.draw_rect(self.mmap_cur_rect);
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
