use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

use rand::Rng;

use ecs::DataHelper;
use ecs::system::{System};
use ecs::EntityIter;
use ecs::Entity;
use ecs::EntityData;
use ecs::IndexedEntity;

use enum_primitive::FromPrimitive;

use ::iscript::IScript;
use ::gamedata::LOXCache;
use ::unitsdata::{ImagesDat, WeaponsDat};
use ::iscript::{OpCode, AnimationType};
use ::unit_ecs::{IScriptEntityAction, UnitComponents, UnitServices};

use std::f32;

pub fn angle_to_discrete32(angle: f32) -> u8 {
    let pi = f32::consts::PI;
    // x+24 % 32 ~ x + 90°
    (((angle+pi)*32. / (2.*pi)).round() + 24.) as u8 % 32
}

pub fn discrete32_to_angle(discrete_angle: u8) -> f32 {
    let pi = f32::consts::PI;
    (((discrete_angle + 8) % 32) as f32 / 32.) * (2.*pi) - pi
}

macro_rules! var_read {
    (u8, $gd: ident, $file:expr) => ($file.read_u8($gd));
    (u16, $gd: ident, $file:expr) => ($file.read_u16($gd));
    (u32, $gd: ident, $file:expr) => ($file.read_u32($gd));
}
macro_rules! def_opcodes {
    (
        $self_var:expr,
        $gd:ident,
        $debug:expr,
        $code_var:ident,

        dynamic_parameters:
        $( $opcode_:pat => ($num_var:ident: $num_tpe:ident,
                            $vec_name:ident: <$vec_tpe:ident>)
           $code_:block),*

        fixed_parameters:
        $( $opcode:pat => ( $( $param:ident : $tpe:ident),*)
           $code:block),*
    )
        =>
    {

            match $code_var {
                $(
                    $opcode_ => {
                        let $num_var = var_read!($num_tpe, $gd, $self_var);
                        let mut $vec_name = Vec::<$vec_tpe>::with_capacity($num_var as usize);
                        for _ in 0..$num_var {
                            $vec_name.push(var_read!($vec_tpe, $gd, $self_var));
                        }
                        $code_
                    }
                ),*


                $(
                    $opcode => {
                        if $debug {
                            print!("op: {:?}(", $code_var);
                        }
                        $(
                            let $param = var_read!($tpe, $gd, $self_var);
                            if $debug {
                                print!("{}: {} = {}, ", stringify!($param),
                                         stringify!($tpe), $param);
                            }
                        )*
                            if $debug {
                                println!(")");
                            }
                            $code
                    }
                ),*

                _ => panic!("unknown opcode: {:?}", $code_var),
            }
    }
}


pub struct IScriptSteppingSys {
    // ugly HACK, bc we cannot borrow refs to service.xyz and dh.yyy in process()
    pub iscript_copy: IScript,
    pub images_dat: ImagesDat,
    pub weapons_dat: WeaponsDat,
    pub lox_cache: Rc<RefCell<LOXCache>>,

    pub iscript_entity_actions: Vec<IScriptEntityAction>,

    pub interested: HashMap<Entity, IndexedEntity<UnitComponents>>,
}
impl System for IScriptSteppingSys {
    type Components = UnitComponents;
    type Services = UnitServices;

