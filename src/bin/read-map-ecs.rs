use std::mem;
use std::env;
use std::collections::HashMap;

#[macro_use]
extern crate scrust;
use scrust::{GameContext, GameState, View, ViewAction, GameEvents, MousePointerType};
use scrust::terrain::Map;
use scrust::gamedata::{GameData, GRPCache};
use scrust::unitsdata::WeaponBehavior;

use scrust::LayerTrait;
use scrust::ui::UiLayer;

use scrust::iscriptsys::IScriptSteppingSys;

#[macro_use]
extern crate ecs;

use ecs::World;
use ecs::DataHelper;
use ecs::EntityData;
use ecs::ModifyData;

use scrust::unit_ecs::{UnitComponents, UnitSystems, UnitServices};
use scrust::unit_ecs::IScriptEntityAction;
use scrust::unit_ecs::{UnderlayComponent, OverlayComponent, SCWeaponComponent};
use scrust::unit_ecs::{create_scimage, create_scsprite, create_scflingy, create_scunit};
use scrust::unit_ecs::UnitCommand;

extern crate sdl2;
use sdl2::pixels::Color;
use sdl2::rect::Point;

extern crate enum_primitive;
use enum_primitive::FromPrimitive;

fn draw_scimage(e: EntityData<UnitComponents>,
                dh: &DataHelper<UnitComponents, UnitServices>,
                cx: i32,
                cy: i32,
                gd: &GameData,
                buffer: &mut [u8],
                buffer_pitch: u32,
                grp_cache: &GRPCache) {
    // every entity is an scimage
    let scimg_comp = &dh.scimage[e];
    let grp = grp_cache.get_ro(scimg_comp.grp_id);
    let fridx = scimg_comp.frame_idx(&dh.iscript_state[e]);
    // this seems like a hack
    if fridx >= grp.frames.len() {
        println!("WARNING: suspicious frame index");
        return;
    }
    let draw_flipped = scimg_comp.draw_flipped(&dh.iscript_state[e]);

    // let (cx, cy) = (200, 200);
    let x_center = cx + dh.iscript_state[e].rel_x as i32;
    let y_center = cy + dh.iscript_state[e].rel_y as i32;

    scimg_comp.draw(&grp.frames[fridx],
                    grp.header.width as u32,
                    grp.header.height as u32,
                    draw_flipped,
                    x_center,
                    y_center,
                    buffer,
                    buffer_pitch,
                    scimg_comp.reindexing_table(gd));
}

struct UnitsLayer {
    // XXX distinguish high & low layer
    // sprites: Vec<SCSprite>,
    world: World<UnitSystems>,

    cursor_over_unit: bool,
}
impl UnitsLayer {
    fn from_map(gd: &GameData, context: &mut GameContext, state: &mut GameState, map: Rc<Map>) -> Self {
        let mut world = World::<UnitSystems>::new();
        world.systems.iscript_stepping_sys.init(IScriptSteppingSys {
            iscript_copy: gd.iscript.clone(),
            images_dat: gd.images_dat.clone(),
            weapons_dat: gd.weapons_dat.clone(),
            lox_cache: gd.lox_cache.clone(),
            iscript_entity_actions: Vec::<IScriptEntityAction>::new(),
            interested: HashMap::new(),
        });

        // create map units
        for mapunit in &map.data.units {
            // XXX: make use of mapunit data
            // let unit = SCUnit::new(&context.gd,
            //                        mapunit.unit_id as usize,
            //                        mapunit.x,
            //                        mapunit.y,
            //                        mapunit.player_no as usize);
            // let _ = state.unit_instances.put(unit);
            let _ = create_scunit(&mut world, gd, mapunit.unit_id as usize,
                                  mapunit.x, mapunit.y, mapunit.player_no as usize);
        }

        world.systems.scunit_stepping_sys.map = Some(map);

        // let mut sprites = Vec::<SCSprite>::new();
        // for mapsprite in &map.data.sprites {
        //     let sprite = SCSprite::new(&context.gd, mapsprite.sprite_no, mapsprite.x, mapsprite.y);
        //     sprites.push(sprite);
        // }
        UnitsLayer {
            // sprites: sprites,
            world: world,
            cursor_over_unit: false,
        }
    }

