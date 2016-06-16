extern crate read_pcx;
use read_pcx::stormlib::{MPQArchive};
use read_pcx::grp::{GRP};

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
