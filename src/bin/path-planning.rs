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

    (x1 as f32 - x2 as f32).hypot(y1 as f32 - y2 as f32)
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
// XXX consider writing own heap structure (allowing update of priority)
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
    fn lt(&self, other: &PrioIndex) -> bool {
        self.0 > other.0
    }
    fn le(&self, other: &PrioIndex) -> bool {
        self.0 >= other.0
    }
    fn gt(&self, other: &PrioIndex) -> bool {
        self.0 < other.0
    }
    fn ge(&self, other: &PrioIndex) -> bool {
        self.0 <= other.0
    }
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
        let (x, y) = (current_idx % map_width, current_idx / map_width);
        // XXX debug
        considered.push(Point::new(x as i32, y as i32));

        if current_idx == end_idx {
            println!("path found, considered {} nodes", considered.len());
            let mut result = Vec::<Point>::new();

            let mut idx = end_idx;
            while idx != start_idx {
                let (x, y) = (idx % map_width, idx / map_width);
                result.push(Point::new(x as i32, y as i32));
                idx = *best_reachable_from.get(&idx).unwrap();
            }

            return (result, considered);
        }

        openset.remove(&current_idx);
        closedset.insert(current_idx);
        // check neighbors
        let offvec = [(-1, 0, 1.),
                      (-1, -1, diag_dist),
                      (-1, 1, diag_dist),
                      (0, 1, 1.),
                      (0, -1, 1.),
                      (1, -1, diag_dist),
                      (1, 0, 1.),
                      (1, 1, diag_dist)];
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

/// ///////////////////////

struct JPSNode(f32, usize, i32, i32);
impl Eq for JPSNode {}
impl PartialEq for JPSNode {
    fn eq(&self, other: &JPSNode) -> bool {
        self.0 == other.0
    }
}
// XXX consider writing own heap structure (allowing update of priority)
/// reversed ordering (for binheap)
impl PartialOrd for JPSNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let (a, b) = (self.0, other.0);
        if a < b {
            Some(Ordering::Less)
        } else if a > b {
            Some(Ordering::Greater)
        } else {
            None
        }
    }
    fn lt(&self, other: &Self) -> bool {
        self.0 > other.0
    }
    fn le(&self, other: &Self) -> bool {
        self.0 >= other.0
    }
    fn gt(&self, other: &Self) -> bool {
        self.0 < other.0
    }
    fn ge(&self, other: &Self) -> bool {
        self.0 <= other.0
    }
}
impl Ord for JPSNode {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.partial_cmp(&other) {
            Some(ord) => ord,
            None => unreachable!(),
        }
    }
}

fn scan_horizontal(paridx: usize,
                   dist: f32,
                   endidx: usize,
                   dx: i32,
                   passable_tiles: &[bool],
                   map_w: usize,
                   map_h: usize)
                   -> Vec<JPSNode> {
    let mut idx = paridx as i32;
    let mut dist = dist;
    let map_w = map_w as i32;
    let map_size = map_w * map_h as i32;

    let mut nodes = Vec::<JPSNode>::new();

    while nodes.is_empty() {
        idx = idx + dx;
        {
            // let (x, y) = (idx % map_w, idx / map_w);
            // println!("checking {}, {}", x, y);
        }
        if (idx >= map_size as i32) || (idx < 0) {
            // moved out of map bounds
            // println!("out of map bounds");
            return nodes;
        }
        if !passable_tiles[idx as usize] {
            // ran into obstacle
            // println!("ran into obstacle");
            return nodes;
        }

        if idx == endidx as i32 {
            // XXX goal found
            // println!("found goal");
            let n = JPSNode(dist + 1.0, idx as usize, 0, 0);
            nodes.push(n);
            return nodes;
        }

        // open space at idx
        dist += 1.0;
        // two tiles on
        let idx2 = idx + dx;

        // c . _ _
        // b > _ _
        // a . _ _
        //   1 2 3
        // b2 ~ idx
        // b3 ~ idx2
        let a2 = idx + map_w;
        let a3 = idx2 + map_w;

        // below
        if (a2 < map_size) && (a3 < map_size) && (!passable_tiles[a2 as usize]) &&
           (passable_tiles[a3 as usize]) {
            // new jump point at b2
            let n = JPSNode(dist, idx as usize, dx, 1);
           nodes.push(n);
        }

        let c2 = idx - map_w;
        let c3 = idx2 - map_w;
        // above
        if (c2 >= 0) && (c3 >= 0) && (!passable_tiles[c2 as usize]) &&
           (passable_tiles[c3 as usize]) {
           // new jump point at c2
               let n = JPSNode(dist, idx as usize, dx, -1);
               nodes.push(n);
        }

    }
    let n = JPSNode(dist, idx as usize, dx, 0);
    nodes.push(n);
    return nodes;
}


