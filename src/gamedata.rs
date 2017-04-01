use std::collections::HashMap;

use std::path::Path;

use std::cell::RefCell;

use scformats::stormlib::{MPQArchive, MPQArchiveFile};
use scformats::font::{Font, FontSize};
use scformats::pcx::PCX;
use scformats::tbl::read_tbl;
use scformats::pal::Palette;
use scformats::iscript::IScript;
use scformats::grp::GRP;
use scformats::lox::LOX;

use scformats::unitsdata::{ImagesDat, UnitsDat, SpritesDat, FlingyDat, WeaponsDat, OrdersDat};

use Video;
use smacker::SMK;

pub struct FontReindexingStore {
    game_pcx: PCX,
    menu_pcxs: Vec<PCX>,
}
impl FontReindexingStore {
    pub fn load<F>(load_func: F) -> Self where F: Fn(&str) -> MPQArchiveFile {
        let menu_shortcuts = [
            "mm",
            "cs",
            "nl",
            "rz",
            "rt",
            "rp",
            // TODO
        ];

        let menu_pcxs = menu_shortcuts.into_iter()
            .map(|sc|
                 PCX::read(&mut load_func(&format!("glue/pal{}/tfont.pcx", sc)))
        ).collect();

        let game_pcx = PCX::read(&mut load_func("game/tfontgam.pcx"));
        FontReindexingStore {
            menu_pcxs: menu_pcxs,
            game_pcx: game_pcx,
        }
    }

    fn menuname2idx(short_name: &str) -> usize {
        match short_name {
            "mm" => {0},
            "cs" => {1},
            "nl" => {2},
            "rz" => {3},
            "rt" => {4},
            "rp" => {5},

            _ => { unreachable!(); },
        }
    }

    pub fn get_menu_reindex(&self, short_name: &str) -> &PCX {
        &self.menu_pcxs[FontReindexingStore::menuname2idx(short_name)]
    }
    pub fn get_game_reindex(&self) -> &PCX {
        &self.game_pcx
    }
}

use std::rc::Rc;
pub struct GameData {
    mpq_archives: Vec<MPQArchive>,

    fonts: Vec<Font>,
    // pub font_reindex: PCX,
    // pub fontmm_reindex: PCX,
    pub font_reindexing_store: FontReindexingStore,
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

    pub weapons_dat: WeaponsDat,
    pub orders_dat: OrdersDat,

    pub install_pal: Palette,

    pub iscript: IScript,

    pub grp_cache: RefCell<GRPCache>,
    pub lox_cache: Rc<RefCell<LOXCache>>,
    pub pcx_cache: RefCell<PCXCache>,
    pub video_cache: RefCell<VideoCache>,

    pub unit_wireframe_grp: GRP,
}

// FIXME: ugly
use scformats::terrain::GameDataTrait;
impl GameDataTrait for GameData {
    fn open(&self, filename: &str) -> Option<MPQArchiveFile> {
        GameData::open_(&self.mpq_archives, filename)
    }

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
        let images_tbl = read_tbl(&mut GameData::open_(&archives, "arr\\images.tbl").unwrap());
        let stat_txt_tbl = read_tbl(&mut GameData::open_(&archives, "rez/stat_txt.tbl").unwrap());

        let images_dat = ImagesDat::read(&mut GameData::open_(&archives, "arr/images.dat")
            .unwrap());
        let units_dat = UnitsDat::read(&mut GameData::open_(&archives, "arr/units.dat").unwrap());
        let sprites_dat = SpritesDat::read(&mut GameData::open_(&archives, "arr/sprites.dat")
            .unwrap());
        let flingy_dat = FlingyDat::read(&mut GameData::open_(&archives, "arr/flingy.dat")
            .unwrap());

