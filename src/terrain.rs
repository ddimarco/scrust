use std::io::{Read, Seek, SeekFrom};
use std::cmp;

extern crate byteorder;
use byteorder::{LittleEndian, ReadBytesExt};

use ::utils::{read_u8buf, read_u16buf};
use ::pal::Palette;
use ::gamedata::{TileSet, GameData};
use ::stormlib::{MPQArchive};

pub struct MapData {
    pub mpq_archive: MPQArchive,
    pub owners: [u8; 12],
    pub tileset: TileSet,
    pub width: u16,
    pub height: u16,
    pub mtxm: Vec<u16>,
    pub units: Vec<MapUnit>,
    pub sprites: Vec<MapSprite>,
    pub strings: Vec<String>,
    pub scenario_name_str_idx: usize,
    pub scenario_desc_str_idx: usize,
}
pub struct Map {
    pub data: MapData,
    pub terrain_info: TerrainInfo,
}

// XXX: almost same as in read-units, merge
macro_rules! var_read {
    (u8, $file:ident) => ($file.read_u8());
    (u16, $file:ident) => ($file.read_u16::<LittleEndian>());
    (u32, $file:ident) => ($file.read_u32::<LittleEndian>());
}
macro_rules! def_chk {
    (
        $file_var:ident,
        $debug:expr,
        $code_var:ident,
        $data_size:ident,
        $( $opcode:pat => ( $( $param:ident : $tpe:ident),*)
           $code:block),*
    )
        =>
    {

            match &$code_var as &str {
                $(
                    $opcode => {
                        if $debug {
                            println!("chk section: {:?}", $code_var);
                        }
                        $(
                            let $param = var_read!($tpe, $file_var).unwrap();
                            if $debug {
                                println!(" param: {}: {} = {}", stringify!($param),
                                         stringify!($tpe), $param);
                            }
                        )*
                            $code
                    }
                ),*
                //_ => panic!("unknown chk section: {:?}", $code_var),
                    _ => {
                        println!("ignoring section: {:?}", $code_var);
                        $file_var.seek(SeekFrom::Current($data_size as i64)).ok();
                    }
            }
    };
}


def_bin_struct! (
    MapUnit {
        // u32: The unit's class instance (sort of a "serial number")
        instance_id: u32,
        // U16: X coordinate of unit
        x: u16,
        // U16: Y coordinate of unit
        y: u16,
        // u16: Unit ID
        unit_id: u16,
        // u16: Type of relation to another building (i.e. add-on, nydus link)
        // Bit 9 - Nydus Link
        // Bit 10 - Addon Link
        rel_type: u16,
        // u16: Flags of special properties which can be applied to the unit and are valid:
        // Bit 0 - Cloak is valid
        // Bit 1 - Burrow is valid
        // Bit 2 - In transit is valid
        // Bit 3 - Hallucinated is valid
        // Bit 4 - Invincible is valid
        // Bit 5-15 - Unused
        special_prop_flags: u16,
        // u16: Out of the elements of the unit data, the properties which can
        // be changed by the map maker:
        // Bit 0 - Owner player is valid (the unit is not a critter, start
        // location, etc.; not a neutral unit)
        // Bit 1 - HP is valid
        // Bit 2 - Shields is valid
        // Bit 3 - Energy is valid (unit is a wraith, etc.)
        // Bit 4 - Resource amount is valid (unit is a mineral patch, vespene geyser, etc.)
        // Bit 5 - Amount in hangar is valid (unit is a reaver, carrier, etc.)
        // Bit 6-15 - Unused
        changeable_props: u16,
        // u8: Player number of owner (0-based)
        player_no: u8,
        // u8: Hit points % (1-100)
        hit_points: u8,
        // u8: Shield points % (1-100)
        shield_points: u8,
        // u8: Energy points % (1-100)
        energy_points: u8,
        // u32: Resource amount
        resource_amount: u32,
        // u16: Number of units in hangar
        units_in_hangar: u16,
        // u16: Unit state flags
        // Bit 0 - Unit is cloaked
        // Bit 1 - Unit is burrowed
        // Bit 2 - Building is in transit
        // Bit 3 - Unit is hallucinated
        // Bit 4 - Unit is invincible
        // Bit 5-15 - Unused
        state_flags: u16,
        _unused: u32,
        // u32: Class instance of the unit to which this unit is related to
        // (i.e. via an add-on, nydus link, etc.). It is "0" if the unit is not
        // linked to any other unit.
        rel_instance_id: u32
    }
);


