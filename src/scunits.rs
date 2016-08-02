use std::io::Result;
use std::io::Read;

extern crate byteorder;
use byteorder::{ReadBytesExt, LittleEndian};

extern crate num;
use num::FromPrimitive;

use ::rand::Rng;

use std::rc::Rc;

use ::grp::GRP;
use ::unitsdata::ImagesDat;
use ::GameContext;
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

// extern crate linked_list;
// use linked_list::LinkedList;
// struct IScriptEntityLayer {
//     pub imgs: LinkedList<SCImage>,
// }
// impl IScriptEntityLayer {
//     fn new() -> IScriptEntityLayer {
//         IScriptEntityLayer {
//             imgs: LinkedList::<SCImage>::new(),
//         }
//     }
// }

enum IScriptEntityAction {
    CreateImageUnderlay {image_id: u16, rel_x: u8, rel_y: u8},
    CreateImageOverlay {image_id: u16, rel_x: u8, rel_y: u8},
}

pub struct IScriptState {
    pub iscript_id: u32,
    /// current position in iscript
    pub pos: u16,

    // TODO: couldn't find a better way to keep multiple immutable refs to gamedata
    gd: Rc<GameData>,

    waiting_ticks_left: usize,
    rel_x: u8,
    rel_y: u8,
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
        return Ok(buf.len());
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
    pub fn new(gd: &Rc<GameData>, iscript_id: u32, map_x: u16, map_y: u16) -> IScriptState {
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

    pub fn images_dat(&self) -> &ImagesDat {
        &self.gd.images_dat
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
        if parent.is_some() {
            self.direction = parent.unwrap().direction;
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
                rel_x: rel_x,
                rel_y: rel_y,
            });
        },
        OpCode::ImgOl => (image_id: u16, rel_x: u8, rel_y: u8) {
            // e.g. explosions on death
            return Some(IScriptEntityAction::CreateImageOverlay {
                image_id: image_id,
                rel_x: rel_x,
                rel_y: rel_y,
            });
        },
        OpCode::SprOl => (sprite_id: u16, rel_x: u8, rel_y: u8) {
            // independent overlay, e.g. scanner sweep
            // FIXME
            println!("--- not implemented yet ---");
        },
        OpCode::LowSprUl => (sprite_id: u16, rel_x: u8, rel_y: u8) {
        // independent underlay, e.g. gore
        // FIXME
            println!("--- not implemented yet ---");
        },

        OpCode::CreateGasOverlays => (overlay_no: u8) {
            // FIXME
            println!("--- not implemented yet ---");
        },

        OpCode::PlayFram => (frame: u16) {
            // println!("playfram: {}", frame);
            self.frameset = frame;
        },
        OpCode::PlayFramTile => (frame: u16) {
            println!("--- not implemented yet ---");
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
            self.rel_y = val;
        },
        OpCode::SetHorPos => (val: u8) {
            self.rel_x = val;
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
        return None;
    }

}
pub struct SCImage {
    pub image_id: u16,
    // FIXME: avoid duplicated grps
    //grp: GRP,
    pub grp_id: u32,
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
        // let name = "unit\\".to_string() + &gd.images_tbl[(grp_id as usize) - 1];
        // println!("grp id: {}, filename: {}", grp_id, name);
        {
            gd.grp_cache.borrow_mut().load(gd, grp_id);
        }
        //let grp = GRP::read(&mut gd.open(&name).unwrap());
        // let grp = (*grp_cache.grp(gd, grp_id)).clone();


