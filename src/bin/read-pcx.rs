
extern crate sdl2;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
// use sdl2::rect::Rect;
use sdl2::render::{Renderer// , Texture
};

extern crate read_pcx;
use read_pcx::stormlib::{MPQArchive};
use read_pcx::pcx::{PCX};


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
    }).ok();
    return texture;
}

fn main() {
    println!("opening pcx");

    let mpq = MPQArchive::open("/home/dm/code/mysc/data/STARDAT.MPQ");
    let mut file = mpq.open_file("glue\\title\\title.pcx");

    let pcx = PCX::read(&mut file);

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
