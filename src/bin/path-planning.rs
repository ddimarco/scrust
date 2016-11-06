use std::env;

#[macro_use]
extern crate scrust;
use scrust::{GameContext, GameState, View, ViewAction, GameEvents};
use scrust::terrain::Map;
use scrust::font::{FontSize, RenderText};

use scrust::LayerTrait;
use scrust::ui::UiLayer;

extern crate sdl2;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};

extern crate pathplanning;
use pathplanning::jps::{jps_a_star, PlanningMapTrait};

struct MapView {
    map: Map,

    start_tile: Option<Point>,
    end_tile: Option<Point>,
    path: Vec<Point>,
    considered: Vec<Point>,

    ui_layer: UiLayer,
}
const MAP_RENDER_W: u16 = 20;
const MAP_RENDER_H: u16 = 12;
impl MapView {
    fn new(context: &mut GameContext, _: &mut GameState, mapfn: &str) -> Self {
        let map = Map::read(&context.gd, mapfn);
        println!("map name: {}", map.name());
        println!("map desc: {}", map.description());
        context.screen.set_palette(&map.terrain_info.pal.to_sdl()).ok();
        let ui_layer = UiLayer::new(context, &map);

        MapView {
            map: map,
            ui_layer: ui_layer,
            start_tile: None,
            end_tile: None,
            path: Vec::<Point>::new(),
            considered: Vec::<Point>::new(),
        }
    }
}
impl View for MapView {
    fn update(&mut self, context: &mut GameContext, state: &mut GameState) {
        self.ui_layer.update(context, state);
    }
    fn render(&mut self, context: &mut GameContext, state: &GameState, _: f64) -> ViewAction {
        if context.events.now.quit || context.events.now.key_escape == Some(true) {
            return ViewAction::Quit;
        }
        let planning_str_rect = Rect::new(50, 300, 300, 50);
        let gd = &context.gd;
        let fnt = gd.font(FontSize::Font16);
        let fnt_reindex = &gd.font_reindex.data;

        // clear the screen
        context.screen.fill_rect(None, Color::RGB(0, 0, 0)).ok();
        let screen_pitch = context.screen.pitch();

        let map_x = state.map_pos.x() as u16;
        let map_y = state.map_pos.y() as u16;

        {
            let mut screen = &mut context.screen;
            screen.with_lock_mut(|buffer: &mut [u8]| {
                self.map.render(map_x,
                                map_y,
                                MAP_RENDER_W,
                                MAP_RENDER_H,
                                buffer,
                                screen_pitch);

                // draw opened nodes
                for spt in &self.considered {
                    let tl_x = spt.x() * 32 - state.map_pos.x();
                    let tl_y = spt.y() * 32 - state.map_pos.y();
                    self.map.mark_megatile(buffer, tl_x, tl_y, 10);
                }

                for spt in &self.path {
                    let tl_x = spt.x() * 32 - state.map_pos.x();
                    let tl_y = spt.y() * 32 - state.map_pos.y();
                    self.map.mark_megatile(buffer, tl_x, tl_y, 17);
                }

                // draw start/end points
                if let Some(spt) = self.start_tile {
                    let tl_x = spt.x() * 32 - state.map_pos.x();
                    let tl_y = spt.y() * 32 - state.map_pos.y();
                    self.map.mark_megatile(buffer, tl_x, tl_y, 117u8);
                }
                if let Some(spt) = self.end_tile {
                    let tl_x = spt.x() * 32 - state.map_pos.x();
                    let tl_y = spt.y() * 32 - state.map_pos.y();
                    self.map.mark_megatile(buffer, tl_x, tl_y, 23);
                }

                // diagnostic text
                let pp_txt = format!("path length: {}, considered {} nodes",
                                     self.path.len(),
                                     self.considered.len());
                fnt.render_textbox(pp_txt.as_ref(),
                                   0,
                                   fnt_reindex,
                                   buffer,
                                   screen_pitch,
                                   &planning_str_rect);

            });

        }


        ViewAction::None
    }


    fn render_layers(&mut self, context: &mut GameContext) {
        self.ui_layer.render(&mut context.renderer);
    }
    fn process_layer_events(&mut self, _: &mut GameContext, state: &mut GameState) {
        for ev in &state.game_events {
            if self.ui_layer.process_event(ev) {
                continue;
            }

            match *ev {
                GameEvents::MoveMap(x, y) => {
                    state.map_pos = Point::new(x, y);
                }
                _ => {}
            }
        }


        state.game_events.clear();
    }

    fn generate_layer_events(&mut self, context: &mut GameContext, state: &mut GameState) {
        let vecevents = self.ui_layer.generate_events(context, state);
        state.game_events.extend(vecevents);

        let mpos_map = state.map_pos + context.events.mouse_pos;
        let mut new_problem = false;
        if context.events.now.mouse_left {
            // set start pos
            let megax = mpos_map.x() / 32;
            let megay = mpos_map.y() / 32;
            self.start_tile = Some(Point::new(megax, megay));
            println!("start tile selected: {}, {}", megax, megay);
            new_problem = true;
        }
        if context.events.now.mouse_right {
            // set start pos
            let megax = mpos_map.x() / 32;
            let megay = mpos_map.y() / 32;
            self.end_tile = Some(Point::new(megax, megay));
            println!("end tile selected: {}, {}", megax, megay);
            new_problem = true;
        }

        if new_problem && self.start_tile.is_some() && self.end_tile.is_some() {
            // replan
            let s = &self.start_tile.unwrap();
            let e = &self.end_tile.unwrap();
            {
                println!("planning problem:");
                let cons_pmap = vec![(self.map.xy2idx(s.x(), s.y()), 's'),
                                         (self.map.xy2idx(e.x(), e.y()), 'e')];
                self.map.print(&cons_pmap);
            }
            let (p, c) = jps_a_star(
                self.map.xy2idx(s.x(), s.y()),
                self.map.xy2idx(e.x(), e.y()),
                &self.map);

            {
                println!("resulting path:");
                let mut cons_pmap = vec![(self.map.xy2idx(s.x(), s.y()), 's'),
                                         (self.map.xy2idx(e.x(), e.y()), 'e')];
                for c in &p {
                    let cidx = self.map.xy2idx(c.x, c.y);
                    cons_pmap.push((cidx, 'x'));
                }
                self.map.print(&cons_pmap);
            }

            self.path = p.into_iter().map(|pp: pathplanning::jps::Point| {
                Point::new(pp.x, pp.y)
            }).collect();
            self.considered = c.into_iter().map(|pp: pathplanning::jps::Point| {
                Point::new(pp.x, pp.y)
            }).collect();
        }
    }
}


fn main() {
    ::scrust::spawn("path planning",
                    "/home/dm/.wine/drive_c/StarCraft/",
                    |gc, state| {
        let args: Vec<String> = env::args().collect();
        let mapfn = if args.len() == 2 {
            args[1].clone()
        } else {
            String::from("/home/dm/.wine/drive_c/StarCraft/Maps/(2)Challenger.scm")
        };
        Box::new(MapView::new(gc, state, &mapfn))
    });

}

