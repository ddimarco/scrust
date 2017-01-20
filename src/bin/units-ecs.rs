use std::mem;
use std::collections::HashMap;

extern crate sdl2;
use sdl2::pixels::Color;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;

extern crate scrust;
use scrust::gamedata::{GameData, GRPCache};
use scrust::{GameContext, GameState, View, ViewAction};

use scrust::font::FontSize;
use scrust::font::RenderText;
use scrust::iscriptsys::IScriptSteppingSys;
use scrust::iscript::{IScript, AnimationType};

#[macro_use]
extern crate ecs;

use ecs::World;
use ecs::DataHelper;
use ecs::Entity;
use ecs::ModifyData;
use ecs::EntityData;

use scrust::unit_ecs::{UnitComponents, UnitSystems, UnitServices};
use scrust::unit_ecs::IScriptEntityAction;
use scrust::unit_ecs::{UnderlayComponent, OverlayComponent, SelectableComponent};
use scrust::unit_ecs::{SCUnitComponent, SCFlingyComponent, SCImageComponent, SCSpriteComponent, FlingyMoveControl};
use scrust::unit_ecs::EntityInit;
use scrust::unit_ecs::IScriptStateElement;

/// Public*****************************************

// ****************************************************************************

fn create_scimage(world: &mut World<UnitSystems>,
                  gd: &GameData,
                  image_id: usize,
                  map_x: u16,
                  map_y: u16,
                  parent: Option<Entity>)
                  -> Entity {
    let iscript_id = gd.images_dat.iscript_id[image_id];

    world.create_entity(EntityInit {
        iscript_state: Some(IScriptStateElement::new(&gd.iscript,
                                                     iscript_id,
                                                     map_x,
                                                     map_y,
                                                     parent)),
        scimage: Some(SCImageComponent::new(gd, image_id as u16)),
        ..Default::default()
    })
}

fn create_scsprite(world: &mut World<UnitSystems>,
                   gd: &GameData,
                   sprite_id: usize,
                   map_x: u16,
                   map_y: u16,
                   parent: Option<Entity>) -> Entity {
    let image_id = gd.sprites_dat.image_id[sprite_id];
    let entity = create_scimage(world, gd, image_id as usize, map_x, map_y, parent);

    world.modify_entity(entity,
                        |e: ModifyData<UnitComponents>, data: &mut UnitComponents| {
        data.scsprite.insert(&e, SCSpriteComponent { sprite_id: sprite_id as u16 });

        // not all sprites are selectable
        if sprite_id >= 130 {
            let circle_img = gd.sprites_dat.selection_circle_image[(sprite_id - 130) as usize];
            let circle_grp_id = gd.images_dat.grp_id[561 + circle_img as usize];

            let (sel_width, sel_height) = {
                let mut grp_cache = gd.grp_cache.borrow_mut();
                let grp = grp_cache.get(gd, circle_grp_id);
                (grp.header.width, grp.header.height)
            };

            data.selectable
                .insert(&e,
                        SelectableComponent {
                            health_bar: gd.sprites_dat.health_bar[(sprite_id - 130) as usize],
                            circle_offset:
                                gd.sprites_dat.selection_circle_offset[(sprite_id - 130) as usize],
                            circle_grp_id: circle_grp_id,
                            sel_width: sel_width,
                            sel_height: sel_height,
                        });
        }
    });
    entity
}

fn create_scflingy(world: &mut World<UnitSystems>,
                   gd: &GameData,
                   flingy_id: usize,
                   map_x: u16,
                   map_y: u16) -> Entity {
    let sprite_id = gd.flingy_dat.sprite_id[flingy_id as usize];
    let move_control =
        match gd.flingy_dat.move_control[flingy_id as usize] {
            0 => FlingyMoveControl::FlingyDat,
            1 => FlingyMoveControl::PartiallyMobile,
            2 => FlingyMoveControl::IScriptBin,
            _ => unimplemented!(),
        };
    let entity = create_scsprite(world, gd, sprite_id as usize, map_x, map_y, None);

    world.modify_entity(entity, |e: ModifyData<UnitComponents>, data: &mut UnitComponents| {
        data.scflingy.insert(&e,
                             SCFlingyComponent {
                                 flingy_id: flingy_id as u16,
                                 move_control: move_control,
                             });
                        });

    entity
}

