// fn draw_rect(buffer: &mut [u8], buf_stride: u32, rect: &Rect, col: u8) {
//     let mut outpos = rect.left() as usize + (rect.top() * buf_stride as i32) as usize;
//     let capped_width = (min(buf_stride as i32, rect.right()) - rect.left()) as usize;
//     for x in 0..capped_width {
//         buffer[outpos + x] = col;
//     }
//     outpos += buf_stride as usize;
//     if rect.height() >= 2 {
//         for _ in 0..rect.height() - 2 {
//             buffer[outpos] = col;
//             buffer[outpos + capped_width] = col;
//             outpos += buf_stride as usize;
//         }
//     }
//     for x in 0..capped_width {
//         buffer[outpos + x] = col;
//     }
// }


// FIXME: a lot of time spent here, speed this up
macro_rules! render_function {
    ($fname:ident, $func:expr; $($param:ident: $param_ty:ty),* ) => {
        pub fn $fname(inbuffer: &[u8], width: u32, height: u32, flipped: bool,
                      cx: i32, cy: i32, buffer: &mut [u8], buffer_pitch: u32,
                      $($param: $param_ty), *) {
            unsafe {
                let halfheight = (height as i32)/2;
                let halfwidth = (width as i32)/2;
                let buffer_height = buffer.len() as u32/buffer_pitch;

                if (cx - halfwidth > buffer_pitch as i32) ||
                    (cy - halfheight > buffer_height as i32) {
                    return;
                }

                let yoffset =
                    (if cy < halfheight  {
                        halfheight - cy
                    } else {
                        0
                    }) as u32;
                let xoffset =
                    (if cx < halfwidth {
                        halfwidth - cx
                    } else {
                        0
                    }) as u32;

                let youtend = cy + halfheight;
                let ystop =
                    if youtend as u32 > buffer_height {
                        let rest = youtend as u32 - buffer_height;
                        if height < rest {
                            height
                        } else {
                            height - rest
                        }
                    } else {
                        height
                    };

                let xoutend = cx + halfwidth;
                let xstop =
                    if xoutend as u32 > buffer_pitch {
                        let rest = xoutend as u32 - buffer_pitch;
                        if width < rest {
                            width
                        } else {
                            width - rest
                        }
                    } else {
                        width
                    };
                let x_skip = width - xstop;

                let mut outpos = (cy + yoffset as i32 - halfheight) as u32 * buffer_pitch +
                    (cx + xoffset as i32 - halfwidth) as u32;

                if flipped {
                    for y in yoffset..ystop {
                        for x in xoffset..xstop {
                            let col = inbuffer.get_unchecked((y*width + (width - x - 1)) as usize);
                            $func(*col, buffer, outpos as usize,
                                  $($param),*
                            );
                            outpos += 1;
                        }
                        outpos += buffer_pitch - width + xoffset + x_skip;
                    }
                } else {
                    let mut inpos = yoffset*width + xoffset;
                    for _ in yoffset..ystop {
                        for _ in xoffset..xstop {
                            let col = inbuffer.get_unchecked(inpos as usize);
                            $func(*col, buffer, outpos as usize,
                                  $($param),*);
                            outpos += 1;
                            inpos += 1;
                        }
                        outpos += buffer_pitch - width + xoffset + x_skip;
                        inpos += xoffset + x_skip;
                    }
                }
            }
        }
    }
}

render_function!(render_buffer_solid, |col: u8, buffer: &mut [u8], outpos: usize| {
    let ob = buffer.get_unchecked_mut(outpos);
    if col > 0 {
        *ob = col;
    }
};);
render_function!(render_buffer_with_transparency_reindexing,
                 |col: u8, buffer: &mut [u8], outpos: usize, reindex: &[u8]| {
     let ob = buffer.get_unchecked_mut(outpos);
     if col > 0 {
         *ob = *reindex.get_unchecked(((col as usize) - 1)*256 + *ob as usize);
     }
}; reindex: &[u8]);
render_function!(render_buffer_with_solid_reindexing,
                 |col: u8, buffer: &mut [u8], outpos: usize, reindex: &[u8]| {
                     let ob = buffer.get_unchecked_mut(outpos);
                     if col > 0 {
                         *ob = *reindex.get_unchecked(col as usize - 1);
                     }
                 }; reindex: &[u8]);
