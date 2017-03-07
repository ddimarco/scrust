use std::io::Read;
use byteorder::{LittleEndian, ReadBytesExt};

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

enum_from_primitive! {
#[derive(PartialEq)]
#[derive(Debug)]
pub enum ControlType {
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
