use std::path::Path;
use std::env;

extern crate scrust;
use scrust::gamedata::GameData;
use scrust::unitsdata::{WeaponsDat, UnitsDat, FlingyDat};

fn print_usage(args: &[String]) {
    println!("usage: {} [dattype] [index]", args[0]);
}

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() < 3 {
        print_usage(&args);
        return;
    }

    let gd = GameData::init(&Path::new("/home/dm/.wine/drive_c/StarCraft/"));
    let dattype_str = &args[1];
    let i = args[2].parse::<usize>().expect("index needs to be an integer!");

    match dattype_str.as_ref() {
        "images" => gd.images_dat.print_entry(i),
        "sprites" => gd.sprites_dat.print_entry(i),
        "flingy" => gd.flingy_dat.print_entry(i),
        "units" => gd.units_dat.print_entry(i),
        "weapons" => gd.weapons_dat.print_entry(i),
        _ => { print_usage(&args); },
    }
}
