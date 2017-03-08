#[macro_use]
extern crate bencher;
use bencher::Bencher;

extern crate scrust;
use scrust::render::{render_buffer_solid};

// safe array access
// running 2 tests
// test b640480          ... bench: 211,184,536 ns/iter (+/- 14,352,191)
// test b640480_nolambda ... bench: 100,655,522 ns/iter (+/- 6,257,356)

// unsafe (only _nolambda)
// running 2 tests
// test b640480          ... bench: 200,804,924 ns/iter (+/- 8,519,462)
// test b640480_nolambda ... bench:  82,239,161 ns/iter (+/- 2,291,978)

// TODO: other ideas:
// - does inpos adder make a difference?
// - flipped <-> non-flipped: approx same speed

fn simple_map(col: u8, _: u8) -> Option<u8> {
    if col == 0 {
        return None;
    }
    Some(col)
}

// fn b640480(bench: &mut Bencher) {
//     let (screen_width, screen_height) = (80, 60);
//     let (sw, sh) = (64, 32);
//     let mut screen = vec![0u8; (screen_width*screen_height) as usize];
//     let sprite = vec![255u8; (sw*sh) as usize];

//     bench.iter(|| {
//         for flipped in vec![false, true] {
//             for y in 0..screen_height {
//                 for x in 0..screen_width {
//                     render_buffer(&sprite, sw, sh,
//                                   flipped,
//                                   x, y,
//                                   &mut screen, screen_width, &simple_map);
//                 }
//             }
//         }
//     });
// }

fn b640480_nolambda(bench: &mut Bencher) {
    let (screen_width, screen_height) = (80, 60);
    let (sw, sh) = (64, 32);
    let mut screen = vec![0u8; (screen_width*screen_height) as usize];
    let sprite = vec![255u8; (sw*sh) as usize];

    bench.iter(|| {
        for flipped in vec![false, true] {
            for y in 0..screen_height {
                for x in 0..screen_width {
                    render_buffer_solid(&sprite, sw, sh,
                                         flipped,
                                         x, y,
                                         &mut screen, screen_width as u32);
                }
            }
        }
    });
}


benchmark_group!(benches, b640480_nolambda);
benchmark_main!(benches);