def_bin_struct! (
    MapSprite {
        // u16: Unit/Sprite number of the sprite
        sprite_no: u16,
        // u16: X coordinate of the doodad unit
        x: u16,
        // u16: Y coordinate of the doodad unit
        y: u16,
        // u8: Player number that owns the doodad
        player_no: u8,
        // u8: Unused
        _unused: u8,
        // u16: Flags
        // Bit 0-11 - Unused
        // Bit 12 - Draw as sprite (Determines if it is a sprite or a unit sprite)
        // Bit 13 - Unused
        // Bit 14 - Unused
        // Bit 15 - Disabled (Only valid if Draw as sprite is unchecked, disables the unit)
        flags: u16
    }
);


impl MapData {
    fn read_section<T: Read + Seek>(&mut self, chk_file: &mut T) -> Option<usize> {
        // read section header
        let mut name_buf = [0 as u8; 4];
        let read_bytes = chk_file.read(&mut name_buf).unwrap();
        if read_bytes == 0 {
            return None;
        }
        let name_string = String::from_utf8_lossy(&name_buf);
        let size = chk_file.read_u32::<LittleEndian>().unwrap();
        println!("name: {}, size: {}", name_string, size);

        def_chk!(
            chk_file,
            true,
            name_string,
            size,
            "TYPE" => (maptype: u32) {
                let typestring = match maptype {
                    0x53574152 => String::from("StarCraft >= 1.04"),
                    0x42574152 => String::from("BroodWar"),
                    _ => format!("unknown map type: {}", maptype),
                };
                println!(" maptype: {}", typestring);
            },
            "VER " => (version: u16) {
                let verstring = match version {
                    59 => String::from("StarCraft 1.00"),
                    63 => String::from("StarCraft >= 1.04"),
                    205 => String::from("BroodWar"),
                    _ => format!("unknown version: {}", version),
                };
                println!(" version: {}", verstring);
            },
            "IVER" => (add_ver: u16) {
                let str =
                    match add_ver {
                        9 => String::from("beta/obsolete"),
                        10 => String::from("current"),
                        _ => format!("unknown additional version: {}", add_ver),
                    };
                println!(" additional version: {}", str);
            },
            "IVE2" => (add_ver: u16) {
                // ignore
            },
            "VCOD" => () {
                // verification checksum
                chk_file.seek(SeekFrom::Current(size as i64)).ok();
            },
            "IOWN" => () {
                // staredit player types
                chk_file.seek(SeekFrom::Current(size as i64)).ok();
            },
            "OWNR" => () {
                // starcraft player types
                // This section designates the controller of a particular player. It is exactly the same as the "IOWN" section, except there is an additional value, 0x00 for Inactive.
                // u8[12]: One byte for each player, specifies the owner of the player:
                // 00 - Inactive
                // 01 - Computer (game)
                // 02 - Occupied by Human Player
                // 03 - Rescue Passive
                // 04 - Unused
                // 05 - Computer
                // 06 - Human (Open Slot)
                // 07 - Neutral
                // 08 - Closed slot
                let owners = read_u8buf(chk_file, 12);
                println!(" owners: {:?}", owners);
                for i in 0..12 {
                    self.owners[i] = owners[i];
                }
            },
            "ERA " => (tileset: u16) {
                // This section indicates the tileset of the scenario.
                // u16: Designates tileset:
                // 00 - Badlands
                // 01 - Space Platform
                // 02 - Installation
                // 03 - Ashworld
                // 04 - Jungle
                // 05 - Desert
                // 06 - Arctic
                // 07 - Twilight
                // StarCraft masks the tileset indicator's bit value, so bits after the third place (anything after the value "7") are removed. Thus, 9 (1001 in binary) is interpreted as 1 (0001), 10 (1010) as 2 (0010), etc.
                // Desert, Arctic, and Twilight are Brood War-only tilesets.
                let ts = match tileset {
                    0 => TileSet::Badlands,
                    1 => TileSet::SpacePlatform,
                    2 => TileSet::Installation,
                    3 => TileSet::Ashworld,
                    4 => TileSet::Jungle,
                    5 => TileSet::Desert,
                    6 => TileSet::Arctic,
                    7 => TileSet::Twilight,
                    _ => panic!("invalid tileset: {}", tileset),
                };
                println!(" tileset: {:?}", ts);
                self.tileset = ts;
            },
            "DIM " => (width: u16, height: u16) {
                // The Width/Height of the map is measured in the number of square 32x32p tiles.
                // Standard Dimensions are 64, 96, 128, 192, and 256.
                println!("w: {}, h: {}", width, height);
                self.width = width;
                self.height = height;
            },
            "SIDE" => () {
                // This section contains the species/race of each player.
                // u8[12]: 1 byte per player the species of that player:
                // 00 - Zerg
                // 01 - Terran
                // 02 - Protoss
                // 03 - Invalid (Independent)
                // 04 - Invalid (Neutral)
                // 05 - User Selectable
                // 06 - Random (Forced; Acts as a selected race)
                // 07 - Inactive
                // Italicized settings denote invalid map options. Note Players 9-11 are defaultly Inactive and Player 12 is defaultly Neutral.
                let species = read_u8buf(chk_file, 12);
                println!(" side: {:?}", species);
            },
            "MTXM" => () {
                // Terrain section that contains a map of the level's
                // appearance. StarEdit disregards this section and instead uses
                // TILE; it is only used in Starcraft.
                // u16[map width * height]: one integer for each tile.
                //
                // The Width/Height of the map is measured in the number of square 32x32p tiles.
                // Tiles in this section are listed from left to right, top to bottom.
                // The values for each integer are their respective "MegaTile"
                // values in the scenario's tileset. If the size of this section
                // is greater than width*height*2, the data following is
                // ignored. If the size of this section is less, the resulting
                // tiles that have not been defined will be null tiles.
                // This section includes doodads as terrain; TILE, which is
                // otherwise identical, doesn't. Out of the terrain sections
                // (TILE, ISOM, and MTXM), SC only reads MTXM for the sake of
                // not having to generate this data on-the-fly: it contains the
                // exact representation of the level's appearance, including
                // doodads. TILE, on the other hand, is directly tied via a tile
                // lookup function to ISOM, and exists for the sake of not
                // having to generate tiles from ISOM on-the-fly in StarEdit.
                // If MTXM section is smaller than (map width*height), then the
                // remaining tiles will be filled with null tiles or tiles
                // specified by previous MTXM sections.
                //let terrain = read_u8buf(chk_file, size as usize);
                let tile_count = self.width as usize * self.height as usize;
                assert_eq!(tile_count, size as usize / 2);
                let mut terrain = Vec::<u16>::with_capacity(tile_count);
                for _ in 0..tile_count {
                    let val = chk_file.read_u16::<LittleEndian>().unwrap();
                    terrain.push(val);
                }
                self.mtxm = terrain;
            },
            "PUNI" => () {
                // player restrictions
                chk_file.seek(SeekFrom::Current(size as i64)).ok();
            },
            "UPGR" => () {
                // upgrade restrictions
                chk_file.seek(SeekFrom::Current(size as i64)).ok();
            },
            "PTEC" => () {
                // tech restrictions
                chk_file.seek(SeekFrom::Current(size as i64)).ok();
            },
            "UNIT" => () {
                // The X/Y coordinates are the center of the sprite of the unit
                // (in pixels).
                // Default values will apply if bit values are unchecked.
                // Defaults: 100% HP, 100% SP, 100% EP, 0 resources, 0 hangar
                // count.
                // This section can be split. Additional UNIT sections will add more units.
                let unit_count = (size as usize) / 36;
                let mut map_units = Vec::<MapUnit>::with_capacity(unit_count);
                for _ in 0..unit_count {
                    let unit = MapUnit::read(chk_file);
                    map_units.push(unit);
                }
                self.units = map_units;
            },
            "ISOM" => () {
                // isometric terrain, for staredit?
                chk_file.seek(SeekFrom::Current(size as i64)).ok();
            },
            "TILE" => () {
                // staredit terrain
                chk_file.seek(SeekFrom::Current(size as i64)).ok();
            },
            "DD2 " => () {
                // staredit doodads
                chk_file.seek(SeekFrom::Current(size as i64)).ok();
            },
            "THG2" => () {
                // sprites (on doodads)
                let sprite_count = (size as usize) / 10;
                let mut map_sprites = Vec::<MapSprite>::with_capacity(sprite_count);
                for _ in 0..sprite_count {
                    let sprite = MapSprite::read(chk_file);
                    map_sprites.push(sprite);
                }
                self.sprites = map_sprites;
            },
            "MASK" => () {
                // This section contains the data on fog of war for each player.
                // This is whether at the start of the game that levels of black
                // space that is available.
                // u8[ map width * height ]: One byte for each map tile. The
                // bits indicate for each player the fog of war.
                // Bit n - Player n+1's Fog of War. If on, the tile is covered
                // with fog. if off, the tile is visible.
                // Any size greater than width*height will be ignored. Any size
                // less will default missing tiles to 0xFF
                chk_file.seek(SeekFrom::Current(size as i64)).ok();
            },
            "STR " => (string_count: u16) {
                // This section contains all the strings in the map.
                // u16: Number of strings in the section (Default: 1024)
                // u16[Number of strings]: 1 integer for each string specifying
                // the offset (the spot where the string starts in the section
                // from the start of it).
                // Strings: After the offsets, this is where every string in the
                // map goes, one after another. Each one is terminated by a null
                // character.
                let mut string_offsets = Vec::with_capacity(string_count as usize);
                for _ in 0..string_count {
                    string_offsets.push(chk_file.read_u16::<LittleEndian>().unwrap());
                }
                let strings_start = 2 + 2 * (string_count as usize);
                let data = read_u8buf(chk_file, (size as usize) - strings_start);
                let mut strings = Vec::<String>::with_capacity(string_count as usize);
                let mut inpos;
                for i in 0..(string_count as usize) {
                    inpos = string_offsets[i] as usize - strings_start;
                    if inpos == 0 {
                        continue;
                    }
                    let mut res = String::new();
                    loop {
                        let val = data[inpos];
                        if val == 0 {
                            break;
                        }
                        inpos += 1;
                        res.push(val as char);
                    }
                    println!("str: {}", res);
                    strings.push(res);
                }
                self.strings = strings;
            },
            "UPRP" => () {
                // properties trigger
                chk_file.seek(SeekFrom::Current(size as i64)).ok();
            },
            "UPUS" => () {
                // unit slots used
                chk_file.seek(SeekFrom::Current(size as i64)).ok();
            },
            "MRGN" => () {
                // locations
                chk_file.seek(SeekFrom::Current(size as i64)).ok();
            },
            "TRIG" => () {
                // triggers
                chk_file.seek(SeekFrom::Current(size as i64)).ok();
            },
            "MBRF" => () {
                // mission briefings
                chk_file.seek(SeekFrom::Current(size as i64)).ok();
            },
            "SPRP" => (scen_name: u16, scen_desc: u16) {
                // u16: String number of the scenario name
                // u16: String number of the scenarios description.
                // A string index of 0 for the map name will default it to its
                // file name. A string index of 0 description will default to a
                // predefined string.
                self.scenario_name_str_idx = scen_name as usize;
                self.scenario_desc_str_idx = scen_desc as usize;
            },
            "FORC" => () {
                // This section specifies the forces and the information about them.
                //     u8[8]: 1 byte for each active player, specifying which of
                //     the 4 forces (0-based) that the player's on
                //     u16[4]: 1 integer for each force, string number of the name of the force
                //     u8[4]: 1 byte for each force specifying the properties:
                // Bit 0 - Random start location
                //     Bit 1 - Allies
                //     Bit 2 - Allied victory
                //     Bit 3 - Shared vision
                //     Bit 4-7 - Unused
                //     Notes about FORC:
                // If there is no string specified for a force name, it will
                // default to a "Force #" string.
                // If this section is less than 20 bytes, the remaining
                // bytes are defaulted to 0.
                // Players can be on a force greater than 4, however they
                // will not appear in the game lobby.
                chk_file.seek(SeekFrom::Current(size as i64)).ok();
            },
            "WAV " => () {
                // There are 512 wav entires regardless of how many are actually used.
                // u32[512]: 1 long for each WAV. Indicates a string index is
                // used for a WAV path in the MPQ. If the entry is not used, it
                // will be 0.
                chk_file.seek(SeekFrom::Current(size as i64)).ok();
            },
            "UNIS" => () {
                // This section contains the unit settings for the level:
                // u8[228]: 1 byte for each unit, in order of Unit ID
                // 00 - Unit does not use default settings
                // 01 - Unit does use default settings
                // u32[228]: Hit points for unit (Note the displayed value is this value / 256, with the low byte being a fractional HP value)
                // u16[228]: Shield points, in order of Unit ID
                // u8[228]: Armor points, in order of Unit ID
                // u16[228]: Build time (1/60 seconds), in order of Unit ID
                // u16[228]: Mineral cost, in order of Unit ID
                // u16[228]: Gas cost, in order of Unit ID
                // u16[228]: String number, in order of Unit ID
                // u16[228]: Base weapon damage the weapon does, in weapon ID order (#List of Unit Weapon IDs)
                // u16[228]: Upgrade bonus weapon damage, in weapon ID order
                chk_file.seek(SeekFrom::Current(size as i64)).ok();
            }
        );

        return Some(read_bytes);
    }

}
impl Map {
    // XXX scms are just mpq files, so we need to read them from disk
    pub fn read(gd: &GameData, filename: &str) -> Map {
        println!("reading {}", filename);
        let mpq_archive = MPQArchive::open(filename);
        let mut chk_file = mpq_archive.open_file("staredit/scenario.chk");
        let mut mapdata = MapData {
            mpq_archive: mpq_archive,
            owners: [0 as u8; 12],
            tileset: TileSet::Badlands,
            width: 0,
            height: 0,
            mtxm: Vec::<u16>::new(),
            units: Vec::<MapUnit>::new(),
            sprites: Vec::<MapSprite>::new(),
            strings: Vec::<String>::new(),
            scenario_name_str_idx: 0,
            scenario_desc_str_idx: 0,
        };
        while let Some(_) = mapdata.read_section(&mut chk_file) {
        }
        println!("loading terrain");
        let ti = TerrainInfo::read(gd, mapdata.tileset);

        println!("{} units", mapdata.units.len());
        for u in &mapdata.units {
            println!(" unit instance {}", u.instance_id);
            println!(" unit pos {}, {}", u.x, u.y);
            println!(" unit id {}", u.unit_id);
            println!(" unit name {}", gd.stat_txt_tbl[u.unit_id as usize].to_owned());
        }

        Map {
            data: mapdata,
            terrain_info: ti,
        }
    }

