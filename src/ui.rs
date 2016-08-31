extern crate sdl2;
use sdl2::rect::{Rect, Point};
use sdl2::render::{Renderer, Texture};
use sdl2::pixels::Color;

use ::grp::GRP;
use ::pal::{Palette, palimg_to_texture};
use ::pcx::PCX;
use ::{GameContext, LayerTrait, GameEvents, MousePointerType};
use ::terrain::Map;

use std::cmp::{min, max};

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

fn mouse_pointer_type_to_file(tpe: MousePointerType) -> &'static str {
    match tpe {
        MousePointerType::Arrow => "cursor/arrow.grp",
        MousePointerType::ScrollLeft => "cursor/scrolll.grp",
        MousePointerType::ScrollRight => "cursor/scrollr.grp",
        MousePointerType::ScrollUp => "cursor/scrollu.grp",
        MousePointerType::ScrollDown => "cursor/scrolld.grp",
        MousePointerType::ScrollDownLeft => "cursor/scrolldl.grp",
        MousePointerType::ScrollDownRight => "cursor/scrolldr.grp",
        MousePointerType::ScrollUpLeft => "cursor/scrollul.grp",
        MousePointerType::ScrollUpRight => "cursor/scrollur.grp",
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

                    MousePointerType::ScrollDownLeft,
                    MousePointerType::ScrollDownRight,
                    MousePointerType::ScrollUpLeft,
                    MousePointerType::ScrollUpRight,

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
        if tpe != self.cursor_type {
            self.cursor_type = tpe;
            self.frame_idx = 0;
        }
    }
}


struct MiniMap {
    minimap: Texture,
    mmapwratio: f32,
    mmaphratio: f32,
    mmap_cur_rect: Rect,
    mmap_rect: Rect,
    map_size: Point,
}
// FIXME: move into common const module?
const MAP_RENDER_W: u16 = 20;
const MAP_RENDER_H: u16 = 12;
impl MiniMap {
    fn new(context: &mut GameContext, map: &Map) -> Self {
        let mmap_bmp = map.render_minimap();
        let mmap = palimg_to_texture(&mut context.renderer,
                                     map.data.width as u32, map.data.height as u32,
                                     &mmap_bmp, &map.terrain_info.pal);

        let mapw2mmapw_ratio: f32 = 128. / (map.data.width as f32);
        let maph2mmaph_ratio: f32 = 128. / (map.data.height as f32);

        let mmap_cur_rect = Rect::new(0, 0,
                                      (MAP_RENDER_W as f32 * mapw2mmapw_ratio) as u32,
                                      (MAP_RENDER_H as f32 * maph2mmaph_ratio) as u32);

        MiniMap {
            minimap: mmap,
            mmap_rect: Rect::new(6, 348, 128, 128),
            mmap_cur_rect: mmap_cur_rect,
            mmapwratio: mapw2mmapw_ratio,
            mmaphratio: maph2mmaph_ratio,
            map_size: Point::new(map.data.width as i32, map.data.height as i32),
        }
    }

    fn minimap_to_map_coords(&self, screen_pt: &Point) -> Option<Point> {
        if !self.mmap_rect.contains(*screen_pt) {
            None
        } else {
            let screen_offset = *screen_pt - self.mmap_rect.top_left();

            let mapx = (screen_offset.x() as f32 * 32. / self.mmapwratio) as i32
                - (MAP_RENDER_W * 32) as i32/2
                ;
            let mapy = (screen_offset.y() as f32 * 32. / self.mmaphratio) as i32
                - (MAP_RENDER_H * 32) as i32/2
                ;

            Some(Point::new(min(max(mapx as i32, 0),
                                ((self.map_size.x() - 1 - MAP_RENDER_W as i32)) * 32),
                            min(max(mapy as i32, 0),
                                (self.map_size.y() - 1 - MAP_RENDER_H as i32) * 32)
            ))
        }
    }

    fn update(&mut self, map_x: u16, map_y: u16) {
        let new_x = 6 + (map_x as f32 * self.mmapwratio / 32.) as i32;
        let new_y = 348 + (map_y as f32 * self.mmaphratio / 32.) as i32;
        self.mmap_cur_rect.set_x(new_x);
        self.mmap_cur_rect.set_y(new_y);
    }

    fn render(&self, renderer: &mut Renderer) {
        renderer.copy(&self.minimap, None, Some(self.mmap_rect));

        renderer.set_draw_color(Color::RGB(255, 255, 255));
        renderer.draw_rect(self.mmap_cur_rect);
    }
}

pub struct UiLayer {
    pub mp: MousePointer,
    ticks: u16,
    hud_texture: Texture,
    hud_rect: Rect,
    minimap: MiniMap,
}
impl UiLayer {
    pub fn new(context: &mut GameContext, map: &Map) -> UiLayer {
        let hud = PCX::read(&mut context.gd.open("game/tconsole.pcx").unwrap());
        let text = palimg_to_texture(&mut context.renderer,
                                     hud.header.width as u32,
                                     hud.header.height as u32,
                                     &hud.data,
                                     &hud.palette);
        let minimap = MiniMap::new(context, &map);

        UiLayer {
            mp: MousePointer::new(context),
            ticks: 0,
            hud_texture: text,
            hud_rect: Rect::new(0, 0, 640, 480),
            minimap: minimap,
        }
    }

