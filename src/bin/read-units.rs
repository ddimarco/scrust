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
        $code_var:ident,
        $( $opcode:pat => ( $( $param:ident : $tpe:ident),*)
           $code:block),*
    )
        =>
    {
            match $code_var {
                $(
                    $opcode => {
                        $(
                            let $param = var_read!($tpe, $self_var).unwrap();
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

    /// signals the parent to create a new entity
    create_entity_action: Option<CreateIScriptEntity>,
}

impl Read for IScriptState {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        for i in 0..buf.len() {
            if self.pos as usize > self.gd.iscript.data.len() {
                return Ok(i);
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
            println!("header:");
            for anim_idx in 0..iscript_anim_offsets.len() {
                let anim = AnimationType::from_usize(anim_idx).unwrap();
                let pos = iscript_anim_offsets[anim_idx];
                println!("{:?}: {}", anim, pos);
            }
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

        let val = self.read_u8().unwrap();
        let opcode = OpCode::from_u8(val).unwrap();

        def_opcodes! (
        self, opcode,
        OpCode::ImgUl => (image_id: u16, rel_x: u8, rel_y: u8) {
        // shadows and such; img* is associated with the current entity
            println!("imgul: {}, {}, {}", image_id, rel_x, rel_y);
            self.create_entity_action = Some(CreateIScriptEntity::ImageUnderlay {
                image_id: image_id,
                rel_x: rel_x,
                rel_y: rel_y,
            });
        // FIXME
        },
        OpCode::ImgOl => (image_id: u16, rel_x: u8, rel_y: u8) {
        // e.g. explosions on death
            println!("imgul: {}, {}, {}", image_id, rel_x, rel_y);
            self.create_entity_action = Some(CreateIScriptEntity::ImageOverlay {
                image_id: image_id,
                rel_x: rel_x,
                rel_y: rel_y,
            });
        // FIXME
        },
        OpCode::SprOl => (sprite_id: u16, rel_x: u8, rel_y: u8) {
        // independent overlay, e.g. scanner sweep
            println!("sprol: {}, {}, {}", sprite_id, rel_x, rel_y);
        // FIXME
        },
        OpCode::LowSprUl => (sprite_id: u16, rel_x: u8, rel_y: u8) {
        // independent underlay, e.g. gore
            println!("lowsprul: {}, {}, {}", sprite_id, rel_x, rel_y);
        // FIXME
        },
        OpCode::PlayFram => (frame: u16) {
            println!("playfram: {}", frame);
            self.frameset = frame;
        },
        OpCode::PlayFramTile => (frame: u16) {
            println!("playframtile: {}", frame);
        // FIXME
        },

        OpCode::FollowMainGraphic => () {
            println!("followmaingraphic");
            assert!(parent.is_some());

            self.direction = parent.unwrap().direction;
            self.frameset = parent.unwrap().frameset;
        },

        OpCode::Wait => (ticks: u8) {
            println!("wait: {}", ticks);
            self.waiting_ticks_left += ticks as usize;
        },
        OpCode::WaitRand => (minticks: u8, maxticks: u8) {
            println!("waitrand: {}, {}", minticks, maxticks);
            let r = rand::thread_rng().gen_range(minticks, maxticks+1);
            println!(" -> {}", r);
            self.waiting_ticks_left += r as usize;
        },
        OpCode::SigOrder => (signal: u8) {
            println!("sigorder: {}", signal);
        // FIXME
        },
        OpCode::Goto => (target: u16) {
            println!("goto: {}", target);
            self.pos = target;
        },
        OpCode::RandCondJmp => (val: u8, target: u16) {
            println!("randcondjmp: {}, {}", val, target);
            let r = rand::random::<u8>();
            if r < val {
                println!(" jumping!");
                self.pos = target;
            }
        },
        OpCode::TurnRand => (units: u8) {
            println!("turnrand: {}", units);
            if rand::thread_rng().gen_range(0, 100) < 50 {
                self.turn_cwise(units);
            } else {
                self.turn_ccwise(units);
            }
        },
        OpCode::TurnCWise => (units: u8) {
            println!("turnccwise.: {}", units);
            self.turn_cwise(units);
        },
        OpCode::TurnCCWise => (units: u8) {
            println!("turnccwise: {}", units);
            self.turn_ccwise(units);
        },
        OpCode::SetFlDirect => (dir: u8) {
            println!("setfldirect: {}", dir);
            self.set_direction(dir);
        },
        // FIXME: might be signed bytes?
        OpCode::SetVertPos => (val: u8) {
            println!("setvertpos: {}", val);
            self.rel_y = val;
        },
        OpCode::SetHorPos => (val: u8) {
            println!("sethorpos: {}", val);
            self.rel_x = val;
        },

        // FIXME sounds
        OpCode::PlaySndBtwn => (val1: u16, val2: u16) {
            println!("playsndbtwn: {}, {}", val1, val2);
        },
        OpCode::PlaySnd => (sound_id: u16) {
            println!("playsnd: {}", sound_id);
        },

        OpCode::NoBrkCodeStart => () {
            println!("nobrkcodestart");
        // FIXME
        },
        OpCode::NoBrkCodeEnd => () {
            println!("nobrkcodeend");
        // FIXME
        },
        OpCode::Attack => () {
            println!("attack");
        // FIXME
        },
        OpCode::AttackWith => (weapon: u8) {
            println!("attackwith: {}", weapon);
        // FIXME
        },
        OpCode::GotoRepeatAttk => () {
        // Signals to StarCraft that after this point, when the unit's cooldown time
        // is over, the repeat attack animation can be called.
            println!("gotorepeatattack");
        // FIXME
        },
        OpCode::IgnoreRest => () {
        // this causes the script to stop until the next animation is called.
            println!("ignorerest");
        // FIXME
        },

        OpCode::End => () {
            println!("end");
        // FIXME
        }

    );

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
        if context.events.now.key_a == Some(true) {
            // change animation
            self.img.iscript_state
                .set_animation(AnimationType::from_usize(self.next_anim_idx).unwrap());
            loop {
                self.next_anim_idx += 1;
                self.next_anim_idx = self.next_anim_idx % self.img.iscript_state.anim_count();
                if self.img.iscript_state
                    .is_animation_valid(AnimationType::from_usize(self.next_anim_idx)
                                        .unwrap()) {
                        break;
                    }
            }
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
        Box::new(UnitsView::new(gc, 8))
    });

    /*
    println!("opening mpq...");
    let gd = GameData::init(&Path::new("/home/dm/.wine/drive_c/StarCraft/"));

    println!("grp_file len: {}", gd.images_dat.grp_id.len());
    println!("graphics len: {}", gd.units_dat.flingy_id.len());
    println!("image_file len: {}", gd.sprites_dat.image_id.len());
    println!("sprite len: {}", gd.flingy_dat.sprite_id.len());

    let mut unit_id = 0;
    let flingy_id = gd.units_dat.flingy_id[unit_id];
    let sprite_id = gd.flingy_dat.sprite_id[flingy_id as usize];
    let image_id = gd.sprites_dat.image_id[sprite_id as usize];
    println!("unit id: {}, flingy id: {}, sprite id: {}, image id: {}",
             unit_id,
             flingy_id,
             sprite_id,
             image_id);

    // let iscript = IScript::read(&mut gd.open("scripts/iscript.bin").unwrap());

    // sdl
    let sdl_context = sdl2::init().unwrap();
    let mut timer = sdl_context.timer().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("iscript test", 640, 480)
        //.position_centered()
        .opengl()
        .build()
        .unwrap();

    let mut renderer = window.renderer().build().unwrap();

    // FIXME: scimage should not require renderer
    let mut img = SCImage::new(&gd, &mut renderer, image_id);

    let mut event_pump = sdl_context.event_pump().unwrap();
    let interval = 1_000 / 60;
    let mut before = timer.ticks();
    let mut last_second = timer.ticks();
    let mut fps = 0u16;

    let mut current_anim = AnimationType::Init;
    let mut next_anim_idx = 1;

    // labels
    let fnt = gd.font(FontSize::Font16);
    let mut anim_texture = fnt.render_textbox(&format!("Current Animation: {:?}", current_anim),
                                              0,
                                              &mut renderer,
                                              &gd.fontmm_reindex.palette,
                                              &gd.fontmm_reindex.data,
                                              300,
                                              50);
    let mut unit_name_texture =
        fnt.render_textbox(&format!("Unit: {}", gd.stat_txt_tbl[unit_id as usize]),
                           1,
                           &mut renderer,
                           &gd.fontmm_reindex.palette,
                           &gd.fontmm_reindex.data,
                           300,
                           50);

    //img.iscript_state.set_animation(AnimationType::Walking);

    'running: loop {
        // FIXME: encapsulate all this (in a view?)
        let now = timer.ticks();
        let dt = now - before;
        // let elapsed = dt as f64 / 1_000.0;

        if dt < interval {
            timer.delay(interval - dt);
            continue;
        }

        before = now;
        fps += 1;

        if now - last_second > 1_000 {
            println!("FPS: {}", fps);
            last_second = now;
            fps = 0;
        }


        img.step(&gd, &mut renderer);
        {
            let anim = img.iscript_state.current_animation();
            if anim != current_anim {
                println!("--- current animation: {:?} ---", anim);
                current_anim = anim;
                anim_texture = fnt.render_textbox(&format!("Current Animation: {:?}", current_anim),
                                    0,
                                    &mut renderer,
                                    &gd.fontmm_reindex.palette,
                                    &gd.fontmm_reindex.data,
                                    300,
                                    50);
            }
        }

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown { keycode: Some(keycode), .. } => {
                    match keycode {
                        Keycode::Escape => break 'running,
                        Keycode::Q => {
                            // turn cc
                            img.iscript_state.turn_ccwise(1);
                        }
                        Keycode::E => {
                            // turn c
                            img.iscript_state.turn_cwise(1);
                        }
                        Keycode::A => {
                            // change animation
                            img.iscript_state
                                .set_animation(AnimationType::from_usize(next_anim_idx).unwrap());
                            loop {
                                next_anim_idx += 1;
                                next_anim_idx = next_anim_idx % img.iscript_state.anim_count();
                                if img.iscript_state
                                    .is_animation_valid(AnimationType::from_usize(next_anim_idx)
                                        .unwrap()) {
                                    break;
                                }
                            }
                        }
                        Keycode::N => {
                            // next unit
                            unit_id = unit_id + 1;
                            println!("unit: {}", unit_id);
                            let flingy_id = gd.units_dat.flingy_id[unit_id];
                            let sprite_id = gd.flingy_dat.sprite_id[flingy_id as usize];
                            let image_id = gd.sprites_dat.image_id[sprite_id as usize];
                            img = SCImage::new(&gd, &mut renderer, image_id);

                            //img.iscript_state.set_animation(AnimationType::Walking);
                            unit_name_texture = fnt.render_textbox(&format!("Unit: {}",
                                                         gd.stat_txt_tbl[unit_id as usize]),
                                                                   1,
                                                                   &mut renderer,
                                                                   &gd.fontmm_reindex.palette,
                                                                   &gd.fontmm_reindex.data,
                                                                   300,
                                                                   50);
                            next_anim_idx = 0;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        renderer.set_draw_color(Color::RGB(0,0,0));
        renderer.clear();
        img.render(100, 100, &gd.install_pal, &gd, &mut renderer);
        renderer.copy(&anim_texture, None, Some(Rect::new(100, 300, 300, 50)));
        renderer.copy(&unit_name_texture, None, Some(Rect::new(100, 10, 300, 50)));
        renderer.present();
    }
    */
}
