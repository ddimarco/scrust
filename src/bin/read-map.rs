use std::env;

#[macro_use]
extern crate scrust;
use scrust::{GameContext, GameState, View, ViewAction, GameEvents, MousePointerType};
use scrust::terrain::Map;
use scrust::scunits::{SCUnit, SCSprite, IScriptableTrait, SCImageTrait, IScriptEntityAction,
                      SCSpriteTrait};
use scrust::gamedata::GRPCache;

use scrust::LayerTrait;
use scrust::ui::UiLayer;

extern crate sdl2;
use sdl2::pixels::Color;
use sdl2::rect::Point;

struct UnitsLayer {
    // XXX distinguish high & low layer
    sprites: Vec<SCSprite>,

    cursor_over_unit: bool,
}
impl UnitsLayer {
    fn from_map(context: &mut GameContext, state: &mut GameState, map: &Map) -> Self {
        // create map units
        for mapunit in &map.data.units {
            // XXX: make use of mapunit data
            let unit = SCUnit::new(&context.gd,
                                   mapunit.unit_id as usize,
                                   mapunit.x,
                                   mapunit.y,
                                   mapunit.player_no as usize);
            let _ = state.unit_instances.put(unit);
        }

        let mut sprites = Vec::<SCSprite>::new();
        for mapsprite in &map.data.sprites {
            let sprite = SCSprite::new(&context.gd, mapsprite.sprite_no, mapsprite.x, mapsprite.y);
            sprites.push(sprite);
        }
        UnitsLayer {
            sprites: sprites,
            cursor_over_unit: false,
        }
    }

    fn update(&mut self, context: &GameContext, state: &mut GameState) {
        for (_, u) in &mut state.unit_instances {
            let action = u.get_scimg_mut().step(&context.gd);
            match action {
                // XXX distinguish high & low layer
                Some(IScriptEntityAction::CreateSpriteOverlay { sprite_id, x, y }) => {
                    let sprite = SCSprite::new(&context.gd, sprite_id, x, y);
                    self.sprites.push(sprite);
                }
                Some(IScriptEntityAction::CreateSpriteUnderlay { sprite_id, x, y }) => {
                    let sprite = SCSprite::new(&context.gd, sprite_id, x, y);
                    self.sprites.push(sprite);
                }
                _ => {}
            }
        }
    }

    fn generate_events(&mut self, gc: &GameContext, state: &GameState) -> Vec<GameEvents> {
        let mut events = Vec::<GameEvents>::new();

        // mouse over unit?
        let mouse_pos_map = state.map_pos + gc.events.mouse_pos;
        // let sel_radius = 10;
        let mut over_unit_instance = None;
        for (uinstance, u) in &state.unit_instances {
            // TODO: use sdl rects & points?
            match u.get_scsprite().selectable_data {
                Some(ref seldata) => {
                    let halfw = seldata.sel_width as i32 / 2;
                    let halfh = seldata.sel_height as i32 / 2;

                    let ux = u.get_iscript_state().map_pos_x as i32;
                    let uy = u.get_iscript_state().map_pos_y as i32;

                    if mouse_pos_map.x() > ux - halfw && mouse_pos_map.x() < ux + halfw &&
                       mouse_pos_map.y() > uy - halfh &&
                       mouse_pos_map.y() < uy + halfh {
                        over_unit_instance = Some(uinstance);
                        break;
                    }
                }
                _ => {}
            }

        }

        if over_unit_instance.is_some() && !self.cursor_over_unit {
            events.push(GameEvents::ChangeMouseCursor(MousePointerType::MagnifierGreen));

            self.cursor_over_unit = true;
        } else if over_unit_instance.is_none() && self.cursor_over_unit {
            events.push(GameEvents::ChangeMouseCursor(MousePointerType::Arrow));
            self.cursor_over_unit = false;
        }

        if over_unit_instance.is_some() && gc.events.now.mouse_left {
            events.push(GameEvents::SelectUnit(over_unit_instance.unwrap()));
        }

        events
    }

