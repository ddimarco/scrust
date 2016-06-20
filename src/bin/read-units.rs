use std::path::Path;

extern crate read_pcx;

use read_pcx::gamedata::GameData;

fn main() {
    println!("opening mpq...");
    let gd = GameData::init(&Path::new("/home/dm/.wine/drive_c/StarCraft/"));

    println!("grp_file len: {}", gd.images_dat.grp_file.len());
    println!("graphics len: {}", gd.units_dat.graphics.len());
    println!("image_file len: {}", gd.sprites_dat.image_file.len());
    println!("sprite len: {}", gd.flingy_dat.sprite.len());
}