fn create_scunit(world: &mut World<UnitSystems>,
                 gd: &GameData,
                 unit_id: usize,
                 map_x: u16,
                 map_y: u16) -> Entity {
    let gd_weapon = gd.units_dat.ground_weapon[unit_id] as usize;
    if gd_weapon < 130 {
        gd.weapons_dat.print_entry(gd_weapon);
        println!("ground weapon label: {}", gd.stat_txt_tbl[gd.weapons_dat.label[gd_weapon] as usize]);
    }

    let flingy_id = gd.units_dat.flingy_id[unit_id as usize];

    let entity = create_scflingy(world, gd, flingy_id as usize, map_x, map_y);
    world.modify_entity(entity, |e: ModifyData<UnitComponents>, data: &mut UnitComponents| {
        data.scunit.insert(&e,  SCUnitComponent {
            unit_id: unit_id as u16,
            kill_count: 0,
        });
    });

    entity
}

struct UnitsECSView {
    world: World<UnitSystems>,
    main_unit: Entity,

    unit_id: usize,
    unit_name_str: String,
}

use std::env;
impl UnitsECSView {
    fn new(gd: &GameData, context: &mut GameContext) -> UnitsECSView {
        let pal = gd.install_pal.to_sdl();
        context.screen.set_palette(&pal).ok();

        let mut world = World::<UnitSystems>::new();

        world.systems.iscript_stepping_sys.init(IScriptSteppingSys {
            iscript_copy: gd.iscript.clone(),
            images_dat: gd.images_dat.clone(),
            lox_cache: gd.lox_cache.clone(),
            iscript_entity_actions: Vec::<IScriptEntityAction>::new(),
            interested: HashMap::new(),
        });

        let args: Vec<String> = env::args().collect();
        let unit_id = if args.len() == 2 {
            args[1].parse::<usize>().expect("command line argument should be an integer")
        } else {
            0
        };

        let main_unit = create_scunit(&mut world, gd, unit_id, 0, 0);
        let unit_name_str = format!("{}: {}", unit_id, gd.stat_txt_tbl[unit_id].to_owned());

        UnitsECSView {
            world: world,
            main_unit: main_unit,
            unit_id: unit_id,
            unit_name_str: unit_name_str,
        }
    }

