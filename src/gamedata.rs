use std::collections::HashMap;

use std::path::Path;

use std::cell::RefCell;

use ::stormlib::{MPQArchive, MPQArchiveFile};
use ::font::{Font, FontSize};
use ::pcx::PCX;
use ::tbl::read_tbl;
use ::pal::Palette;
use ::iscript::IScript;
use ::grp::GRP;
use ::lox::LOX;

use ::unitsdata::{ImagesDat, UnitsDat, SpritesDat, FlingyDat};

#[derive(Copy, Clone, Debug)]
pub enum TileSet {
    Badlands = 0,
    SpacePlatform = 1,
    Installation = 2,
    Ashworld = 3,
    Jungle = 4,
    Desert = 5,
    Arctic = 6,
    Twilight = 7,
}

pub struct GameData {
    mpq_archives: Vec<MPQArchive>,

    fonts: Vec<Font>,
    pub font_reindex: PCX,
    pub fontmm_reindex: PCX,
    pub images_tbl: Vec<String>,
    pub stat_txt_tbl: Vec<String>,

    // pub unit_reindexing_tbl: Vec<u8>,
    pub ofire_reindexing: PCX,
    pub bfire_reindexing: PCX,
    pub gfire_reindexing: PCX,
    pub bexpl_reindexing: PCX,
    // pub unit_reindexing: PCX,
    // pub dark_reindexing: PCX,
    pub null_reindexing: Vec<u8>,
    pub shadow_reindexing: Vec<u8>,
    pub player_reindexing: Vec<u8>,
    pub twire_reindexing: Vec<u8>,

    // TODO: encapsulate this stuff
    pub images_dat: ImagesDat,
    pub units_dat: UnitsDat,
    pub sprites_dat: SpritesDat,
    pub flingy_dat: FlingyDat,

    pub install_pal: Palette,

    pub iscript: IScript,

    pub grp_cache: RefCell<GRPCache>,
    pub lox_cache: RefCell<LOXCache>,
    pub pcx_cache: RefCell<PCXCache>,

    pub unit_wireframe_grp: GRP,
}

impl GameData {
    pub fn init(data_path: &Path) -> GameData {
        let data_filenames =
            ["patch_rt.mpq", "BroodWar.mpq", "BrooDat.mpq", "StarDat.mpq", "Starcraft.mpq"];
        let mut archives = Vec::<MPQArchive>::new();
        for filename in &data_filenames {
            let combined = data_path.join(filename);
            if combined.exists() {
                archives.push(MPQArchive::open(combined.to_str().unwrap()));
            }
        }

        let fonts = GameData::load_fonts(&archives);
        let font_reindex = PCX::read(&mut GameData::open_(&archives, "game\\tfontgam.pcx")
            .unwrap());
        let fontmm_reindex = PCX::read(&mut GameData::open_(&archives, "glue\\palmm\\tfont.pcx")
            .unwrap());
        let images_tbl = read_tbl(&mut GameData::open_(&archives, "arr\\images.tbl").unwrap());
        let stat_txt_tbl = read_tbl(&mut GameData::open_(&archives, "rez/stat_txt.tbl").unwrap());

        let images_dat = ImagesDat::read(&mut GameData::open_(&archives, "arr/images.dat")
            .unwrap());
        let units_dat = UnitsDat::read(&mut GameData::open_(&archives, "arr/units.dat").unwrap());
        let sprites_dat = SpritesDat::read(&mut GameData::open_(&archives, "arr/sprites.dat")
            .unwrap());
        let flingy_dat = FlingyDat::read(&mut GameData::open_(&archives, "arr/flingy.dat")
            .unwrap());

        let install_pal = Palette::read_wpe(&mut GameData::open_(&archives, "tileset/install.wpe")
            .unwrap());

        let iscript = IScript::read(&mut GameData::open_(&archives, "scripts/iscript.bin")
            .unwrap());

        // FIXME depends on tileset
        let ofire_reindexing =
            PCX::read(&mut GameData::open_(&archives, "tileset/install/ofire.pcx").unwrap());
        let bfire_reindexing =
            PCX::read(&mut GameData::open_(&archives, "tileset/install/bfire.pcx").unwrap());
        let gfire_reindexing =
            PCX::read(&mut GameData::open_(&archives, "tileset/install/gfire.pcx").unwrap());
        let bexpl_reindexing =
            PCX::read(&mut GameData::open_(&archives, "tileset/install/bexpl.pcx").unwrap());
        let unit_reindexing = PCX::read(&mut GameData::open_(&archives, "game\\tunit.pcx")
            .unwrap());
        let dark_reindexing =
            PCX::read(&mut GameData::open_(&archives, "tileset\\install\\dark.pcx").unwrap());

        // FIXME: figure out how to apply this
        // 24 × 1 pixel
        let twire_reindexing = PCX::read(&mut GameData::open_(&archives, "game/twire.pcx")
            .unwrap());

        let mut null_reindexing = vec![0 as u8; 256*256];
        for i in 0..255 {
            for j in 0..255 {
                null_reindexing[i * 256 + j] = (i + 1) as u8;
            }
        }

        let mut shadow_reindexing = vec![0 as u8; 256*256];
        for r in 0..256 {
            let mut inpos = 256 * 16;
            for c in 0..256 {
                shadow_reindexing[r * 256 + c] = dark_reindexing.data[inpos];
                inpos += 1;
            }
        }

        // player colors reindex
        let mut player_reindexing = vec![0 as u8; 12*256];
        for p in 0..11 {
            for c in 0..255 {
                // color indices depending on player no
                player_reindexing[p * 256 + c] = if (c > 7) && (c < 16) {
                    // 128 × 1 pixel
                    unit_reindexing.data[p * 8 + (c - 8)]
                } else {
                    (c + 1) as u8
                };
            }
        }

        let unit_wireframe_grp =
            GRP::read(&mut GameData::open_(&archives, "unit/wirefram/wirefram.grp").unwrap());

        GameData {
            mpq_archives: archives,
            fonts: fonts,
            font_reindex: font_reindex,
            fontmm_reindex: fontmm_reindex,
            null_reindexing: null_reindexing,

            install_pal: install_pal,

            images_tbl: images_tbl,
            stat_txt_tbl: stat_txt_tbl,

            images_dat: images_dat,
            units_dat: units_dat,
            sprites_dat: sprites_dat,
            flingy_dat: flingy_dat,

            iscript: iscript,
            ofire_reindexing: ofire_reindexing,
            bfire_reindexing: bfire_reindexing,
            gfire_reindexing: gfire_reindexing,
            bexpl_reindexing: bexpl_reindexing,
            // dark_reindexing: dark_reindexing,
            shadow_reindexing: shadow_reindexing,
            player_reindexing: player_reindexing,
            twire_reindexing: twire_reindexing.data,

            // FIXME: move out of here, get rid of refcell?
            grp_cache: RefCell::new(GRPCache::new()),
            lox_cache: RefCell::new(LOXCache::new()),
            pcx_cache: RefCell::new(PCXCache::new()),

            unit_wireframe_grp: unit_wireframe_grp,
        }
    }
    fn load_fonts(archives: &[MPQArchive]) -> Vec<Font> {
        let mut fonts = Vec::<Font>::with_capacity(4);
        let font_files = ["files/font/font10.fnt",
                          "files/font/font14.fnt",
                          "files/font/font16.fnt",
                          "files/font/font16x.fnt"];
        for ff in &font_files {
            fonts.push(Font::read(&mut GameData::open_(&archives, ff).unwrap()));
        }

        fonts
    }

