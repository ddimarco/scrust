use std::io::Read;

use byteorder::{LittleEndian, ReadBytesExt};


// pub fn read_u8buf(file: &mut Read, size: usize) -> Vec<u8> {
//     let mut res = vec![0 as u8; size];
//     let read_bytes = file.read(&mut res).unwrap();
//     assert_eq!(read_bytes, size);
//     res
// }

// pub fn read_u16buf(file: &mut Read, size: usize, res: &mut [u16]) {
//     for i in 0..size {
//         res[i] = file.read_u16::<LittleEndian>().unwrap();
//     }
// }
