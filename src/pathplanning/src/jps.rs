use std::collections::BinaryHeap;
use std::collections::BTreeSet;
use std::collections::BTreeMap;
use std::cmp::Ordering;

pub trait PlanningMapTrait {
    fn is_passable(&self, idx: usize) -> bool;
    fn width(&self) -> usize;
    fn height(&self) -> usize;

    fn dist(&self, from: usize, to: usize) -> f32 {
        idx_manhattan_dist(from, to, self.width())
    }
    fn xy2idx(&self, x: i32, y: i32) -> usize {
        (y as usize) * self.width() + (x as usize)
    }
    fn idx2xy(&self, idx: usize) -> Point {
        Point::new((idx % self.width()) as i32, (idx / self.width()) as i32)
    }

    fn print(&self, posmap: &[(usize, char)]) {
        for y in 0..self.height() {
            for x in 0..self.width() {
                let idx = y * self.width() + x;
                let mut c = if self.is_passable(idx) { '·' } else { '#' };
                for &(pos, ch) in posmap {
                    if pos == idx {
                        c = ch;
                    }
                }
                print!("{}", c);
            }
            println!("");
        }
        println!("");
    }
}

pub struct Map {
    passable: Vec<bool>,
    width: usize,
    height: usize,
}
impl PlanningMapTrait for Map {
    fn is_passable(&self, idx: usize) -> bool {
        self.passable[idx]
    }
    fn width(&self) -> usize { self.width }
    fn height(&self) -> usize { self.height }
}
impl Map {
    pub fn new(width: usize, height: usize) -> Self {
        let passable = vec![true; width*height];
        Map {
            passable: passable,
            width: width,
            height: height,
        }
    }
    pub fn parse_strmap(strmap: &[String]) -> (Self, usize, usize) {
        let width = strmap[0].len();
        let height = strmap.len();
        let mut passable = Vec::<bool>::with_capacity(width * height);
        let mut start_idx = 0;
        let mut end_idx = 0;
        for (y, line) in strmap.iter().enumerate() {
            assert!(line.len() == width);
            for (x, c) in line.chars().enumerate() {
                if c == 's' {
                    start_idx = y * width + x;
                } else if c == 'e' {
                    end_idx = y * width + x;
                }
                let obstacle = c == '#';
                passable.push(!obstacle);
            }
        }
        assert!(passable.len() == width * height);
        (Map {
            passable: passable,
            width: width,
            height: height,
        },
         start_idx,
         end_idx)
    }



    // pub fn dist(&self, from: usize, to: usize) -> f32 {
    //     idx_manhattan_dist(from, to, self.width)
    // }
}


pub struct Point {
    pub x: i32,
    pub y: i32,
}
impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Point { x: x, y: y }
    }

    // fn x(&self) -> i32 {
    //     self.x
    // }
    // fn y(&self) -> i32 {
    //     self.y
    // }
}
/// /////////////
// fn megatilexy_to_idx(x: i32, y: i32, map_w: usize) -> usize {
//     assert!(x >= 0 && y >= 0);
//     y as usize * map_w + x as usize
// }

fn idx_manhattan_dist(idx1: usize, idx2: usize, mapwidth: usize) -> f32 {
    let (x1, y1) = (idx1 % mapwidth, idx1 / mapwidth);
    let (x2, y2) = (idx2 % mapwidth, idx2 / mapwidth);

    (x1 as f32 - x2 as f32).abs() + (y1 as f32 - y2 as f32).abs()
}
/// ////////////
#[derive(Debug)]
struct JPSNode {
    /// dist from start to goal via this node
    f_dist: f32,
    /// dist from start to this node
    g_dist: f32,
    idx: usize,
    dx: i32,
    dy: i32,
}

