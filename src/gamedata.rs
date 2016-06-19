use std;
use std::path::Path;

use ::stormlib::{MPQArchive, MPQArchiveFile};
use ::font::{Font, FontSize};
use ::pcx::PCX;

pub struct GameData {
    mpq_archives: Vec<MPQArchive>,

    fonts: Vec<Font>,
    pub fontmm_reindex: PCX,
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
        GameData {
            mpq_archives: archives,
            fonts: fonts,
            fontmm_reindex: fontmm_reindex,

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
                println!("found {} in {}", filename, mpq.filename);
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
