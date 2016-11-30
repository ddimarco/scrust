use std::cmp::min;
use std::io::{Read, Seek, SeekFrom};

extern crate sdl2;
use sdl2::rect::Rect;
use sdl2::render::{Renderer, Texture};
use sdl2::pixels::Color;

extern crate byteorder;
use byteorder::{LittleEndian, ReadBytesExt};

#[macro_use]
extern crate scrust;
use scrust::font::FontSize;

use scrust::font::RenderText;
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
    DefaultButton = 0x1,
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

struct Control {
    rect: Rect,
    control_type: ControlType,
    flags: DialogFlags,
    dlgstring: Option<String>,
    responsive_area: Rect,
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
        Control {
            rect: rect,
            responsive_area: responsive_rect,
            control_type: ctrltype,
            flags: flags,
            dlgstring: dlgstring,
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
            let mut ll_ctrls = Vec::<DialogLLStruct>::new();
            loop {
                let lldlg = DialogLLStruct::read(file);
                let next = lldlg.next_entry;
                ll_ctrls.push(lldlg);
                if next == 0 {
                    break;
                }
                file.seek(SeekFrom::Start(next as u64)).ok();
            }
            println!("got {} ll dlg controls", ll_ctrls.len());
            controls = ll_ctrls.iter()
                .map(|lldlg: &DialogLLStruct| Dialog::ll_dlg_to_control(lldlg, file))
                .collect();
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
    fn new(context: &mut GameContext, menufile: &str) -> Self {
        let dlg = Dialog::read(&mut context.gd.open(menufile).unwrap());

        let pal = context.gd.fontmm_reindex.palette.to_sdl();
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
    fn render(&mut self, context: &mut GameContext, _: &GameState, _: f64) -> ViewAction {
        if context.events.now.quit || context.events.now.key_escape == Some(true) {
            return ViewAction::Quit;
        }
        // let fnt = &context.gd.font(self.font_size);
        let screen_pitch = context.screen.pitch();
        // let reindex = &context.gd.fontmm_reindex.data;
        context.screen.with_lock_mut(|buffer: &mut [u8]| {
            // fnt.render_textbox(self.text.as_ref(),
            //                    self.color_idx,
            //                    reindex,
            //                    buffer,
            //                    screen_pitch,
            //                    &self.trg_rect);
            for ctrl in &self.dlg.controls {
                // context.renderer.draw_rect(ctrl.rect);
                draw_rect(buffer, screen_pitch, &ctrl.rect, 21);
                draw_rect(buffer, screen_pitch, &ctrl.responsive_area, 10);
            }
        });

        // context.renderer.set_draw_color(Color::RGB(255, 255, 255));

        ViewAction::None
    }
}

fn main() {
    ::scrust::spawn("menu rendering",
                    "/home/dm/.wine/drive_c/StarCraft/",
                    |gc, _| {
                        Box::new(MenuView::new(gc,
                                               //"rez/gluexpcmpgn.bin"
                                               "rez/glumain.bin"))
                    });

}
