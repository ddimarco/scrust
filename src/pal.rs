use std::io::{Read};

pub struct Palette {
    pub data: Vec<u8>,
}
impl Palette {
    pub fn from_buffer(buffer: &[u8; 256*3]) -> Palette {
        let mut vec = Vec::<u8>::with_capacity(256*3);
        for i in buffer.iter() {
            vec.push(*i);
        }
        Palette {
            data: vec,
        }
    }

    pub fn read_wpe<T: Read>(f: &mut T) -> Palette {
        let mut data = Vec::<u8>::with_capacity(256*3);
        let bytes_read = f.read(&mut data).unwrap();
        assert_eq!(bytes_read, 256*3);

        Palette {
            data: data,
        }
    }
}
