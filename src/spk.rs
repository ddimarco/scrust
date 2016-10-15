use std::io::{Read, Seek, SeekFrom};
use std::collections::HashMap;

extern crate byteorder;
use byteorder::{ReadBytesExt, LittleEndian};

use ::utils::read_vec_u16;

pub struct SPK {
    pub layers: Vec<Vec<SPKStar>>,
    pub images: HashMap<u32, SPKImage>,
}

pub struct SPKStar {
    pub x: u16,
    pub y: u16,
    pub offset: u32,
}

pub struct SPKImage {
    pub width: u16,
    pub height: u16,
    pub data: Vec<u8>,
}

impl SPK {
    pub fn read_spk<T: Read + Seek>(f: &mut T) -> Self {

        let layers_count = f.read_u16::<LittleEndian>().unwrap() as usize;
        let images_per_layer = read_vec_u16(f, layers_count);

        let mut layers = Vec::<Vec<SPKStar>>::with_capacity(layers_count);
        for images_count in images_per_layer {
            let mut images = Vec::<SPKStar>::with_capacity(images_count as usize);
            for _ in 0..images_count {
                let x = f.read_u16::<LittleEndian>().unwrap();
                let y = f.read_u16::<LittleEndian>().unwrap();
                let offset = f.read_u32::<LittleEndian>().unwrap();
                images.push(SPKStar {
                    x: x,
                    y: y,
                    offset: offset,
                });
            }

            layers.push(images);
        }

        let mut img_map = HashMap::<u32, SPKImage>::new();

        // read actual images
        for layer in &layers {
            for star in layer {
                match img_map.get(&star.offset) {
                    None => {
                        f.seek(SeekFrom::Start(star.offset as u64)).ok();
                        let w = f.read_u16::<LittleEndian>().unwrap();
                        let h = f.read_u16::<LittleEndian>().unwrap();
                        let mut buffer = vec![0; (w*h) as usize];
                        f.read(&mut buffer).ok();
                        img_map.insert(star.offset,
                                       SPKImage {
                                           width: w,
                                           height: h,
                                           data: buffer,
                                       });
                    }
                    Some(_) => {}
                }
            }
        }
        SPK {
            layers: layers,
            images: img_map,
        }
    }
}
