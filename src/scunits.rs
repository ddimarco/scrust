use std::io::Result;
use std::io::Read;

extern crate byteorder;
use byteorder::{ReadBytesExt, LittleEndian};

extern crate num;
use num::FromPrimitive;

use ::rand::Rng;

use std::rc::Rc;

use ::gamedata::{GameData, GRPCache};
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
    CreateImageUnderlay {image_id: u16, rel_x: i8, rel_y: i8},
    CreateImageOverlay {image_id: u16, rel_x: i8, rel_y: i8},
    CreateSpriteOverlay {sprite_id: u16, x: u16, y: u16},
    CreateSpriteUnderlay {sprite_id: u16, x: u16, y: u16},
}

pub struct IScriptState {
    pub iscript_id: u32,
    /// current position in iscript
    pub pos: u16,

    // XXX: nasty: duplicate information
    image_id: u16,

    // TODO: couldn't find a better way to keep multiple immutable refs to gamedata
    gd: Rc<GameData>,

    waiting_ticks_left: usize,
    // FIXME: signed or unsigned?
    rel_x: i8,
    rel_y: i8,
    direction: u8,
    frameset: u16,
    follow_main_graphic: bool,
    visible: bool,
    alive: bool,

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

#[derive(Debug, Clone, Copy)]
pub enum SCImageRemapping {
    Normal,
    OFire,
    GFire,
    BFire,
    BExpl,
    Shadow,
}

impl IScriptState {
    pub fn new(gd: &Rc<GameData>, iscript_id: u32, image_id: u16, map_x: u16, map_y: u16) -> IScriptState {
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
            follow_main_graphic: false,
            alive: true,
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

    fn _interpret_iscript(&mut self, parent: Option<&IScriptState>) -> Option<IScriptEntityAction> {
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
            Some(s) => {self.direction = s.direction;},
            _ => {},
        }

        if self.follow_main_graphic && parent.is_some() {
            self.direction = parent.unwrap().direction;
            self.frameset = parent.unwrap().frameset;
        }

        def_opcodes! (
            self,
            // show debug output?
            //parent.is_some(),
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
            // FIXME
            println!("move not implemented!");
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
pub struct SCImage {
    pub image_id: u16,
    pub grp_id: u32,
    pub player_id: usize,
    iscript_state: IScriptState,
    underlays: Vec<SCImage>,
    overlays: Vec<SCImage>,
}
pub trait IScriptableTrait {
    fn get_iscript_state<'a>(&'a self) -> &'a IScriptState;
    fn get_iscript_state_mut<'a>(&'a mut self) -> &'a mut IScriptState;
}
impl IScriptableTrait for SCImage {
    fn get_iscript_state<'a>(&'a self) -> &'a IScriptState {
        &self.iscript_state
    }
    fn get_iscript_state_mut<'a>(&'a mut self) -> &'a mut IScriptState {
        &mut self.iscript_state
    }
}
pub trait SCImageTrait : IScriptableTrait {
    fn get_scimg<'a>(&'a self) -> &'a SCImage;
    fn get_scimg_mut<'a>(&'a mut self) -> &'a mut SCImage;
}
impl SCImageTrait for SCImage {
    fn get_scimg<'a>(&'a self) -> &'a SCImage {
        self
    }
    fn get_scimg_mut<'a>(&'a mut self) -> &'a mut SCImage {
        self
    }
}

impl SCImage {
    pub fn new(gd: &Rc<GameData>, image_id: u16, map_x: u16, map_y: u16) -> SCImage {
        let iscript_id = gd.images_dat.iscript_id[image_id as usize];
        let grp_id = gd.images_dat.grp_id[image_id as usize];
        {
            gd.grp_cache.borrow_mut().load(gd, grp_id);
        }

        SCImage {
            image_id: image_id,
            grp_id: grp_id,
            iscript_state: IScriptState::new(&gd, iscript_id, image_id, map_x, map_y),
            // FIXME we probably only need 1 overlay & 1 underlay
            underlays: Vec::<SCImage>::new(),
            overlays: Vec::<SCImage>::new(),
            player_id: 0,
        }
    }