impl JPSNode {
    fn new(f_dist: f32, g_dist: f32, idx: usize, dx: i32, dy: i32) -> Self {
        JPSNode {
            f_dist: f_dist,
            g_dist: g_dist,

            idx: idx,
            dx: dx,
            dy: dy,
        }
    }
}
impl Eq for JPSNode {}
impl PartialEq for JPSNode {
    fn eq(&self, other: &JPSNode) -> bool {
        self.f_dist == other.f_dist
    }
}
// XXX consider writing own heap structure (allowing update of priority)
/// reversed ordering (for binheap)
impl PartialOrd for JPSNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let (a, b) = (self.f_dist, other.f_dist);
        if a < b {
            Some(Ordering::Less)
        } else if a > b {
            Some(Ordering::Greater)
        } else {
            None
        }
    }
    fn lt(&self, other: &Self) -> bool {
        self.f_dist > other.f_dist
    }
    fn le(&self, other: &Self) -> bool {
        self.f_dist >= other.f_dist
    }
    fn gt(&self, other: &Self) -> bool {
        self.f_dist < other.f_dist
    }
    fn ge(&self, other: &Self) -> bool {
        self.f_dist <= other.f_dist
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

struct PlanningProblem<'a> {
    map: &'a PlanningMapTrait,
    start_idx: usize,
    end_idx: usize,

    reachable_from: BTreeMap<usize, (usize, f32)>,
    prio_heap: BinaryHeap<JPSNode>,
}


impl<'a> PlanningProblem<'a> {
    fn new(map: &'a PlanningMapTrait, start_idx: usize, end_idx: usize) -> Self {
        PlanningProblem {
            map: map,
            start_idx: start_idx,
            end_idx: end_idx,

            prio_heap: BinaryHeap::<JPSNode>::new(),
            reachable_from: BTreeMap::<usize, (usize, f32)>::new(),
        }
    }

    fn scan_horizontal(&mut self, fromidx: usize, dist: f32, dx: i32) -> bool {
        let (sx, sy) = ((fromidx % self.map.width()), (fromidx / self.map.width()));
        let widthi32 = self.map.width() as i32;
        let mut cx = sx as i32;
        let mut g_dist = dist;
        let mut found_sth = false;
        loop {
            if (cx < 0) || (cx >= widthi32) {
                // println!("out of bounds, aborting scan");
                return found_sth;
            }
            let idx = self.map.width() * sy + (cx as usize);
            if !self.map.is_passable(idx) {
                // println!("ran into obstacle, aborting scan");
                return found_sth;
            }

            if idx == self.end_idx {
                // println!("found goal");
                self.insert_parent(idx, fromidx, g_dist);
                self.prio_heap.push(JPSNode::new(g_dist, g_dist, idx, 0, 0));
                return true;
            }

            let f_dist = g_dist + self.map.dist(idx, self.end_idx);

            // check for forced neighbors
            // obstacle above
            // . # /
            // . x -
            // . . .
            if (sy > 0) && (cx + dx < widthi32) && (cx + dx >= 0) {
                let aboveidx = idx - self.map.width();
                let above_behind = (aboveidx as i32 + dx) as usize;
                if !self.map.is_passable(aboveidx) && self.map.is_passable(above_behind) {
                    // diagonal jump point
                    self.insert_parent(idx, fromidx, g_dist);
                    self.prio_heap.push(JPSNode::new(f_dist, g_dist, idx, dx, -1));
                    found_sth = true;
                }
            }

            // obstacle below
            // . . .
            // . x -
            // . # \
            if (self.map.height() - sy > 1) && (cx + dx < widthi32) && (cx + dx >= 0) {
                let belowidx = idx + self.map.width();
                let below_behind = (belowidx as i32 + dx) as usize;
                if !self.map.is_passable(belowidx) && self.map.is_passable(below_behind) {
                    // diagonal jump point
                    self.insert_parent(idx, fromidx, g_dist);
                    self.prio_heap.push(JPSNode::new(f_dist, g_dist, idx, dx, 1));
                    found_sth = true;
                }
            }

            cx += dx;
            g_dist += 1.0;
        }
    }


