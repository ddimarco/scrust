use std::io::{Read};

extern crate sdl2;
use sdl2::pixels::{Color};
use sdl2::render::{Renderer, Texture};

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
            data[i*3   ] = read_buf[0];
            data[i*3 + 1] = read_buf[1];
            data[i*3 + 2] = read_buf[2];
        }

        Palette {
            data: data,
        }
    }

    pub fn to_sdl(&self) -> sdl2::pixels::Palette {
        let mut cols = [Color::RGB(0, 0, 0); 256];
        for i in 0..256 {
            let r = self.data[i*3   ];
            let g = self.data[i*3 + 1];
            let b = self.data[i*3 + 2];
            cols[i] = Color::RGB(r,g,b);
        }
        sdl2::pixels::Palette::with_colors(&cols).unwrap()
    }

}

pub fn palimg_to_texture(renderer: &mut Renderer,
                     width: u32,
                     height: u32,
                     inbuf: &[u8],
                     pal: &Palette)
                     -> Texture {
    // XXX need to specify the pixel_mask like this, otherwise we get wrong colors
    let pixel_mask = sdl2::pixels::PixelMasks {
        bpp: 32,
        rmask: 0x000000FF,
        gmask: 0x0000FF00,
        bmask: 0x00FF0000,
        amask: 0xFF000000,
    };
    let mut surf = sdl2::surface::Surface::from_pixelmasks(width, height, pixel_mask).unwrap();

    surf.with_lock_mut(|buffer: &mut [u8]| {
        let mut outidx = 0;
        for i in 0..inbuf.len() {
            let col = inbuf[i] as usize;
            let a = if col == 0 {
                0
            } else {
                255
            };
            let r = pal.data[col * 3 + 0];
            let g = pal.data[col * 3 + 1];
            let b = pal.data[col * 3 + 2];
            buffer[outidx] = r;
            outidx += 1;
            buffer[outidx] = g;
            outidx += 1;
            buffer[outidx] = b;
            outidx += 1;
            buffer[outidx] = a;
            outidx += 1;
        }
    });
    renderer.create_texture_from_surface(surf).unwrap()
}


