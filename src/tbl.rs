use std;

use std::io::{Read, Seek, SeekFrom};

extern crate byteorder;
use byteorder::{LittleEndian, ReadBytesExt};

fn read_string<T: Read>(file: &mut T, length: Option<u16>) -> String {
    let mut res_str = String::new();

    let mut i=0;
    loop {
        if (length != None) && (i >= length.unwrap()) {
            break;
        }
        match file.read_u8() {
            Ok(val) => {
                // FIXME:
                if val > 0 {
                    res_str.push(val as char);
                }
            },
            Err(_) => {
                break;
            }
        };
        i += 1;
    }

    res_str
}

pub fn read_tbl<T: Read + Seek>(file: &mut T) -> std::vec::Vec<String> {
    let string_count = file.read_u16::<LittleEndian>().unwrap() as usize;
    let mut string_offsets = Vec::with_capacity(string_count);
    let mut strings = Vec::with_capacity(string_count);

    for _ in 0..string_count {
        string_offsets.push(file.read_u16::<LittleEndian>().unwrap());
    }
    for i in 0..(string_count ) {
        file.seek(SeekFrom::Start(string_offsets[i] as u64)).ok();
        let len =
            if i == (string_count - 1) {
                None
            } else {
                Some(string_offsets[i+1] - string_offsets[i])
            };
        strings.push(read_string(file, len));
    }

    strings
}

