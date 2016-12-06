use std::cmp::min;
use std::io::{Read, Seek, SeekFrom};
extern crate sdl2;
use sdl2::rect::Rect;
use sdl2::rect::Point;
use sdl2::pixels::Color;

extern crate byteorder;
use byteorder::{LittleEndian, ReadBytesExt};

#[macro_use]
extern crate scrust;
use scrust::font::FontSize;

use scrust::font::{RenderText, HorizontalAlignment, VerticalAlignment};
use scrust::gamedata::GameData;
use scrust::{GameContext, GameState, View, ViewAction};

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate enum_primitive;
extern crate num;
use num::FromPrimitive;

/// //////////////////////////////////////
/// low level structs

def_bin_struct!(DialogLLStruct {
    next_entry: u32,
    left_pos: u16,
    top_pos: u16,
    right_pos: u16,
    bottom_pos: u16,
    width: u16,
    height: u16,
    running_flags: u32,
    string_offset: u32,
    flags: u32,
    _skip: u32,
    control_id: i16,
    control_type: u32,
    __skip: u32,
    update_func1: u32,
    update_func2: u32,
    parent_dlg: u32,
    response_area_left: u16,
    response_area_top: u16,
    response_area_right: u16,
    response_area_bottom: u16,
    ___skip: u32,
    smk_offset: u32,
    text_offset_x: u16,
    text_offset_y: u16,
    response_area_width: u16,
    response_area_height: u16,
    ____skip: u32,
    _____skip: u32
});

def_bin_struct!(SMKLLStruct {
    overlay_offset: u32,
    flags: u16,
    _skip: u32,
    filename: u32,
    __skip: u32,
    overlay_x_pos: u16,
    overlay_y_pos: u16,
    ___skip: u32,
    ____skip: u32
});

fn read_0terminated_string(file: &mut Read) -> String {
    let mut res = String::new();
    loop {
        let val = file.read_u8().unwrap();
        if val == 0 {
            break;
        }
        res.push(val as char);
    }
    res
}

/// //////////////////////////////////////

enum_from_primitive! {
#[derive(PartialEq)]
#[derive(Debug)]
enum ControlType {
    DialogBox = 0x0,
    // draw with rounded corners
    // FIXME: from dlgs/tile.grp?
    DefaultButton = 0x1,
    // draw with rounded corners
    Button = 0x2,
    OptionButton = 0x3,
    CheckBox = 0x4,
    Image = 0x5,
    HorizontalScrollBar = 0x6,
    VerticalScrollBar = 0x7,
    TextBox = 0x8,
    LabelLeftAlign = 0x9,
    LabelCenterAlign = 0xa,
    LabelRightAlign = 0xb,
    ListBox = 0xc,
    ComboBox = 0xd,
    LightupButton = 0xe,
}
}

// FIXME: doesn't seem to be fully correct
bitflags! {
    // following BinEdit II flags
    pub flags DialogFlags: u32 {
        const DLG_UNKNOWN0  = 0x1,
        const DLG_DISABLED  = 0x2,
        const DLG_ACTIVE  = 0x4,
        const DLG_VISIBLE  = 0x8,
        const DLG_RESPOND_TO_EVENTS  = 0x10,
        const DLG_UNKNOWN5  = 0x20,
        const DLG_CANCEL_BUTTON  = 0x40,
        const DLG_NO_HOVER_SOUND  = 0x80,
        const DLG_VIRTUAL_KEY  = 0x100,
        const DLG_HAS_HOTKEY  = 0x200,
        const DLG_FONT10  = 0x400,
        const DLG_FONT16  = 0x800,
        const DLG_UNKNOWN12  = 0x1000,
        const DLG_COL0_TRANSPARENT  = 0x00002000,
        const DLG_FONT16X  = 0x00004000,
        const DLG_ALTERNATE_STYLE  = 0x00008000,
        const DLG_FONT14  = 0x00010000,
        const DLG_REMOVE_STYLES  = 0x00020000,
        const DLG_APPLY_TRANSLUCENCY  = 0x00040000,
        const DLG_DEFAULT_BUTTON  = 0x00080000,
        const DLG_BRING_TO_FRONT  = 0x00100000,
        const DLG_HORIZONTAL_ALIGNMENT_CENTER  = 0x00200000,
        const DLG_HORIZONTAL_ALIGNMENT_RIGHT  = 0x00400000,
        const DLG_HORIZONTAL_ALIGNMENT_CENTER2  = 0x00800000,
        const DLG_VERTICAL_ALIGNMENT_TOP  = 0x01000000,
        const DLG_VERTICAL_ALIGNMENT_MIDDLE  = 0x02000000,
        const DLG_VERTICAL_ALIGNMENT_BOTTOM  = 0x04000000,
        const DLG_UNKNOWN_27  = 0x08000000,
        const DLG_REVERSE_DIALOG_DIRECTION  = 0x10000000,
        const DLG_USE_ALTERNATE_STYLE  = 0x20000000,
        const DLG_NO_CLICK_SOUND = 0x40000000,
        const DLG_UNKNOWN31 = 0x80000000
    }
}
bitflags! {
    pub flags SMKFlags: u16 {
        const SMK_FADE_IN = 0x01,
        const SMK_DARK = 0x02,
        const SMK_REPEAT_FOREVER = 0x04,
        const SMK_SHOW_IF_OVER = 0x08,
        const SMK_UNKNOWN4 = 0x10
    }
}