    fn scan_vertical(&mut self, fromidx: usize, dist: f32, dy: i32) -> bool {
        let (sx, sy) = ((fromidx % self.map.width()), (fromidx / self.map.width()));
        let widthi32 = self.map.width() as i32;
        let heighti32 = self.map.height() as i32;
        let mut cy = sy as i32;
        let mut g_dist = dist;
        let mut found_sth = false;
        loop {
            if (cy < 0) || (cy >= heighti32) {
                // println!("out of bounds, aborting scan");
                return found_sth;
            }
            let idx = self.map.width() * (cy as usize) + sx;
            if !self.map.is_passable(idx) {
                // println!("ran into obstacle, aborting scan");
                return found_sth;
            }
            if idx == self.end_idx {
                // println!("found goal");
                self.insert_parent(idx, fromidx, g_dist);
                self.prio_heap.push(JPSNode::new(g_dist, g_dist, idx, 0, 0));
                return true;
            }

            let f_dist = g_dist + self.map.dist(idx, self.end_idx);

            // check for forced neighbors
            // \ | .
            // # x .
            // . . .
            if (sx > 0) && (cy + dy < heighti32) && (cy + dy >= 0) {
                let leftidx = idx - 1;
                let left_behind = (leftidx as i32 + (dy * widthi32)) as usize;
                if !self.map.is_passable(leftidx) && self.map.is_passable(left_behind) {
                    // diagonal jump point
                    self.insert_parent(idx, fromidx, g_dist);
                    self.prio_heap.push(JPSNode::new(f_dist, g_dist, idx, -1, dy));
                    found_sth = true;
                }
            }

            // . | /
            // . x #
            // . . .
            if (self.map.width() - sx > 1) && (cy + dy < heighti32) && (cy + dy >= 0) {
                let rightidx = idx + 1;
                let right_behind = (rightidx as i32 + (dy * widthi32)) as usize;
                if !self.map.is_passable(rightidx) && self.map.is_passable(right_behind) {
                    self.insert_parent(idx, fromidx, g_dist);
                    self.prio_heap.push(JPSNode::new(f_dist, g_dist, idx, 1, dy));
                    found_sth = true;
                }
            }

            cy += dy;
            g_dist += 1.0;
        }
    }

    fn insert_parent(&mut self, new: usize, from: usize, dist: f32) {
        if new != from {
            match self.reachable_from.get(&new) {
                Some(&(_, dist_old)) => {
                    if dist_old > dist {
                        self.reachable_from.insert(new, (from, dist));
                    }
                },
                None => {
                    self.reachable_from.insert(new, (from, dist));
                }
            }
        }
    }

    fn scan_diagonal(&mut self, fromidx: usize, dist: f32, dx: i32, dy: i32) {
        // XXX: stepping once into dx, dy, because the jump point is already checked hor/vert
        let (sx, sy) = ((fromidx % self.map.width()) as i32 + dx,
                        (fromidx / self.map.width()) as i32 + dy);
        let widthi32 = self.map.width() as i32;
        let heighti32 = self.map.height() as i32;
        let diag_dist = 2f32.sqrt();
        let mut cx = sx;
        let mut cy = sy;
        let mut g_dist = dist;
        loop {
            if (cx < 0) || (cy < 0) || (cx >= widthi32) || (cy >= heighti32) {
                // println!("out of bounds, aborting scan");
                return ;
            }
            let idx = (widthi32 * cy + cx) as usize;
            // println!("diagscan situation:");
            // let mppos = vec![(self.start_idx, 's'), (self.end_idx, 'e'), (idx, 'x')];
            // self.map.print(&mppos);
            if !self.map.is_passable(idx) {
                // println!("ran into obstacle, aborting scan");
                return ;
            }
            if idx == self.end_idx {
                println!("found goal");
                self.insert_parent(idx, fromidx, g_dist);
                self.prio_heap.push(JPSNode::new(g_dist, g_dist, idx, 0, 0));
                return ;
            }

            let f_dist = g_dist + self.map.dist(idx, self.end_idx);
            // expand horizontally
            let hor_nodes = self.scan_horizontal(idx, g_dist, dx);
            if hor_nodes {
                // FIXME: finish scan before aborting (similar to hor/vert)
                // add current node to open set and return it
                self.insert_parent(idx, fromidx, g_dist);
                self.prio_heap.push(JPSNode::new(f_dist, g_dist, idx, dx, dy));
                // self.prio_heap.push(JPSNode::new(f_dist, g_dist, idx, dx, 0));
                self.prio_heap.push(JPSNode::new(f_dist, g_dist, idx, 0, dy));
                return ;
            }

            // expand vertically
            let vert_nodes = self.scan_vertical(idx, g_dist, dy);
            if vert_nodes {
                // FIXME: add only once
                // add current node to open set and return it
                // println!("found sth in vertical scan!");
                self.insert_parent(idx, fromidx, g_dist);
                self.prio_heap.push(JPSNode::new(f_dist, g_dist, idx, dx, dy));
                // self.prio_heap.push(JPSNode::new(f_dist, g_dist, idx, dx, 0));
                //self.prio_heap.push(JPSNode::new(f_dist, g_dist, idx, 0, dy));
                return ;
            }

            g_dist += diag_dist;
            cx += dx;
            cy += dy;
        }
    }
}