    fn draw_scimage(&self,
                   e: EntityData<UnitComponents>,
                   dh: &DataHelper<UnitComponents, UnitServices>,
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

        let (cx, cy) = (200, 200);
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
}

/// set animation state for an entity an all its children
/// necessary, so they also get destroyed properly
// FIXME: with this we can probably get rid of the parent check - self-remove
// hacks in interpret_iscript
fn set_animation_rec(world: &mut World<UnitSystems>,
                     iscript: &IScript,
                     entity: Entity,
                     anim: AnimationType) {
    for e in world.with_entity_data(&entity, |ent, data| {
        data.iscript_state[ent].set_animation(&iscript, anim.clone());
        data.iscript_state[ent].children.clone()
    }).unwrap_or(Vec::new()) {
        world.with_entity_data(&e, |ent, data| {
            data.iscript_state[ent].set_animation(&iscript, anim.clone());
        });
    }
}

impl View for UnitsECSView {
    fn render(&mut self,
              gd: &GameData,
              context: &mut GameContext,
              _: &GameState,
              _: f64)
              -> ViewAction {
        if context.events.now.quit ||
           context.events.now.is_key_pressed(&sdl2::keyboard::Keycode::Escape) {
            return ViewAction::Quit;
        }
        context.screen.fill_rect(None, Color::RGB(0, 0, 120)).ok();

        if context.events.now.is_key_pressed(&Keycode::N) {
            if self.unit_id < 227 {
                self.unit_id += 1;

                self.unit_name_str = format!("{}: {}",
                                             self.unit_id,
                                             gd.stat_txt_tbl[self.unit_id].to_owned());

                if !self.world.data.with_entity_data(&self.main_unit, |_, _| {}).is_none() {
                    set_animation_rec(&mut self.world, &gd.iscript, self.main_unit, AnimationType::Death);
                    self.world.remove_entity(self.main_unit);
                }
                self.main_unit = create_scunit(&mut self.world, gd, self.unit_id, 0, 0);
            }

        } else if context.events.now.is_key_pressed(&Keycode::P) {
            if self.unit_id > 0 {
                self.unit_id -= 1;

                self.unit_name_str = format!("{}: {}",
                                             self.unit_id,
                                             gd.stat_txt_tbl[self.unit_id].to_owned());
                if !self.world.data.with_entity_data(&self.main_unit, |_, _| {}).is_none() {
                    set_animation_rec(&mut self.world, &gd.iscript, self.main_unit, AnimationType::Death);
                    self.world.remove_entity(self.main_unit);
                }
                self.main_unit = create_scunit(&mut self.world, gd, self.unit_id, 0, 0);
            }
        }

        if context.events.now.is_key_pressed(&Keycode::Q) {
            self.world.with_entity_data(&self.main_unit, |ent, data| {
                data.iscript_state[ent].turn_ccwise(1);
            });
        } else if context.events.now.is_key_pressed(&Keycode::E) {
            self.world.with_entity_data(&self.main_unit, |ent, data| {
                data.iscript_state[ent].turn_cwise(1);
            });
        }

        if context.events.now.is_key_pressed(&Keycode::W) {
            self.world.with_entity_data(&self.main_unit, |ent, data| {
                data.iscript_state[ent].set_animation(&gd.iscript,
                                                      AnimationType::Walking);
            });

        } else if context.events.now.is_key_pressed(&Keycode::A) {
            self.world.with_entity_data(&self.main_unit, |ent, data| {
                data.iscript_state[ent].set_animation(&gd.iscript,
                                                      AnimationType::GndAttkInit);
            });
        } else if context.events.now.is_key_pressed(&Keycode::D) {
            set_animation_rec(&mut self.world, &gd.iscript, self.main_unit, AnimationType::Death);
        }

        // interpret iscript for units
        self.world.update();
        self.world.flush_queue();

        // generate new images, units
        let actions =
            mem::replace(&mut self.world.systems.iscript_stepping_sys.inner.as_mut().unwrap().iscript_entity_actions,
                         Vec::<IScriptEntityAction>::new());
        for action in actions {
            match action {
                IScriptEntityAction::RemoveEntity {entity} => {
                    self.world.remove_entity(entity);
                },
                IScriptEntityAction::CreateImageUnderlay { parent, image_id, rel_x, rel_y } => {
                    let ent =
                        create_scimage(&mut self.world, gd, image_id as usize, 0, 0, Some(parent));
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
                    let ent =
                        create_scimage(&mut self.world, gd, image_id as usize, 0, 0, Some(parent));
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
                },
                IScriptEntityAction::CreateSpriteOverlay { sprite_id, x, y } => {
                    let ent = create_scsprite(&mut self.world, gd, sprite_id as usize, x, y, None);
                    self.world.modify_entity(ent, |e: ModifyData<UnitComponents>,
                                             data: &mut UnitComponents| {
                         data.overlay.insert(&e, OverlayComponent {});
                    });
                },
                IScriptEntityAction::CreateSpriteUnderlay { parent, sprite_id, x, y, use_parent_dir } => {
                    let ent = create_scsprite(&mut self.world, gd, sprite_id as usize, x, y, parent);
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
                         }});
                },
                // _ => {
                //     println!("ignoring {:?} iscript create action", action);
                // }
            }
        }

        let grp_cache = gd.grp_cache.borrow();

        let fnt = gd.font(FontSize::Font16);
        let fnt_reindex = &gd.font_reindexing_store.get_game_reindex().data;
        let unitname_rect = Rect::new(10, 10, 300, 50);

        let buffer_pitch = context.screen.pitch();
        context.screen.with_lock_mut(|buffer: &mut [u8]| {
            fnt.render_textbox(self.unit_name_str.as_ref(),
                               1,
                               fnt_reindex,
                               buffer,
                               buffer_pitch,
                               &unitname_rect);

            let dh = &self.world.data;

            for e in self.world.entities().filter(aspect!(<UnitComponents> all: [underlay]), &self.world)
            {
                if !dh.iscript_state[e].alive {
                    continue;
                }
                self.draw_scimage(e, dh, gd, buffer, buffer_pitch, &*grp_cache);
            }

            // NOTE order is random in this loop!
            for e in self.world.entities().filter(aspect!(<UnitComponents> none: [underlay, overlay]), &self.world) {
                // TODO we should remove dead entities instead
                if !dh.iscript_state[e].alive  {
                    continue;
                }
                assert!(!dh.underlay.has(&e) && !dh.overlay.has(&e));

                // draw selection circle if available
                if dh.selectable.has(&e) {
                    // dh.selectable[e].draw_healthbar(200, 230,
                    //                                 buffer,
                    //                                 buffer_pitch);
                    dh.selectable[e].draw_selection_circle(&*grp_cache,
                                                           200, 200,
                                                           buffer,
                                                           buffer_pitch);
                }
                self.draw_scimage(e, dh, gd, buffer, buffer_pitch, &*grp_cache);
            }

            for e in self.world.entities().filter(aspect!(<UnitComponents> all: [overlay]), &self.world) {
                if !dh.iscript_state[e].alive {
                    continue;
                }
                self.draw_scimage(e, dh, gd, buffer, buffer_pitch, &*grp_cache);
            }
        });


        ViewAction::None
    }
}

fn main() {
    ::scrust::spawn("units ecs",
                    "/home/dm/.wine/drive_c/StarCraft/",
                    |gd, gc, _| Box::new(UnitsECSView::new(gd, gc)));
}