    fn update(&mut self, gd: &GameData, context: &GameContext, state: &mut GameState) {
        // for (_, u) in &mut state.unit_instances {
        //     let action = u.get_scimg_mut().step(&context.gd);
        //     match action {
        //         // XXX distinguish high & low layer
        //         Some(IScriptEntityAction::CreateSpriteOverlay { sprite_id, x, y }) => {
        //             let sprite = SCSprite::new(&context.gd, sprite_id, x, y);
        //             self.sprites.push(sprite);
        //         }
        //         Some(IScriptEntityAction::CreateSpriteUnderlay { sprite_id, x, y }) => {
        //             let sprite = SCSprite::new(&context.gd, sprite_id, x, y);
        //             self.sprites.push(sprite);
        //         }
        //         _ => {}
        //     }
        // }

        // interpret iscript for units
        self.world.update();
        self.world.flush_queue();

        // generate new images, units
        let actions = mem::replace(&mut self.world
                                       .systems
                                       .iscript_stepping_sys
                                       .inner
                                       .as_mut()
                                       .unwrap()
                                       .iscript_entity_actions,
                                   Vec::<IScriptEntityAction>::new());
        for action in actions {
            match action {
                IScriptEntityAction::RemoveEntity { entity } => {
                    self.world.remove_entity(entity);
                }
                IScriptEntityAction::CreateImageUnderlay { parent, image_id, rel_x, rel_y } => {
                    let (par_mx, par_my, player_id) = self.world.with_entity_data(&parent, |e, data| {
                        (data.iscript_state[e].map_pos_x, data.iscript_state[e].map_pos_y,
                        data.scimage[e].player_id)
                        }).expect("couldn't get parent map pos");
                    let ent =
                        create_scimage(&mut self.world, gd, image_id as usize, par_mx, par_my,
                                       Some(parent), player_id);
                   self.world.modify_entity(parent, |e: ModifyData<UnitComponents>,
                                              data: &mut UnitComponents| {
                        data.iscript_state[e].children.push(ent);
                    });
                    self.world.modify_entity(ent, |e: ModifyData<UnitComponents>,
                                              data: &mut UnitComponents| {
                        data.iscript_state[e].rel_x = rel_x;
                        data.iscript_state[e].rel_y = rel_y;
                        data.underlay.insert(&e, UnderlayComponent {});
                    });

                }
                IScriptEntityAction::CreateImageOverlay { parent, image_id, rel_x, rel_y } => {
                    let (par_mx, par_my, player_id) =
                        self.world.with_entity_data(&parent, |e, data| {
                            (data.iscript_state[e].map_pos_x, data.iscript_state[e].map_pos_y,
                            data.scimage[e].player_id)
                        }).expect("couldn't get parent map pos");
                    let ent =
                        create_scimage(&mut self.world, gd, image_id as usize, par_mx, par_my,
                                       Some(parent), player_id);
                    self.world.modify_entity(parent, |e: ModifyData<UnitComponents>,
                                              data: &mut UnitComponents| {
                        data.iscript_state[e].children.push(ent);
                    });
                    self.world.modify_entity(ent, |e: ModifyData<UnitComponents>,
                                              data: &mut UnitComponents| {
                        data.iscript_state[e].rel_x = rel_x;
                        data.iscript_state[e].rel_y = rel_y;
                        data.overlay.insert(&e, OverlayComponent {});
                    });
                }
                IScriptEntityAction::CreateSpriteOverlay { sprite_id, x, y } => {
                    let ent = create_scsprite(&mut self.world, gd, sprite_id as usize, x, y, None, 0);
                    self.world.modify_entity(ent, |e: ModifyData<UnitComponents>,
                                              data: &mut UnitComponents| {
                        data.overlay.insert(&e, OverlayComponent {});
                    });
                }
                IScriptEntityAction::CreateSpriteUnderlay { parent,
                                                            sprite_id,
                                                            x,
                                                            y,
                                                            use_parent_dir } => {
                    let ent =
                        create_scsprite(&mut self.world, gd, sprite_id as usize, x, y, parent, 0);
                    let parent_dir = if use_parent_dir {
                        self.world.with_entity_data(&parent.unwrap(), |e, data| {
                            data.iscript_state[e].children.push(ent);
                            data.iscript_state[e].direction
                        })
                    } else {
                        None
                    };
                    self.world.modify_entity(ent, |e: ModifyData<UnitComponents>,
                                              data: &mut UnitComponents| {
                        data.underlay.insert(&e, UnderlayComponent {});
                        if let Some(initial_dir) = parent_dir {
                            data.iscript_state[e].direction = initial_dir;
                        }
                    });
                }
                IScriptEntityAction::CreateWeaponsFlingy { weapon_id, rel_x, rel_y } => {
                    let ent = create_scflingy(&mut self.world,
                                              gd,
                                              gd.weapons_dat.graphics[weapon_id as usize] as usize,
                                              // FIXME: use proper location
                                              0,
                                              0,
                    0);

                    let behavior = WeaponBehavior::from_u8(gd.weapons_dat.behavior[weapon_id as
                                                            usize])
                        .expect("could not get weapon behavior!");
                    self.world.modify_entity(ent, |e: ModifyData<UnitComponents>,
                                              data: &mut UnitComponents| {
                        data.scweapon.insert(&e,
                                             SCWeaponComponent {
                                                 weapon_id: weapon_id,
                                                 behavior: behavior,
                                                 age: 0,
                                             });
                    });
                }
                // _ => {
                //     println!("ignoring {:?} iscript create action", action);
                // }
            }
        }
    }

