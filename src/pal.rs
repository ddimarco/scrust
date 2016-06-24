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
        let mut data = vec![0 as u8; 3*256];
        let mut read_buf = [0 as u8; 4];
        for i in 0..256 {
            let bytes_read = f.read(&mut read_buf).unwrap();
            assert_eq!(bytes_read, 4);
            data[i*3 + 0] = read_buf[0];
            data[i*3 + 1] = read_buf[1];
            data[i*3 + 2] = read_buf[2];
        }

        Palette {
            data: data,
        }
    }
}