pub fn jps_a_star(start_idx: usize, end_idx: usize, mp: &PlanningMapTrait) -> (Vec<Point>, Vec<Point>) {
    if !mp.is_passable(start_idx) || !mp.is_passable(end_idx) {
        return (Vec::<Point>::new(), Vec::<Point>::new());
    }

    let mut pp = PlanningProblem::new(mp, start_idx, end_idx);

    // add initial neighbors
    {
        pp.scan_horizontal(start_idx, 0f32, 1);
        pp.scan_horizontal(start_idx, 0f32, -1);
        pp.scan_vertical(start_idx, 0f32, 1);
        pp.scan_vertical(start_idx, 0f32, -1);

        pp.scan_diagonal(start_idx, 0f32, 1, 1);
        pp.scan_diagonal(start_idx, 0f32, -1, 1);
        pp.scan_diagonal(start_idx, 0f32, -1, -1);
        pp.scan_diagonal(start_idx, 0f32, 1, -1);
    }

    let mut closed_set = BTreeSet::<(usize, i32, i32)>::new();
    let mut considered = Vec::<Point>::new();
    let mut result = Vec::<Point>::new();
    while let Some(current) = pp.prio_heap.pop() {
        // avoid running into infinite loops
        if closed_set.contains(&(current.idx, current.dx, current.dy)) {
            continue;
        }
        considered.push(mp.idx2xy(current.idx));
        // println!("considering {}", current.idx);
        closed_set.insert((current.idx, current.dx, current.dy));

        if current.idx == end_idx {
            println!("path found, considered {} nodes", considered.len());
            let mut idx = end_idx;
            // FIXME: what if there is a cycle?
            let mut seen_set = BTreeSet::<usize>::new();
            while idx != start_idx {
                if seen_set.contains(&idx) {
                    println!("detected loop! aborting...");
                    assert!(false);
                }
                let (x, y) = (idx % mp.width(), idx / mp.width());
                result.push(Point::new(x as i32, y as i32));
                // println!("from {} to {}", idx, *pp.reachable_from.get(&idx).unwrap());
                seen_set.insert(idx);
                idx = pp.reachable_from.get(&idx).unwrap().0;
            }

            return (result, considered);
        }

        let (dx, dy) = (current.dx, current.dy);
        if (dx != 0) && (dy == 0) {
            pp.scan_horizontal(current.idx, current.g_dist, dx);
        } else if (dx == 0) && (dy != 0) {
            pp.scan_vertical(current.idx, current.g_dist, dy);
        } else if (dx != 0) && (dy != 0) {
            pp.scan_diagonal(current.idx, current.g_dist, dx, dy);
        } else {
            unreachable!();
        }
    }

    println!("no path found, considered {} nodes", considered.len());
    (Vec::<Point>::new(), considered)
}



#[cfg(test)]
mod tests {
    // use jps::JPSNode;
    // use jps::PlanningProblem;
    use jps::jps_a_star;
    use std::str::FromStr;
    use jps::Map;
    use jps::PlanningMapTrait;
    use jps::Point;
    use ::maze::Maze;

    // fn jps_vecs_equal(a: &[JPSNode], b: &[JPSNode]) -> bool {
    //     let mut res = a.len() == b.len();
    //     for (aval, bval) in a.iter().zip(b) {
    //         res = (aval.idx == bval.idx) && (aval.dx == bval.dx) && (aval.dy == bval.dy) &&
    //             ((aval.f_dist - bval.f_dist).abs() < 0.001);
    //     }
    //     return res;
    // }


    // macro_rules! jps_testhelper {
    //     ($mpdef:expr, $func:expr) => {
    //         let (mp, start_idx, end_idx) = Map::parse_strmap(& $mpdef.into_iter().map(|l| {
    //             String::from_str(l).unwrap()
    //         }).collect::<Vec<_>>());

    //         let mut pp = PlanningProblem::new(&mp, start_idx, end_idx);

    //         $func(&mut pp);
    //     }
    // }