// TODO: merge with scan_horizontal?
fn scan_vertical(paridx: usize,
                 dist: f32,
                 endidx: usize,
                 dy: i32,
                 passable_tiles: &[bool],
                 map_w: usize,
                 map_h: usize)
                 -> Vec<JPSNode> {
    let mut idx = paridx as i32;
    let mut dist = dist;
    let map_w = map_w as i32;
    let map_size = map_w * map_h as i32;

    let mut nodes = Vec::<JPSNode>::new();

    while nodes.is_empty() {
        idx = idx + (dy * map_w);
        {
            // let (x, y) = (idx % map_w, idx / map_w);
            // println!("checking {}, {}", x, y);
        }
        if (idx >= map_size as i32) || (idx < 0) {
            // moved out of map bounds
            // println!("out of map bounds");
            return nodes;
        }
        if !passable_tiles[idx as usize] {
            // ran into obstacle
            // println!("ran into obstacle");
            return nodes;
        }

        if idx == endidx as i32 {
            // XXX goal found
            // println!("found goal");
            nodes.push(JPSNode(dist + 1.0, idx as usize, 0, 0));
            return nodes;
        }

        // open space at idx
        dist += 1.0;
        // two tiles on
        let idx2 = idx + (dy * map_w);

        // c _ _ _
        // b _ _ _
        // a _ _ _
        //   1 2 3
        // b2 ~ idx
        // b3 ~ idx2
        let a2 = idx + 1;
        let a3 = idx2 + 1;

        // to the right
        if (a2 < map_size) && (a3 < map_size) && (!passable_tiles[a2 as usize]) &&
           (passable_tiles[a3 as usize]) {
            // new jump point at b2
               nodes.push(JPSNode(dist, idx as usize, 1, dy));
        }

        let c2 = idx - 1;
        let c3 = idx2 - 1;
        // to the left
        if (c2 >= 0) && (c3 >= 0) && (!passable_tiles[c2 as usize]) &&
           (passable_tiles[c3 as usize]) {
           // new jump point at c2
               nodes.push(JPSNode(dist, idx as usize, -1, dy));
        }

    }
    let n = JPSNode(dist, idx as usize, 0, dy);
    nodes.push(n);
    return nodes;
}

