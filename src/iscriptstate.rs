use std::io::Result;
use std::io::Read;

use std::rc::Rc;

use ::rand::Rng;

extern crate num;
use num::FromPrimitive;

extern crate byteorder;
use byteorder::{ReadBytesExt, LittleEndian};

use ::gamedata::{GameData};
use ::iscript::{AnimationType, OpCode};

macro_rules! var_read {
    (u8, $file:ident) => ($file.read_u8());
    (u16, $file:ident) => ($file.read_u16::<LittleEndian>());
    (u32, $file:ident) => ($file.read_u32::<LittleEndian>());
}
macro_rules! def_opcodes {
    (
        $self_var:ident,
        $debug:expr,
        $code_var:ident,
        $( $opcode:pat => ( $( $param:ident : $tpe:ident),*)
           $code:block),*
    )
        =>
    {

            match $code_var {
                $(
                    $opcode => {
                        if $debug {
                            println!("op: {:?}", $code_var);
                        }
                        $(
                            let $param = var_read!($tpe, $self_var).unwrap();
                            if $debug {
                                println!(" param: {}: {} = {}", stringify!($param),
                                         stringify!($tpe), $param);
                            }
                        )*
                            $code
                    }
                ),*

                _ => panic!("unknown opcode: {:?}", $code_var),
            }
    }
}

pub enum IScriptEntityAction {
    CreateImageUnderlay { image_id: u16, rel_x: i8, rel_y: i8 },
    CreateImageOverlay { image_id: u16, rel_x: i8, rel_y: i8 },
    CreateSpriteOverlay { sprite_id: u16, x: u16, y: u16 },
    CreateSpriteUnderlay { sprite_id: u16, x: u16, y: u16 },
}


pub enum IScriptCurrentAction {
    Idle,
    Moving(i32, i32),
}

pub struct IScriptState {
    pub iscript_id: u32,
    /// current position in iscript
    pub pos: u16,

    // XXX: nasty: duplicate information
    pub image_id: u16,

    // TODO: couldn't find a better way to keep multiple immutable refs to gamedata
    pub gd: Rc<GameData>,

    pub waiting_ticks_left: usize,
    // FIXME: signed or unsigned?
    pub rel_x: i8,
    pub rel_y: i8,
    pub direction: u8,
    pub frameset: u16,
    pub follow_main_graphic: bool,
    pub visible: bool,
    pub alive: bool,
    pub current_action: IScriptCurrentAction,
    pub movement_angle: f32,

    pub map_pos_x: u16,
    pub map_pos_y: u16,
}

impl Read for IScriptState {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        for i in 0..buf.len() {
            if self.pos as usize > self.gd.iscript.data.len() {
                if i > 0 {
                    return Ok(i);
                } else {
                    panic!("could not read anything!");
                }
            }
            buf[i] = self.gd.iscript.data[self.pos as usize];
            self.pos = self.pos + 1;
        }
        Ok(buf.len())
    }
}

impl IScriptState {
    pub fn new(gd: &Rc<GameData>,
               iscript_id: u32,
               image_id: u16,
               map_x: u16,
               map_y: u16)
               -> IScriptState {
        let start_pos;
        {
            let ref iscript_anim_offsets = gd.iscript.id_offsets_map.get(&iscript_id).unwrap();
            // println!("header:");
            // for anim_idx in 0..iscript_anim_offsets.len() {
            //     let anim = AnimationType::from_usize(anim_idx).unwrap();
            //     let pos = iscript_anim_offsets[anim_idx];
            // println!("{:?}: {}", anim, pos);
            // }
            start_pos = iscript_anim_offsets[AnimationType::Init as usize];
        }
        IScriptState {
            iscript_id: iscript_id,
            image_id: image_id,
            pos: start_pos,
            gd: gd.clone(),
            waiting_ticks_left: 0,
            visible: true,
            rel_x: 0,
            rel_y: 0,
            frameset: 0,
            direction: 0,
            movement_angle: 0f32,
            follow_main_graphic: false,
            alive: true,
            current_action: IScriptCurrentAction::Idle,
            map_pos_x: map_x,
            map_pos_y: map_y,
        }
    }

