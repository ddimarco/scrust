use std::path::Path;
use std::env;

extern crate scrust;
use scrust::gamedata::GameData;


fn main() {
    let args: Vec<_> = env::args().collect();
    println!("args: {:?}", args);
    if args.len() < 3 {
        println!("usage: {} infile outfile", args[0]);
        return;
    }

    let ref infile = args[1];
    let ref outfile = args[2];
    let gd = GameData::init(&Path::new("/home/dm/.wine/drive_c/StarCraft/"));
    gd.extract(&infile, &outfile);
}