    macro_rules! planning_test {
        ($mpdef:expr, $func:expr) => {
            let (mp, start_idx, end_idx) = Map::parse_strmap(& $mpdef.into_iter().map(|l| {
                String::from_str(l).unwrap()
            }).collect::<Vec<_>>());
            let (path, cons) = jps_a_star(start_idx, end_idx, &mp);
            {
                println!("resulting path:");
                let mut cons_pmap = vec![(start_idx, 's'), (end_idx, 'e')];
                for c in &path {
                    let cidx = mp.xy2idx(c.x, c.y);
                    cons_pmap.push((cidx, 'x'));
                }
                mp.print(&cons_pmap);
            }
            $func(&path, &cons);
        }
    }
    #[test]
    fn empty_map() {
        let m1 = Map::new(100, 100);
        let (path, _) = jps_a_star(1, 99*100, &m1);
        assert!(!path.is_empty());
    }
    #[test]
    fn planning() {
        planning_test!(["s.....",
                        "###.##",
                        "......",
                        "#####.",
                        ".....e"], |path: &[Point], _: &[Point]| {
            assert!(!path.is_empty());
        });
        planning_test!(["s.....",
                        "#####.",
                        "......",
                        "...###",
                        ".....e"], |path: &[Point], _: &[Point]| {
            assert!(!path.is_empty());
        });
        planning_test!(["######",
                        "...s..",
                        "##..##",
                        "e.....",
                        "..##.."], |path: &[Point], _: &[Point]| {
            assert!(!path.is_empty());
        });
        planning_test!(["......",
                        "..s...",
                        "......",
                        "......",
                        ".....e"], |path: &[Point], _: &[Point]| {
            assert!(!path.is_empty());
        });
    }
    #[test]
    fn impossible_map() {
        planning_test!(["s.....",
                        "###.##",
                        "....#.",
                        "#####.",
                        ".....e"], |path: &[Point], _: &[Point]| {
            assert!(path.is_empty());
        });
    }

    #[test]
    fn loop_in_res() {
        planning_test!(["....................",
                        ".#s################.",
                        ".#.#...#.....#......",
                        ".#.#.#.#.#.#.###.#..",
                        ".#.#.#.#.#.#...#.#..",
                        ".#.#.#.#.#.###.#.#..",
                        ".#.#.#.#.#.#...#.#..",
                        ".#.#.#.###.#.###.#..",
                        ".#...#...#.#.#...#..",
                        ".#######.#.#.###.#..",
                        ".#.#.....#.#.....#..",
                        ".#.#.#####.#######..",
                        ".#...#.....#.....#..",
                        ".#.#####.#####.###..",
                        ".#.....#.#...#.#....",
                        ".#####.#.#.#.#.#.##.",
                        ".#...#.#...#.#.#.#..",
                        ".#.###.#####.#.#.#..",
                        ".#...........#....e.",
                        "...................."], |path: &[Point], _: &[Point]| {
            assert!(!path.is_empty());
        });
    }

    #[test]
    fn random_maps() {
        for _ in 0..10 {
            let maze = Maze::generate(20, 20);
            maze.show();
            let start_idx = maze.xy2idx(2,1);
            let end_idx = maze.xy2idx(maze.width() as i32-2, maze.height() as i32 - 2);
            let (path, _) = jps_a_star(start_idx, end_idx, &maze);
            assert!(!path.is_empty());
            {
                println!("resulting path:");
                let mut cons_pmap = vec![(start_idx, 's'), (end_idx, 'e')];
                for c in path {
                    let cidx = maze.xy2idx(c.x, c.y);
                    cons_pmap.push((cidx, 'x'));
                }
                maze.print(&cons_pmap);
            }
        }
    }