    /// reference to iscript animation offsets
    pub fn iscript_anim_offsets(&self) -> &Vec<u16> {
        self.gd.iscript.id_offsets_map.get(&self.iscript_id).unwrap()
    }

    pub fn set_animation(&mut self, anim: AnimationType) {
        self.pos = self.iscript_anim_offsets()[anim as usize];
    }
    pub fn is_animation_valid(&self, anim: AnimationType) -> bool {
        self.iscript_anim_offsets()[anim as usize] > 0
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

    pub fn current_animation(&self) -> AnimationType {
        let mut nearest_label = AnimationType::Init;
        let mut nearest_dist = 10000;
        for lbl_idx in 0..self.iscript_anim_offsets().len() {
            let lbl_pos = self.iscript_anim_offsets()[lbl_idx];
            if self.pos >= lbl_pos {
                let dist = self.pos - lbl_pos;
                if dist < nearest_dist {
                    nearest_label = AnimationType::from_usize(lbl_idx).unwrap();
                    nearest_dist = dist;
                }
            }
        }
        nearest_label
    }

    pub fn anim_count(&self) -> usize {
        self.iscript_anim_offsets().len()
    }

    pub fn _interpret_iscript(&mut self, parent: Option<&IScriptState>) -> Option<IScriptEntityAction> {
        if !self.alive {
            return None;
        }
        // FIXME: is waiting actually counted in frames?
        if self.waiting_ticks_left > 0 {
            self.waiting_ticks_left -= 1;
            return None;
        }

        let val = self.read_u8().unwrap();
        let opcode = OpCode::from_u8(val).unwrap();

        // FIXME: is this right? seems required for phoenix walking overlay
        match parent {
            Some(s) => {
                self.direction = s.direction;
            }
            _ => {}
        }

        if self.follow_main_graphic && parent.is_some() {
            self.direction = parent.unwrap().direction;
            self.frameset = parent.unwrap().frameset;
        }

        def_opcodes! (
            self,
        // show debug output?
        // parent.is_some(),
            false,
            opcode,
        OpCode::ImgUl => (image_id: u16, rel_x: u8, rel_y: u8) {
        // shadows and such; img* is associated with the current entity
            return Some(IScriptEntityAction::CreateImageUnderlay {
                image_id: image_id,
                rel_x: rel_x as i8,
                rel_y: rel_y as i8,
            });
        },
        OpCode::ImgOl => (image_id: u16, rel_x: u8, rel_y: u8) {
        // e.g. explosions on death
            return Some(IScriptEntityAction::CreateImageOverlay {
                image_id: image_id,
                rel_x: rel_x as i8,
                rel_y: rel_y as i8,
            });
        },
        OpCode::SprOl => (sprite_id: u16, rel_x: u8, rel_y: u8) {
        // independent overlay, e.g. scanner sweep
        // FIXME
            println!("--- sprol not implemented yet ---");
            return Some(IScriptEntityAction::CreateSpriteOverlay {
                sprite_id: sprite_id,
                x: (rel_x as u16) + (self.rel_x as u16) + self.map_pos_x,
                y: (rel_y as u16) + (self.rel_y as u16) + self.map_pos_y,
            });
        },
        OpCode::LowSprUl => (sprite_id: u16, rel_x: u8, rel_y: u8) {
        // independent underlay, e.g. gore
        // FIXME
            println!("--- lowsprul not implemented yet ---");
            return Some(IScriptEntityAction::CreateSpriteUnderlay {
                sprite_id: sprite_id,
                x: (rel_x as u16) + (self.rel_x as u16) + self.map_pos_x,
                y: (rel_y as u16) + (self.rel_y as u16) + self.map_pos_y,
            });
        },

        OpCode::CreateGasOverlays => (overlay_no: u8) {
            let smoke_img_id = 430 + overlay_no as u16;
            let overlay_id = self.gd.images_dat.special_overlay[self.image_id as usize];
            let (rx, ry) = {
                let mut c = self.gd.lox_cache.borrow_mut();
                let lo = c.lox(&self.gd, overlay_id) ;
                lo.frames[0].offsets[overlay_no as usize]
            };
            return Some(IScriptEntityAction::CreateImageOverlay {
                image_id: smoke_img_id,
        // FIXME signed or unsigned?
                rel_x: rx,
                rel_y: ry,
            });
        },

        OpCode::PlayFram => (frame: u16) {
        // println!("playfram: {}", frame);
            self.frameset = frame;
        },
        OpCode::PlayFramTile => (frame: u16) {
            println!("--- playframtile not implemented yet ---");
        // FIXME
        },
        OpCode::EngFrame => (frame: u8) {
        // FIXME is this right: same as playfram?
            self.frameset = frame as u16;
        },
        OpCode::FollowMainGraphic => () {
            assert!(parent.is_some());
            self.follow_main_graphic = true;
        },
        OpCode::EngSet => () {
        // same as FollowMainGraphic
            assert!(parent.is_some());
            self.follow_main_graphic = true;
        },

        OpCode::Wait => (ticks: u8) {
            self.waiting_ticks_left += ticks as usize;
        },
        OpCode::WaitRand => (minticks: u8, maxticks: u8) {
            let r = ::rand::thread_rng().gen_range(minticks, maxticks+1);
        // println!(" -> {}", r);
            self.waiting_ticks_left += r as usize;
        },
        OpCode::SigOrder => (signal: u8) {
        // FIXME
            println!("--- not implemented yet ---");
        },
        OpCode::Goto => (target: u16) {
            self.pos = target;
        },
        OpCode::RandCondJmp => (val: u8, target: u16) {
            let r = ::rand::random::<u8>();
            if r < val {
        // println!(" jumping!");
                self.pos = target;
            }
        },
        OpCode::TurnRand => (units: u8) {
            if ::rand::thread_rng().gen_range(0, 100) < 50 {
                self.turn_cwise(units);
            } else {
                self.turn_ccwise(units);
            }
        },
        OpCode::TurnCWise => (units: u8) {
            self.turn_cwise(units);
        },
        OpCode::TurnCCWise => (units: u8) {
            self.turn_ccwise(units);
        },
        OpCode::SetFlDirect => (dir: u8) {
            self.set_direction(dir);
        },
        // FIXME: might be signed bytes?
        OpCode::SetVertPos => (val: u8) {
            self.rel_y = val as i8;
        },
        OpCode::SetHorPos => (val: u8) {
            self.rel_x = val as i8;
        },
        OpCode::Move => (dist: u8) {
            let fdist = dist as f32;
            let (dx, dy) = (self.movement_angle.cos() * fdist ,
                            self.movement_angle.sin() * fdist);
            self.map_pos_x = (self.map_pos_x as i32 + dx.round() as i32) as u16;
            self.map_pos_y = (self.map_pos_y as i32 + dy.round() as i32) as u16;
        },

        // FIXME sounds
        OpCode::PlaySndBtwn => (val1: u16, val2: u16) {
        },
        OpCode::PlaySnd => (sound_id: u16) {
        },

        OpCode::NoBrkCodeStart => () {
        // FIXME
        },
        OpCode::NoBrkCodeEnd => () {
        // FIXME
        },
        OpCode::TmpRmGraphicStart => () {
        // Sets the current image overlay state to hidden
            println!("tmprmgraphicstart, has parent: {}", parent.is_some());
            self.visible = false;
        },
        OpCode::TmpRmGraphicEnd => () {
        // Sets the current image overlay state to visible
            println!("tmprmgraphicend, has parent: {}", parent.is_some());
            self.visible = true;
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
        // FIXME
        },
        OpCode::SetFlSpeed => (speed: u16) {
        // FIXME
        },

        OpCode::End => () {
        // FIXME
            self.alive = false;
        }

    );
        None
    }
}