fn scan_diagonal(paridx: usize,
                 dist: f32,
                 endidx: usize,
                 dx: i32,
                 dy: i32,
                 passable_tiles: &[bool],
                 map_w_us: usize,
                 map_h: usize)
                 -> Vec<JPSNode> {
    let mut idx = paridx as i32;
    let mut dist = dist;
    let map_w = map_w_us as i32;
    let map_size = map_w * map_h as i32;
    let diag_dist = 2f32.sqrt();

    let mut nodes = Vec::<JPSNode>::new();

    loop {
        let nidx = idx + dx + dy * map_w;
        {
            let (x, y) = (nidx % map_w, nidx / map_w);
            println!("checking {}, {}", x, y);
        }
        if (nidx >= map_size as i32) || (nidx < 0) {
            println!("out of map bounds");
            return nodes;
        }
        if !passable_tiles[nidx as usize] {
            println!("ran into obstacle");
            return nodes;
        }
        if nidx == endidx as i32 {
            println!("found goal");
            let n = JPSNode(dist + diag_dist, nidx as usize, 0, 0);
            nodes.push(n);
            return nodes;
        }

        // open space at nidx
        dist += diag_dist;

        // 3 _ _ _
        // 2 _ _ _
        // 1 x _ .
        //   a b c
        // idx ~ a1
        // nidx ~ b2
        let a2 = nidx - dx;
        let a3 = a2 - (dy * map_w);

        // 3 _ _ _
        // 2 o _ _
        // 1 x _ .
        //   a b c
        if (a2 >= 0) && (a3 >= 0) && (a2 < map_size) && (a3 < map_size) &&
            !passable_tiles[a2 as usize] && passable_tiles[a3 as usize] {
            nodes.push(JPSNode(dist, nidx as usize, -dx, dy));
        }

        let b1 = idx + dx;
        let c1 = b1 + dx;
        // 3 _ _ _
        // 2 _ _ _
        // 1 x o _
        //   a b c
        if (b1 < map_size) && (c1 < map_size) && (b1 >= 0) && (c1 >= 0) &&
            !passable_tiles[b1 as usize] && passable_tiles[c1 as usize] {
            nodes.push(JPSNode(dist, nidx as usize, dx, -dy));
        }

        let mut hor_done = false;
        let mut vert_done = false;

        if nodes.is_empty() {
            let mut subnodes = scan_horizontal(nidx as usize, dist, endidx, dx, passable_tiles, map_w_us, map_h);
            hor_done = true;
            if !subnodes.is_empty() {
                // FIXME: set parent
                nodes.append(&mut subnodes);
            }
        }

        if nodes.is_empty() {
            let mut subnodes = scan_vertical(nidx as usize, dist, endidx, dy, passable_tiles, map_w_us, map_h);
            vert_done = true;
            if !subnodes.is_empty() {
                // FIXME: set parent
                nodes.append(&mut subnodes);
            }
        }

        if !nodes.is_empty() {
            if !hor_done {
                nodes.push(JPSNode(dist, nidx as usize, dx, 0));
            }
            if !vert_done {
                nodes.push(JPSNode(dist, nidx as usize, 0, dy));
            }
            nodes.push(JPSNode(dist, nidx as usize, dx, dy));
            return nodes;
        }

        idx = nidx;
    }
}

fn neighbors(start_idx: usize, end_idx: usize, passable_tiles: &[bool],
             map_width: usize, map_height: usize) -> Vec<JPSNode> {
    let mut jps_nodes = Vec::<JPSNode>::new();
    jps_nodes.append(&mut scan_horizontal(start_idx, 0f32, end_idx, 1, &passable_tiles,
                                      map_width, map_height));
    jps_nodes.append(&mut scan_horizontal(start_idx, 0f32, end_idx, -1, &passable_tiles,
                                      map_width, map_height));

    jps_nodes.append(&mut scan_vertical(start_idx, 0f32, end_idx, 1, &passable_tiles,
                                        map_width, map_height));
    jps_nodes.append(&mut scan_vertical(start_idx, 0f32, end_idx, -1, &passable_tiles,
                                        map_width, map_height));
    jps_nodes.append(&mut scan_diagonal(start_idx, 0f32, end_idx, 1, -1,
                                        &passable_tiles,
                                        map_width, map_height));
    jps_nodes.append(&mut scan_diagonal(start_idx, 0f32, end_idx, 1, 1,
                                        &passable_tiles,
                                        map_width, map_height));
    jps_nodes.append(&mut scan_diagonal(start_idx, 0f32, end_idx, -1, 1,
                                        &passable_tiles,
                                        map_width, map_height));
    jps_nodes.append(&mut scan_diagonal(start_idx, 0f32, end_idx, -1, -1,
                                        &passable_tiles,
                                        map_width, map_height));
    return jps_nodes;
}