        let weapons_dat = WeaponsDat::read(&mut GameData::open_(&archives, "arr/weapons.dat")
            .unwrap());
        let orders_dat = OrdersDat::read(&mut GameData::open_(&archives, "arr/orders.dat")
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

        let fnt_reindex_store = FontReindexingStore::load(|filename| {
            GameData::open_(&archives, filename).unwrap()
        });
        let lox_cache = Rc::new(RefCell::new(LOXCache::new()));

        let gd = GameData {
            mpq_archives: archives,
            fonts: fonts,
            font_reindexing_store: fnt_reindex_store,
            null_reindexing: null_reindexing,

            install_pal: install_pal,

            images_tbl: images_tbl,
            stat_txt_tbl: stat_txt_tbl,

            images_dat: images_dat,
            units_dat: units_dat,
            sprites_dat: sprites_dat,
            flingy_dat: flingy_dat,
            weapons_dat: weapons_dat,
            orders_dat: orders_dat,

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
            lox_cache: lox_cache,
            pcx_cache: RefCell::new(PCXCache::new()),
            video_cache: RefCell::new(VideoCache::new()),

            unit_wireframe_grp: unit_wireframe_grp,
        };

        {
            // load all overlays
            let mut lc = gd.lox_cache.borrow_mut();
            for (idx, e) in gd.images_tbl.iter().enumerate() {
                let l = e.len();
                if &e[l-3..l-1] == "lo" {
                    lc.load(&gd, (idx +1) as u32);
                }
            }
        }

        gd
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

macro_rules! def_cache_struct {
    ($name:ident, $keytype:ident, $valuetype:ident, $func:expr) =>
        {
            pub struct $name {
                cache: HashMap<$keytype, $valuetype>,
            }
            impl $name {
                pub fn new() -> Self {
                    $name { cache: HashMap::new() }
                }

                pub fn load(&mut self, gd: &GameData, key: $keytype) {
                    if !self.cache.contains_key(&key) {
                        self.cache.insert(key, $func(gd, key));
                    }
                }

                pub fn get(&mut self, gd: &GameData, key: $keytype) -> & $valuetype {
                    self.load(gd, key);
                    self.cache.get(&key).unwrap()
                }

                pub fn get_ro(&self, key: $keytype) -> & $valuetype {
                    self.cache.get(&key).unwrap()
                }
            }
        }
}

def_cache_struct! (GRPCache, u32, GRP, |gd: &GameData, grp_id: u32| {
    let name = "unit\\".to_string() + &gd.images_tbl[(grp_id as usize) - 1];
    println!("grp id: {}, filename: {}", grp_id, name);

    GRP::read(&mut gd.open(&name).unwrap())
});
def_cache_struct! (LOXCache, u32, LOX, |gd: &GameData, lox_id: u32| {
    let name = "unit/".to_string() + &gd.images_tbl[(lox_id as usize) - 1];
    println!("lox id: {}, filename: {}", lox_id, name);
    LOX::read(&mut gd.open(&name).unwrap())
});
// def_cache_struct! (PCXCache, String, PCX, |gd: &GameData, path: &String| {
//     PCX::read(&mut gd.open(&path).unwrap())
// });

pub struct VideoCache {
    cache: HashMap<String, Video>,
}
impl VideoCache {
    pub fn new() -> Self {
        VideoCache { cache: HashMap::new() }
    }

    pub fn load(&mut self, gd: &GameData, path: &str) {
        let pathstr = path.to_owned();
        if !self.cache.contains_key(&pathstr) {
            // let pcx = PCX::read(&mut gd.open(path).unwrap());
            let mut file = gd.open(path).unwrap();
            //let fsize = file.get_filesize();
            let fsize = file.get_ref().len();
            let mut smk = SMK::read(&mut file, fsize);

            let video = Video::from_smk(&mut smk);
            self.cache.insert(pathstr, video);
        }
    }
    pub fn get(&mut self, gd: &GameData, path: &str) -> &Video {
        self.load(gd, path);
        self.cache.get(path).unwrap()
    }

    pub fn get_ro(&self, path: &str) -> &Video {
        self.cache.get(path).unwrap()
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
    pub fn get(&mut self, gd: &GameData, path: &str) -> &PCX {
        self.load(gd, path);
        self.pcx_cache.get(path).unwrap()
    }

    pub fn get_ro(&self, path: &str) -> &PCX {
        self.pcx_cache.get(path).unwrap()
    }
}
