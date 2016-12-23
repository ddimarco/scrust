extern crate sdl2;
use sdl2::pixels::Color;

extern crate scrust;
use scrust::gamedata::GameData;
use scrust::{GameContext, GameState, View, ViewAction};
use scrust::render::{render_buffer_solid};
use scrust::pal::Palette;

extern crate smacker;
use smacker::{SMK, FrameIterationStatus};

// TODO: convert smks into video structure?
struct SMKView {
    smk: SMK,
}
impl SMKView {
    fn new(gd: &GameData, context: &mut GameContext, smk_filename: &str) -> Self {
        let mut file = gd.open(smk_filename).unwrap();
        let fsize = file.get_filesize();
        let smk = SMK::read(&mut file, fsize);
        let frame = smk.get_frame();
        // assuming palette stays constant
        let pal = Palette::from_buffer(&frame.palette);
        context.screen.set_palette(&pal.to_sdl()).expect("could not set palette!");
        SMKView {
            smk: smk,
        }
    }
}

impl View for SMKView {
    fn render(&mut self, _: &GameData, context: &mut GameContext, _: &GameState, _: f64) -> ViewAction {
        if context.events.now.quit || context.events.now.key_escape == Some(true) {
            return ViewAction::Quit;
        }
        // clear the screen
        context.screen.fill_rect(None, Color::RGB(0, 0, 0)).ok();

        let frame = self.smk.get_frame();
        let data = &frame.data;
        let screen_pitch = context.screen.pitch();
        context.screen.with_lock_mut(|buffer: &mut [u8]| {
            render_buffer_solid(&data, frame.width as u32, frame.height as u32,
                                false,
                                320, 240, buffer, screen_pitch);
        });
        match self.smk.go_next_frame() {
            FrameIterationStatus::Done => {
                self.smk.go_first_frame();
            },
            _ => {}
        }

        ViewAction::None
    }
}

fn main() {
    ::scrust::spawn("smk viewer",
                    "/home/dm/.wine/drive_c/StarCraft/",
                    |gd, gc, _| Box::new(SMKView::new(gd, gc, "glue/mainmenu/Editor.smk")));
}
