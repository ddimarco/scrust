extern crate sdl2;
use sdl2::rect::Rect;
use sdl2::render::{Renderer, Texture};

use ::grp::GRP;
use ::pal::{Palette, palimg_to_texture};
use ::pcx::PCX;
use ::{GameContext, LayerTrait};


fn grp_to_textures(renderer: &mut Renderer, grp: &GRP, pal: &Palette) -> Vec<Texture> {
    let header = &grp.header;
    let mut res = Vec::<Texture>::with_capacity(header.frame_count);
    for framedata in &grp.frames {
        let text = palimg_to_texture(renderer,
                                     header.width as u32,
                                     header.height as u32,
                                     framedata,
                                     pal);
        res.push(text);
    }
    res
}

#[derive(Copy,Clone)]
pub enum MousePointerType {
    Arrow = 0,
    ScrollLeft,
    ScrollRight,
    ScrollUp,
    ScrollDown,
    /// XXX some scrolling missing
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
fn mouse_pointer_type_to_file(tpe: MousePointerType) -> &'static str {
    match tpe {
        MousePointerType::Arrow => "cursor/arrow.grp",
        MousePointerType::ScrollLeft => "cursor/scrolll.grp",
        MousePointerType::ScrollRight => "cursor/scrollr.grp",
        MousePointerType::ScrollUp => "cursor/scrollu.grp",
        MousePointerType::ScrollDown => "cursor/scrolld.grp",
        MousePointerType::Drag => "cursor/drag.grp",
        MousePointerType::Illegal => "cursor/illegal.grp",
        MousePointerType::Time => "cursor/time.grp",
        MousePointerType::TargetGreen => "cursor/targg.grp",
        MousePointerType::TargetYellow => "cursor/targn.grp",
        MousePointerType::TargetRed => "cursor/targr.grp",
        MousePointerType::TargetYellowStatic => "cursor/targy.grp",
        MousePointerType::MagnifierGreen => "cursor/magg.grp",
        MousePointerType::MagnifierRed => "cursor/magr.grp",
        MousePointerType::MagnifierYellow => "cursor/magy.grp",
    }
}


pub struct MousePointer {
    frame_idx: usize,
    cursor_type: MousePointerType,
    textures: Vec<Vec<Texture>>,
    rect: sdl2::rect::Rect,
}
impl MousePointer {
    pub fn new(gc: &mut GameContext) -> MousePointer {
        let mut all_texts = Vec::<Vec<Texture>>::new();
        for mpt in [MousePointerType::Arrow,
                    MousePointerType::ScrollLeft,
                    MousePointerType::ScrollRight,
                    MousePointerType::ScrollUp,
                    MousePointerType::ScrollDown,
                    MousePointerType::Drag,
                    MousePointerType::Illegal,
                    MousePointerType::Time,
                    MousePointerType::TargetGreen,
                    MousePointerType::TargetYellow,
                    MousePointerType::TargetRed,
                    MousePointerType::TargetYellowStatic,
                    // when over own units
                    MousePointerType::MagnifierGreen,
                    MousePointerType::MagnifierRed,
                    // when over resources
                    MousePointerType::MagnifierYellow]
            .iter() {
            let grp = GRP::read(&mut gc.gd.open(mouse_pointer_type_to_file(*mpt)).unwrap());
            // XXX hardcoded palette
            let textures = grp_to_textures(&mut gc.renderer, &grp, &gc.gd.install_pal);
            all_texts.push(textures);
        }

        MousePointer {
            frame_idx: 0,
            textures: all_texts,
            cursor_type: MousePointerType::Arrow,
            rect: sdl2::rect::Rect::new(0, 0, 128, 128),
        }
    }

    pub fn render(&self, renderer: &mut Renderer) {
        let cursor_idx = self.cursor_type as usize;
        let ref texture = self.textures[cursor_idx];
        renderer.copy(&texture[self.frame_idx], None, Some(self.rect));
    }

    pub fn update(&mut self) {
        let cursor_idx = self.cursor_type as usize;
        self.frame_idx = (self.frame_idx + 1) % self.textures[cursor_idx].len();
    }

    pub fn update_pos(&mut self, x: i32, y: i32) {
        self.rect.set_x(x - 64);
        self.rect.set_y(y - 64);
    }

    pub fn set_type(&mut self, tpe: MousePointerType) {
        self.cursor_type = tpe;
    }
}

pub struct UiLayer {
    pub mp: MousePointer,
    ticks: u16,
    hud_texture: Texture,
    hud_rect: Rect,
}
impl UiLayer {
    pub fn new(context: &mut GameContext) -> UiLayer {
        let hud = PCX::read(&mut context.gd.open("game/tconsole.pcx").unwrap());
        let text = palimg_to_texture(&mut context.renderer,
                                     hud.header.width as u32,
                                     hud.header.height as u32,
                                     &hud.data,
                                     &hud.palette);

        UiLayer {
            mp: MousePointer::new(context),
            ticks: 0,
            hud_texture: text,
            hud_rect: Rect::new(0, 0, 640, 480),
        }
    }
}
impl LayerTrait for UiLayer {
    fn update(&mut self, gc: &GameContext) {
        if let Some((mouse_x, mouse_y)) = gc.events.now.mouse_move {
            self.mp.update_pos(mouse_x, mouse_y);
        }

        self.ticks = (self.ticks + 1) % 1000;
        if self.ticks % 10 == 0 {
            self.mp.update();
        }
    }
    fn render(&self, renderer: &mut Renderer) {
        renderer.copy(&self.hud_texture, None, Some(self.hud_rect));
        self.mp.render(renderer);
    }
}
