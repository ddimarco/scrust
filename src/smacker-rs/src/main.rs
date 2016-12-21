use std::env;

extern crate smacker;
use smacker::{SMK};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("this tool saves all frames from a smk video into /tmp/frameXXX.ppm");
        println!(" usage: {} <smk-file>", args[0]);
        return;
    }

    let mut smk = SMK::from_file(args[1].as_str());
    println!("frames: {}", smk.frame_count);
    println!("w: {}, h: {}, ysm: {}, usf: {}",
             smk.width,
             smk.height,
             smk.y_scale_mode,
             smk.usf);
    smk.go_first_frame();
    for frame in smk {
        frame.to_ppm(format!("/tmp/frame{:03}.ppm", frame.frame_idx).as_str());
    }
}
