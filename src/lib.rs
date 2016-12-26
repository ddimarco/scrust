extern crate byteorder;
extern crate libc;
#[macro_use]
extern crate enum_primitive;
extern crate num;
extern crate sdl2;
extern crate rand;
extern crate stash;

extern crate pathplanning;
extern crate smacker;
use smacker::SMK;

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
#[macro_use]
pub mod utils;
pub mod terrain;
pub mod iscriptstate;
pub mod scunits;
pub mod lox;
pub mod spk;
pub mod ui;
pub mod render;

use std::path::Path;
use sdl2::render::Renderer;
use sdl2::pixels::PixelFormatEnum;
use sdl2::surface::Surface;
use gamedata::GameData;
use scunits::SCUnit;
use pal::Palette;

use stash::Stash;
use std::collections::HashSet;

struct_events! (
    mouse: {
        mouse_left: Left,
        mouse_right: Right
    },
    else: {
        quit: Quit { .. }
    }
);

// FIXME: use same structure for GRPs?
pub struct Video {
    pub pal: Palette,
    pub frames: Vec<Vec<u8>>,
    pub width: usize,
    pub height: usize,
}
impl Video {
    pub fn from_smk(smk: &mut SMK) -> Self {
        let mut frames = Vec::<Vec<u8>>::with_capacity(smk.frame_count);
        smk.go_first_frame();

        let pal = Palette::from_buffer(&smk.copy_palette());

        for _ in 0..smk.frame_count {
            let framedata = smk.copy_frame();
            frames.push(framedata);
            smk.go_next_frame();
        }

        Video {
            width: smk.width,
            height: smk.height,
            pal: pal,
            frames: frames,
        }
    }
}

#[derive(Copy,Clone,PartialEq,Debug)]
pub enum MousePointerType {
    Arrow = 0,
    ScrollLeft,
    ScrollRight,
    ScrollUp,
    ScrollDown,

    ScrollDownLeft,
    ScrollDownRight,
    ScrollUpLeft,
    ScrollUpRight,

    Drag,
    Illegal,
    Time,
    TargetGreen,
    TargetYellow,
    TargetRed,
    TargetYellowStatic,
    MagnifierGreen,
    MagnifierRed,
    MagnifierYellow,
}

// game changing events are handled via GameEvent messages
pub enum GameEvents {
    ChangeMouseCursor(MousePointerType),
    MoveMap(i32, i32),
    SelectUnit(usize),
}

pub struct GameState {
    pub unit_instances: Stash<SCUnit>,
    pub selected_units: Vec<usize>,

    pub game_events: Vec<GameEvents>,
    pub map_pos: Point,
}
impl GameState {
    fn new() -> Self {
        GameState {
            unit_instances: Stash::<SCUnit>::new(),
            selected_units: Vec::<usize>::new(),
            game_events: Vec::<GameEvents>::new(),
            map_pos: Point::new(0, 0),
        }
    }
}


pub trait LayerTrait {
    fn render(&self, renderer: &mut Renderer);
    fn update(&mut self, gd: &GameData, gc: &mut GameContext, state: &mut GameState);
    fn generate_events(&mut self, gc: &GameContext, state: &GameState) -> Vec<GameEvents>;

    /// return true when processed, false if not
    fn process_event(&mut self, event: &GameEvents) -> bool;
}


pub struct GameContext<'window> {
    // gamedata is ref-counted so that refs can be kept by other objects
    // pub gd: Rc<GameData>,
    pub events: Events,
    pub renderer: Renderer<'window>,
    pub screen: Surface<'window>, /* pub game_events: Vec<GameEvents>,
                                   *
                                   * pub map_pos: Point,
                                   *
                                   * debug stuff
                                   * pub timer: Timer<'window>, */
}
impl<'window> GameContext<'window> {
    fn new(//gd: GameData,
           events: Events,
           renderer: Renderer<'window> /* timer: Timer<'window> */)
           -> GameContext<'window> {
        GameContext {
            // gd: Rc::new(gd),
            events: events,
            renderer: renderer,
            screen: Surface::new(640, 480, PixelFormatEnum::Index8).unwrap(), // timer: timer,
        }
    }

    pub fn output_size(&self) -> (u32, u32) {
        let (w, h) = self.renderer.output_size().unwrap();
        (w, h)
    }
}

pub enum ViewAction {
    None,
    Quit,
    ChangeView(Box<View>),
}

pub trait View {
    fn update(&mut self, _: &GameData, _: &mut GameContext, _: &mut GameState) {}

    /// renders the current view into context.screen
    fn render(&mut self, gd: &GameData, context: &mut GameContext, state: &GameState, elapsed: f64) -> ViewAction;

    fn render_layers(&mut self, _: &mut GameContext) {}

    fn generate_layer_events(&mut self, _: &mut GameContext, _: &mut GameState) {}

    fn process_layer_events(&mut self, _: &mut GameContext, _: &mut GameState) {}
}

// useful links for SDL2 & 8bit rendering:
// http://comments.gmane.org/gmane.comp.lib.sdl/64885
//
// My quick experiment used a reusable SDL_Surface to hold the 8-bit greyscale pixels. Using
// SDL_GetTicks(), it seems pretty clear that, on my system, using:
//
// SDL_Texture *t8 = SDL_CreateTextureFromSurface(renderer, surf8);
//
// SDL_SetRenderTarget(renderer, texture);
// SDL_RenderCopy(renderer, t8, NULL, NULL);
// SDL_SetRenderTarget(renderer, NULL);
//
// SDL_DestroyTexture(t8);
//



pub fn spawn<F>(title: &str, scdata_path: &str, init: F)
    where F: Fn(&GameData, &mut GameContext, &mut GameState) -> Box<View>
{
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem.window(title, 640, 480)
        .position_centered()
        //.fullscreen()
        .opengl()
        .build()
        .unwrap();

    let mut timer = sdl_context.timer().unwrap();

    let gd = GameData::init(&Path::new(scdata_path));

    // FIXME: set a default palette for screen surface
    let mut context = GameContext::new(Events::new(sdl_context.event_pump().unwrap()),
                                       window.renderer().accelerated().build().unwrap());
    sdl_context.mouse().show_cursor(false);
    let mut state = GameState::new();
    let mut current_view = init(&gd, &mut context, &mut state);

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
        current_view.update(&gd, &mut context, &mut state);

        current_view.generate_layer_events(&mut context, &mut state);
        current_view.process_layer_events(&mut context, &mut state);

        let start = timer.ticks();
        let render_res = current_view.render(&gd, &mut context, &state, elapsed);
        let end = timer.ticks();
        render_time_sum += end - start;
        render_count += 1;
        // println!("render function took {} ms", dur);

        // let start = timer.ticks();
        {
            // XXX make screen another layer
            let t8 = context.renderer.create_texture_from_surface(&context.screen).unwrap();
            let _ = context.renderer.copy(&t8, None, None);

            current_view.render_layers(&mut context);

            context.renderer.present();
        }
        // let end = timer.ticks();
        // println!("rendering took {} ticks", end-start);

        match render_res {
            ViewAction::None => context.renderer.present(),
            ViewAction::Quit => break,
            ViewAction::ChangeView(new_view) => current_view = new_view,
        }

    }
}