    fn open_(archives: &[MPQArchive], filename: &str) -> Option<MPQArchiveFile> {
        for mpq in archives {
            if mpq.has_file(filename) {
                // println!("found {} in {}", filename, mpq.filename);
                let res: MPQArchiveFile = mpq.open_file(filename);
                return Some(res);
            }
        }
        None
    }

    pub fn open(&self, filename: &str) -> Option<MPQArchiveFile> {
        GameData::open_(&self.mpq_archives, filename)
    }

    pub fn font(&self, size: FontSize) -> &Font {
        &self.fonts[size as usize]
    }

    pub fn extract(&self, in_fn: &str, out_fn: &str) {
        for mpq in &self.mpq_archives {
            if mpq.has_file(in_fn) {
                // println!("found {} in {}", filename, mpq.filename);
                mpq.extract(in_fn, out_fn);
                return;
            }
        }
    }
}


pub struct GRPCache {
    grp_cache: HashMap<u32, GRP>,
}
impl GRPCache {
    pub fn new() -> GRPCache {
        GRPCache { grp_cache: HashMap::new() }
    }

    pub fn load(&mut self, gd: &GameData, grp_id: u32) {
        if !self.grp_cache.contains_key(&grp_id) {
            let name = "unit\\".to_string() + &gd.images_tbl[(grp_id as usize) - 1];
            println!("grp id: {}, filename: {}", grp_id, name);

            let grp = GRP::read(&mut gd.open(&name).unwrap());
            self.grp_cache.insert(grp_id, grp);
        }
    }

    pub fn grp(&mut self, gd: &GameData, grp_id: u32) -> &GRP {
        self.load(gd, grp_id);
        self.grp_cache.get(&grp_id).unwrap()
    }

    pub fn grp_ro(&self, grp_id: u32) -> &GRP {
        if self.grp_cache.contains_key(&grp_id) {
            self.grp_cache.get(&grp_id).unwrap()
        } else {
            panic!("grp_ro() called and grp {} not in cache!", grp_id);
        }
    }
}

pub struct PCXCache {
    pcx_cache: HashMap<String, PCX>,
}
impl PCXCache {
    pub fn new() -> Self {
        PCXCache { pcx_cache: HashMap::new() }
    }

    pub fn load(&mut self, gd: &GameData, path: &str) {
        let pathstr = path.to_owned();
        if !self.pcx_cache.contains_key(&pathstr) {
            let pcx = PCX::read(&mut gd.open(path).unwrap());
            self.pcx_cache.insert(pathstr, pcx);
        }
    }
    pub fn pcx(&mut self, gd: &GameData, path: &str) -> &PCX {
        self.load(gd, path);
        self.pcx_cache.get(path).unwrap()
    }

    pub fn pcx_ro(&self, path: &str) -> &PCX {
        self.pcx_cache.get(path).unwrap()
    }
}


pub struct LOXCache {
    lox_cache: HashMap<u32, LOX>,
}
impl LOXCache {
    pub fn new() -> Self {
        LOXCache { lox_cache: HashMap::new() }
    }

    pub fn load(&mut self, gd: &GameData, lox_id: u32) {
        if !self.lox_cache.contains_key(&lox_id) {
            let name = "unit/".to_string() + &gd.images_tbl[(lox_id as usize) - 1];
            println!("lox id: {}, filename: {}", lox_id, name);
            let lox = LOX::read(&mut gd.open(&name).unwrap());
            self.lox_cache.insert(lox_id, lox);
        }
    }

    pub fn lox(&mut self, gd: &GameData, lox_id: u32) -> &LOX {
        self.load(gd, lox_id);
        self.lox_cache.get(&lox_id).unwrap()
    }
}
