use std::mem;

extern crate sdl2;
use sdl2::pixels::Color;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;

extern crate scrust;
use scrust::gamedata::{GameData, GRPCache};
use scrust::{GameContext, GameState, View, ViewAction};

use scrust::iscript::{IScript, AnimationType, OpCode};

use scrust::render::{render_buffer_with_transparency_reindexing,
                     render_buffer_with_solid_reindexing,
                     render_buffer_solid};
use scrust::font::FontSize;
use scrust::font::RenderText;

extern crate byteorder;
use byteorder::LittleEndian;
use byteorder::ByteOrder;

extern crate rand;
use rand::Rng;

extern crate num;
use num::FromPrimitive;

#[macro_use]
extern crate ecs;

use ecs::World;
use ecs::DataHelper;
use ecs::system::{System};
use ecs::EntityIter;
use ecs::ServiceManager;
use ecs::Entity;
use ecs::ModifyData;
use ecs::EntityData;

/// Public*****************************************

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

#[derive(Debug)]
pub enum IScriptEntityAction {
    CreateImageUnderlay {
        parent: Entity,
        image_id: u16,
        rel_x: i8,
        rel_y: i8,
    },
    CreateImageOverlay {
        parent: Entity,
        image_id: u16,
        rel_x: i8,
        rel_y: i8,
    },
    CreateSpriteOverlay { sprite_id: u16, x: u16, y: u16 },
    CreateSpriteUnderlay { parent: Option<Entity>, sprite_id: u16, x: u16, y: u16, use_parent_dir: bool },
    RemoveEntity {entity: Entity},
}
/// *****************************************

/// current unit state
pub enum IScriptCurrentUnitState {
    Idle,
    Moving(i32, i32),
}
pub struct IScriptStateElement {
    pub iscript_id: u32,
    /// current position in iscript
    pub pos: usize,
    /// positions stack to jump back after a call
    call_stack: Vec<usize>,

    pub waiting_ticks_left: usize,
    // FIXME: signed or unsigned?
    pub rel_x: i8,
    pub rel_y: i8,
    pub direction: u8,
    pub frameset: u16,
    pub visible: bool,
    pub alive: bool,
    pub current_state: IScriptCurrentUnitState,
    pub movement_angle: f32,

    pub map_pos_x: u16,
    pub map_pos_y: u16,
    parent_entity: Option<Entity>,
    children: Vec<Entity>,
    /// stops iscript interpretation (for opcode IgnoreRest)
    paused: bool
}
impl IScriptStateElement {
    pub fn new(iscript: &IScript,
               iscript_id: u32,
               map_x: u16,
               map_y: u16,
               parent_entity: Option<Entity>)
               -> Self {
        let start_pos = {
            let ref iscript_anim_offsets = iscript.id_offsets_map.get(&iscript_id).unwrap();
            // println!("header:");
            // for anim_idx in 0..iscript_anim_offsets.len() {
            //     let anim = AnimationType::from_usize(anim_idx).unwrap();
            //     let pos = iscript_anim_offsets[anim_idx];
            // println!("{:?}: {}", anim, pos);
            // }
            iscript_anim_offsets[AnimationType::Init as usize]
        };
        IScriptStateElement {
            iscript_id: iscript_id,
            pos: start_pos as usize,
            call_stack: Vec::new(),
            waiting_ticks_left: 0,
            visible: true,
            rel_x: 0,
            rel_y: 0,
            frameset: 0,
            direction: 0,
            movement_angle: 0f32,
            alive: true,
            current_state: IScriptCurrentUnitState::Idle,
            map_pos_x: map_x,
            map_pos_y: map_y,
            parent_entity: parent_entity,
            children: Vec::new(),
            paused: false,
        }
    }