        SCImage {
            image_id: image_id,
            //grp: grp,
            grp_id: grp_id,
            iscript_state: IScriptState::new(&gd, iscript_id, map_x, map_y),
            // FIXME we probably only need 1 overlay & 1 underlay
            underlays: Vec::<SCImage>::new(),
            overlays: Vec::<SCImage>::new(),
        }
    }

    fn can_turn(&self) -> bool {
        (self.iscript_state.images_dat().graphic_turns[self.image_id as usize] > 0)
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

    pub fn get_reindexing_table(gd: &GameData, remap: SCImageRemapping) -> &Vec<u8>{
        match remap {
            SCImageRemapping::OFire => &gd.ofire_reindexing.data,
            SCImageRemapping::BFire => &gd.bfire_reindexing.data,
            SCImageRemapping::GFire => &gd.gfire_reindexing.data,
            SCImageRemapping::BExpl => &gd.bexpl_reindexing.data,
            SCImageRemapping::Normal => &gd.null_reindexing,
            SCImageRemapping::Shadow => &gd.shadow_reindexing,
        }
    }

    fn _draw(&self, grp_cache: &GRPCache, cx: u32, cy: u32, buffer: &mut [u8], buffer_pitch: u32, has_parent: bool) {
        if !self.iscript_state.visible {
            return;
        }

        let remap = self.remapping(self.iscript_state.gd.as_ref());
        let reindex = SCImage::get_reindexing_table(self.iscript_state.gd.as_ref(),
                                                    remap);

        let fridx = self.frame_idx();
        let grp = grp_cache.grp_ro(self.grp_id);
        // this seems like a hack
        if fridx >= grp.frames.len() && has_parent {
            println!("WARNING: suspicious frame index");
            return;
        }
        let udata = &grp.frames[fridx];

        let w = grp.header.width;
        let h = grp.header.height;
        let x_center = cx + self.iscript_state.rel_x as u32;
        let (in_pitch, x_in_offset, x_start) =
            if x_center < (w as u32 / 2) {
                // clip (image is at left border)
                let xio = (w as u32 / 2) - x_center;
                ((w as u32)-xio, xio, 0)
            } else {
                let x_start = x_center - (w as u32 / 2);
                (w as u32, 0, x_start as usize)
            };
        if x_start > buffer_pitch as usize {
            return;
        }
        let in_width =
            if (x_center + in_pitch / 2) > buffer_pitch {
                buffer_pitch - (x_start as u32)
            } else {
                in_pitch
            };

        let y_center = cy + self.iscript_state.rel_y as u32;
        let (_y_in_height, y_in_offset, y_start) =
            if y_center < (h as u32 / 2) {
                // clip (top border)
                let yio = (h as u32 / 2) - y_center;
                ((h as u32)-yio, yio, 0)
            } else {
                let y_start = y_center - (h as u32 / 2);
                (h as u32, 0, y_start)
            };
        let buf_height = buffer.len() / buffer_pitch as usize;
        if y_start as usize > buf_height {
            return;
        }
        let y_in_height =
            if (y_center + _y_in_height / 2) as usize > buf_height {
                buf_height as u32 - y_start
            } else {
                _y_in_height
            };

        let mut outpos = (y_start * buffer_pitch) as usize + x_start;
        let mut inpos = (y_in_offset as usize * (w as usize)) + x_in_offset as usize;
        let flipped = self.draw_flipped();
        if !flipped {
            for _ in 0..y_in_height {
                for _ in 0..in_width {
                    let dest = udata[inpos] as usize;
                    if dest > 0 {
                        let src = buffer[outpos] as usize;
                        buffer[outpos] = reindex[(dest - 1) * 256 + src];
                    }
                    outpos += 1;
                    inpos += 1;
                }
                let to_skip = (w as u32 - in_width) as usize;
                inpos += to_skip;
                outpos += to_skip + (buffer_pitch - w as u32) as usize;
            }
        } else {
            // draw flipped
            for y in (y_in_offset as usize)..(y_in_offset as usize + y_in_height as usize) {
                for x in (x_in_offset as usize)..(x_in_offset as usize + in_width as usize) {
                    let dest = udata[(y*(w as usize) + ((w as usize) - x - 1)) as usize] as usize;
                    if dest > 0 {
                        let src = buffer[outpos] as usize;
                        buffer[outpos] = reindex[(dest - 1) * 256 + src];
                    }
                    outpos += 1;
                }
                let to_skip = (w as u32 - in_width) as usize;
                outpos += to_skip + (buffer_pitch - w as u32) as usize;
            }
        }
    }

    pub fn draw(&self, grp_cache: &GRPCache, cx: u32, cy: u32, buffer: &mut [u8], buffer_pitch: u32) {
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
                gd: &Rc<GameData>) {
        // FIXME: death animation for marine: shadow tries to display wrong frameset
        for ul in &mut self.underlays {
            let _ = ul.iscript_state._interpret_iscript(Some(&self.iscript_state));
            // assuming they do not create additional under/overlays
        }
        let iscript_action = self.iscript_state._interpret_iscript(None);
        for ol in &mut self.overlays {
            let _ = ol.iscript_state._interpret_iscript(Some(&self.iscript_state));
            // assuming they do not create additional under/overlays
        }

        // create additional entities if necessary
        match iscript_action {
            Some(IScriptEntityAction::CreateImageUnderlay {image_id, rel_x, rel_y}) => {
                println!("creating underlay");

                let mut underlay = SCImage::new(gd, image_id, 0, 0);
                underlay.iscript_state.rel_x = rel_x;
                underlay.iscript_state.rel_y = rel_y;
                self.underlays.push(underlay);
            },
            Some(IScriptEntityAction::CreateImageOverlay {image_id, rel_x, rel_y}) => {
                println!("create overlay");
                let mut overlay = SCImage::new(gd, image_id, 0, 0);
                overlay.iscript_state.rel_x = rel_x;
                overlay.iscript_state.rel_y = rel_y;
                self.overlays.push(overlay);
            },
            _ => {},
        }
    }
}