struct SMKElement {
    overlay: Option<Box<SMKElement>>,
    filename: String,
    flags: SMKFlags,
    overlay_x: u16,
    overlay_y: u16,
}

struct Control {
    rect: Rect,
    control_type: ControlType,
    flags: DialogFlags,
    dlgstring: Option<String>,
    responsive_area: Rect,
    smk_overlay: Option<SMKElement>,
    text_offset: Option<Point>,
}
impl Control {
    fn draw(&self, gd: &GameData, buffer: &mut [u8], screen_pitch: u32) {
        // FIXME: distinguish control types

        if !self.flags.contains(DLG_VISIBLE) {
            return;
        }
        draw_rect(buffer, screen_pitch, &self.rect, 21);
        draw_rect(buffer, screen_pitch, &self.responsive_area, 10);

        let dlgstring = &self.dlgstring;

        // render text
        match dlgstring {
            &Some(ref txt) => {
                // FIXME: precalc?
                let font_size =
                    if self.flags.contains(DLG_FONT16) {
                        FontSize::Font16
                    } else if self.flags.contains(DLG_FONT14) {
                        FontSize::Font14
                    } else if self.flags.contains(DLG_FONT10) {
                        FontSize::Font10
                    } else if self.flags.contains(DLG_FONT16X) {
                        FontSize::Font16X
                    } else {
                        FontSize::Font16
                    };

                let fnt = gd.font(font_size);
                let txt = if self.flags.contains(DLG_HAS_HOTKEY) {
                    &txt[1..]
                } else {
                    txt
                };


                match self.text_offset {
                    Some(offset) => {
                        let mut rect = self.rect;
                        rect.offset(offset.x(), offset.y());
                        fnt.render_text_aligned(txt.as_ref(), 0,
                                                &gd.fontmm_reindex.data, buffer,
                                                screen_pitch, &rect,
                                                HorizontalAlignment::Left,
                                                VerticalAlignment::Top);
                    },
                    None => {
                        let halign = if self.control_type == ControlType::Button {
                            HorizontalAlignment::Center
                        } else if self.flags.contains(DLG_HORIZONTAL_ALIGNMENT_CENTER) ||
                            self.flags.contains(DLG_HORIZONTAL_ALIGNMENT_CENTER2) {
                                HorizontalAlignment::Center
                            } else if self.flags.contains(DLG_HORIZONTAL_ALIGNMENT_RIGHT) {
                                HorizontalAlignment::Right
                            } else {
                                HorizontalAlignment::Left
                            };
                        let valign =
                            if self.flags.contains(DLG_VERTICAL_ALIGNMENT_TOP) {
                                VerticalAlignment::Top
                            } else if self.flags.contains(DLG_VERTICAL_ALIGNMENT_MIDDLE) {
                                VerticalAlignment::Center
                            } else if self.flags.contains(DLG_VERTICAL_ALIGNMENT_BOTTOM) {
                                VerticalAlignment::Bottom
                            } else {
                                VerticalAlignment::Center
                            };
                        fnt.render_text_aligned(txt.as_ref(), 0,
                                                &gd.fontmm_reindex.data, buffer,
                                                screen_pitch, &self.rect,
                                                halign,
                                                valign,
                        );
                    }
                }

            },
            _ => {}
        }
    }
}
struct Dialog {
    controls: Vec<Control>,
}
impl Dialog {
    fn ll_dlg_to_control<T: Read + Seek>(lldlg: &DialogLLStruct, file: &mut T) -> Control {
        let rect = Rect::new(lldlg.left_pos as i32,
                             lldlg.top_pos as i32,
                             lldlg.width as u32,
                             lldlg.height as u32);
        let responsive_rect = Rect::new(lldlg.response_area_left as i32 + rect.left(),
                                        lldlg.response_area_top as i32 + rect.top(),
                                        lldlg.response_area_width as u32,
                                        lldlg.response_area_height as u32);
        let dlgstring = if lldlg.string_offset > 0 {
            file.seek(SeekFrom::Start(lldlg.string_offset as u64)).ok();
            Some(read_0terminated_string(file))
        } else {
            None
        };
        let ctrltype = ControlType::from_u32(lldlg.control_type).unwrap();
        let flags = DialogFlags::from_bits(lldlg.flags).unwrap();
        println!("id: {}, string: {:?}, controltype: {:?}",
                 lldlg.control_id,
                 dlgstring,
                 ctrltype);
        println!(" {:?}", flags);

        let smk_overlay = if lldlg.smk_offset > 0 {
            file.seek(SeekFrom::Start(lldlg.smk_offset as u64)).ok();
            let llstruct = SMKLLStruct::read(file);
            Some(Dialog::ll_smk_to_struct(&llstruct, file))
        } else {
            None
        };

        let text_offset =
            if (lldlg.text_offset_x != 0) && (lldlg.text_offset_y != 0) {
                Some(Point::new(lldlg.text_offset_x as i32, lldlg.text_offset_y as i32))
            } else {
                None
            };

        Control {
            rect: rect,
            responsive_area: responsive_rect,
            control_type: ctrltype,
            flags: flags,
            dlgstring: dlgstring,
            smk_overlay: smk_overlay,
            text_offset: text_offset,
        }
    }