    /// reference to iscript animation offsets
    pub fn iscript_anim_offsets<'a>(&self, iscript: &'a IScript) -> &'a Vec<u16> {
        iscript.id_offsets_map.get(&self.iscript_id).unwrap()
    }

    pub fn set_animation(&mut self, iscript: &IScript, anim: AnimationType) {
        // FIXME: need to also change the animation for its children
        self.waiting_ticks_left = 0;
        self.pos = self.iscript_anim_offsets(iscript)[anim as usize] as usize;
    }
    pub fn is_animation_valid(&self, iscript: &IScript, anim: AnimationType) -> bool {
        self.iscript_anim_offsets(iscript)[anim as usize] > 0
    }

    pub fn set_direction(&mut self, dir: u8) {
        self.direction = dir % 32;
    }
    pub fn turn_cwise(&mut self, units: u8) {
        let new_dir = self.direction + units;
        self.set_direction(new_dir);
    }
    pub fn turn_ccwise(&mut self, units: u8) {
        let new_dir = ((self.direction as i16 - units as i16) + 32) % 32;
        assert!(new_dir >= 0);
        self.set_direction(new_dir as u8);
    }

    pub fn anim_count(&self, iscript: &IScript) -> usize {
        self.iscript_anim_offsets(iscript).len()
    }

    fn read_u8(&mut self, iscript: &IScript) -> u8 {
        let val = iscript.data[self.pos as usize];
        self.pos += 1;
        val
    }
    fn read_u16(&mut self, iscript: &IScript) -> u16 {
        let val = LittleEndian::read_u16(&iscript.data[(self.pos as usize)..]);
        self.pos += 2;
        val
    }
    // fn read_u32(&mut self, iscript: &IScript) -> u32 {
    //     let val = LittleEndian::read_u32(&iscript.data[(self.pos as usize)..]);
    //     self.pos += 4;
    //     val
    // }
}
/// *****************************************

/// unit service definition
#[derive(Default)]
pub struct UnitServices {
}
impl UnitServices {
}
impl ServiceManager for UnitServices {}

// component definitions

use ecs::IndexedEntity;
use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

use scrust::gamedata::LOXCache;
use scrust::unitsdata::ImagesDat;
pub struct IScriptSteppingSys {
    // ugly HACK, bc we cannot borrow refs to service.xyz and dh.yyy in process()
    iscript_copy: IScript,
    images_dat: ImagesDat,
    lox_cache: Rc<RefCell<LOXCache>>,

    iscript_entity_actions: Vec<IScriptEntityAction>,

    interested: HashMap<Entity, IndexedEntity<UnitComponents>>,
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
                         e: ecs::EntityData<UnitComponents>,
                         dh: &mut DataHelper<UnitComponents, UnitServices>)
                         -> Option<IScriptEntityAction> {

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
            // FIXME
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
            println!("--- sigorder not implemented yet ---");
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
        // OpCode::PlaySndRand => dynamic_parameters(no_sounds: u8, snd1: u8, ...) {
        // },

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

        // FIXME
        },
        OpCode::NoBrkCodeEnd => () {
            // Allows the processing of player orders after a nobrkcodestart instruction.
        // FIXME
        },
        OpCode::TmpRmGraphicStart => () {
        // Sets the current image overlay state to hidden
            // only used for nukes
            // println!("tmprmgraphicstart, has parent: {}", parent.is_some());
            dh.iscript_state[e].visible = false;
        },
        OpCode::TmpRmGraphicEnd => () {
        // Sets the current image overlay state to visible
            // println!("tmprmgraphicend, has parent: {}", parent.is_some());
            dh.iscript_state[e].visible = true;
        },
        OpCode::AttkShiftProj => (distance: u8) {
            // Creates the weapon flingy at a particular distance in front of the unit.
            println!("attkshiftproj; dist: {}", distance);
        },
        OpCode::Attack => () {
        // FIXME
        },
        OpCode::AttackWith => (weapon: u8) {
        // FIXME
        },
        OpCode::GotoRepeatAttk => () {
        // Signals to StarCraft that after this point, when the unit's cooldown time
        // is over, the repeat attack animation can be called.
        // FIXME
        },
        OpCode::IgnoreRest => () {
        // this causes the script to stop until the next animation is called.
            dh.iscript_state[e].paused = true;
        },
        OpCode::SetFlSpeed => (speed: u16) {
        // FIXME
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
            // TODO: unnecessary here?
            // there should be a better way to check if an entity exists
            // if let Some(parent_entity) = dh.iscript_state[e].parent_entity {
            //     if dh.with_entity_data(&parent_entity, |_, _| {}).is_none() {
            //         dh.remove_entity(**e);
            //         continue;
            //     }
            // }

            let create_action = self.interpret_iscript(&cpy, e, dh);
            if let Some(action) = create_action {
                self.iscript_entity_actions.push(action);
            }
        }
    }
}

