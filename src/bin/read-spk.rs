extern crate scrust;
use scrust::stormlib::MPQArchive;
use scrust::spk::SPK;

fn main() {
    let mpq = MPQArchive::open("/home/dm/code/mysc/data/STARDAT.MPQ");
    let mut file = mpq.open_file("parallax/star.spk");
    let spk = SPK::read_spk(&mut file);

    println!("read {} layers, {} images",
             spk.layers.len(),
             spk.images.len());
}
