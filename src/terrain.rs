use std::io::{Read, Seek, SeekFrom};

extern crate byteorder;
use byteorder::{LittleEndian, ReadBytesExt};

use ::utils::{read_u8buf, read_u16buf};
use ::pal::Palette;
use ::gamedata::{TileSet, GameData};

// XXX can't use def_bin_struct with arrays
pub struct CV5 {
    pub index: u16,
    pub buildability: u8,
    pub ground_height: u8,
    // left_edge: u16,
    // top_edge: u16,
    // right_edge: u16,
    // bottom_edge: u16,
    // dd_databin_idx: u16,
    // dd_width: u16,
    // dd_height: u16,
    //_unused: u16,
    pub mega_tiles: [u16; 16]
}
impl CV5 {
    fn read<T: Read + Seek>(infile: &mut T) -> Option<CV5> {
        let index_option = infile.read_u16::<LittleEndian>();
        match index_option {
            Ok(index) => {
                let buildability = infile.read_u8().unwrap();
                let ground_height = infile.read_u8().unwrap();
                infile.seek(SeekFrom::Current(16)).ok();
                let mut mega_tiles = [0 as u16; 16];
                read_u16buf(infile, 16, &mut mega_tiles);
                Some(CV5{
                    index: index,
                    buildability: buildability,
                    ground_height: ground_height,
                    mega_tiles: mega_tiles,
                })
            },
            Err(_) => {
                None
            }
        }
    }

}

struct Doodad {
    pub index: u16,
    pub buildability: u8,
    pub ground_height: u8,
    pub overlay_id: u16,
    // _unused: u16
    pub group_string_id: u16,
    // _unused: u16
    pub dddata_idx: u16,
    pub width: u16,
    pub height: u16,
    // _unused: u16
    pub mega_tiles: [u16; 16],
}
impl Doodad {
    pub fn read<T: Read + Seek>(infile: &mut T) -> Option<Doodad> {
        let index_option = infile.read_u16::<LittleEndian>();
        match index_option {
            Ok(index) => {
                let buildability = infile.read_u8().unwrap();
                let ground_height = infile.read_u8().unwrap();
                let overlay_id = infile.read_u16::<LittleEndian>().unwrap();
                infile.seek(SeekFrom::Current(2)).ok();
                let group_string_id = infile.read_u16::<LittleEndian>().unwrap();
                infile.seek(SeekFrom::Current(2)).ok();
                let dddata_idx = infile.read_u16::<LittleEndian>().unwrap();
                let width = infile.read_u16::<LittleEndian>().unwrap();
                let height = infile.read_u16::<LittleEndian>().unwrap();
                let mut mega_tiles = [0 as u16; 16];
                infile.seek(SeekFrom::Current(2)).ok();
                read_u16buf(infile, 16, &mut mega_tiles);
                Some(Doodad {
                    index: index,
                    buildability: buildability,
                    ground_height: ground_height,
                    overlay_id: overlay_id,
                    group_string_id: group_string_id,
                    dddata_idx: dddata_idx,
                    width: width,
                    height: height,
                    mega_tiles: mega_tiles,
                })
            },
            Err(_) => {
                None
            }
        }
    }
}

struct VX4 {
    data: [u16; 16],
}
impl VX4 {
    pub fn read(infile: &mut Read) -> Option<VX4> {
        let mut data = [0 as u16; 16];
        for i in 0..16 {
            let val = infile.read_u16::<LittleEndian>();
            match val {
                Ok(v) => {data[i] = v;},
                Err(_) => { return None;},
            }
        }
        Some(VX4 {
            data: data,
        })
    }
}

struct VR4 {
    bitmap: [u8; 64],
}
impl VR4 {
    pub fn read(infile: &mut Read) -> Option<VR4> {
        let mut data = [0 as u8; 64];
        for i in 0..64 {
            let val = infile.read_u8();
            match val {
                Ok(v) => {data[i] = v;},
                Err(_) => { return None;},
            }
        }
        Some(VR4 {
            bitmap: data,
        })
    }
}
struct VF4 {
    flags: [u16; 16],
}
impl VF4 {
    pub fn read(infile: &mut Read) -> Option<VF4> {
        let mut data = [0 as u16; 16];
        //read_u16buf(infile, 16, &mut data);
        for i in 0..16 {
            let val = infile.read_u16::<LittleEndian>();
            match val {
                Ok(v) => {data[i] = v;},
                Err(_) => { return None;},
            }
        }
        Some(VF4 {
            flags: data,
        })
    }
}

fn make_tileset_filename(tileset: TileSet, ending: &str) -> String {
    format!("tileset/{}{}",
            match tileset {
                TileSet::Badlands => "badlands",
                TileSet::SpacePlatform => "platform",
                TileSet::Installation => "install",
                TileSet::Ashworld => "ashworld",
                TileSet::Jungle => "jungle",
                TileSet::Desert => "desert",
                TileSet::Arctic => "ice",
                TileSet::Twilight => "twilight",
            },
            ending)
}

pub struct TerrainInfo {
    pal: Palette,
    cv5: Vec<CV5>,
    doodads: Vec<Doodad>,
    vx4: Vec<VX4>,
    vr4: Vec<VR4>,
    vf4: Vec<VF4>,
}
impl TerrainInfo {
    pub fn read(gd: &GameData, tileset: TileSet) -> TerrainInfo {
        let pal = Palette::read_wpe(&mut gd.open(make_tileset_filename(tileset, ".wpe")
                                                 .as_str())
                                    .unwrap());
        let mut cv5 = Vec::<CV5>::new();
        let mut doodads = Vec::<Doodad>::new();
        {
            let mut infile = gd.open(make_tileset_filename(tileset, ".cv5").as_str()).unwrap();
            while let Some(cv5_entry) = CV5::read(&mut infile) {
                cv5.push(cv5_entry);
                if cv5.len() >= 1024 {
                    break;
                }
            }
            while let Some(dd_entry) = Doodad::read(&mut infile) {
                doodads.push(dd_entry);
            }

        }
        let mut vx4 = Vec::<VX4>::new();
        {
            let mut infile = gd.open(make_tileset_filename(tileset, ".vx4").as_str()).unwrap();
            while let Some(vx4_entry) = VX4::read(&mut infile) {
                vx4.push(vx4_entry);
            }
        }
        let mut vr4 = Vec::<VR4>::new();
        {
            let mut infile = gd.open(make_tileset_filename(tileset, ".vr4").as_str()).unwrap();
            while let Some(vr4_entry) = VR4::read(&mut infile) {
                vr4.push(vr4_entry);
            }
        }
        let mut vf4 = Vec::<VF4>::new();
        {
            let mut infile = gd.open(make_tileset_filename(tileset, ".vf4").as_str()).unwrap();
            while let Some(vf4_entry) = VF4::read(&mut infile) {
                vf4.push(vf4_entry);
            }
        }
        TerrainInfo {
            pal: pal,
            cv5: cv5,
            doodads: doodads,
            vx4: vx4,
            vr4: vr4,
            vf4: vf4,
        }
    }
}