    #[test]
    fn big_map() {
        let strmap = [
            "############...###..........######..............################################################",
            "############..########....######................################################################",
            "##########....################....................##############################################",
            "########........############........................############################################",
            "########..............####............................##########################################",
            "########................................................########################################",
            "########..................................................####################....##############",
            "########....s...............................................################............########",
            "########..........................................##........##############................######",
            "########..........................................#.........##############................######",
            "######........................................................############................######",
            "######........................................................##########..................######",
            "########......................................................##########..................######",
            "########........................................................########..................######",
            "############....................................................########................########",
            "##############..................................................##########..............########",
            "##############......##..........................................##########..............########",
            "##############......##..........................................##########..............########",
            "################..........##......................................########..............########",
            "################..........##......................................########............##########",
            "..##############......##..........................................##########..........##########",
            "....##############....##..........................................##########............########",
            "......############................................................##########............########",
            "........##########................................................##########..........##########",
            "........############....####....................................############..........##########",
            "........######################................##...........##...############..##....##..########",
            "..........####################................##...........###..##########....##....#...########",
            "..........####################................................############....##......##########",
            "..........##################################................################........############",
            "............##################################..............####################################",
            "..............##################################..............##################################",
            "................##################################............##################################",
            "................####################################..........##################################",
            "................####################################............################################",
            "..................##################################..............############################..",
            "##..................########................########................########################....",
            "####..................####......###...........####....................##################........",
            "######..........................####....................................################........",
            "########........................####......................................############..........",
            "########....................................................................####................",
            "######..........................................................................................",
            "######..........................................................................................",
            "######......................................................####................................",
            "####......................................................########..............................",
            "##......................................................############............................",
            "......................................................################........................##",
            "........................................####..........##################................########",
            "..................################....########........##################..............##########",
            "................################################......##################..............##########",
            "................################################........################..............##########",
            "................################################..........############..............############",
            "..........######################################............########................############",
            "........########################################......................................##########",
            "........########################################........................................########",
            "........##############........################............................................####..",
            "......##############..........##############....................................................",
            "......############............############......................................................",
            "........########..............############......................................................",
            "........########..............##############................########............................",
            "........########................############..............############..........................",
            "........########................############............################........................",
            "......############..............############............##################....................##",
            "......##############............##########..........########################................####",
            "........############............##########........############################............######",
            "..........##########............##########........############################..........########",
            "............##############....##########..........############################..........########",
            "..............########################..........################################..........######",
            "................####################..........##############........############............####",
            "................####################..........########................##########..............##",
            "..............######################..........########................############..............",
            "..............####################..........##########..............################............",
            "................################..........############..............################............",
            "..................########................##############..............############..............",
            "....................####..................##############..............##########..............##",
            "..........................................##############............############............####",
            "............................................############..........##############..........######",
            "................................e.............##########........##############..........########",
            "................................................##############################..........########",
            "................................................##############################............######",
            "..............########..........................############################................####",
            "............############..........................########################....................##",
            "..........################............####..........####################........................",
            "..........################..........########..........################..........................",
            "##..........##############..........##########........############..............................",
            "####..........############..........##########........############..............................",
            "######........############........############........##########................................",
            "######........############........############........########..................####............",
            "####............########..........############..........####..................########..........",
            "####..............####..........##############..............................##########..........",
            "######........................##############................................##########..........",
            "######........................############..................................############........",
            "##########......................########..................................##############........",
            "############......................####................................##..##############........",
            "##############........................................................#...################......",
            "##############..........................................................####################....",
            "##################............................................####....######################....",
            "####################................................########################################....",
            "######################............................##############################################",
            "##....##################........................################################################",
            "............############......................##################################################",
            "........##....##########....................####################################################",
            "........##......##########....####..........####################################################",
            "................##########..########........##########..########################################",
            "................####################........##########..########################....############",
            "................####################..........########..##########........####......############",
            "................######################..........####....########....................############",
            "..................####################....................####......................############",
            "....................##################......................................##......############",
            "....................##################......................................##..##....##########",
            "##..................################............................................##......########",
            "####................##############......................................................########",
            "####..............##############..................................................##....########",
            "##................##############..........##......................................##......######",
            "##................##############..........##..............................................######",
            "##................############............................................................######",
            "##................############............................................................######",
            "####..............############............................................................######",
            "####..............############............................................................######",
            "####............##############..........................................................########",
            "##########......##############..........................................................########",
            "############....################........................................................########",
            "############....################........................................................########",
            "################################..........................................................######",
            "##################################......................................####..............######",
            "##################################....................................########..........########",
            "##################################..............................########....####......##########",
            "####################################..........................########........###...############",
            "####################################........................####..####........####..############",
        ];
        let (mp, start_idx, end_idx) = Map::parse_strmap(&strmap.into_iter().map(|l| {
            String::from_str(l).unwrap()
        }).collect::<Vec<_>>());
        let (path, cons) = jps_a_star(start_idx, end_idx, &mp);
        {
            println!("resulting path:");
            let mut cons_pmap = vec![(start_idx, 's'), (end_idx, 'e')];
            for c in &path {
                let cidx = mp.xy2idx(c.x, c.y);
                cons_pmap.push((cidx, 'x'));
            }
            mp.print(&cons_pmap);
        }
        assert!(!path.is_empty());
    }

}
// #[test]
// fn scan_horiz() {
//     jps_testhelper!([".#e",
//                      "s.."],
//                     |pp: &mut PlanningProblem| {
//                         let nodes = pp.scan_horizontal(pp.start_idx, 0f32, 1);
//                         assert!(jps_vecs_equal(&nodes, &[JPSNode::new(1f32, 4, 1, -1)]));
//                     });

