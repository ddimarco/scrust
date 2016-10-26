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

//
use std::collections::BinaryHeap;
use std::collections::BTreeSet;
use std::collections::BTreeMap;
use std::cmp::Ordering;

fn megatilexy_to_idx(x: i32, y: i32, map_w: usize) -> usize {
    assert!(x >= 0 && y >= 0);
    y as usize * map_w + x as usize
}

fn idx_euclid_dist(idx1: usize, idx2: usize, mapwidth: usize) -> f32 {
    let (x1, y1) = (idx1 % mapwidth, idx1 / mapwidth);
    let (x2, y2) = (idx2 % mapwidth, idx2 / mapwidth);

    (x1 as f32 - x2 as f32).hypot(
        y1 as f32 - y2 as f32)
}
fn idx_manhattan_dist(idx1: usize, idx2: usize, mapwidth: usize) -> f32 {
    let (x1, y1) = (idx1 % mapwidth, idx1 / mapwidth);
    let (x2, y2) = (idx2 % mapwidth, idx2 / mapwidth);

    (x1 as f32 - x2 as f32).abs() + (y1 as f32 - y2 as f32).abs()
}

struct PrioIndex(f32, usize);

impl Eq for PrioIndex {}
impl PartialEq for PrioIndex {
    fn eq(&self, other: &PrioIndex) -> bool {
        self.0 == other.0
    }
}
/// reversed ordering (for binheap)
impl PartialOrd for PrioIndex {
    fn partial_cmp(&self, other: &PrioIndex) -> Option<Ordering> {
        let (a, b) = (self.0, other.0);
        if a < b {
            Some(Ordering::Less)
        } else if a > b {
            Some(Ordering::Greater)
        } else {
            None
        }
    }
    fn lt(&self, other: &PrioIndex) -> bool { self.0 > other.0 }
    fn le(&self, other: &PrioIndex) -> bool { self.0 >= other.0 }
    fn gt(&self, other: &PrioIndex) -> bool { self.0 < other.0 }
    fn ge(&self, other: &PrioIndex) -> bool { self.0 <= other.0 }
}
impl Ord for PrioIndex {
    fn cmp(&self, other: &PrioIndex) -> Ordering {
        match self.partial_cmp(&other) {
            Some(ord) => ord,
            None => unreachable!(),
        }
    }
}

fn naive_a_star(start: &Point, end: &Point, map: &Map) -> (Vec<Point>, Vec<Point>) {
    let mut considered = Vec::<Point>::new();

    /// heap mapping fscore to indices of nodes
    let mut prio_heap = BinaryHeap::<PrioIndex>::new();
    /// open set (on indices of nodes)
    let mut openset = BTreeSet::<usize>::new();
    let mut closedset = BTreeSet::<usize>::new();
    /// cost start -> node
    let mut gscore = BTreeMap::<usize, f32>::new();
    /// cost start -> goal via this node
    let mut fscore = BTreeMap::<usize, f32>::new();

    let mut best_reachable_from = BTreeMap::<usize, usize>::new();


    let idx_dist = idx_manhattan_dist;

    let map_width = map.data.width as usize;
    let diag_dist = 2f32.sqrt();
    let passable_tiles = &map.passable_megatiles;

    let start_idx = megatilexy_to_idx(start.x(), start.y(), map_width);
    let end_idx = megatilexy_to_idx(end.x(), end.y(), map_width);

    if !passable_tiles[start_idx] || !passable_tiles[end_idx] {
        println!("start or end impassable!");
        return (considered, Vec::<Point>::new());
    }

    gscore.insert(start_idx, 0.0);
    fscore.insert(start_idx, idx_dist(start_idx, end_idx, map_width));
    println!("path dist: {:?}", fscore.get(&start_idx).unwrap());

    prio_heap.push(PrioIndex(0.0, start_idx));
    openset.insert(start_idx);

    let max_val = 10000f32;


    while let Some(current) = prio_heap.pop() {
        let current_idx = current.1;
        if closedset.contains(&current_idx) {
            // we already got here before
            continue;
        }
        let (x,y) = (current_idx % map_width, current_idx / map_width);
        // XXX debug
        considered.push(Point::new(x as i32, y as i32));

        if current_idx == end_idx {
            println!("path found, considered {} nodes", considered.len());
            let mut result = Vec::<Point>::new();

            let mut idx = end_idx;
            while idx != start_idx {
                let (x,y) = (idx % map_width, idx / map_width);
                result.push(Point::new(x as i32, y as i32));
                idx = *best_reachable_from.get(&idx).unwrap();
            }

            return (result, considered);
        }

        openset.remove(&current_idx);
        closedset.insert(current_idx);
        // check neighbors
        let offvec = [(-1, 0, 1.), (-1, -1, diag_dist), (-1, 1, diag_dist),
                      (0, 1, 1.), (0, -1, 1.),
                      (1, -1, diag_dist), (1, 0, 1.), (1, 1, diag_dist)];
        for noff in &offvec {
            let nx = x as i32 + noff.0;
            let ny = y as i32 + noff.1;
            let ndist = noff.2;
            if nx < 0 || ny < 0 {
                continue;
            }
            let nidx = megatilexy_to_idx(nx, ny, map_width);
            if closedset.contains(&nidx) || !passable_tiles[nidx] {
                continue;
            }

            let ngscore = gscore.get(&current_idx).unwrap() + ndist;
            if ngscore >= *gscore.get(&nidx).unwrap_or(&max_val) {
                // not a good path
                continue;
            }

            let nfscore = ngscore + idx_dist(nidx, end_idx, map_width);
            if !openset.contains(&nidx) {
                openset.insert(nidx);
            }
            // if the openset already contains the node, just push it with the new fscore
            // the one with the lower fscore will be selected first anyway
            prio_heap.push(PrioIndex(nfscore, nidx));
            best_reachable_from.insert(nidx, current_idx);
            fscore.insert(nidx, nfscore);
            gscore.insert(nidx, ngscore);
        }

    }

    println!("no path found!");
    (Vec::<Point>::new(), considered)
}


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
                                     self.path.len(), self.considered.len());
                fnt.render_textbox(pp_txt.as_ref(), 0, fnt_reindex, buffer, screen_pitch,
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
            let (p, c) = naive_a_star(&self.start_tile.unwrap(), &self.end_tile.unwrap(), &self.map);
            self.path = p;
            self.considered = c;
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