    fn can_turn(&self) -> bool {
        (self.iscript_state.gd.images_dat.graphic_turns[self.image_id as usize] > 0)
    }
    fn draw_flipped(&self) -> bool {
        self.can_turn() && self.iscript_state.direction > 16
    }

    fn frame_idx(&self) -> usize {
        if !self.can_turn() {
            self.iscript_state.frameset as usize
        } else if self.iscript_state.direction > 16 {
            (self.iscript_state.frameset + 32 - self.iscript_state.direction as u16) as usize
        } else {
            (self.iscript_state.frameset + self.iscript_state.direction as u16) as usize
        }
    }

    pub fn remapping(&self, gd: &GameData) -> SCImageRemapping {
        let idat_draw_func = gd.images_dat.draw_function[self.image_id as usize];
        match idat_draw_func {
            10 => SCImageRemapping::Shadow,
            9 => match gd.images_dat.remapping[self.image_id as usize] {
                1 => SCImageRemapping::OFire,
                2 => SCImageRemapping::GFire,
                3 => SCImageRemapping::BFire,
                4 => SCImageRemapping::BExpl,
                x => { panic!("unknown remapping {}!", x); }
            },
            _ => SCImageRemapping::Normal,
        }
    }

    fn _draw(&self, grp_cache: &GRPCache, cx: i32, cy: i32, buffer: &mut [u8], buffer_pitch: u32, has_parent: bool) {
        if !self.iscript_state.visible {
            return;
        }

        let fridx = self.frame_idx();
        let grp = grp_cache.grp_ro(self.grp_id);
        // this seems like a hack
        if fridx >= grp.frames.len() && has_parent {
            println!("WARNING: suspicious frame index");
            return;
        }
        let udata = &grp.frames[fridx];

        let w = grp.header.width as u32;
        let h = grp.header.height as u32;
        let x_center = cx + self.iscript_state.rel_x as i32;
        let y_center = cy + self.iscript_state.rel_y as i32;

        let remap = self.remapping(self.iscript_state.gd.as_ref());
        let reindex =
            match remap {
                SCImageRemapping::OFire => &self.iscript_state.gd.ofire_reindexing.data,
                SCImageRemapping::BFire => &self.iscript_state.gd.bfire_reindexing.data,
                SCImageRemapping::GFire => &self.iscript_state.gd.gfire_reindexing.data,
                SCImageRemapping::BExpl => &self.iscript_state.gd.bexpl_reindexing.data,
                SCImageRemapping::Shadow => &self.iscript_state.gd.shadow_reindexing,
                SCImageRemapping::Normal => {
                    if self.player_id < 11 {
                        let startpt = self.player_id as usize *256;
                        &self.iscript_state.gd.player_reindexing[startpt..startpt+256]
                    } else {
                        // neutral player (i.e. minerals, critters, etc)
                        &self.iscript_state.gd.player_reindexing[0..256]
                    }
                },
            };
        match remap {
            SCImageRemapping::OFire | SCImageRemapping::BFire | SCImageRemapping::GFire |
            SCImageRemapping::BExpl | SCImageRemapping::Shadow => {
                render_buffer_with_transparency_reindexing(udata, w, h, self.draw_flipped(),
                                                           x_center, y_center, buffer, buffer_pitch,
                                                           &reindex);
            },
            SCImageRemapping::Normal => {
                render_buffer_with_solid_reindexing(udata, w, h, self.draw_flipped(),
                                                           x_center, y_center, buffer, buffer_pitch,
                                                           &reindex);
            }
        }


    }

