use std::path::Path;

use std::io::Read;

extern crate byteorder;
use byteorder::{ReadBytesExt, LittleEndian};

extern crate num;
use num::FromPrimitive;

extern crate rand;
use rand::Rng;

extern crate sdl2;
use sdl2::pixels::Color;
use sdl2::rect::Rect;

extern crate read_pcx;
use read_pcx::gamedata::GameData;
use read_pcx::iscript::{AnimationType, OpCode};
use read_pcx::grp::GRP;
use read_pcx::unitsdata::ImagesDat;

use read_pcx::font::FontSize;
use read_pcx::font::RenderText;

use read_pcx::{GameContext, View, ViewAction};

use std::rc::Rc;


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


enum CreateIScriptEntity {
    ImageUnderlay {image_id: u16, rel_x: u8, rel_y: u8},
    ImageOverlay {image_id: u16, rel_x: u8, rel_y: u8},
}

struct IScriptState {
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

    /// signals the parent to create a new entity
    create_entity_action: Option<CreateIScriptEntity>,
}

impl Read for IScriptState {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
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

#[derive(Debug)]
enum SCImageRemapping {
    Normal,
    OFire,
    GFire,
    BFire,
    BExpl,
    Shadow,
}

impl IScriptState {
    pub fn new(gd: &Rc<GameData>, iscript_id: u32) -> IScriptState {
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
            rel_x: 0,
            rel_y: 0,
            frameset: 0,
            direction: 0,
            follow_main_graphic: false,
            create_entity_action: None,
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

    // TODO: move into trait
    pub fn interpret_iscript(&mut self, parent: Option<&IScriptState>) {
        assert!(self.create_entity_action.is_none());

        // FIXME: is waiting actually counted in frames?
        if self.waiting_ticks_left > 0 {
            self.waiting_ticks_left -= 1;
            return;
        }

        let pos1 = self.pos;
        let val = self.read_u8().unwrap();
        let pos2 = self.pos;
        assert!(pos1 < pos2);
        let opcode = OpCode::from_u8(val).unwrap();


        // FIXME: is this right? seems required for phoenix walking overlay
        if parent.is_some() {
            self.direction = parent.unwrap().direction;
        }

        def_opcodes! (
            self,
            // show debug output?
            false,
            opcode,
        OpCode::ImgUl => (image_id: u16, rel_x: u8, rel_y: u8) {
            // shadows and such; img* is associated with the current entity
            self.create_entity_action = Some(CreateIScriptEntity::ImageUnderlay {
                image_id: image_id,
                rel_x: rel_x,
                rel_y: rel_y,
            });
        },
        OpCode::ImgOl => (image_id: u16, rel_x: u8, rel_y: u8) {
            // e.g. explosions on death
            self.create_entity_action = Some(CreateIScriptEntity::ImageOverlay {
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
            let r = rand::thread_rng().gen_range(minticks, maxticks+1);
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
            let r = rand::random::<u8>();
            if r < val {
                println!(" jumping!");
                self.pos = target;
            }
        },
        OpCode::TurnRand => (units: u8) {
            if rand::thread_rng().gen_range(0, 100) < 50 {
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
        }

    );

        if self.follow_main_graphic && parent.is_some() {
            self.direction = parent.unwrap().direction;
            self.frameset = parent.unwrap().frameset;
        }
        }
}
struct SCImage {
    pub image_id: u16,
    // FIXME: avoid duplicated grps
    grp: GRP,
    iscript_state: IScriptState,
    underlays: Vec<SCImage>,
    overlays: Vec<SCImage>,
}

impl SCImage {
    pub fn new(gd: &Rc<GameData>,
               image_id: u16)
               -> SCImage {
        let iscript_id = gd.images_dat.iscript_id[image_id as usize];
        let grp_id = gd.images_dat.grp_id[image_id as usize];
        let name = "unit\\".to_string() + &gd.images_tbl[(grp_id as usize) - 1];
        println!("grp id: {}, filename: {}", grp_id, name);
        let grp = GRP::read(&mut gd.open(&name).unwrap());

        SCImage {
            image_id: image_id,
            grp: grp,
            iscript_state: IScriptState::new(&gd, iscript_id),
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
            SCImageRemapping::Normal => &gd.null_reindexing,
            SCImageRemapping::Shadow => &gd.shadow_reindexing,
            _ => {panic!("unknown remapping enum: {:?}", remap);}
        }
    }

    fn _draw(&self, cx: u32, cy: u32, buffer: &mut [u8], buffer_pitch: u32) {
        let remap = self.remapping(self.iscript_state.gd.as_ref());
        let reindex = SCImage::get_reindexing_table(self.iscript_state.gd.as_ref(),
                                                    remap);

        let w = self.grp.header.width;
        let h = self.grp.header.height;
        let x_start = (cx + self.iscript_state.rel_x as u32) - (w as u32 / 2);
        let y_start = (cy + self.iscript_state.rel_y as u32) - (h as u32 / 2);
        let fridx = self.frame_idx();
        let udata = &self.grp.frames[fridx];

        let mut outpos = ((y_start * buffer_pitch) + x_start as u32) as usize;
        let mut inpos = 0 as usize;
        let flipped = self.draw_flipped();
        if !flipped {
            for _ in 0..h {
                for _ in 0..w {
                    let dest = udata[inpos] as usize;
                    if dest > 0 {
                        let src = buffer[outpos] as usize;
                        buffer[outpos] = reindex[(dest ) * 256 + src];
                    }
                    outpos += 1;
                    inpos += 1;
                }
                outpos += (buffer_pitch - w as u32) as usize;
            }
        } else {
            // draw flipped
            for y in 0..h {
                for x in 0..w {
                    let dest = udata[(y*w + (w - x - 1)) as usize] as usize;
                    if dest > 0 {
                        let src = buffer[outpos] as usize;
                        buffer[outpos] = reindex[(dest ) * 256 + src];
                    }
                    outpos += 1;
                }
                outpos += (buffer_pitch - w as u32) as usize;
            }
        }

    }

    pub fn draw(&self, cx: u32, cy: u32, buffer: &mut [u8], buffer_pitch: u32) {
        // draw underlays
        for ul in &self.underlays {
            ul._draw(cx, cy, buffer, buffer_pitch);
        }
        // draw main image
        self._draw(cx, cy, buffer, buffer_pitch);
        // draw overlays
        for ol in &self.overlays {
            ol._draw(cx, cy, buffer, buffer_pitch);
        }
    }

    pub fn step(&mut self,
                // just for creating new entities
                gd: &Rc<GameData>) {
        // FIXME: death animation for marine: shadow tries to display wrong frameset
        for ul in &mut self.underlays {
            ul.iscript_state.interpret_iscript(Some(&self.iscript_state));
            // assuming they do not create additional under/overlays
        }
        self.iscript_state.interpret_iscript(None);
        for ol in &mut self.overlays {
            ol.iscript_state.interpret_iscript(Some(&self.iscript_state));
            // assuming they do not create additional under/overlays
        }

        // create additional entities if necessary
        match self.iscript_state.create_entity_action {
            Some(CreateIScriptEntity::ImageUnderlay {image_id, rel_x, rel_y}) => {
                println!("creating underlay");

                let mut underlay = SCImage::new(gd, image_id);
                underlay.iscript_state.rel_x = rel_x;
                underlay.iscript_state.rel_y = rel_y;
                self.underlays.push(underlay);

                self.iscript_state.create_entity_action = None;
            },
            Some(CreateIScriptEntity::ImageOverlay {image_id, rel_x, rel_y}) => {
                println!("create overlay");
                let mut overlay = SCImage::new(gd, image_id);
                overlay.iscript_state.rel_x = rel_x;
                overlay.iscript_state.rel_y = rel_y;
                self.overlays.push(overlay);

                self.iscript_state.create_entity_action = None;
            },
            _ => {},
        }
    }
}


struct UnitsView {
    unit_id: usize,
    current_anim: AnimationType,
    next_anim_idx: usize,
    anim_str: String,
    unit_name_str: String,
    img: SCImage,
}
impl UnitsView {
    fn new(gc: &mut GameContext, unit_id: usize) -> UnitsView {
        let current_anim = AnimationType::Init;
        let anim_str = format!("Animation: {:?}", current_anim);
        let gd = gc.gd.clone();
        let unit_name_str = format!("{}: {}", unit_id, gd.stat_txt_tbl[unit_id].to_owned());
        let flingy_id = gd.units_dat.flingy_id[unit_id];
        let sprite_id = gd.flingy_dat.sprite_id[flingy_id as usize];
        let image_id = gd.sprites_dat.image_id[sprite_id as usize];

        // FIXME: move this to some generic initialization function
        let pal = gd.install_pal.to_sdl();
        gc.screen.set_palette(&pal).ok();

        UnitsView {
            unit_id: unit_id,
            current_anim: current_anim,
            next_anim_idx: 1,
            anim_str: anim_str,
            unit_name_str: unit_name_str,
            img: SCImage::new(&gd, image_id),
        }
    }
}
impl View for UnitsView {
    fn render(&mut self, context: &mut GameContext, elapsed: f64) -> ViewAction {
        if context.events.now.quit || context.events.now.key_escape == Some(true) {
            return ViewAction::Quit;
        }
        // clear the screen
        context.screen.fill_rect(None, Color::RGB(0,0,120)).ok();
        let gd = &context.gd;
        if context.events.now.key_n == Some(true) {
            self.unit_id += 1;

            self.unit_name_str = format!("{}: {}", self.unit_id,
                                         gd.stat_txt_tbl[self.unit_id].to_owned());
            let flingy_id = gd.units_dat.flingy_id[self.unit_id];
            let sprite_id = gd.flingy_dat.sprite_id[flingy_id as usize];
            let image_id = gd.sprites_dat.image_id[sprite_id as usize];
            self.img = SCImage::new(&gd, image_id);
        } else if context.events.now.key_p == Some(true) {
            if self.unit_id > 0 {
                self.unit_id -= 1;

                self.unit_name_str = format!("{}: {}", self.unit_id,
                                             gd.stat_txt_tbl[self.unit_id].to_owned());
                let flingy_id = gd.units_dat.flingy_id[self.unit_id];
                let sprite_id = gd.flingy_dat.sprite_id[flingy_id as usize];
                let image_id = gd.sprites_dat.image_id[sprite_id as usize];
                self.img = SCImage::new(&gd, image_id);
            }
        }
        if context.events.now.key_q == Some(true) {
            self.img.iscript_state.turn_ccwise(1);
        } else if context.events.now.key_e == Some(true) {
            self.img.iscript_state.turn_cwise(1);
        }
        if context.events.now.key_d == Some(true) {
            self.img.iscript_state.set_animation(AnimationType::Death);
        } else if context.events.now.key_w == Some(true) {
            self.img.iscript_state.set_animation(AnimationType::Walking);
        }

        // FIXME: reduce cloning
        let gd = context.gd.clone();
        {
            self.img.step(&gd);
            {
                let anim = self.img.iscript_state.current_animation();
                if anim != self.current_anim {
                    println!("--- current animation: {:?} ---", anim);
                    self.current_anim = anim;
                    self.anim_str = format!("Current Animation: {:?}", self.current_anim);
                }
            }
        }

        let fnt = gd.font(FontSize::Font16);
        let screen_pitch = context.screen.pitch();
        let fnt_reindex = &gd.font_reindex.data;

        let unitname_rect = Rect::new(10, 10, 300, 50);
        let animstr_rect = Rect::new(10, 300, 300, 50);

        context.screen.with_lock_mut(|buffer: &mut [u8]| {
            // unit name
            fnt.render_textbox(self.unit_name_str.as_ref(),
                               1,
                               fnt_reindex,
                               buffer,
                               screen_pitch,
                               &unitname_rect);
            // animation
            fnt.render_textbox(self.anim_str.as_ref(),
                               0,
                               fnt_reindex,
                               buffer,
                               screen_pitch,
                               &animstr_rect);

            // unit
            self.img.draw(100, 100, buffer, screen_pitch);

        });

        ViewAction::None
    }
}

fn main() {
    ::read_pcx::spawn("font rendering", "/home/dm/.wine/drive_c/StarCraft/", |gc| {
        Box::new(UnitsView::new(gc, 50))
    });
}
