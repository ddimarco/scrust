extern crate sdl2;
use sdl2::pixels::{Color, Palette};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
// use sdl2::rect::Rect;
use sdl2::render::{Renderer, Texture};
use sdl2::surface::{Surface};

use sdl2::TimerSubsystem;

extern crate read_pcx;
use read_pcx::stormlib::{MPQArchive};
use read_pcx::pcx::{PCX};


fn pal_to_sdl_pal(pal: &read_pcx::pal::Palette) -> sdl2::pixels::Palette {
    let mut cols = [Color::RGB(0, 0, 0); 256];
    for i in 0..256 {
        let r = pal.data[i*3 + 0];
        let g = pal.data[i*3 + 1];
        let b = pal.data[i*3 + 2];
        cols[i] = Color::RGB(r,g,b);
    }
    let pal = sdl2::pixels::Palette::from_colors(&cols);
    pal
}

// useful links for SDL2 & 8bit rendering:
// http://comments.gmane.org/gmane.comp.lib.sdl/64885
/*
My quick experiment used a reusable SDL_Surface to hold the 8-bit greyscale pixels. Using
SDL_GetTicks(), it seems pretty clear that, on my system, using:

  SDL_Texture *t8 = SDL_CreateTextureFromSurface(renderer, surf8);

  SDL_SetRenderTarget(renderer, texture);
  SDL_RenderCopy(renderer, t8, NULL, NULL);
  SDL_SetRenderTarget(renderer, NULL);

  SDL_DestroyTexture(t8);
*/

fn measure_time<F>(timer: &mut TimerSubsystem, closure: F) -> u32
    where F: Fn() {
    let start = timer.ticks();
    closure();
    let end = timer.ticks();
    end - start
}


fn main() {
    println!("opening pcx");

    let mpq = MPQArchive::open("/home/dm/code/mysc/data/STARDAT.MPQ");
    let mut file = mpq.open_file("glue\\title\\title.pcx");
    let pcx = PCX::read(&mut file);
    println!("pcx width: {0}, height: {1}", pcx.header.width, pcx.header.height);

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem.window("rust-sdl2 demo: Video", 640, 480)
        .position_centered()
        .opengl()
        .build()
        .unwrap();
    let mut renderer = window.renderer().build().unwrap();

    let mut timer = sdl_context.timer().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut screen = Surface::new(640, 480, PixelFormatEnum::Index8).unwrap();
    let ps = timer.ticks();
    let mut pal = pal_to_sdl_pal(&pcx.palette);
    let pe = timer.ticks();
    println!("converting palette took {} ms", pe - ps);
    screen.set_palette(&pal).expect("could not set palette");
    screen.with_lock_mut(|buffer: &mut [u8]| {
        buffer.clone_from_slice(&pcx.data.as_slice());
    });


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

        let start = timer.ticks();
        {
            let t8 = renderer.create_texture_from_surface(&screen).unwrap();
            renderer.copy(&t8, None, None);
            renderer.present();
        }
        let end = timer.ticks();
        //println!("rendering took {} ticks", end-start);
    }
}
