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
use sdl2::pixels::PixelFormatEnum;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;
use sdl2::render::{Renderer, Texture};

extern crate read_pcx;
use read_pcx::gamedata::GameData;
use read_pcx::iscript::{AnimationType, OpCode};
use read_pcx::grp::GRP;
use read_pcx::pal::Palette;
use read_pcx::unitsdata::ImagesDat;

use read_pcx::font::FontSize;
use read_pcx::font::RenderText;


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

struct IScriptState<'iscript> {
    pub iscript_id: u32,
    /// current position in iscript
    pub pos: u16,
    /// reference to iscript animation offsets
    iscript_anim_offsets: &'iscript Vec<u16>,
    // FIXME: class wide instance would be enough
    /// reference to iscript buffer
    iscript_data: &'iscript Vec<u8>,
    images_dat: &'iscript ImagesDat,

    waiting_ticks_left: usize,
    rel_x: u8,
    rel_y: u8,
    direction: u8,
    frameset: u16,

    /// signals the parent to create a new entity
    create_entity_action: Option<CreateIScriptEntity>,
}

// trait IScriptable<'a> {
//     fn state(&'a mut self) -> &'a mut IScriptState;
//     fn step(&'a mut self) {
//         self.state().interpret_iscript();
//     }
// }

/**
iscript needs to
  - create scimages over/underlays (& add to parent)
  - create sprites over/underlays (& add to global layer)
  - signal end of life
**/


