extern crate read_pcx;
use read_pcx::stormlib::{MPQArchive};
use read_pcx::grp::{GRP};
use read_pcx::font::Font;
use read_pcx::tbl::read_tbl;

use std::fs::File;

#[test]
fn read_grp() {
    let mpq = MPQArchive::open("/home/dm/code/mysc/data/STARDAT.MPQ");
    let mut file = mpq.open_file("unit\\cmdbtns\\cmdicons.grp");

    let grp = GRP::read(&mut file);
    let ref grpheader = grp.header;
    println!("frames: {}, w: {}, h: {}", grpheader.frame_count,
             grpheader.width, grpheader.height);

    assert_eq!(grpheader.frame_count, grp.frames.len());

    grp.frame_to_ppm(0, "/tmp/frame0.ppm");

}

#[test]
fn read_fnt() {
    let mpq = MPQArchive::open("/home/dm/code/mysc/data/install.exe");
    let mut file = mpq.open_file("files\\font\\font14.fnt");
    let fnt = Font::read(&mut file);
    println!("low-id: {}, high-idx: {}, max-width: {}, max-height: {}",
             fnt.header.low_idx, fnt.header.high_idx,
             fnt.header.max_width, fnt.header.max_height);
    assert_eq!(fnt.header.high_idx as usize - fnt.header.low_idx as usize,
               fnt.letters.len());
}

#[test]
fn read_tbl_test() {
    let mut file=File::open("/home/dm/code/rust/read-pcx/images.tbl").unwrap();
    let strings = read_tbl(&mut file);
    assert_eq!(strings.len(), 929);

    assert_eq!(strings[0], "zerg\\avenger.grp");
    assert_eq!(strings[1], "protoss\\mshield.los");
    assert_eq!(strings[2], "zerg\\zavBirth.grp");
    assert_eq!(strings[3], "zerg\\zavbirth.lob");
    assert_eq!(strings[4], "zerg\\zavDeath.grp");
    assert_eq!(strings[928], "thingy\\MaelHit.grp");
}