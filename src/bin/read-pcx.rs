//use std::io;
use std::io::prelude::*;

use std::fs::File;
use std::io::SeekFrom;

extern crate byteorder;
use byteorder::{LittleEndian, ReadBytesExt};

extern crate sdl2;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
// use sdl2::rect::Rect;
use sdl2::render::{Renderer// , Texture
};

struct PCXHeader {
    version: u8,
    encoding: u8,
    bpp: u8,

    width: u16,
    height: u16,

    clrmap: [u8; 16*3],
    bpl: u16,
}

struct PCX {
    header: PCXHeader,
    data: Vec<u8>,
    palette: [u8; 256*3],
}

fn read_pcx(file: &mut File) -> PCX {
    let mut pcxhead = PCXHeader {
        version: 0,
        encoding: 0,
        bpp: 0,
        width: 0,
        height: 0,
        clrmap: unsafe{std::mem::uninitialized()},
        bpl: 0,
    };

    let id: u8 = file.read_u8().unwrap();
    pcxhead.version = file.read_u8().unwrap();
    pcxhead.encoding = file.read_u8().unwrap();
    pcxhead.bpp = file.read_u8().unwrap();
    //println!("id: {0}, version: {1}, encoding: {2}, bpp: {3}", id, version, encoding, bpp);

    let xmin = file.read_u16::<LittleEndian>().unwrap();
    let ymin = file.read_u16::<LittleEndian>().unwrap();
    let xmax = file.read_u16::<LittleEndian>().unwrap();
    let ymax = file.read_u16::<LittleEndian>().unwrap();
    let hres = file.read_u16::<LittleEndian>().unwrap();
    let vres = file.read_u16::<LittleEndian>().unwrap();

    pcxhead.clrmap = unsafe{std::mem::uninitialized()};
    file.read(&mut pcxhead.clrmap);

    let _ = file.read_u8().unwrap();
    let num_planes = file.read_u8().unwrap();
    pcxhead.bpl = file.read_u16::<LittleEndian>().unwrap();
    let pal = file.read_u16::<LittleEndian>().unwrap();

    file.seek(SeekFrom::Current(58));
    println!("xmin: {0}, ymin: {1}, xmax: {2}, ymax: {3}",
             xmin, ymin, xmax, ymax);

    // read data
    pcxhead.width = xmax - xmin + 1;
    pcxhead.height = ymax - ymin + 1;
    let bufsize = (pcxhead.width as usize)*(pcxhead.height as usize);
    let mut pcx = PCX {
        header: pcxhead,
        data: vec![0; bufsize],
        palette: [0; 256*3],
    };

    let mut outpos = 0;
    for _ in 0..pcx.header.height {
        let mut x = 0;
        while (x < pcx.header.bpl) && (outpos < bufsize as usize) {
            let val = file.read_u8().unwrap();
            if val > 192 {
                let repeat = val - 192;
                let color = file.read_u8().unwrap();
                for i in 0..repeat {
                    pcx.data[outpos] = color;
                    outpos += 1;
                    x += 1;
                }
            } else {
                pcx.data[outpos] = val;
                outpos += 1;
                x += 1;
            }
        }
    }

    // read palette
    let first_byte = file.read_u8().unwrap();
    assert!(first_byte == 12);
    file.read(&mut pcx.palette);
    return pcx;
}

fn pcx_to_ppm(pcx: PCX, outfile: &str) {
    let mut outfile = File::create(outfile).unwrap();
    outfile.write_fmt(format_args!("P3\n"));
    outfile.write_fmt(format_args!("{0} {1}\n", pcx.header.width, pcx.header.height));
    outfile.write_fmt(format_args!("255\n"));
    for i in 0..(pcx.header.width as usize)*(pcx.header.height as usize) {
        let pal_idx: usize = 3 * (pcx.data[i as usize] as usize);
        outfile.write_fmt(format_args!("{0} {1} {2}\n",
                                       pcx.palette[pal_idx + 0],
                                       pcx.palette[pal_idx + 1],
                                       pcx.palette[pal_idx + 2]));
    }
}

fn pcx_to_texture(renderer: &mut Renderer, pcx: PCX) -> sdl2::render::Texture {
    let mut texture = renderer.create_texture_streaming(
        PixelFormatEnum::RGB24, pcx.header.width as u32, pcx.header.height as u32).unwrap();

    texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
        let mut i = 0;
        for y in 0..pcx.header.height as usize {
            for x in 0..pcx.header.width as usize {
                let offset = y*pitch + x*3;
                let col = pcx.data[i];
                let r = pcx.palette[col as usize*3  + 0];
                let g = pcx.palette[col as usize*3 + 1];
                let b = pcx.palette[col as usize*3 + 2];

                buffer[offset + 0] = r;
                buffer[offset + 1] = g;
                buffer[offset + 2] = b;
                i += 1;
            }
        }
    });
    return texture;
}

fn main() {
    println!("opening pcx");
    let mut file=File::open("/home/dm/code/rust/read-pcx/backgnd.pcx").unwrap();
    let pcx = read_pcx(& mut file);

    println!("pcx width: {0}, height: {1}", pcx.header.width, pcx.header.height);
    //pcx_to_ppm(pcx, "/tmp/out.ppm");

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("rust-sdl2 demo: Video", 640, 480)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let mut renderer = window.renderer().build().unwrap();

    ////
    let texture = pcx_to_texture(&mut renderer, pcx);
    ////

    renderer.set_draw_color(Color::RGB(0, 0, 0));
    renderer.clear();
    renderer.copy(&texture, None, None);
    renderer.present();
    let mut event_pump = sdl_context.event_pump().unwrap();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                _ => {}
            }
        }
        // The rest of the game loop goes here...
    }

}