    /// cx, cy: screen coordinates
    pub fn draw(&self, grp_cache: &GRPCache, cx: i32, cy: i32, buffer: &mut [u8], buffer_pitch: u32) {
        // draw underlays
        for ul in &self.underlays {
            ul._draw(grp_cache, cx, cy, buffer, buffer_pitch, true);
        }
        // draw main image
        self._draw(grp_cache, cx, cy, buffer, buffer_pitch, false);
        // draw overlays
        for ol in &self.overlays {
            ol._draw(grp_cache, cx, cy, buffer, buffer_pitch, true);
        }
    }

    pub fn step(&mut self,
                // just for creating new entities
                gd: &Rc<GameData>) -> Option<IScriptEntityAction> {
        // FIXME: death animation for marine: shadow tries to display wrong frameset
        for ul in &mut self.underlays {
            let action = ul.iscript_state._interpret_iscript(Some(&self.iscript_state));
            // assuming they do not create additional under/overlays
            assert!(action.is_none());
        }
        self.underlays.retain(|ref ul| ul.iscript_state.alive);

        let iscript_action = self.iscript_state._interpret_iscript(None);
        for ol in &mut self.overlays {
            let action = ol.iscript_state._interpret_iscript(Some(&self.iscript_state));
            // assuming they do not create additional under/overlays
            assert!(action.is_none());
        }
        self.overlays.retain(|ref ol| ol.iscript_state.alive);

        // create additional entities if necessary
        match iscript_action {
            Some(IScriptEntityAction::CreateImageUnderlay {image_id, rel_x, rel_y}) => {
                let mut underlay = SCImage::new(gd, image_id, 0, 0);
                underlay.iscript_state.rel_x = rel_x;
                underlay.iscript_state.rel_y = rel_y;
                self.underlays.push(underlay);
                None
            },
            Some(IScriptEntityAction::CreateImageOverlay {image_id, rel_x, rel_y}) => {
                let mut overlay = SCImage::new(gd, image_id, 0, 0);
                overlay.iscript_state.rel_x = rel_x;
                overlay.iscript_state.rel_y = rel_y;
                self.overlays.push(overlay);
                None
            },
            _ => {
                iscript_action
            },
        }
    }
}

////////////////////////////////////////

// sprite: additional features:
// - health bar
// - selection circle

pub struct SCSpriteSelectable {
    /// from sprites.dat: length of health bar in pixels
    health_bar: u8,
    // circle_img: u8,
    circle_offset: u8,
    // FIXME: inefficient
    circle_grp_id: u32,

    pub sel_width: u16,
    pub sel_height: u16,
}

pub struct SCSprite {
    pub sprite_id: u16,
    pub img: SCImage,
    pub selectable_data: Option<SCSpriteSelectable>,
}