pub enum SCImageRemapping {
    Normal,
    OFire,
    GFire,
    BFire,
    BExpl,
    Shadow,
}
pub struct SCImageComponent {
    pub image_id: u16,
    pub grp_id: u32,
    pub player_id: usize,
    can_turn: bool,
    remapping: SCImageRemapping,
}
impl SCImageComponent {
    pub fn new(gd: &GameData, image_id: u16) -> Self {
        let grp_id = gd.images_dat.grp_id[image_id as usize];
        {
            gd.grp_cache.borrow_mut().load(gd, grp_id);
        }
        let can_turn = gd.images_dat.graphic_turns[image_id as usize] > 0;

        let draw_func = gd.images_dat.draw_function[image_id as usize];
        let remapping = match draw_func {
            10 => SCImageRemapping::Shadow,
            9 => {
                match gd.images_dat.remapping[image_id as usize] {
                    1 => SCImageRemapping::OFire,
                    2 => SCImageRemapping::GFire,
                    3 => SCImageRemapping::BFire,
                    4 => SCImageRemapping::BExpl,
                    x => {
                        panic!("unknown remapping {}!", x);
                    }
                }
            }
            _ => SCImageRemapping::Normal,
        };

        SCImageComponent {
            image_id: image_id,
            grp_id: grp_id,
            player_id: 0,
            can_turn: can_turn,
            remapping: remapping,
        }
    }

    fn reindexing_table<'a>(&self, gd: &'a GameData) -> &'a [u8] {
        match self.remapping {
            SCImageRemapping::OFire => &gd.ofire_reindexing.data,
            SCImageRemapping::BFire => &gd.bfire_reindexing.data,
            SCImageRemapping::GFire => &gd.gfire_reindexing.data,
            SCImageRemapping::BExpl => &gd.bexpl_reindexing.data,
            SCImageRemapping::Shadow => &gd.shadow_reindexing,
            SCImageRemapping::Normal => {
                if self.player_id < 11 {
                    let startpt = self.player_id as usize * 256;
                    &gd.player_reindexing[startpt..startpt + 256]
                } else {
                    // neutral player (i.e. minerals, critters, etc)
                    &gd.player_reindexing[0..256]
                }
            }
        }
    }

    fn draw(&self,
            inbuf: &[u8],
            w: u32,
            h: u32,
            flipped: bool,
            cx: i32,
            cy: i32,
            outbuf: &mut [u8],
            outbuf_pitch: u32,
            reindex: &[u8]) {
        match self.remapping {
            SCImageRemapping::OFire | SCImageRemapping::BFire | SCImageRemapping::GFire |
            SCImageRemapping::BExpl | SCImageRemapping::Shadow => {
                render_buffer_with_transparency_reindexing(inbuf,
                                                           w,
                                                           h,
                                                           flipped,
                                                           cx,
                                                           cy,
                                                           outbuf,
                                                           outbuf_pitch,
                                                           &reindex);
            }
            SCImageRemapping::Normal => {
                render_buffer_with_solid_reindexing(inbuf,
                                                    w,
                                                    h,
                                                    flipped,
                                                    cx,
                                                    cy,
                                                    outbuf,
                                                    outbuf_pitch,
                                                    &reindex);
            }
        }
    }

    // TODO: it might make sense to join scimage & iscriptstate
    fn frame_idx(&self, iscript_state: &IScriptStateElement) -> usize {
        if !self.can_turn {
            iscript_state.frameset as usize
        } else if iscript_state.direction > 16 {
            (iscript_state.frameset + 32 - iscript_state.direction as u16) as usize
        } else {
            (iscript_state.frameset + iscript_state.direction as u16) as usize
        }
    }

    fn draw_flipped(&self, iscript_state: &IScriptStateElement) -> bool {
        self.can_turn && (iscript_state.direction > 16)
    }
}