impl<'iscript> Read for IScriptState<'iscript> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        for i in 0..buf.len() {
            if self.pos as usize > self.iscript_data.len() {
                return Ok(i);
            }
            buf[i] = self.iscript_data[self.pos as usize];
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

impl<'iscript> IScriptState<'iscript> {
    pub fn new(gd: &'iscript GameData, iscript_id: u32) -> IScriptState<'iscript> {
        let ref iscript_anim_offsets = gd.iscript.id_offsets_map.get(&iscript_id).unwrap();

        println!("header:");
        for anim_idx in 0..iscript_anim_offsets.len() {
            let anim = AnimationType::from_usize(anim_idx).unwrap();
            let pos = iscript_anim_offsets[anim_idx];
            println!("{:?}: {}", anim, pos);
        }
        let start_pos = iscript_anim_offsets[AnimationType::Init as usize];
        IScriptState {
            iscript_id: iscript_id,
            pos: start_pos,
            iscript_anim_offsets: iscript_anim_offsets,
            iscript_data: &gd.iscript.data,
            images_dat: &gd.images_dat,
            waiting_ticks_left: 0,
            rel_x: 0,
            rel_y: 0,
            frameset: 0,
            direction: 0,
            create_entity_action: None,
        }
    }

    pub fn set_animation(&mut self, anim: AnimationType) {
        self.pos = self.iscript_anim_offsets[anim as usize];
    }
    pub fn is_animation_valid(&self, anim: AnimationType) -> bool {
        self.iscript_anim_offsets[anim as usize] > 0
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
        for lbl_idx in 0..self.iscript_anim_offsets.len() {
            let lbl_pos = self.iscript_anim_offsets[lbl_idx];
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
        self.iscript_anim_offsets.len()
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
struct SCImage<'iscript> {
    pub image_id: u16,

    // FIXME: avoid copying
    grp: GRP,

    texture: Texture,

    iscript_state: IScriptState<'iscript>,

    underlays: Vec<SCImage<'iscript>>,
    overlays: Vec<SCImage<'iscript>>,

    prerender_buffer: Vec<u8>,
}

impl<'gamedata> SCImage<'gamedata> {
    pub fn new(gd: &'gamedata GameData,
               renderer: &mut Renderer,
               image_id: u16)
               -> SCImage<'gamedata> {
        // image id -> iscript id:
        let iscript_id = gd.images_dat.iscript_id[image_id as usize];

        let grp_id = gd.images_dat.grp_id[image_id as usize];
        let name = "unit\\".to_string() + &gd.images_tbl[(grp_id as usize) - 1];
        println!("grp id: {}, filename: {}", grp_id, name);
        let grp = GRP::read(&mut gd.open(&name).unwrap());


        // FIXME: need to increase the texture size
        let (w, h) = //(320, 240);
            (grp.header.width as u32, grp.header.height as u32);
        let texture = renderer.create_texture_streaming(PixelFormatEnum::RGB24,
                                      //(grp.header.width + increase) as u32,
                                                        //(grp.header.height + increase) as u32
                                                        w, h
        )
            .unwrap();
        //let ds = (grp.header.width + increase) as usize * (grp.header.height + increase) as usize;
        let ds = (w as usize * h as usize);
        SCImage {
            image_id: image_id,
            grp: grp,
            texture: texture,
            iscript_state: IScriptState::new(gd, iscript_id),
            underlays: Vec::<SCImage>::new(),
            overlays: Vec::<SCImage>::new(),
            prerender_buffer: vec![0; ds],
        }
    }

    fn can_turn(&self) -> bool {
        (self.iscript_state.images_dat.graphic_turns[self.image_id as usize] > 0)
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
            _ => {panic!("unknown remapping enum: {:?}", remap);}
        }
    }


    // FIXME: expensive, try:
    // - only redraw when necessary
    // - cache & combine textures
    // - see http://stackoverflow.com/questions/12506979/what-is-the-point-of-an-sdl2-texture
    pub fn render(&mut self,
                  x: i32,
                  y: i32,
                  pal: &Palette,
                  gd: &GameData,
                  //reindexing_table: Option<&[u8]>,
                  renderer: &mut Renderer) {
        let (w, h) = (self.grp.header.width as u32, self.grp.header.height as u32);
        let frame = self.frame_idx();
        // TODO: render over/underlays
        // FIXME: cache this
        //let ref data = self.grp.frames[frame];

        let underlays = &self.underlays;
        let overlays = &self.overlays;
        {
            let tq = self.texture.query();
            // println!("size: {} x {} = {}", tq.width, tq.height, data.len());
            //assert!(tq.width as usize * tq.height as usize == data.len());

            // first render to prerender_buffer
            for b in &mut self.prerender_buffer[..] {
                *b = 253;
            }
                // draw shadows (with fixed color)
                for ul in underlays {
                    let uf = ul.frame_idx();
                    let udata = &ul.grp.frames[uf];
                    // calculate offset: centerpoints should match
                    let x_offset = tq.width / 2 - ul.grp.header.width as u32 / 2;
                    let y_offset = tq.height / 2 - ul.grp.header.height as u32 / 2;
                    for (i, col) in udata.iter().enumerate() {
                        if *col != 0 {
                            // make more efficient?
                            let y = i / (ul.grp.header.width as usize);
                            let x = i % (ul.grp.header.width as usize);
                            let idx = (y as usize + y_offset as usize) * tq.width as usize +
                                (x as usize + x_offset as usize);
                            self.prerender_buffer[idx] = *col;
                        }
                    }
                }

            // draw main image
            let x_offset = (tq.width / 2) - (self.grp.header.width as u32 / 2);
            let y_offset = (tq.height / 2) - (self.grp.header.height as u32 / 2);
            for (i, col) in self.grp.frames[frame].iter().enumerate() {
                let y = i / (self.grp.header.width as usize);
                let x = i % (self.grp.header.width as usize);
                let idx = (y as usize + y_offset as usize) * tq.width as usize +
                    (x as usize + x_offset as usize);
                if *col != 0 {
                    self.prerender_buffer[idx] = *col;
                }
            }

            for ol in overlays {
                // get proper reindexing table
                let remap = ol.remapping(gd);
                let reindex = SCImage::get_reindexing_table(gd, remap);

                let of = ol.frame_idx();
                let odata = &ol.grp.frames[of];
                let x_offset = (tq.width / 2) - (ol.grp.header.width as u32 / 2);
                let y_offset = (tq.height / 2) - (ol.grp.header.height as u32 / 2);
                for (i, col) in odata.iter().enumerate() {
                    if *col == 0 {
                        continue;
                    }
                    // make more efficient?
                    let y = i / (ol.grp.header.width as usize);
                    let x = i % (ol.grp.header.width as usize);
                    let idx = (y as usize + y_offset as usize) * tq.width as usize +
                        (x as usize + x_offset as usize);

                    let src = self.prerender_buffer[idx] as usize;
                    let dest = *col as usize;
                    assert!(dest < 64);
                    // reindexing: already there: y, overlay color: x
                    let color = reindex[(dest -1) * 255 + src];
                    println!("src: {}, dest: {}, res: {}", src, dest, color);
                    self.prerender_buffer[idx] = color;
                }
            }
            let prerender = &self.prerender_buffer;

            self.texture.with_lock(None, |buffer: &mut [u8], _: usize| {
                for (i, col) in prerender.iter().enumerate() {
                    let color = (*col as usize) * 3;
                    if color != 0 {
                        buffer[i * 3 + 0] = pal.data[color + 0];
                        buffer[i * 3 + 1] = pal.data[color + 1];
                        buffer[i * 3 + 2] = pal.data[color + 2];
                    }
                }
                })
                .ok();
        }

        let iscript_state = &self.iscript_state;

        let rect = Rect::new(x + iscript_state.rel_x as i32,
                             y + iscript_state.rel_y as i32,
                             w,
                             h);
        // TODO: rel_x/y should probably done in the texture already
        renderer.copy_ex(&self.texture,
                     None,
                     Some(rect),
                     0.,
                     None,
                     self.draw_flipped(),
                     false)
            .ok();
        renderer.set_draw_color(Color::RGB(255, 255, 255));
        renderer.draw_rect(rect).ok();
    }

    pub fn step(&mut self,
                // these params are just for creating new entities
                gd: &'gamedata GameData, renderer: &mut Renderer) {

        for ul in &mut self.underlays {
            ul.iscript_state.interpret_iscript(Some(&self.iscript_state));
            // assuming they do not create additional under/overlays
        }

        self.iscript_state.interpret_iscript(None);

        // create additional entities if necessary
        match self.iscript_state.create_entity_action {
            Some(CreateIScriptEntity::ImageUnderlay {image_id, rel_x, rel_y}) => {
                println!("creating underlay");

                let mut underlay = SCImage::new(gd, renderer, image_id);
                underlay.iscript_state.rel_x = rel_x;
                underlay.iscript_state.rel_y = rel_y;
                self.underlays.push(underlay);

                self.iscript_state.create_entity_action = None;
            },
            Some(CreateIScriptEntity::ImageOverlay {image_id, rel_x, rel_y}) => {
                println!("create overlay");
                let mut overlay = SCImage::new(gd, renderer, image_id);
                overlay.iscript_state.rel_x = rel_x;
                overlay.iscript_state.rel_y = rel_y;
                self.overlays.push(overlay);

                self.iscript_state.create_entity_action = None;
            },
            _ => {},
        }
    }
}



fn main() {
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

}
