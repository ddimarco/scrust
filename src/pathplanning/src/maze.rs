// Maze generator in Rust.
// 2014-01-11 <> 20151208
// Joe Wingbermuehle

use ::rand;
use rand::Rng;
use std::vec::Vec;

use ::jps::PlanningMapTrait;

enum Element {
    Wall,
    Passage,
}

pub struct Maze {
    width:      usize,
    height:     usize,
    data:       Vec<Element>,
}

impl PlanningMapTrait for Maze {
    fn is_passable(&self, idx: usize) -> bool {
        match self.data[idx] {
            Element::Passage => true,
            _ => false
        }
    }
    fn width(&self) -> usize {
        self.width
    }
    fn height(&self) -> usize {
        self.height
    }
}

impl Maze {

    // Create an empty maze structure.
    fn init(width: usize, height: usize) -> Maze {
        let mut maze = Maze {
            width:  width,
            height: height,
            data:   Vec::new(),
        };
        for y in 0 .. height {
            for x in 0 .. width {
                let value =
                    if y == 0 || y == height - 1 {
                        Element::Passage
                    } else if x == 0 || x == width - 1 {
                        Element::Passage
                    } else {
                        Element::Wall
                    };
                maze.data.push(value);
            }
        }
        maze
    }

    // Show the maze.
    pub fn show(&self) {
        for y in 0 .. self.height {
            for x in 0 .. self.width {
                match self.data[y * self.width + x] {
                   Element::Wall => print!("#"),
                   Element::Passage => print!("."),
                }
            }
            println!("")
        }
    }

    // pub fn to_stringvec(&self) -> Vec<String> {
    //     let mut res = Vec::<String>::new();
    //     for y in 0 .. self.height {
    //         let mut s = String::new();
    //         for x in 0 .. self.width {
    //             s.push(
    //             match self.data[y * self.width + x] {
    //                 Element::Wall => '#',
    //                 Element::Passage => '.',
    //             });
    //         }
    //         res.push(s);
    //     }
    //     res
    // }

    fn is_wall(&self, x: isize, y: isize) -> bool {
        let (ux, uy) = (x as usize, y as usize);
        match self.data[(uy * self.width + ux) as usize] {
            Element::Wall => true,
            Element::Passage => false,
        }
    }

    // Carve the maze starting at x, y.
    fn carve<R: rand::Rng>(&mut self, rng: &mut R, x: usize, y: usize) {
        let xdirs = [ 1, -1, 0, 0 ];
        let ydirs = [ 0, 0, 1, -1 ];
        self.data[y * self.width + x] = Element::Passage;
        let d = rng.gen::<usize>() % 4;
        for i in 0 .. 4 {
            let dx: isize = xdirs[(d + i) % 4];
            let dy: isize = ydirs[(d + i) % 4];
            let x2 = (x as isize) + dx;
            let y2 = (y as isize) + dy;
            if self.is_wall(x2, y2) {
                let nx = x2 + dx;
                let ny = y2 + dy;
                if self.is_wall(nx, ny) {
                    let index = (y2 as usize) * self.width + (x2 as usize);
                    self.data[index] = Element::Passage;
                    self.carve(rng, nx as usize, ny as usize);
                }
            }
        }

    }

    // Generate a maze.
    pub fn generate(width: usize, height: usize) -> Maze {
        let mut maze = Maze::init(width, height);
        let mut rng = rand::thread_rng();
        maze.carve(&mut rng, 2, 2);
        maze.data[1 * width + 2] = Element::Passage;
        maze.data[(height - 2) * width + (width - 3)] = Element::Passage;
        maze
    }

}