    fn generate_events(&mut self, gc: &GameContext, state: &mut GameState) -> Vec<GameEvents> {
        let mut events = Vec::<GameEvents>::new();

        // mouse over unit?
        let mouse_pos_map = state.map_pos + gc.events.mouse_pos;
        let mut over_unit_instance = None;
        {
            let dh = &self.world.data;
            for e in self.world
                .entities()
                .filter(aspect!(<UnitComponents> all: [selectable]), &self.world) {
                    if !dh.iscript_state[e].alive {
                        continue;
                    }

                    let seldata = &dh.selectable[e];
                    let iss = &dh.iscript_state[e];
                    let halfw = seldata.sel_width as i32 / 2;
                    let halfh = seldata.sel_height as i32 / 2;

                    let ux = iss.map_pos_x as i32;
                    let uy = iss.map_pos_y as i32;

                    if mouse_pos_map.x() > ux - halfw && mouse_pos_map.x() < ux + halfw &&
                        mouse_pos_map.y() > uy - halfh &&
                        mouse_pos_map.y() < uy + halfh {
                            over_unit_instance = Some(**e);
                            break;
                        }

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

        // commands for selected unit
        if !state.selected_units.is_empty() {
            if !self.cursor_over_unit && gc.events.now.mouse_right {
                // move command
                println!("moving to {}, {}", mouse_pos_map.x(), mouse_pos_map.y());
                for e in &state.selected_units {
                    self.world.with_entity_data(&e, |e, data| {
                        let cmd = UnitCommand::Move(mouse_pos_map.x(), mouse_pos_map.y());
                        // overwrite old command
                        if data.scunit[e].commands.len() > 0 {
                            data.scunit[e].commands[0] = cmd;
                        } else {
                            data.scunit[e].commands.push(cmd);
                        }
                    });
                }
            }
        }

        events
    }

    fn render(&self,
              gd: &GameData,
              state: &GameState,
              map_x: u16,
              map_y: u16,
              grp_cache: &GRPCache,
              buffer: &mut [u8],
              buffer_pitch: u32) {
            let dh = &self.world.data;

            for e in self.world
                .entities()
                .filter(aspect!(<UnitComponents> all: [underlay]), &self.world) {
                if !dh.iscript_state[e].alive {
                    continue;
                }
                let cx = dh.iscript_state[e].map_pos_x as i32 - map_x as i32;
                let cy = dh.iscript_state[e].map_pos_y as i32 - map_y as i32;
                draw_scimage(e, dh, cx, cy, gd, buffer, buffer_pitch, &*grp_cache);
            }

            // NOTE order is random in this loop!
            for e in self.world
                .entities()
                .filter(aspect!(<UnitComponents> none: [underlay, overlay]),
                        &self.world) {
                // TODO we should remove dead entities instead
                if !dh.iscript_state[e].alive {
                    continue;
                }
                assert!(!dh.underlay.has(&e) && !dh.overlay.has(&e));

                let cx = dh.iscript_state[e].map_pos_x as i32 - map_x as i32;
                let cy = dh.iscript_state[e].map_pos_y as i32 - map_y as i32;

                // draw path if available
                    if dh.scunit.has(&e) {
                        match &dh.scunit[e].path {
                            &Some(ref p) => {
                                // p.mark_tiles(
                                //     self.world.systems.scunit_stepping_sys.map.as_ref().unwrap(),
                                //     map_x as isize, map_y as isize,
                                //     buffer,
                                //     buffer_pitch);
                                p.draw(cx as isize, cy as isize,
                                       map_x as isize, map_y as isize,
                                       buffer, buffer_pitch);
                            },
                            _ => {},
                        }
                    }

                // draw selection circle if available
                if dh.selectable.has(&e) {
                    let is_selected = state.selected_units.contains(&**e);
                    if is_selected {
                        dh.selectable[e]
                            .draw_selection_circle(&*grp_cache, cx, cy, buffer, buffer_pitch);
                    // dh.selectable[e].draw_healthbar(200, 230,
                    //                                 buffer,
                    //                                 buffer_pitch);
                    }
                }
                draw_scimage(e, dh, cx, cy, gd, buffer, buffer_pitch, &*grp_cache);
            }

            for e in self.world
                .entities()
                .filter(aspect!(<UnitComponents> all: [overlay]), &self.world) {
                if !dh.iscript_state[e].alive {
                    continue;
                }
                let cx = dh.iscript_state[e].map_pos_x as i32 - map_x as i32;
                let cy = dh.iscript_state[e].map_pos_y as i32 - map_y as i32;
                draw_scimage(e, dh, cx, cy, gd, buffer, buffer_pitch, &*grp_cache);
            }
    }
}

use std::rc::Rc;

struct MapView {
    map: Rc<Map>,

    units_layer: UnitsLayer,
    ui_layer: UiLayer,
}
const MAP_RENDER_W: u16 = 20;
const MAP_RENDER_H: u16 = 12;
impl MapView {
    fn new(gd: &GameData, context: &mut GameContext, state: &mut GameState, mapfn: &str) -> Self {
        let map = Rc::new(Map::read(gd, mapfn));
        println!("map name: {}", map.name());
        println!("map desc: {}", map.description());
        context.screen.set_palette(&map.terrain_info.pal.to_sdl()).ok();
        let units_layer = UnitsLayer::from_map(gd, context, state, map.clone());
        let ui_layer = UiLayer::new(gd, context, &*map);

        MapView {
            map: map,
            units_layer: units_layer,
            ui_layer: ui_layer,
        }
    }
}
impl View for MapView {
    fn update(&mut self, gd: &GameData, context: &mut GameContext, state: &mut GameState) {
        self.units_layer.update(gd, context, state);
        self.ui_layer.update(gd, context, state);
    }
    fn render(&mut self, gd: &GameData, context: &mut GameContext, state: &GameState, _: f64) -> ViewAction {
        if context.events.now.quit // || context.events.now.key_escape == Some(true)
        {
            return ViewAction::Quit;
        }

        // clear the screen
        context.screen.fill_rect(None, Color::RGB(0, 0, 0)).ok();
        let screen_pitch = context.screen.pitch();

        let map_x = state.map_pos.x() as u16;
        let map_y = state.map_pos.y() as u16;

        {
            let grp_cache = &*gd.grp_cache.borrow();
            let mut screen = &mut context.screen;
            screen.with_lock_mut(|buffer: &mut [u8]| {
                self.map.render(map_x,
                                map_y,
                                MAP_RENDER_W,
                                MAP_RENDER_H,
                                buffer,
                                screen_pitch);

                self.units_layer.render(gd, &state, map_x, map_y, grp_cache, buffer, screen_pitch);

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

        let over_hud = self.ui_layer.is_over_hud(context.events.mouse_pos.x(),
                                                 context.events.mouse_pos.y());
        if over_hud {
            // change to arrow cursor
            vecevents.push(GameEvents::ChangeMouseCursor(MousePointerType::Arrow));
        } else {
            vecevents.extend(self.units_layer.generate_events(context, state));
        }

        state.game_events.extend(vecevents);
    }
}


fn main() {
    ::scrust::spawn("map rendering",
                    "/home/dm/.wine/drive_c/StarCraft/",
                    |gd, gc, state| {
        let args: Vec<String> = env::args().collect();
        let mapfn = if args.len() == 2 {
            args[1].clone()
        } else {
            String::from("simple.scm")
        };
        Box::new(MapView::new(gd, gc, state, &mapfn))
    });

}