    fn make_map_move_from_scroll(&self, scroll_horizontal: i16, scroll_vertical: i16,
                                 gc: &GameContext) -> GameEvents {
        const SCROLLING_SPEED: i32 = 10;
        let map_x =
            if scroll_horizontal < 0 {
                max(0, (gc.map_pos.x() -
                        SCROLLING_SPEED as i32))
            } else if scroll_horizontal > 0 {
                min((self.minimap.map_size.x() - MAP_RENDER_W as i32) * 32,
                    gc.map_pos.x() + SCROLLING_SPEED)
            } else {
                gc.map_pos.x()
            };

        let map_y =
            if scroll_vertical < 0 {
                max(0, (gc.map_pos.y() -
                        SCROLLING_SPEED as i32))
            } else if scroll_vertical > 0 {
                min((self.minimap.map_size.y() - MAP_RENDER_H as i32) * 32,
                    gc.map_pos.y() + SCROLLING_SPEED)
            } else {
                gc.map_pos.y()
            };
        GameEvents::MoveMap(map_x, map_y)
    }

}
impl LayerTrait for UiLayer {
    fn update(&mut self, gc: &GameContext) {
        self.minimap.update(gc.map_pos.x() as u16, gc.map_pos.y() as u16);

        if let Some((mouse_x, mouse_y)) = gc.events.now.mouse_move {
            self.mp.update_pos(mouse_x, mouse_y);
        }

        self.ticks = (self.ticks + 1) % 1000;
        if self.ticks % 10 == 0 {
            self.mp.update();
        }
    }
    fn process_event(&mut self, event: &GameEvents) -> bool {
        match *event {
            GameEvents::ChangeMouseCursor(tpe) => {
                self.mp.set_type(tpe);
                true
            },
            _ => {
                false
            }
        }
    }

    fn generate_events(&self, gc: &GameContext) -> Vec<GameEvents> {
        let mut events = Vec::<GameEvents>::new();
        let mpos = gc.events.mouse_pos;

        let scroll_horizontal =
            if mpos.x() > 620 {
                1
            } else if mpos.x() < 20 {
                -1
            } else {
                0
            };
        let scroll_vertical =
            if mpos.y() > 460 {
                1
            } else if mpos.y() < 20 {
                -1
            } else {
                0
            };
        if scroll_vertical != 0 || scroll_horizontal != 0 {
            let mpt: MousePointerType = if scroll_vertical > 0 {
                // down
                if scroll_horizontal > 0 {
                    // right
                    MousePointerType::ScrollDownRight
                } else if scroll_horizontal < 0 {
                    // left
                    MousePointerType::ScrollDownLeft
                } else {
                    MousePointerType::ScrollDown
                }
            } else if scroll_vertical < 0 {
                // up
                if scroll_horizontal > 0 {
                    // right
                    MousePointerType::ScrollUpRight
                } else if scroll_horizontal < 0 {
                    // left
                    MousePointerType::ScrollUpLeft
                } else {
                    MousePointerType::ScrollUp
                }
            } else if scroll_horizontal > 0 {
                // only right
                MousePointerType::ScrollRight
            } else if scroll_horizontal < 0 {
                MousePointerType::ScrollLeft
            } else {
                panic!("logic error!");
            };
            events.push(GameEvents::ChangeMouseCursor(mpt));

            events.push(self.make_map_move_from_scroll(scroll_horizontal, scroll_vertical,
                                                       gc));

        } else if scroll_vertical == 0 && scroll_horizontal == 0 {
            // stop scrolling
            events.push(GameEvents::ChangeMouseCursor(MousePointerType::Arrow));
        }

        // minimap click events
        if gc.events.now.mouse_left {
            match self.minimap.minimap_to_map_coords(&gc.events.mouse_pos) {
                Some(map_pos) => {
                    events.push(GameEvents::MoveMap(map_pos.x() , map_pos.y() ));
                },
                None => {},
            }
        }

        // keyboard events
        {
            let scroll_horizontal = if gc.events.key_right {
                1
            } else if gc.events.key_left {
                -1
            } else {
                0
            };
            let scroll_vertical = if gc.events.key_down {
                1
            } else if gc.events.key_up {
                -1
            } else {
                0
            };
            if scroll_horizontal != 0 || scroll_vertical != 0 {
                events.push(self.make_map_move_from_scroll(scroll_horizontal,
                                                           scroll_vertical,
                                                           gc));
            }
        }

        events
    }
    fn render(&self, renderer: &mut Renderer) {
        renderer.copy(&self.hud_texture, None, Some(self.hud_rect));
        self.minimap.render(renderer);
        self.mp.render(renderer);
    }
}