    fn ll_smk_to_struct<T: Read + Seek>(llstruct: &SMKLLStruct, file: &mut T) -> SMKElement {
        let smkflags = SMKFlags::from_bits(llstruct.flags).unwrap();
        file.seek(SeekFrom::Start(llstruct.filename as u64)).ok();
        let smkfile = read_0terminated_string(file);

        println!(" smk overlay: {}, flags: {:?}, next overlay: {}",
                 smkfile,
                 smkflags,
                 llstruct.overlay_offset);

        let overlay = if llstruct.overlay_offset > 0 {
            file.seek(SeekFrom::Start(llstruct.overlay_offset as u64)).ok();
            let llstruct = SMKLLStruct::read(file);
            Some(Box::new(Dialog::ll_smk_to_struct(&llstruct, file)))
        } else {
            None
        };

        SMKElement {
            overlay: overlay,
            flags: smkflags,
            filename: smkfile,
            overlay_x: llstruct.overlay_x_pos,
            overlay_y: llstruct.overlay_y_pos,
        }
    }

    pub fn read<T: Read + Seek>(file: &mut T) -> Self {
        println!("reading dialog...");
        println!(" reading low level dialog struct.");

        // The first entry in all .bin files is setting up the dialog area.
        // The format of it is slightly different, such as the SMK offset
        // becomes the offset to the first control.
        let mainlldlg = DialogLLStruct::read(file);
        assert!(mainlldlg.next_entry == 0);

        let mut controls = Vec::<Control>::new();
        if mainlldlg.smk_offset > 0 {
            println!(" reading controls");
            file.seek(SeekFrom::Start(mainlldlg.smk_offset as u64)).ok();
            loop {
                let lldlg = DialogLLStruct::read(file);

                controls.push(Dialog::ll_dlg_to_control(&lldlg, file));
                let next = lldlg.next_entry;
                // ll_ctrls.push(lldlg);
                if next == 0 {
                    break;
                }
                file.seek(SeekFrom::Start(next as u64)).ok();
            }
        }

        Dialog { controls: controls }
    }
}

/// //////////////////////////////////////

struct MenuView {
    menufile: String,
    dlg: Dialog,
}
impl MenuView {
    fn new(gd: &GameData, context: &mut GameContext, menufile: &str) -> Self {
        let dlg = Dialog::read(&mut gd.open(menufile).unwrap());

        let pal = gd.fontmm_reindex.palette.to_sdl();
        context.screen.set_palette(&pal).ok();
        MenuView {
            menufile: menufile.to_owned(),
            dlg: dlg,
        }
    }
}

fn draw_rect(buffer: &mut [u8], buf_stride: u32, rect: &Rect, col: u8) {
    let mut outpos = rect.left() as usize + (rect.top() * buf_stride as i32) as usize;
    let capped_width = (min(buf_stride as i32, rect.right()) - rect.left()) as usize;
    for x in 0..capped_width {
        buffer[outpos + x] = col;
    }
    outpos += buf_stride as usize;
    if rect.height() >= 2 {
        for _ in 0..rect.height() - 2 {
            buffer[outpos] = col;
            buffer[outpos + capped_width] = col;
            outpos += buf_stride as usize;
        }
    }
    for x in 0..capped_width {
        buffer[outpos + x] = col;
    }
}

impl View for MenuView {
    fn render(&mut self, gd: &GameData, context: &mut GameContext, _: &GameState, _: f64) -> ViewAction {
        if context.events.now.quit || context.events.now.key_escape == Some(true) {
            return ViewAction::Quit;
        }
        let screen_pitch = context.screen.pitch();
        // let reindex = &context.gd.fontmm_reindex.data;
        context.screen.with_lock_mut(|buffer: &mut [u8]| {
            for ctrl in &self.dlg.controls {
                ctrl.draw(&gd, buffer, screen_pitch);
            }
        });

        ViewAction::None
    }
}

fn main() {
    ::scrust::spawn("menu rendering",
                    "/home/dm/.wine/drive_c/StarCraft/",
                    |gd, gc, _| {
                        Box::new(MenuView::new(gd, gc,
                                               // "rez/gluexpcmpgn.bin"
                                               // "rez/glucmpgn.bin"
                                               "rez/glumain.bin"
                                               // "rez/gamemenu.bin"
                                               // "rez/glugamemode.bin"
                        ))
                    });

}
