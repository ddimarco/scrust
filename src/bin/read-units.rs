use std::path::Path;

use std::io::Read;

extern crate byteorder;
use byteorder::{ReadBytesExt, LittleEndian};

extern crate num;
use num::FromPrimitive;

extern crate rand;
use rand::Rng;

extern crate read_pcx;
use read_pcx::gamedata::GameData;
use read_pcx::iscript::{IScript, AnimationType, OpCode};



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

struct SCImage<'iscript> {
    pub image_id: u16,
    // current position in iscript
    pub pos: u16,
    // reference to iscript animation offsets
    iscript_anim_offsets: &'iscript Vec<u16>,
    // reference to iscript buffer
    iscript_data: &'iscript Vec<u8>,
}

impl<'iscript> Read for SCImage<'iscript> {
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

impl<'iscript> SCImage<'iscript> {
    pub fn new(gd: &GameData, image_id: u16, iscript: &'iscript IScript) -> SCImage<'iscript> {
        // image id -> iscript id:
        let iscript_id = gd.images_dat.iscript_id[image_id as usize];
        let ref iscript_anim_offsets = iscript.id_offsets_map.get(&iscript_id).unwrap();

        println!("header:");
        for anim_idx in 0..iscript_anim_offsets.len() {
            let anim = AnimationType::from_usize(anim_idx).unwrap();
            let pos = iscript_anim_offsets[anim_idx];
            println!("{:?}: {}", anim, pos);
        }

        let start_pos = iscript_anim_offsets[AnimationType::Init as usize];
        SCImage {
            pos: start_pos,
            image_id: image_id,
            iscript_anim_offsets: iscript_anim_offsets,
            iscript_data: &iscript.data,
        }
    }

    pub fn set_animation(&mut self, anim: AnimationType) {
        self.pos = self.iscript_anim_offsets[anim as usize];
        println!("new iscript pos: {}", self.pos);
    }

    pub fn current_animation(&self) -> AnimationType {
        let mut nearest_label = AnimationType::Init;
        let mut nearest_dist = 10000;
        for lbl_idx in 0..self.iscript_anim_offsets.len() {
            let lbl_pos = self.iscript_anim_offsets[lbl_idx];
            //println!("checking label: {} - {:?}", lbl_pos, AnimationType::from_usize(lbl_idx).unwrap());
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

    // TODO: move into trait
    // TODO: join methods
    pub fn interpret_iscript(&mut self) {
        let val = self.read_u8().unwrap();
        let opcode = OpCode::from_u8(val).unwrap();

    def_opcodes! (
        self, opcode,
        OpCode::ImgUl => (image_id: u16, rel_x: u8, rel_y: u8) {
            println!("imgul: {}, {}, {}", image_id, rel_x, rel_y);
        },
        OpCode::ImgOl => (image_id: u16, rel_x: u8, rel_y: u8) {
            println!("imgul: {}, {}, {}", image_id, rel_x, rel_y);
        },
        OpCode::SprOl => (sprite_id: u16, rel_x: u8, rel_y: u8) {
            println!("sprol: {}, {}, {}", sprite_id, rel_x, rel_y);
        },
        OpCode::PlayFram => (frame: u16) {
            println!("playfram: {}", frame);
        },
        OpCode::PlayFramTile => (frame: u16) {
            println!("playframtile: {}", frame);
        },
        OpCode::Wait => (ticks: u8) {
            println!("wait: {}", ticks);
        },
        OpCode::WaitRand => (minticks: u8, maxticks: u8) {
            println!("waitrand: {}, {}", minticks, maxticks);
        },
        OpCode::SigOrder => (signal: u8) {
            println!("sigorder: {}", signal);
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
        },

        // FIXME sounds
        OpCode::PlaySndBtwn => (val1: u16, val2: u16) {
            println!("playsndbtwn: {}, {}", val1, val2);
        },
        OpCode::PlaySnd => (sound_id: u16) {
            println!("playsnd: {}", sound_id);
        }
    );

    }
}



fn main() {
    println!("opening mpq...");
    let mut gd = GameData::init(&Path::new("/home/dm/.wine/drive_c/StarCraft/"));

    println!("grp_file len: {}", gd.images_dat.grp_id.len());
    println!("graphics len: {}", gd.units_dat.flingy_id.len());
    println!("image_file len: {}", gd.sprites_dat.image_id.len());
    println!("sprite len: {}", gd.flingy_dat.sprite_id.len());

    let unit_id = 0;
    let flingy_id = gd.units_dat.flingy_id[unit_id];
    let sprite_id = gd.flingy_dat.sprite_id[flingy_id as usize];
    let image_id = gd.sprites_dat.image_id[sprite_id as usize];
    let grp_id = gd.images_dat.grp_id[image_id as usize];
    println!("unit id: {}, flingy id: {}, sprite id: {}, image id: {}",
             unit_id, flingy_id, sprite_id, image_id);
    gd.grp(grp_id);

    let iscript = IScript::read(&mut gd.open("scripts/iscript.bin").unwrap());
    let mut img = SCImage::new(&gd, image_id, &iscript);

    //img.set_animation(AnimationType::Death);
    for _ in 0..20 {
        println!("pos: {}, nearest label: {:?}", img.pos, img.current_animation());
        img.interpret_iscript();
    }


}
