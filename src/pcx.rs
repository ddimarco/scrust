use std;

use std::fs::File;
use std::io::{Read, Write, Seek, SeekFrom};

extern crate byteorder;
use byteorder::{LittleEndian, ReadBytesExt};

use ::pal::Palette;

pub struct PCXHeader {
    pub version: u8,
    pub encoding: u8,
    pub bpp: u8,

    pub width: u16,
    pub height: u16,

    pub clrmap: [u8; 16 * 3],
    pub bpl: u16,
}

pub struct PCX {
    pub header: PCXHeader,
    pub data: Vec<u8>,
    // pub palette: [u8; 256*3],
    pub palette: Palette,
}

impl PCX {
    pub fn read<T: Read + Seek>(file: &mut T) -> PCX {
        let mut pcxhead = PCXHeader {
            version: 0,
            encoding: 0,
            bpp: 0,
            width: 0,
            height: 0,
            clrmap: unsafe { std::mem::uninitialized() },
            bpl: 0,
        };

        // id
        let _ = file.read_u8().unwrap();
        pcxhead.version = file.read_u8().unwrap();
        pcxhead.encoding = file.read_u8().unwrap();
        pcxhead.bpp = file.read_u8().unwrap();
        // println!("id: {0}, version: {1}, encoding: {2}, bpp: {3}", id, version, encoding, bpp);

        let xmin = file.read_u16::<LittleEndian>().unwrap();
        let ymin = file.read_u16::<LittleEndian>().unwrap();
        let xmax = file.read_u16::<LittleEndian>().unwrap();
        let ymax = file.read_u16::<LittleEndian>().unwrap();
        // hres, vres
        let _ = file.read_u16::<LittleEndian>().unwrap();
        let _ = file.read_u16::<LittleEndian>().unwrap();

        pcxhead.clrmap = unsafe { std::mem::uninitialized() };
        file.read(&mut pcxhead.clrmap).ok();

        let _ = file.read_u8().unwrap();
        // num_planes
        let _ = file.read_u8().unwrap();
        pcxhead.bpl = file.read_u16::<LittleEndian>().unwrap();
        // pal
        let _ = file.read_u16::<LittleEndian>().unwrap();

        file.seek(SeekFrom::Current(58)).ok();
        // println!("xmin: {0}, ymin: {1}, xmax: {2}, ymax: {3}",
        //          xmin, ymin, xmax, ymax);

        // read data
        pcxhead.width = xmax - xmin + 1;
        pcxhead.height = ymax - ymin + 1;
        let bufsize = (pcxhead.width as usize) * (pcxhead.height as usize);
        // let mut pcx = PCX {
        //     header: pcxhead,
        //     data: vec![0; bufsize],
        //     palette: [0; 256*3],
        // };
        let mut data = vec![0; bufsize];

        let mut outpos = 0;
        for _ in 0..pcxhead.height {
            let mut x = 0;
            while (x < pcxhead.bpl) && (outpos < bufsize as usize) {
                let val = file.read_u8().unwrap();
                if val > 192 {
                    let repeat = val - 192;
                    let color = file.read_u8().unwrap();
                    for _ in 0..repeat {
                        data[outpos] = color;
                        outpos += 1;
                        x += 1;
                    }
                } else {
                    data[outpos] = val;
                    outpos += 1;
                    x += 1;
                }
            }
        }

        // read palette
        let first_byte = file.read_u8().unwrap();
        assert!(first_byte == 12);
        let mut buf = [0; 256 * 3];
        file.read(&mut buf).ok();

        PCX {
            header: pcxhead,
            data: data,
            palette: Palette::from_buffer(&buf),
        }
    }

    pub fn to_ppm(self: &PCX, outfile: &str) {
        let mut outfile = File::create(outfile).unwrap();
        outfile.write_fmt(format_args!("P3\n")).ok();
        outfile.write_fmt(format_args!("{0} {1}\n", self.header.width, self.header.height)).ok();
        outfile.write_fmt(format_args!("255\n")).ok();
        for i in 0..(self.header.width as usize) * (self.header.height as usize) {
            let pal_idx: usize = 3 * (self.data[i as usize] as usize);
            outfile.write_fmt(format_args!("{0} {1} {2}\n",
                                           self.palette.data[pal_idx + 0],
                                           self.palette.data[pal_idx + 1],
                                           self.palette.data[pal_idx + 2]))
                .ok();
        }
    }
}
