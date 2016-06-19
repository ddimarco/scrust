use std::path::Path;

use ::stormlib::{MPQArchive, MPQArchiveFile};

pub struct GameData {
    mpq_archives: Vec<MPQArchive>,
}
impl GameData {
    pub fn init(data_path: &Path) -> GameData {
        let data_filenames = ["patch_rt.mpq", "BroodWar.mpq", "BrooDat.mpq",
                              "StarDat.mpq", "Starcraft.mpq"];
        let mut archives = Vec::<MPQArchive>::new();
        for &filename in data_filenames.iter() {
            let combined = data_path.join(filename);
            if combined.exists() {
                //println!("found mpq: {}", filename);
                archives.push(MPQArchive::open(combined.to_str().unwrap()));
            }
        }

        GameData {
            mpq_archives: archives,
        }
    }

    pub fn open(self: &GameData, filename: &str) -> Option<MPQArchiveFile> {
        for mpq in self.mpq_archives.iter() {
            if mpq.has_file(filename) {
                println!("found {} in {}", filename, mpq.filename);
                let res: MPQArchiveFile = mpq.open_file(filename);
                return Some(res);
            }
        }
        None
    }
}