////////////////////////////////////////

// sprite: additional features:
// - health bar
// - selection circle
pub struct SCSprite {
    pub sprite_id: u16,
    pub img: SCImage,
    /// from sprites.dat: length of health bar in pixels
    health_bar: u8,
    // circle_img: u8,
    circle_offset: u8,
    // FIXME: inefficient
   circle_grp: GRP,
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

impl SCSprite {
    pub fn new(gd: &Rc<GameData>, sprite_id: u16, map_x: u16, map_y: u16) -> SCSprite {
        let image_id = gd.sprites_dat.image_id[sprite_id as usize];
        let img = SCImage::new(gd, image_id, map_x, map_y);

        let circle_img = gd.sprites_dat.selection_circle_image[(sprite_id - 130) as usize];
        let circle_grp_id = gd.images_dat.grp_id[561 + circle_img as usize];
        let name = "unit\\".to_string() + &gd.images_tbl[(circle_grp_id as usize) - 1];
        let grp = GRP::read(&mut gd.open(&name).unwrap());
        SCSprite {
            sprite_id: sprite_id,
            img: img,
            health_bar: gd.sprites_dat.health_bar[(sprite_id - 130) as usize],
            circle_offset: gd.sprites_dat.selection_circle_offset[(sprite_id - 130) as usize],
            circle_grp: grp,
        }
    }

    pub fn draw_healthbar(&self, cx: u32, cy: u32, buffer: &mut [u8], buffer_pitch: u32) {
        let boxes = self.health_bar as u32 / 3;
        let box_width = 3;
        if self.health_bar == 0 {
            println!("healthbar == 0, not drawing");
            return;
        }
        let width = 2 + (box_width * boxes) + (boxes - 1);
        let height = 8;

        let mut outpos = (cy - height / 2) * buffer_pitch + (cx - width / 2);
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

    pub fn draw_selection_circle(&self, cx: u32, cy: u32, buffer: &mut [u8], buffer_pitch: u32) {
        let width = self.circle_grp.header.width as u32;
        let height = self.circle_grp.header.height as u32;

        let mut outpos = ((cy + self.circle_offset as u32) - height / 2) * buffer_pitch + (cx - width / 2);
        let mut inpos = 0;
        // FIXME: reindexing
        for _ in 0..height {
            for _ in 0..width {
                let col = self.circle_grp.frames[0][inpos as usize];
                if col > 0 {
                    buffer[outpos as usize] = col;
                }
                outpos += 1;
                inpos += 1;
            }
            outpos += buffer_pitch - width;
        }
    }
}

pub struct SCUnit {
    unit_id: usize,
    // merging flingy and unit for now
    flingy_id: usize,
    sprite: SCSprite,
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
    pub fn new(gd: &Rc<GameData>, unit_id: usize, map_x: u16, map_y: u16) -> SCUnit {
        let flingy_id = gd.units_dat.flingy_id[unit_id];
        let sprite_id = gd.flingy_dat.sprite_id[flingy_id as usize];
        let sprite = SCSprite::new(gd, sprite_id, map_x, map_y);
        SCUnit {
            unit_id: unit_id as usize,
            flingy_id: flingy_id as usize,
            sprite: sprite,
        }
    }
}
