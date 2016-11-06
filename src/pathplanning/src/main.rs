extern crate pathplanning;
use pathplanning::jps::{jps_a_star, Map, PlanningMapTrait};

use std::str::FromStr;

fn main() {
    let strmap = ["s.....",
                  "###.##",
                  "......",
                  "#####.",
                  ".....e"];
    let (mp, start_idx, end_idx) = Map::parse_strmap(&strmap.into_iter()
        .map(|l| String::from_str(l).unwrap())
        .collect::<Vec<_>>());

    // let maze = Maze::generate(10, 10);
    // let (mp, _, _) = Map::parse_strmap(&maze.to_string());

    mp.print(&[(start_idx, 's'), (end_idx, 'e')]);

    let (path, cons) = jps_a_star(start_idx, end_idx, &mp);

    {
        println!("considered:");
        let mut cons_pmap = vec![(start_idx, 's'), (end_idx, 'e')];
        for c in cons {
            let cidx = mp.xy2idx(c.x, c.y);
            cons_pmap.push((cidx, 'c'));
        }
        mp.print(&cons_pmap);
    }

    {
        println!("resulting path:");
        let mut cons_pmap = vec![(start_idx, 's'), (end_idx, 'e')];
        for c in path {
            let cidx = mp.xy2idx(c.x, c.y);
            cons_pmap.push((cidx, 'x'));
        }
        mp.print(&cons_pmap);
    }

}


