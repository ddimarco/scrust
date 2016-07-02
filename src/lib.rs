extern crate byteorder;
extern crate libc;
#[macro_use] extern crate enum_primitive;
extern crate num;
extern crate sdl2;

#[macro_use]
pub mod events;

pub mod stormlib;
pub mod pcx;
pub mod tbl;
pub mod grp;
pub mod font;
pub mod pal;
pub mod gamedata;
pub mod unitsdata;
pub mod iscript;


use std::path::Path;
use sdl2::render::Renderer;
use sdl2::pixels::PixelFormatEnum;
use sdl2::surface::Surface;
use gamedata::GameData;

struct_events! (
    keyboard: {
        key_escape: Escape,
        key_up: Up,
        key_down: Down,
        key_left: Left,
        key_right: Right,
        key_space: Space,
        key_return: Return
    },
    else: {
        quit: Quit { .. }
    }
);

pub struct GameContext<'window> {
    pub gd: GameData,
    pub events: Events,
    pub renderer: Renderer<'window>,
    pub screen: Surface<'window>,

    // debug stuff
    //pub timer: Timer<'window>,
}
impl<'window> GameContext<'window> {
    fn new(gd: GameData, events: Events, renderer: Renderer<'window>,
           /*timer: Timer<'window>*/) -> GameContext<'window> {
        GameContext {
            gd: gd,
            events: events,
            renderer: renderer,
            screen: Surface::new(640, 480, PixelFormatEnum::Index8).unwrap(),
            // timer: timer,
        }
    }

    pub fn output_size(&self) -> (u32, u32) {
        let (w, h) = self.renderer.output_size().unwrap();
        (w,h)
    }
}

pub enum ViewAction {
    None,
    Quit,
    ChangeView(Box<View>),
}

pub trait View {
    /// renders the current view into context.screen
    fn render(&mut self, context: &mut GameContext, elapsed: f64) -> ViewAction;
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


pub fn spawn<F>(title: &str, scdata_path: &str, init: F)
where F: Fn(&mut GameContext) -> Box<View> {
    let gd = GameData::init(&Path::new(scdata_path));

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem.window(title, 640, 480)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let mut timer = sdl_context.timer().unwrap();

    // FIXME: set a default palette for screen surface
    let mut context = GameContext::new(
        gd,
        Events::new(sdl_context.event_pump().unwrap()),
        window.renderer().accelerated().build().unwrap(),
    );
    let mut current_view = init(&mut context);

    let interval = 1_000 / 60;
    let mut before = timer.ticks();
    let mut fps_shown_last = timer.ticks();
    let mut fps = 0u16;

    let mut render_time_sum = 0;
    let mut render_count = 0;

    'running: loop {
        let now = timer.ticks();
        let dt = now - before;
        let elapsed = dt as f64 / 1_000.0;

        if dt < interval {
            timer.delay(interval - dt);
            continue;
        }

        before = now;
        fps += 1;

        if now - fps_shown_last > 1_000 {
            println!("FPS: {}, avg render() dur: {}", fps, render_time_sum  / render_count);
            fps_shown_last = now;
            fps = 0;
            render_time_sum = 0;
            render_count = 0;
        }

        context.events.pump(&mut context.renderer);

        let start = timer.ticks();
        let render_res = current_view.render(&mut context, elapsed);
        let end = timer.ticks();
        render_time_sum += end - start;
        render_count += 1;
        // println!("render function took {} ms", dur);

        // let start = timer.ticks();
        {
            let t8 = context.renderer.create_texture_from_surface(&context.screen).unwrap();
            context.renderer.copy(&t8, None, None);
            context.renderer.present();
        }
        // let end = timer.ticks();
        //println!("rendering took {} ticks", end-start);

        match render_res {
            ViewAction::None => context.renderer.present(),
            ViewAction::Quit => break,
            ViewAction::ChangeView(new_view) =>
                current_view = new_view,
        }

    }
}