    fn render(&self,
              state: &GameState,
              map_x: u16,
              map_y: u16,
              grp_cache: &GRPCache,
              buffer: &mut [u8],
              screen_pitch: u32) {
        // FIXME: draw in proper order
        for u in &self.sprites {
            {
                let cx = u.get_iscript_state().map_pos_x as i32 - map_x as i32;
                let cy = u.get_iscript_state().map_pos_y as i32 - map_y as i32;

                u.get_scimg().draw(grp_cache, cx, cy, buffer, screen_pitch);
            }
        }

        for (idx, u) in &state.unit_instances {
            {
                let cx = u.get_iscript_state().map_pos_x as i32 - map_x as i32;
                let cy = u.get_iscript_state().map_pos_y as i32 - map_y as i32;

                let is_selected = state.selected_units.contains(&idx);
                if is_selected {
                    u.get_scsprite()
                        .draw_selection_circle(&grp_cache, cx, cy, buffer, screen_pitch);
                }
                u.get_scimg().draw(grp_cache, cx, cy, buffer, screen_pitch);

                // FIXME
                // if is_selected {
                //     u.get_scsprite().draw_healthbar(cx, cy, buffer, screen_pitch);
                // }

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
    fn new(context: &mut GameContext, state: &mut GameState, mapfn: &str) -> Self {
        let map = Map::read(&context.gd, mapfn);
        println!("map name: {}", map.name());
        println!("map desc: {}", map.description());
        context.screen.set_palette(&map.terrain_info.pal.to_sdl()).ok();
        let units_layer = UnitsLayer::from_map(context, state, &map);
        let ui_layer = UiLayer::new(context, &map);

        MapView {
            map: map,
            units_layer: units_layer,
            ui_layer: ui_layer,
        }
    }
}
impl View for MapView {
    fn update(&mut self, context: &mut GameContext, state: &mut GameState) {
        self.units_layer.update(context, state);
        self.ui_layer.update(context, state);
    }
    fn render(&mut self, context: &mut GameContext, state: &GameState, _: f64) -> ViewAction {
        if context.events.now.quit || context.events.now.key_escape == Some(true) {
            return ViewAction::Quit;
        }

        // clear the screen
        context.screen.fill_rect(None, Color::RGB(0, 0, 0)).ok();
        let screen_pitch = context.screen.pitch();

        let map_x = state.map_pos.x() as u16;
        let map_y = state.map_pos.y() as u16;

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

                self.units_layer.render(&state, map_x, map_y, grp_cache, buffer, screen_pitch);

            });
        }


        ViewAction::None
    }

    fn render_layers(&mut self, context: &mut GameContext) {
        self.ui_layer.render(&mut context.renderer);
    }
    fn process_layer_events(&mut self, _: &mut GameContext, state: &mut GameState) {
        for ev in &state.game_events {
            if self.ui_layer.process_event(ev) {
                continue;
            }

            match *ev {
                GameEvents::MoveMap(x, y) => {
                    state.map_pos = Point::new(x, y);
                }
                GameEvents::SelectUnit(uid) => {
                    state.selected_units.clear();
                    state.selected_units.push(uid);
                }
                _ => {}
            }
        }


        state.game_events.clear();
    }

    fn generate_layer_events(&mut self, context: &mut GameContext, state: &mut GameState) {
        let mut vecevents = self.ui_layer.generate_events(context, state);

        vecevents.extend(self.units_layer.generate_events(context, state));

        state.game_events.extend(vecevents);
    }
}


fn main() {
    ::scrust::spawn("map rendering",
                    "/home/dm/.wine/drive_c/StarCraft/",
                    |gc, state| {
        let args: Vec<String> = env::args().collect();
        let mapfn = if args.len() == 2 {
            args[1].clone()
        } else {
            String::from("test.scx")
        };
        Box::new(MapView::new(gc, state, &mapfn))
    });

}