impl IScriptableTrait for SCSprite {
    fn get_iscript_state<'a>(&'a self) -> &'a IScriptState {
        self.img.get_iscript_state()
    }
    fn get_iscript_state_mut<'a>(&'a mut self) -> &'a mut IScriptState {
        self.img.get_iscript_state_mut()
    }
}
impl SCImageTrait for SCSprite {
    fn get_scimg<'a>(&'a self) -> &'a SCImage {
        &self.img
    }
    fn get_scimg_mut<'a>(&'a mut self) -> &'a mut SCImage {
        &mut self.img
    }
}
pub trait SCSpriteTrait: SCImageTrait {
    fn get_scsprite<'a>(&'a self) -> &'a SCSprite;
    fn get_scsprite_mut<'a>(&'a mut self) -> &'a mut SCSprite;
}
impl SCSpriteTrait for SCSprite {
    fn get_scsprite<'a>(&'a self) -> &'a SCSprite { self }
    fn get_scsprite_mut<'a>(&'a mut self) -> &'a mut SCSprite { self }
}
macro_rules! render_function {
    ($fname:ident, $func:expr; $($param:ident: $param_ty:ty),* ) => {
        pub fn $fname(inbuffer: &[u8], width: u32, height: u32, flipped: bool,
                      cx: i32, cy: i32, buffer: &mut [u8], buffer_pitch: u32,
                      $($param: $param_ty), *) {
            unsafe {
                let halfheight = (height as i32)/2;
                let halfwidth = (width as i32)/2;
                let buffer_height = buffer.len() as u32/buffer_pitch;

                if (cx - halfwidth > buffer_pitch as i32) ||
                    (cy - halfheight > buffer_height as i32) {
                    return;
                }

                let yoffset =
                    (if cy < halfheight  {
                        halfheight - cy
                    } else {
                        0
                    }) as u32;
                let xoffset =
                    (if cx < halfwidth {
                        halfwidth - cx
                    } else {
                        0
                    }) as u32;

                let youtend = cy + halfheight;
                let ystop =
                    if youtend as u32 > buffer_height {
                        let rest = youtend as u32 - buffer_height;
                        if height < rest {
                            height
                        } else {
                            height - rest
                        }
                    } else {
                        height
                    };

                let xoutend = cx + halfwidth;
                let xstop =
                    if xoutend as u32 > buffer_pitch {
                        let rest = xoutend as u32 - buffer_pitch;
                        if width < rest {
                            width
                        } else {
                            width - rest
                        }
                    } else {
                        width
                    };
                let x_skip = width - xstop;

                let mut outpos = (cy + yoffset as i32 - halfheight) as u32 * buffer_pitch +
                    (cx + xoffset as i32 - halfwidth) as u32;

                if flipped {
                    for y in yoffset..ystop {
                        for x in xoffset..xstop {
                            let col = inbuffer.get_unchecked((y*width + (width - x - 1)) as usize);
                            $func(*col, buffer, outpos as usize,
                                  $($param),*
                            );
                            outpos += 1;
                        }
                        outpos += buffer_pitch - width + xoffset + x_skip;
                    }
                } else {
                    let mut inpos = yoffset*width + xoffset;
                    for _ in yoffset..ystop {
                        for _ in xoffset..xstop {
                            let col = inbuffer.get_unchecked(inpos as usize);
                            $func(*col, buffer, outpos as usize,
                                  $($param),*);
                            outpos += 1;
                            inpos += 1;
                        }
                        outpos += buffer_pitch - width + xoffset + x_skip;
                        inpos += xoffset + x_skip;
                    }
                }
            }
        }
    }
}

// FIXME: a lot of time spent here, speed this up
render_function!(render_buffer_with_transparency, |col: u8, buffer: &mut [u8], outpos: usize| {
    let ob = buffer.get_unchecked_mut(outpos);
    if col > 0 {
        *ob = col;
    }
};);
render_function!(render_buffer_with_transparency_reindexing,
                 |col: u8, buffer: &mut [u8], outpos: usize, reindex: &[u8]| {
     let ob = buffer.get_unchecked_mut(outpos);
     if col > 0 {
         *ob = *reindex.get_unchecked(((col as usize) - 1)*256 + *ob as usize);
     }
}; reindex: &[u8]);
render_function!(render_buffer_with_solid_reindexing,
                 |col: u8, buffer: &mut [u8], outpos: usize, reindex: &[u8]| {
                     let ob = buffer.get_unchecked_mut(outpos);
                     if col > 0 {
                         *ob = *reindex.get_unchecked(col as usize - 1);
                     }
                 }; reindex: &[u8]);

