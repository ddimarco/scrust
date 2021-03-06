use std::io::Read;

use ::byteorder::{LittleEndian, ReadBytesExt};

pub fn read_vec_u32(file: &mut Read, count: usize) -> Vec<u32> {
    let mut res = Vec::<u32>::with_capacity(count);
    for _ in 0..count {
        let val = file.read_u32::<LittleEndian>().unwrap();
        res.push(val);
    }
    res
}
pub fn read_vec_u16(file: &mut Read, count: usize) -> Vec<u16> {
    let mut res = Vec::<u16>::with_capacity(count);
    for _ in 0..count {
        let val = file.read_u16::<LittleEndian>().unwrap();
        res.push(val);
    }
    res
}
pub fn read_vec_u8(file: &mut Read, count: usize) -> Vec<u8> {
    let mut res = Vec::<u8>::with_capacity(count);
    for _ in 0..count {
        let val = file.read_u8().unwrap();
        res.push(val);
    }
    res
}

#[macro_export]
macro_rules! read_helper {
    (u8, $file:ident) => ($file.read_u8());
    (u16, $file:ident) => ($file.read_u16::<LittleEndian>());
    (u32, $file:ident) => ($file.read_u32::<LittleEndian>());
    (i16, $file:ident) => ($file.read_i16::<LittleEndian>());
}

#[macro_export]
macro_rules! def_bin_struct {
    ($name:ident {
        $($field_name:ident : $tpe:ident),*
    }
    )
        => {
            pub struct $name {
                $(
                    pub $field_name: $tpe,
                )*
            }

            impl $name {
                pub fn read(file: &mut Read) -> $name {
                    $(
                        let $field_name = read_helper!($tpe, file).unwrap();
                    )*
                        $name {
                            $( $field_name: $field_name, )*
                        }
                }
            }
        }
}