fn jps_a_star(start: &Point, end: &Point, map: &Map) ->  (Vec<Point>, Vec<Point>) {
    let mut considered = Vec::<Point>::new();

    let map_width = map.data.width as usize;
    let map_height = map.data.height as usize;
    let passable_tiles = &map.passable_megatiles;
    // let diag_dist = 2f32.sqrt();

    let start_idx = megatilexy_to_idx(start.x(), start.y(), map_width);
    let end_idx = megatilexy_to_idx(end.x(), end.y(), map_width);

    if !passable_tiles[start_idx] || !passable_tiles[end_idx] {
        println!("start or end impassable!");
        return (considered, Vec::<Point>::new());
    }

    // let mut jps_nodes = Vec::<JPSNode>::new();

    let mut prio_heap = BinaryHeap::<JPSNode>::new();
    let mut openset = BTreeSet::<usize>::new();
    let mut closedset = BTreeSet::<usize>::new();
    /// cost start -> node
    let mut gscore = BTreeMap::<usize, f32>::new();
    /// cost start -> goal via this node
    let mut fscore = BTreeMap::<usize, f32>::new();

    let idx_dist = idx_manhattan_dist;
    gscore.insert(start_idx, 0.0);
    fscore.insert(start_idx, idx_dist(start_idx, end_idx, map_width));
    println!("path dist: {:?}", fscore.get(&start_idx).unwrap());

    for node in neighbors(start_idx, end_idx, &passable_tiles, map_width, map_height) {
        let idx = node.1;
        prio_heap.push(node);
        openset.insert(idx);
    }

    while let Some(current) = prio_heap.pop() {
        let current_idx = current.1;
        {
            let (x, y) = (current_idx % map_width, current_idx / map_width);
            considered.push(Point::new(x as i32, y as i32));
        }

        if current_idx == end_idx {
            println!("path found, considered {} nodes", considered.len());
            // FIXME

            return (Vec::<Point>::new(), considered);
        }

        openset.remove(&current_idx);
        closedset.insert(current_idx);

        // check neighbors
        // direction
        let (dx, dy) = (current.2, current.3);
        let current_dist = current.0;
        let neighbors =
            if dy == 0 && dx != 0 {
                scan_horizontal(current_idx, current_dist, end_idx, dx, passable_tiles,
                                map_width, map_height)
            } else if (dx == 0) && (dy != 0) {
                scan_vertical(current_idx, current_dist, end_idx, dy, passable_tiles,
                              map_width, map_height)
            } else if dx != 0 && dy != 0 {
                scan_diagonal(current_idx, current_dist, end_idx, dx, dy, passable_tiles,
                              map_width, map_height)
            } else {
                unreachable!();
            };

        for n in neighbors {
            // TODO
            prio_heap.push(n);
        }

    }

    println!("no path found!");
    (Vec::<Point>::new(), considered)
}

/// ///////////////////////

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
            // let (p, c) = naive_a_star(&self.start_tile.unwrap(),
            //                           &self.end_tile.unwrap(),
            //                           &self.map);
            let (p, c) = jps_a_star(&self.start_tile.unwrap(),
                                    &self.end_tile.unwrap(),
                                    &self.map);
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

#[cfg(test)]
mod test {
    use std::str::FromStr;
    use std::collections::BTreeSet;
    use super::{scan_horizontal, scan_vertical, scan_diagonal};

    fn parse_strmap(strmap: &[String]) -> (Vec<bool>, usize, usize) {
        let width = strmap[0].len();
        let height = strmap.len();
        let mut passable = Vec::<bool>::with_capacity(width*height);
        for line in strmap {
            assert!(line.len() == width);
            for c in line.chars() {
                let obstacle = c == '#';
                passable.push(!obstacle);
            }
        }
        assert!(passable.len() == width*height);
        (passable, width, height)
    }