impl SCSprite {
    pub fn new(gd: &Rc<GameData>, sprite_id: u16, map_x: u16, map_y: u16) -> SCSprite {
        let image_id = gd.sprites_dat.image_id[sprite_id as usize];
        let img = SCImage::new(gd, image_id, map_x, map_y);

        let selectable_data =
        if sprite_id >= 130 {
            let circle_img = gd.sprites_dat.selection_circle_image[(sprite_id - 130) as usize];
            let circle_grp_id = gd.images_dat.grp_id[561 + circle_img as usize];

            let sel_width;
            let sel_height;
            {
                let mut grpcache = gd.grp_cache.borrow_mut();
                grpcache.load(gd, circle_grp_id);
                sel_width = grpcache.grp_ro(circle_grp_id).header.width.clone();
                sel_height = grpcache.grp_ro(circle_grp_id).header.height.clone();
            }

            Some(SCSpriteSelectable {
                health_bar: gd.sprites_dat.health_bar[(sprite_id - 130) as usize],
                circle_offset: gd.sprites_dat.selection_circle_offset[(sprite_id - 130) as usize],
                circle_grp_id: circle_grp_id,
                sel_width: sel_width,
                sel_height: sel_height,
            })
        } else {
            None
        };
        SCSprite {
            sprite_id: sprite_id,
            img: img,
            selectable_data: selectable_data,
        }
    }

    // FIXME: clipping
    pub fn draw_healthbar(&self, cx: u32, cy: u32, buffer: &mut [u8], buffer_pitch: u32) {
        match self.selectable_data {
            None => {
                panic!();
            },
            Some(ref selectable) => {
                let boxes = selectable.health_bar as u32 / 3;
                let box_width = 3;
                if selectable.health_bar == 0 {
                    println!("healthbar == 0, not drawing");
                    return;
                }
                let width = 2 + (box_width * boxes) + (boxes - 1);
                let height = 8;

                let mut outpos = ((cy + selectable.circle_offset as u32) - height / 2)
                    * buffer_pitch + (cx - width / 2);
                for y in 0..height {
                    for x in 0..width {
                        let outer_border = y == 0 || y == height-1 || x == 0 || x == (width-1);
                        let inner_border = x % (box_width+1) == 0;
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
    }


    pub fn draw_selection_circle(&self, grp_cache: &GRPCache, cx: i32, cy: i32, buffer: &mut [u8], buffer_pitch: u32) {
        match self.selectable_data {
            Some(ref selectable) => {
                let grp = grp_cache.grp_ro(selectable.circle_grp_id);
                render_buffer_with_transparency(&grp.frames[0],
                                                grp.header.width as u32,
                                                grp.header.height as u32,
                                                false,
                                                cx, cy + selectable.circle_offset as i32,
                                                buffer, buffer_pitch);
            },
            None => {
                panic!();
            }
        }
    }
}

pub struct SCUnit {
    pub unit_id: usize,
    // merging flingy and unit for now
    flingy_id: usize,
    sprite: SCSprite,
    pub kill_count: usize,
}
impl IScriptableTrait for SCUnit {
    fn get_iscript_state<'a>(&'a self) -> &'a IScriptState {
        self.sprite.get_iscript_state()
    }
    fn get_iscript_state_mut<'a>(&'a mut self) -> &'a mut IScriptState {
        self.sprite.get_iscript_state_mut()
    }
}
impl SCImageTrait for SCUnit {
    fn get_scimg<'a>(&'a self) -> &'a SCImage {
        &self.sprite.get_scimg()
    }
    fn get_scimg_mut<'a>(&'a mut self) -> &'a mut SCImage {
        self.sprite.get_scimg_mut()
    }
}
impl SCSpriteTrait for SCUnit {
    fn get_scsprite<'a>(&'a self) -> &'a SCSprite { &self.sprite }
    fn get_scsprite_mut<'a>(&'a mut self) -> &'a mut SCSprite { &mut self.sprite }
}
impl SCUnit {
    pub fn new(gd: &Rc<GameData>, unit_id: usize, map_x: u16, map_y: u16,
               player_id: usize) -> SCUnit {
        let flingy_id = gd.units_dat.flingy_id[unit_id];
        let sprite_id = gd.flingy_dat.sprite_id[flingy_id as usize];
        let mut sprite = SCSprite::new(gd, sprite_id, map_x, map_y);
        sprite.get_scimg_mut().player_id = player_id;
        SCUnit {
            unit_id: unit_id as usize,
            flingy_id: flingy_id as usize,
            sprite: sprite,
            kill_count: 0,
        }
    }
}
