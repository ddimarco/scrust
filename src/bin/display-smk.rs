extern crate sdl2;
use sdl2::pixels::Color;

extern crate scrust;
use scrust::gamedata::GameData;
use scrust::{GameContext, GameState, View, ViewAction};
use scrust::render::{render_buffer_solid};
use scrust::Video;

extern crate smacker;
use smacker::SMK;

extern crate scformats;
use scformats::terrain::GameDataTrait;

// TODO: convert smks into video structure?
struct SMKView {
    // smk: SMK,
    video: Video,
    frame: usize,
}
impl SMKView {
    fn new(gd: &GameData, context: &mut GameContext, smk_filename: &str) -> Self {
        let mut file = gd.open(smk_filename).unwrap();
        let fsize = file.get_ref().len();
        let mut smk = SMK::read(&mut file, fsize);

        let video = Video::from_smk(&mut smk);
        // assuming palette stays constant
        context.screen.set_palette(&video.pal.to_sdl()).expect("could not set palette!");
        SMKView {
            // smk: smk,
            video: video,
            frame: 0,
        }
    }
}

impl View for SMKView {
    fn render(&mut self, _: &GameData, context: &mut GameContext, _: &GameState, _: f64) -> ViewAction {
        if context.events.now.quit || context.events.now.is_key_pressed(&sdl2::keyboard::Keycode::Escape) {
            return ViewAction::Quit;
        }
        // clear the screen
        context.screen.fill_rect(None, Color::RGB(0, 0, 0)).ok();

        let data = &self.video.frames[self.frame];
        let screen_pitch = context.screen.pitch();
        context.screen.with_lock_mut(|buffer: &mut [u8]| {
            render_buffer_solid(&data, self.video.width as u32, self.video.height as u32,
                                false,
                                320, 240, buffer, screen_pitch);
        });
        self.frame = (self.frame + 1) % self.video.frames.len();

        ViewAction::None
    }
}

fn main() {
    ::scrust::spawn("smk viewer",
                    "/home/dm/.wine/drive_c/StarCraft/",
                    |gd, gc, _| Box::new(SMKView::new(gd, gc, "glue/mainmenu/Editor.smk")));
}