    fn activated(&mut self, entity: &EntityData<Self::Components>, _: &Self::Components, _: &mut Self::Services) {
        self.interested.insert(***entity, (**entity).__clone());
    }
    fn deactivated(&mut self, entity: &EntityData<Self::Components>, _: &Self::Components, _: &mut Self::Services)
    {
        self.interested.remove(entity);
    }
}
impl IScriptSteppingSys {
    fn interpret_iscript(&self,
                         cpy: &IScript,
                         e: EntityData<UnitComponents>,
                         dh: &mut DataHelper<UnitComponents, UnitServices>)
                         -> Option<IScriptEntityAction> {

        let na = dh.iscript_state[e].next_animation.clone();
        match na {
            Some(a) => {
                dh.iscript_state[e].set_animation(&self.iscript_copy, a);
            },
            _ => {}
        }
        dh.iscript_state[e].next_animation = None;

        if !dh.iscript_state[e].alive || dh.iscript_state[e].paused {
            return None;
        }
        // FIXME: waiting is actually counted in ticks, not in frames
        if dh.iscript_state[e].waiting_ticks_left > 0 {
            dh.iscript_state[e].waiting_ticks_left -= 1;
            return None;
        }

        let opcode = OpCode::from_u8(dh.iscript_state[e].read_u8(cpy))
            .expect("couldn't read opcode!");
        def_opcodes! (
                dh.iscript_state[e],
                cpy,
                false,
                opcode,

            dynamic_parameters:
            OpCode::PlaySndRand => (no_sounds: u8, soundids: <u16>) {
                // plays a random sound from a list.
                println!("playsndrand: {}/{} sounds", no_sounds, soundids.len());
                for si in soundids {
                    println!("sound: {}", si);
                }
            },
            OpCode::AttackMelee => (no_sounds: u8, soundids: <u16>) {
                // applies damage to target without creating a flingy and plays a sound.
                println!("attackmelee: {}/{} sounds", no_sounds, soundids.len());
                for si in soundids {
                    println!("sound: {}", si);
                }
            }

            fixed_parameters:
            OpCode::ImgUl => (image_id: u16, rel_x: u8, rel_y: u8) {
                // shadows and such; img* is associated with the current entity
                return Some(IScriptEntityAction::CreateImageUnderlay {
                    parent: **e,
                    image_id: image_id,
                    rel_x: rel_x as i8,
                    rel_y: rel_y as i8,
                });
            },
            OpCode::ImgUlNextId => (rel_x: u8, rel_y: u8) {
                // Displays an active image overlay at the shadow animation
                // level at a specified offset position. The image overlay that
                // will be displayed is the one that is after the current image
                // overlay in images.dat.
                return Some(IScriptEntityAction::CreateImageUnderlay {
                    parent: **e,
                    image_id: dh.scimage[e].image_id+1,
                    rel_x: rel_x as i8,
                    rel_y: rel_y as i8,
                });

            },
        OpCode::ImgOl => (image_id: u16, rel_x: u8, rel_y: u8) {
        // e.g. explosions on death
            return Some(IScriptEntityAction::CreateImageOverlay {
                parent: **e,
                image_id: image_id,
                rel_x: rel_x as i8,
                rel_y: rel_y as i8,
            });
        },
        OpCode::SprOl => (sprite_id: u16, rel_x: u8, rel_y: u8) {
        // independent overlay, e.g. scanner sweep
            return Some(IScriptEntityAction::CreateSpriteOverlay {
                sprite_id: sprite_id,
                x: (rel_x as u16) + (dh.iscript_state[e].rel_x as u16) +
                    dh.iscript_state[e].map_pos_x,
                y: (rel_y as u16) + (dh.iscript_state[e].rel_y as u16) +
                    dh.iscript_state[e].map_pos_y,
            });
        },
        OpCode::LowSprUl => (sprite_id: u16, rel_x: u8, rel_y: u8) {
        // independent underlay, e.g. gore
            return Some(IScriptEntityAction::CreateSpriteUnderlay {
                parent: None,
                sprite_id: sprite_id,
                x: (rel_x as u16) + (dh.iscript_state[e].rel_x as u16) +
                    dh.iscript_state[e].map_pos_x,
                y: (rel_y as u16) + (dh.iscript_state[e].rel_y as u16) +
                    dh.iscript_state[e].map_pos_y,
                use_parent_dir: false,
            });
        },
            OpCode::SprUlUseLo => (sprite_id: u16, rel_x: u8, rel_y: u8) {
                // The new sprite inherits the direction of the current sprite.
                return Some(IScriptEntityAction::CreateSpriteUnderlay {
                    parent: Some(**e),
                    sprite_id: sprite_id,
                    x: (rel_x as u16) + (dh.iscript_state[e].rel_x as u16) +
                        dh.iscript_state[e].map_pos_x,
                    y: (rel_y as u16) + (dh.iscript_state[e].rel_y as u16) +
                        dh.iscript_state[e].map_pos_y,
                    use_parent_dir: true,
                });
            },
            OpCode::SprOlUseLo => (sprite_id: u16, overlay_no: u8) {
                // Spawns a sprite one animation level above the current image
                // overlay, using a specified LO* file for the offset position
                // information. The new sprite inherits the direction of the
                // current sprite.
                // FIXME
                println!("sproluselo not implemented yet");
            },
            OpCode::ImgOlUseLo => (sprite_id: u16, rel_x: u8, rel_y: u8) {
                // Displays an active image overlay at an animation level higher
                // than the current image overlay, using a LO* file to determine
                // the offset position.
                // FIXME
                println!("imgoluselo not implemented yet");
            },
        OpCode::CreateGasOverlays => (overlay_no: u8) {
            let smoke_img_id = 430 + overlay_no as u16;
            let overlay_id = self.images_dat.special_overlay[dh.scimage[e].image_id as usize];
            let (rx, ry) = {
                let c = self.lox_cache.borrow();
                let lo = c.get_ro(overlay_id) ;
                lo.frames[0].offsets[overlay_no as usize]
            };
            return Some(IScriptEntityAction::CreateImageOverlay {
                parent: **e,
                image_id: smoke_img_id,
                // FIXME signed or unsigned?
                rel_x: rx,
                rel_y: ry,
            });
        },

        OpCode::PlayFram => (frame: u16) {
            dh.iscript_state[e].frameset = frame;
        },
        OpCode::PlayFramTile => (frame: u16) {
            // plays frame# + tileset. If (frame# + tileset) is >= number of frames in the GRP, it does nothing.
            println!("--- playframtile not implemented yet ---");
        // FIXME
        },
        OpCode::EngFrame => (frame: u8) {
            //  Sets current image's frameset to <frame> and copies the primary image's direction. (Basically followmaingraphic, but without copying the parent image's frameset)
            // for engine glow overlays
            let parent_entity = dh.iscript_state[e].parent_entity.expect("followmaingraphic: no parent entity!");
            match dh.with_entity_data(&parent_entity, |ent, data| {
                data.iscript_state[ent].direction})
            {
                None => { return Some(IScriptEntityAction::RemoveEntity{entity: **e})},
                Some(dir_frame) => {
                    dh.iscript_state[e].direction = dir_frame;
                    dh.iscript_state[e].frameset = frame as u16;
                }
            }
        },
        OpCode::FollowMainGraphic => () {
            // Copies frame, flipstate, and direction of primary image in the sprite. Reapplies palette?
            let parent_entity = dh.iscript_state[e].parent_entity.expect("followmaingraphic: no parent entity!");

            match dh.with_entity_data(&parent_entity, |ent, data| {
                (data.iscript_state[ent].direction,
                 data.iscript_state[ent].frameset)})
            {
                    None => { return Some(IScriptEntityAction::RemoveEntity{entity: **e})},
                    Some(dir_frame) => {
                        dh.iscript_state[e].direction = dir_frame.0;
                        dh.iscript_state[e].frameset = dir_frame.1;
                    }
                }
        },
        OpCode::EngSet => (frameset: u8) {
            // Copes primary image's direction, and sets the frame to the primary image's GRP's frame count * framemult + primary image's frameset.
           // Plays a particular frame set, often used in engine glow animations.
            // assert!(parent.is_some());
            let parent_entity = dh.iscript_state[e].parent_entity.expect("engset: no parent entity!");
            match dh.with_entity_data(&parent_entity, |ent, data| {
                (data.iscript_state[ent].direction,
                 data.iscript_state[ent].frameset)})
            {
                None => { return Some(IScriptEntityAction::RemoveEntity{entity: **e})},
                Some(dir_frame) => {
                    dh.iscript_state[e].direction = dir_frame.0;
                    dh.iscript_state[e].frameset = dir_frame.1;
                }
            }
            // dh.iscript_state[e].frameset = frameset as u16;
        },

        OpCode::Wait => (ticks: u8) {
            dh.iscript_state[e].waiting_ticks_left += ticks as usize;
        },
        OpCode::WaitRand => (minticks: u8, maxticks: u8) {
            let r = ::rand::thread_rng().gen_range(minticks, maxticks+1);
            dh.iscript_state[e].waiting_ticks_left += r as usize;
        },
        OpCode::SigOrder => (signal: u8) {
            // Masks thingy's orderSignal with <signal>, usually a bit or flag. (This was already documented elsewhere)
        // FIXME
            println!("--- sigorder({}) not implemented yet ---", signal);
        },
        OpCode::Goto => (target: u16) {
            dh.iscript_state[e].pos = target as usize;
        },
        OpCode::RandCondJmp => (val: u8, target: u16) {
            let r = ::rand::random::<u8>();
            if r < val {
                dh.iscript_state[e].pos = target as usize;
            }
        },
        OpCode::Call => (offset: u16) {
            // Calls a code block.
            let pos = dh.iscript_state[e].pos;
            dh.iscript_state[e].call_stack.push(pos);
            dh.iscript_state[e].pos = offset as usize;
        },
        OpCode::Return => () {
            let pos = dh.iscript_state[e].call_stack.pop().expect("return encountered, but empty call stack!");
            dh.iscript_state[e].pos = pos;
        },
        OpCode::TurnRand => (units: u8) {
            if ::rand::thread_rng().gen_range(0, 100) < 50 {
                dh.iscript_state[e].turn_cwise(units);
            } else {
                dh.iscript_state[e].turn_ccwise(units);
            }
        },
        OpCode::TurnCWise => (units: u8) {
            dh.iscript_state[e].turn_cwise(units);
        },
        OpCode::TurnCCWise => (units: u8) {
            dh.iscript_state[e].turn_ccwise(units);
        },
        OpCode::SetFlDirect => (dir: u8) {
            dh.iscript_state[e].set_direction(dir);
        },
        // FIXME: might be signed bytes?
        OpCode::SetVertPos => (val: u8) {
            dh.iscript_state[e].rel_y = val as i8;
        },
        OpCode::SetHorPos => (val: u8) {
            dh.iscript_state[e].rel_x = val as i8;
        },
        OpCode::Move => (dist: u8) {
            let mut iss = &mut dh.iscript_state[e];
            let fdist = dist as f32;
            let (dx, dy) = (iss.movement_angle.cos() * fdist ,
                            iss.movement_angle.sin() * fdist);
            iss.map_pos_x = (iss.map_pos_x as i32 + dx.round() as i32) as u16;
            iss.map_pos_y = (iss.map_pos_y as i32 + dy.round() as i32) as u16;
        },

        // FIXME sounds
        OpCode::PlaySndBtwn => (val1: u16, val2: u16) {
        },
        OpCode::PlaySnd => (sound_id: u16) {
        },

        OpCode::SetFlipState => (flipstate: u8) {
            // FIXME
            println!("setflipstate: {}", flipstate);
        },
        OpCode::PwrupCondJmp => (offset: u16) {
            // Jumps to a code block if the current unit is a powerup and it is currently picked up.
            // FIXME
            println!("pwrupcondjmp: {}", offset);
        },

        OpCode::NoBrkCodeStart => () {
            // Holds the processing of player orders until a nobrkcodeend is encountered.
            assert!(dh.scunit.has(&e));
            dh.scunit[e].accepts_player_orders = false;
        },
        OpCode::NoBrkCodeEnd => () {
            // Allows the processing of player orders after a nobrkcodestart instruction.
            assert!(dh.scunit.has(&e));
            dh.scunit[e].accepts_player_orders = true;
        },
        OpCode::TmpRmGraphicStart => () {
        // Sets the current image overlay state to hidden
            // only used for nukes
            dh.iscript_state[e].visible = false;
        },
        OpCode::TmpRmGraphicEnd => () {
        // Sets the current image overlay state to visible
            dh.iscript_state[e].visible = true;
        },
        OpCode::AttkShiftProj => (distance: u8) {
            // Creates the weapon flingy at a particular distance in front of the unit.
            println!("attkshiftproj; dist: {}", distance);
            assert!(dh.scunit.has(&e));
            dh.scunit[e].weapon_shift_proj = distance;
        },
        OpCode::AttackWith => (weapon: u8) {
            // Attack with either the ground or air weapon depending on a parameter.
            // attackwith <ground = 1, air = 2>
            assert!(dh.scunit.has(&e));
            assert!((weapon == 1) || (weapon == 2));
            dh.scunit[e].used_weapon = match weapon {
                1 => {
                    println!("firing ground weapon");
                    dh.scunit[e].ground_weapon_id
                },
                2 => {
                    println!("firing air weapon");
                     dh.scunit[e].air_weapon_id
                },
                _ => { unreachable!(); },
            };
            let forward_offset = self.weapons_dat.forward_offset[dh.scunit[e].used_weapon] +
                dh.scunit[e].weapon_shift_proj;
            dh.scunit[e].weapon_shift_proj = 0;
            let upward_offset = self.weapons_dat.upward_offset[dh.scunit[e].used_weapon];

            // FIXME: use movement_angle (?)
            let dir_f32 = discrete32_to_angle(dh.iscript_state[e].direction);
            let rel_x_f32 = dir_f32.cos() * (forward_offset as f32);
            let rel_y_f32 = dir_f32.sin() * (forward_offset as f32);

            let rel_x = rel_x_f32 as isize;
            let rel_y = (rel_y_f32 as isize) + (upward_offset as isize);
            println!("rel: {}, {}", rel_x, rel_y);

            return Some(IScriptEntityAction::CreateWeaponsFlingy {
                weapon_id: dh.scunit[e].ground_weapon_id as u16,
                // FIXME: calc from offsets
                rel_x: rel_x,
                rel_y: rel_y,
            });

        // FIXME
        },
        OpCode::GotoRepeatAttk => () {
        // Signals to StarCraft that after this point, when the unit's cooldown time
        // is over, the repeat attack animation can be called.
            let wpid = dh.scunit[e].used_weapon;
            let anim = if wpid == dh.scunit[e].air_weapon_id {
                AnimationType::AirAttkRpt
            } else {
                AnimationType::GndAttkRpt
            };
            dh.iscript_state[e].waiting_ticks_left = self.weapons_dat.cooldown[wpid] as usize;
            dh.iscript_state[e].pos = dh.iscript_state[e].iscript_anim_offsets(&self.iscript_copy)
                [anim as usize] as usize;
        },
        OpCode::DoMissileDmg => () {
            // Causes the damage of a weapon flingy to be applied according to its weapons.dat entry.
        },
        OpCode::TrgtRangeCondJmp => (dist: u16, file_offset: u16) {
            // Jumps to a block depending on the distance to the target.
            println!("trgtrangecondjmp not implemented yet! {}, {}", dist, file_offset);
        },
        OpCode::IgnoreRest => () {
        // this causes the script to stop until the next animation is called.
            dh.iscript_state[e].paused = true;
        },
        OpCode::SetFlSpeed => (speed: u16) {
        // FIXME
            println!("setflspeed not implemented yet! {}", speed);
        },

        OpCode::End => () {
            dh.iscript_state[e].alive = false;
            return Some(IScriptEntityAction::RemoveEntity{entity: **e});
        }
        );

        None
    }
}
use ecs::Process;
impl Process for IScriptSteppingSys {
    fn process(&mut self,
               dh: &mut DataHelper<UnitComponents, UnitServices>) {
        let cpy = &self.iscript_copy;
        let iter = EntityIter::Map(self.interested.values());
        for e in iter {
            let create_action = self.interpret_iscript(&cpy, e, dh);
            if let Some(action) = create_action {
                self.iscript_entity_actions.push(action);
            }
        }
    }
}