    fn xy2idx(x: i32, y: i32, mapw: usize) -> usize {
        (y as usize) * mapw + (x as usize)
    }
    // FIXME: macros to reduce boilerplate
    #[test]
    fn test_scan_hor() {
        let lines = [
            ".#..",
            "....",
            ".#..",
        ];
        let (passable, width, height) = parse_strmap(&lines.into_iter().map(|l| {
            String::from_str(l).unwrap()
        }).collect::<Vec<_>>());

        {
            let startidx = xy2idx(0, 1, width);
            let endidx = xy2idx(width as i32 -1, 1, width);
            let nodes = scan_horizontal(startidx, 0f32, endidx, 1, &passable, width, height);
            let expected_jps_indices: BTreeSet<(usize, i32, i32)> = [
                (xy2idx(1, 1, width), 1, 0),
                (xy2idx(1, 1, width), 1, 1),
                (xy2idx(1, 1, width), 1, -1),
            ].iter().cloned().collect();
            assert!(nodes.len() == expected_jps_indices.len());
            for n in nodes {
                let idx_dir = (n.1, n.2, n.3);
                assert!(expected_jps_indices.contains(&idx_dir));
            }
        }

        {
            let endidx = xy2idx(0, 1, width);
            let startidx = xy2idx(width as i32 -1, 1, width);
            let nodes = scan_horizontal(startidx, 0f32, endidx, -1, &passable, width, height);
            let expected_jps_indices: BTreeSet<(usize, i32, i32)> = [
                (xy2idx(1, 1, width), -1, 0),
                (xy2idx(1, 1, width), -1, 1),
                (xy2idx(1, 1, width), -1, -1),
            ].iter().cloned().collect();
            assert!(nodes.len() == expected_jps_indices.len());
            for n in nodes {
                let idx_dir = (n.1, n.2, n.3);
                assert!(expected_jps_indices.contains(&idx_dir));
            }
        }
    }
    #[test]
    fn test_scan_vert() {
        let lines = [
            ".....",
            ".#.#.",
            ".....",
        ];
        let (passable, width, height) = parse_strmap(&lines.into_iter().map(|l| {
            String::from_str(l).unwrap()
        }).collect::<Vec<_>>());

        {
            let startidx = xy2idx(2, 2, width);
            let endidx = xy2idx(0, 0, width);
            let nodes = scan_vertical(startidx, 0f32, endidx, -1, &passable, width, height);
            let expected_jps_indices: BTreeSet<(usize, i32, i32)> = [
                (xy2idx(2, 1, width), 0, -1),
                (xy2idx(2, 1, width), -1, -1),
                (xy2idx(2, 1, width), 1, -1),
            ].iter().cloned().collect();
            assert!(nodes.len() == expected_jps_indices.len());
            for n in nodes {
                let idx_dir = (n.1, n.2, n.3);
                assert!(expected_jps_indices.contains(&idx_dir));
            }
        }
        {
            let endidx = xy2idx(0, 0, width);
            let startidx = xy2idx(2, 0, width);
            let nodes = scan_vertical(startidx, 0f32, endidx, 1, &passable, width, height);
            let expected_jps_indices: BTreeSet<(usize, i32, i32)> = [
                (xy2idx(2, 1, width), 0, 1),
                (xy2idx(2, 1, width), -1, 1),
                (xy2idx(2, 1, width), 1, 1),
            ].iter().cloned().collect();
            assert!(nodes.len() == expected_jps_indices.len());
            for n in nodes {
                let idx_dir = (n.1, n.2, n.3);
                assert!(expected_jps_indices.contains(&idx_dir));
            }
        }

    }

    #[test]
    fn test_scan_diagonal() {

        // TODO: test more directions & cases
        {
            let lines = [
                "...",
                "#..",
                "...",
            ];
            let (passable, width, height) = parse_strmap(&lines.into_iter().map(|l| {
                String::from_str(l).unwrap()
            }).collect::<Vec<_>>());
            let startidx = xy2idx(0, 2, width);
            let endidx = xy2idx(2, 0, width);
            let nodes = scan_diagonal(startidx, 0., endidx, 1, -1, &passable, width, height);
            let expected_jps_indices: BTreeSet<(usize, i32, i32)> = [
                (xy2idx(1, 1, width), -1, -1),
                (xy2idx(1, 1, width), 0, -1),
                (xy2idx(1, 1, width), 1, 0),
                (xy2idx(1, 1, width), 1, -1),
            ].iter().cloned().collect();
            for n in &nodes {
                let idx_dir = (n.1, n.2, n.3);
                assert!(expected_jps_indices.contains(&idx_dir));
            }
            assert!(nodes.len() == expected_jps_indices.len());
        }
        {
            let lines = [
                "...",
                "..#",
                "...",
            ];
            let (passable, width, height) = parse_strmap(&lines.into_iter().map(|l| {
                String::from_str(l).unwrap()
            }).collect::<Vec<_>>());
            let startidx = xy2idx(2, 0, width);
            let endidx = xy2idx(0, 20, width);
            let nodes = scan_diagonal(startidx, 0f32, endidx, -1, 1, &passable, width, height);
            let expected_jps_indices: BTreeSet<(usize, i32, i32)> = [
                (xy2idx(1, 1, width), -1, 0),
                (xy2idx(1, 1, width), -1, 1),
                (xy2idx(1, 1, width), 0, 1),
                (xy2idx(1, 1, width), 1, 1),
            ].iter().cloned().collect();
            for n in &nodes {
                let idx_dir = (n.1, n.2, n.3);
                let (x,y) = (n.1 % width, n.1 / width);
                println!("got node: {:?}, (x: {}, y: {})", idx_dir, x, y);
                assert!(expected_jps_indices.contains(&idx_dir));
            }
            assert!(nodes.len() == expected_jps_indices.len());
        }

    }
}