//     jps_testhelper!(["e#.",
//                      "..s"],
//                     |pp: &mut PlanningProblem| {
//                         let nodes = pp.scan_horizontal(pp.start_idx, 0f32, -1);
//                         assert!(jps_vecs_equal(&nodes, &[JPSNode::new(1f32, 4, -1, -1)]));
//                     });
//     jps_testhelper!(["e#.",
//                      "..s",
//                      ".#."],
//                     |pp: &mut PlanningProblem| {
//                         let nodes = pp.scan_horizontal(pp.start_idx, 0f32, -1);
//                         assert!(jps_vecs_equal(&nodes,
//                                                &[JPSNode::new(1f32, 4, -1, -1),
//                                                  JPSNode::new(1f32, 4, -1, 1)]));
//                     });
// }

// #[test]
// fn scan_vert() {
//     jps_testhelper!([".s.",
//                      "#..",
//                      "e.."],
//                     |pp: &mut PlanningProblem| {
//                         let nodes = pp.scan_vertical(pp.start_idx, 0f32, 1);
//                         assert!(jps_vecs_equal(&nodes,
//                                                &[JPSNode::new(1f32, 4, -1, 1)]));
//                     }
//     );
//     jps_testhelper!([".s.",
//                      "..#",
//                      "e.."],
//                     |pp: &mut PlanningProblem| {
//                         let nodes = pp.scan_vertical(pp.start_idx, 0f32, 1);
//                         assert!(jps_vecs_equal(&nodes,
//                                                &[JPSNode::new(1f32, 4, 1, 1)]));
//                     }
//     );
//     jps_testhelper!([".s.",
//                      "#.#",
//                      "e.."],
//                     |pp: &mut PlanningProblem| {
//                         let nodes = pp.scan_vertical(pp.start_idx, 0f32, 1);
//                         assert!(jps_vecs_equal(&nodes,
//                                                &[JPSNode::new(1f32, 4, -1, 1),
//                                                  JPSNode::new(1f32, 4, 1, 1)]));
//                     }
//     );
// }

// #[test]
// fn scan_diag() {
//     jps_testhelper!([
//         "........",
//         "........",
//         ".....#..",
//         ".....#..",
//         "s....#.e",
//     ], |pp: &mut PlanningProblem| {
//         let nodes = pp.scan_diagonal(pp.start_idx, 0f32, 1, -1);
//         let mut mppos = vec![(pp.start_idx, 's'), (pp.end_idx, 'e')];
//         for n in &nodes {
//             mppos.push((n.idx, 'x'));
//         }
//         pp.map.print(&mppos);
//         assert!(nodes[0].idx == (1 * pp.map.width + 3));
//     });

//     jps_testhelper!([
//         "........",
//         "...s....",
//         ".....#..",
//         ".....#..",
//         ".....#.e",
//     ], |pp: &mut PlanningProblem| {
//         let nodes = pp.scan_horizontal(pp.start_idx, 0f32, 1);
//         let mut mppos = vec![(pp.start_idx, 's'), (pp.end_idx, 'e')];
//         for n in &nodes {
//             mppos.push((n.idx, 'x'));
//         }
//         pp.map.print(&mppos);
//         assert!(nodes[0].idx == (1 * pp.map.width + 5));
//     });

//     jps_testhelper!([
//         "........",
//         ".....s..",
//         ".....#..",
//         ".....#..",
//         ".....#.e",
//     ], |pp: &mut PlanningProblem| {
//         let nodes = pp.scan_diagonal(pp.start_idx, 0f32, 1, 1);
//         let mut mppos = vec![(pp.start_idx, 's'), (pp.end_idx, 'e')];
//         for n in &nodes {
//             mppos.push((n.idx, 'x'));
//         }
//         pp.map.print(&mppos);
//         assert!(nodes[0].idx == (3 * pp.map.width + 7));
//     });
// }