    pub fn name(&self) -> &str {
        if self.data.scenario_name_str_idx > 0 {
            &self.data.strings[self.data.scenario_name_str_idx - 1]
        } else {
            "unnamed"
        }
    }

    pub fn description(&self) -> &str {
        if self.data.scenario_desc_str_idx > 0 {
            &self.data.strings[self.data.scenario_desc_str_idx - 1]
        } else {
            ""
        }
    }

    pub fn render(&self, map_x: u16, map_y: u16, tiles_x: u16, tiles_y: u16,
                  trg_buf: &mut [u8], trg_pitch: u32) {
        let map_x_tiles = map_x / 32;
        let map_y_tiles = map_y / 32;

        let map_w = self.data.width as usize;

        let map_x_rest = map_x as i32 % 32;
        let tiles_x = if map_x_rest == 0 {tiles_x} else {tiles_x + 1};

        let map_y_rest = map_y as i32 % 32;
        let tiles_y = if map_y_rest == 0 {tiles_y} else {tiles_y + 1};

        let buffer_height = trg_buf.len() / trg_pitch as usize;

        for ty in 0..tiles_y {
            let top = (ty as i32) * 32 - map_y_rest;
            for tx in 0..tiles_x {
                let mtxm_tile = self.data.mtxm[map_w * ((map_y_tiles + ty) as usize) +
                                               (map_x_tiles + tx) as usize];
                let left = (tx as i32) * 32 - map_x_rest;

                self.terrain_info.render_mtxm(mtxm_tile, trg_buf,
                                              left as i32, top as i32,
                                              trg_pitch as usize, buffer_height as usize);
            }
        }

        // placeholder for map units
        let right_map_x = map_x + trg_pitch as u16;
        let bottom_map_y = map_y + buffer_height as u16;
        let s = 5;
        for u in &self.data.units {
            if u.x > map_x && u.x < right_map_x && u.y > map_y && u.y < bottom_map_y {
                //println!("{} in view", u.unit_id);
                let cx = (u.x - map_x) as usize;
                let cy = (u.y - map_y) as usize;
                // XXX ugly computation
                for y in cmp::max((s as isize)/2 - cy as isize, 0)..s as isize {
                    for x in cmp::max(((s as isize)/2 - cx as isize) , 0)..s as isize {
                        let ny = cy + y as usize - s/2;
                        let nx = cx + x as usize - s/2;
                        let idx = ny*trg_pitch as usize + nx;
                        if idx < trg_buf.len() {
                            trg_buf[idx] = 255;
                        }
                    }
                }
            }
        }

        for u in &self.data.sprites {
            if u.x > map_x && u.x < right_map_x && u.y > map_y && u.y < bottom_map_y {
                let cx = (u.x - map_x) as usize;
                let cy = (u.y - map_y) as usize;
                for y in 0..s {
                    for x in 0..s {
                        trg_buf[(cy + y - s/2)*trg_pitch as usize + cx + x - s/2] = 254;
                    }
                }
            }
        }
    }
}

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
    pub pal: Palette,
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

    pub fn render_mtxm(&self, mtxm_idx: u16, buffer: &mut [u8],
                       x: i32, y: i32,
                       stride: usize,
                       buffer_height: usize) {
        let cv5_id = mtxm_idx >> 4;
        let sub_id = mtxm_idx & 0x000F;
        let mega_tile_idx = if cv5_id <= 1024 {
            self.cv5[cv5_id as usize].mega_tiles[sub_id as usize]
        } else {
            self.doodads[(cv5_id as usize) - 1024].mega_tiles[sub_id as usize]
        };
        self.render_mega_tile(mega_tile_idx as usize, buffer,
                              x, y,
                              stride, buffer_height);
    }

    fn render_mega_tile(&self, vx4_idx: usize, buffer: &mut [u8],
                        x: i32, y: i32, stride: usize, buffer_height: usize) {
        let ref vx4 = self.vx4[vx4_idx];
        for row in 0..4 {
            for col in 0..4 {
                let top_y = row * 8;
                let left_x = col * 8;
                let vxentry = vx4.data[row*4+col];
                let flipped = (vxentry & 1) > 0;
                let vr4_idx = vxentry >> 1;
                self.render_mini_tile(vr4_idx as usize, buffer,
                                      x + left_x as i32, y + top_y as i32,
                                      stride, buffer_height, flipped);
            }
        }
    }

    fn render_mini_tile(&self, vr4_idx: usize, buffer: &mut [u8],
                        x: i32, y: i32,
                        stride: usize, buffer_height: usize,
                        flipped: bool) {
        let ref imgdata = self.vr4[vr4_idx].bitmap;
        for row in 0..8 {
            for col in 0..8 {
                if (y + row as i32) < 0 || (x + col as i32) < 0 ||
                    (x + col as i32 >= stride as i32) ||
                    (y + row as i32 >= buffer_height as i32) {
                    continue;
                }
                let outpos = (y + row) as usize * stride + (x + col) as usize;
                let imgpos = (row * 8) +
                    if flipped {
                        7 - col
                    } else {
                       col
                    };
                buffer[outpos] = imgdata[imgpos as usize];
            }
        }

    }
}
