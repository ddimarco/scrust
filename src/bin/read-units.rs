use std::env;

extern crate sdl2;
use sdl2::pixels::Color;
use sdl2::rect::Rect;

extern crate scrust;
use scrust::font::FontSize;
use scrust::font::RenderText;
use scrust::{GameContext, GameState, View, ViewAction};
use scrust::iscript::AnimationType;
use scrust::scunits::{SCUnit, IScriptableTrait, SCImageTrait, SCSpriteTrait};


struct UnitsView {
    unit_id: usize,
    current_anim: AnimationType,
    anim_str: String,
    unit_name_str: String,
    unit: SCUnit,
    unit_cx: i32,
    unit_cy: i32,
}
impl UnitsView {
    fn new(gc: &mut GameContext, unit_id: usize) -> UnitsView {
        let current_anim = AnimationType::Init;
        let anim_str = format!("Animation: {:?}", current_anim);
        let unit_name_str = format!("{}: {}", unit_id, gc.gd.stat_txt_tbl[unit_id].to_owned());
        // let flingy_id = gd.units_dat.flingy_id[unit_id];
        // let sprite_id = gd.flingy_dat.sprite_id[flingy_id as usize];
        // let image_id = gd.sprites_dat.image_id[sprite_id as usize];

        // FIXME: move this to some generic initialization function
        let pal = gc.gd.install_pal.to_sdl();
        gc.screen.set_palette(&pal).ok();

        UnitsView {
            unit_id: unit_id,
            current_anim: current_anim,
            anim_str: anim_str,
            unit_name_str: unit_name_str,
            // img: SCImage::new(&gd, image_id),
            // sprite: SCSprite::new(&gd, sprite_id),
            unit: SCUnit::new(&gc.gd, unit_id, 0, 0, 0),
            unit_cx: 100,
            unit_cy: 100,
        }
    }
}
impl View for UnitsView {
    fn render(&mut self, context: &mut GameContext, _: &GameState, _: f64) -> ViewAction {
        if context.events.now.quit || context.events.now.key_escape == Some(true) {
            return ViewAction::Quit;
        }
        // clear the screen
        context.screen.fill_rect(None, Color::RGB(0, 0, 120)).ok();
        let gd = &context.gd;
        if context.events.now.key_n == Some(true) {
            self.unit_id += 1;

            self.unit_name_str = format!("{}: {}",
                                         self.unit_id,
                                         gd.stat_txt_tbl[self.unit_id].to_owned());
            // let flingy_id = gd.units_dat.flingy_id[self.unit_id];
            // let sprite_id = gd.flingy_dat.sprite_id[flingy_id as usize];
            // let image_id = gd.sprites_dat.image_id[sprite_id as usize];
            // self.img = SCImage::new(&gd, image_id);
            // self.sprite = SCSprite::new(&gd, sprite_id);
            self.unit = SCUnit::new(&gd, self.unit_id, 0, 0, 0);
        } else if context.events.now.key_p == Some(true) {
            if self.unit_id > 0 {
                self.unit_id -= 1;

                self.unit_name_str = format!("{}: {}",
                                             self.unit_id,
                                             gd.stat_txt_tbl[self.unit_id].to_owned());
                // let flingy_id = gd.units_dat.flingy_id[self.unit_id];
                // let sprite_id = gd.flingy_dat.sprite_id[flingy_id as usize];
                // let image_id = gd.sprites_dat.image_id[sprite_id as usize];
                // self.img = SCImage::new(&gd, image_id);
                // self.sprite = SCSprite::new(&gd, sprite_id);
                self.unit = SCUnit::new(&gd, self.unit_id, 0, 0, 0);
            }
        }
        if context.events.now.key_q == Some(true) {
            self.unit.get_iscript_state_mut().turn_ccwise(1);
        } else if context.events.now.key_e == Some(true) {
            self.unit.get_iscript_state_mut().turn_cwise(1);
        }
        if context.events.now.key_d == Some(true) {
            self.unit.get_iscript_state_mut().set_animation(AnimationType::Death);
        } else if context.events.now.key_w == Some(true) {
            self.unit.get_iscript_state_mut().set_animation(AnimationType::Walking);
        }

        if context.events.now.key_up == Some(true) {
            self.unit_cy -= 1;
        } else if context.events.now.key_down == Some(true) {
            self.unit_cy += 1;
        }
        if context.events.now.key_left == Some(true) {
            self.unit_cx -= 1;
        } else if context.events.now.key_right == Some(true) {
            self.unit_cx += 1;
        }

        if context.events.now.key_c == Some(true) {
            self.unit.get_scimg_mut().player_id += 1;
        }

        {
            self.unit.get_scimg_mut().step(&gd);
            {
                let anim = self.unit.get_iscript_state().current_animation();
                if anim != self.current_anim {
                    println!("--- current animation: {:?} ---", anim);
                    self.current_anim = anim;
                    self.anim_str = format!("Current Animation: {:?}", self.current_anim);
                }
            }
        }

        let fnt = gd.font(FontSize::Font16);
        let screen_pitch = context.screen.pitch();
        let fnt_reindex = &gd.font_reindex.data;

        let unitname_rect = Rect::new(10, 10, 300, 50);
        let animstr_rect = Rect::new(10, 300, 300, 50);

        let grp_cache = &*context.gd.grp_cache.borrow();

        context.screen.with_lock_mut(|buffer: &mut [u8]| {
            // unit name
            fnt.render_textbox(self.unit_name_str.as_ref(),
                               1,
                               fnt_reindex,
                               buffer,
                               screen_pitch,
                               &unitname_rect);
            // animation
            fnt.render_textbox(self.anim_str.as_ref(),
                               0,
                               fnt_reindex,
                               buffer,
                               screen_pitch,
                               &animstr_rect);

            self.unit.get_scsprite().draw_selection_circle(&grp_cache,
                                                           self.unit_cx,
                                                           self.unit_cy,
                                                           buffer,
                                                           screen_pitch);
            // unit
            self.unit.get_scimg().draw(grp_cache, self.unit_cx, self.unit_cy, buffer, screen_pitch);

            self.unit.get_scsprite().draw_healthbar(100, 140, buffer, screen_pitch);

        });

        ViewAction::None
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let unit_id = if args.len() == 2 {
        args[1].parse::<usize>().unwrap()
    } else {
        0
    };
    ::scrust::spawn("units rendering",
                    "/home/dm/.wine/drive_c/StarCraft/",
                    |gc, _| Box::new(UnitsView::new(gc, unit_id)));
}
