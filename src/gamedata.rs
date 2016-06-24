use std::path::Path;
use std::io::Read;

use std::collections::HashMap;

use ::stormlib::{MPQArchive, MPQArchiveFile};
use ::font::{Font, FontSize};
use ::pcx::PCX;
use ::tbl::read_tbl;
use ::grp::GRP;
use ::pal::Palette;

use ::unitsdata::{ImagesDat, UnitsDat, SpritesDat, FlingyDat};

pub struct GameData {
    mpq_archives: Vec<MPQArchive>,

    fonts: Vec<Font>,
    pub fontmm_reindex: PCX,
    pub images_tbl: Vec<String>,
    pub stat_txt_tbl: Vec<String>,

    // pub unit_reindexing_tbl: Vec<u8>,
    // pub ofire_reindexing_tbl: Vec<u8>,
    // pub bfire_reindexing_tbl: Vec<u8>,
    // pub gfire_reindexing_tbl: Vec<u8>,
    // pub bexpl_reindexing_tbl: Vec<u8>,

    // TODO: encapsulate this stuff
    pub images_dat: ImagesDat,
    pub units_dat: UnitsDat,
    pub sprites_dat: SpritesDat,
    pub flingy_dat: FlingyDat,

    pub install_pal: Palette,
}

impl GameData {
    pub fn init(data_path: &Path) -> GameData {
        let data_filenames = ["patch_rt.mpq", "BroodWar.mpq", "BrooDat.mpq",
                              "StarDat.mpq", "Starcraft.mpq"];
        let mut archives = Vec::<MPQArchive>::new();
        for &filename in data_filenames.iter() {
            let combined = data_path.join(filename);
            if combined.exists() {
                archives.push(MPQArchive::open(combined.to_str().unwrap()));
            }
        }

        let fonts = GameData::load_fonts(&archives);
        let fontmm_reindex = PCX::read(&mut GameData::open_(&archives, "glue\\palmm\\tfont.pcx").unwrap());
        let images_tbl = read_tbl(&mut GameData::open_(&archives, "arr\\images.tbl").unwrap());
        let stat_txt_tbl = read_tbl(&mut GameData::open_(&archives, "rez/stat_txt.tbl").unwrap());

        let images_dat = ImagesDat::read(&mut GameData::open_(&archives, "arr/images.dat").unwrap());
        let units_dat = UnitsDat::read(&mut GameData::open_(&archives, "arr/units.dat").unwrap());
        let sprites_dat = SpritesDat::read(&mut GameData::open_(&archives, "arr/sprites.dat").unwrap());
        let flingy_dat = FlingyDat::read(&mut GameData::open_(&archives, "arr/flingy.dat").unwrap());

        let install_pal = Palette::read_wpe(&mut GameData::open_(&archives, "tileset/install.wpe").unwrap());

        // let unit_pcx = PCX::read(&mut GameData::open_(&archives, "game/tunit.pcx").unwrap());
        GameData {
            mpq_archives: archives,
            fonts: fonts,
            fontmm_reindex: fontmm_reindex,

            install_pal: install_pal,

            images_tbl: images_tbl,
            stat_txt_tbl: stat_txt_tbl,

            images_dat: images_dat,
            units_dat: units_dat,
            sprites_dat: sprites_dat,
            flingy_dat: flingy_dat,

        }
    }
    fn load_fonts(archives: &Vec<MPQArchive>) -> Vec<Font> {
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

    fn open_(archives: &Vec<MPQArchive>, filename: &str) -> Option<MPQArchiveFile> {
        for mpq in archives.iter() {
            if mpq.has_file(filename) {
                //println!("found {} in {}", filename, mpq.filename);
                let res: MPQArchiveFile = mpq.open_file(filename);
                return Some(res);
            }
        }
        None
    }

    pub fn open(&self, filename: &str) -> Option<MPQArchiveFile> {
        GameData::open_(&self.mpq_archives, filename)
    }

    pub fn font<'a>(&'a self, size: FontSize) -> &'a Font {
        &self.fonts[size as usize]
    }


}

/*
pub struct GRPCache<'a> {
    grp_cache: HashMap<u32, GRP>,
    gd: &'a GameData,
}
impl<'a> GRPCache<'a> {
    pub fn new(gd: &'a GameData) -> GRPCache<'a> {
        GRPCache {
            grp_cache: HashMap::new(),
            gd: gd,
        }
    }

    pub fn grp(& mut self, grp_id: u32) -> &GRP {
        // TODO: cache only references
        if self.grp_cache.contains_key(&grp_id) {
            return self.grp_cache.get(&grp_id).unwrap();
        }
        let name = "unit\\".to_string() + &self.gd.images_tbl[(grp_id as usize) - 1];
        println!("grp id: {}, filename: {}", grp_id, name);

        let grp = GRP::read(&mut self.gd.open(&name).unwrap());
        self.grp_cache.insert(grp_id, grp);

        return self.grp_cache.get(&grp_id).unwrap();
    }
}
*/