pub struct SCSpriteComponent {
    pub sprite_id: u16,
}
pub struct SelectableComponent {
    /// from sprites.dat: length of health bar in pixels
    health_bar: u8,
    circle_offset: u8,
    circle_grp_id: u32,

    pub sel_width: u16,
    pub sel_height: u16,
}
impl SelectableComponent {
    pub fn draw_selection_circle(&self,
                                 grp_cache: &GRPCache,
                                 cx: i32,
                                 cy: i32,
                                 buffer: &mut [u8],
                                 buffer_pitch: u32) {
        let grp = grp_cache.get_ro(self.circle_grp_id);
        render_buffer_solid(&grp.frames[0],
                            grp.header.width as u32,
                            grp.header.height as u32,
                            false,
                            cx,
                            cy + self.circle_offset as i32,
                            buffer,
                            buffer_pitch);
    }


    // FIXME: clipping
    pub fn draw_healthbar(&self, cx: u32, cy: u32, buffer: &mut [u8], buffer_pitch: u32) {
        let boxes = self.health_bar as u32 / 3;
        let box_width = 3;
        if self.health_bar == 0 {
            return;
        }
        let width = 2 + (box_width * boxes) + (boxes - 1);
        let height = 8;

        let mut outpos = ((cy + self.circle_offset as u32) - height / 2) *
            buffer_pitch + (cx - width / 2);
        for y in 0..height {
            for x in 0..width {
                let outer_border = y == 0 || y == height - 1 || x == 0 || x == (width - 1);
                let inner_border = x % (box_width + 1) == 0;
                if inner_border || outer_border {
                    // black
                    buffer[outpos as usize] = 0;
                } else {
                    // green
                    buffer[outpos as usize] = 185;
                }
                outpos += 1;
            }
            outpos += buffer_pitch - width;
        }
    }
}
#[derive(Debug)]
pub enum FlingyMoveControl {
    FlingyDat,
    PartiallyMobile,
    IScriptBin,
}
pub struct SCFlingyComponent {
    pub flingy_id: u16,
    pub move_control: FlingyMoveControl,
}
pub struct SCUnitComponent {
    pub unit_id: u16,
    pub kill_count: usize,
}

// TODO: merge into 1?
pub struct UnderlayComponent {}
pub struct OverlayComponent {}

use ecs::system::LazySystem;
components! {
    #[builder(EntityInit)]
    struct UnitComponents {
        #[hot] iscript_state: IScriptStateElement,
        #[hot] scimage: SCImageComponent,
        #[hot] selectable: SelectableComponent,
        #[hot] scsprite: SCSpriteComponent,
        #[hot] scflingy: SCFlingyComponent,
        #[hot] scunit: SCUnitComponent,

        #[hot] underlay: UnderlayComponent,
        #[hot] overlay: OverlayComponent,
    }
}
systems! {
    struct UnitSystems<UnitComponents, UnitServices> {
        active: {
            iscript_stepping_sys: LazySystem<IScriptSteppingSys>
                = LazySystem::<IScriptSteppingSys>::new(),
        },
        passive: {
        }
    }
}

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
