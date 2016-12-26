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

use scrust::render::{render_block};

use scrust::ui::MousePointer;

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

#[macro_use]
extern crate ecs;

use ecs::World;
use ecs::DataHelper;
use ecs::system::{EntityProcess, EntitySystem, System};
use ecs::{EntityIter};
use ecs::BuildData;

pub struct UIElement {
    rect: Rect,
    flags: DialogFlags,
    bwid: u32,
    visible: bool,
}
pub struct ButtonElement {
    responsive_area: Rect,
    hotkey: Option<char>,
    in_focus: bool,
}
#[derive(Clone)]
pub struct LabelElement {
    text: String,
    font_size: FontSize,
    text_offset: Option<Point>,
    horizontal_alignment: HorizontalAlignment,
    vertical_alignment: VerticalAlignment,
}
pub struct ImageElement {
    imgpath: String,
}
struct SMKOverlay {
    smkfile: String,
    offset: Point,
    flags: SMKFlags,
    current_frame: usize,
    frame_count: usize,
}
pub struct SMKOverlaysElement {
    // a single control can have multiple smk overlays
    overlays: Vec<SMKOverlay>,
}
components! {
    struct DialogComponents {
        #[hot] ui_element: UIElement,
        #[hot] button_element: ButtonElement,
        #[hot] label_element: LabelElement,
        #[cold] img_element: ImageElement,
        #[cold] smk_overlays_element: SMKOverlaysElement,
    }
}

pub struct VideoSteppingSys {}
impl System for VideoSteppingSys {
    type Components = DialogComponents;
    type Services = ();
}
impl EntityProcess for VideoSteppingSys {
        fn process(&mut self, entities: EntityIter<DialogComponents>,
                   dh: &mut DataHelper<DialogComponents, ()>) {
            for e in entities {
                for smk_el in &mut dh.smk_overlays_element[e].overlays {
                    smk_el.current_frame = (smk_el.current_frame + 1) %
                        smk_el.frame_count;
                }
            }
        }
}

use scrust::ImmediateEvents;

pub struct InputSys {
    mouse_pos: Point,
    events: ImmediateEvents,
    // output field: bwid of chosen element
    selected_entry: Option<u32>,
}
impl Default for InputSys {
    fn default() -> Self {
        InputSys {
            mouse_pos: Point::new(0, 0),
            events: ImmediateEvents::new(),
            selected_entry: None,
        }
    }
}

impl System for InputSys {
    type Components = DialogComponents;
    type Services = ();
}

impl EntityProcess for InputSys {
    fn process(&mut self, entities: EntityIter<DialogComponents>,
               dh: &mut DataHelper<DialogComponents, ()>) {
        let mp = &self.mouse_pos;
        self.selected_entry = None;
        for e in entities {
            let focused = {
                let rrect = &dh.button_element[e].responsive_area;
                rrect.contains(*mp)
            };
            dh.button_element[e].in_focus = focused;
            if self.events.mouse_left && focused {
                self.selected_entry = Some(dh.ui_element[e].bwid);
            }
            // TODO: implement hotkey handling
        }
    }
}

systems! {
    struct DialogSystems<DialogComponents, ()> {
        active: {
            video_stepping_sys: EntitySystem<VideoSteppingSys>
                = EntitySystem::new(VideoSteppingSys{},
                                    aspect!(<DialogComponents> all: [ui_element, smk_overlays_element])),
        },
        passive: {
            input_sys: EntitySystem<InputSys>
                = EntitySystem::new(InputSys::default(),
                                    aspect!(<DialogComponents> all: [button_element])),
        }
    }
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

struct Dialog {
    world: World<DialogSystems>,
}
impl Dialog {
    fn ll_dlg_to_entity<T: Read + Seek>(gd: &GameData, lldlg: &DialogLLStruct, file: &mut T,
                                        world: &mut World<DialogSystems>) {
        let rect = Rect::new(lldlg.left_pos as i32,
                             lldlg.top_pos as i32,
                             lldlg.width as u32,
                             lldlg.height as u32);
        let flags = DialogFlags::from_bits(lldlg.flags).unwrap();

        let uielement_component = UIElement {
            rect: rect,
            flags: flags,
            bwid: lldlg.control_id as u32,
            visible: flags.contains(DLG_VISIBLE),
        };
        /*let entity =*/ world.create_entity(|entity: BuildData<DialogComponents>, data: &mut DialogComponents| {
            data.ui_element.add(&entity, uielement_component);

            let mut dlgstring = if lldlg.string_offset > 0 {
                file.seek(SeekFrom::Start(lldlg.string_offset as u64)).ok();
                read_0terminated_string(file)
            } else {
                "".to_owned()
            };

            let ctrltype = ControlType::from_u32(lldlg.control_type).unwrap();
            match ctrltype {
                ControlType::Button |
                ControlType::LightupButton |
                ControlType::DefaultButton |
                ControlType::OptionButton |
                ControlType::CheckBox |
                ControlType::ListBox |
                ControlType::ComboBox => {
                    let responsive_rect =
                        Rect::new(lldlg.response_area_left as i32 + rect.left(),
                                  lldlg.response_area_top as i32 + rect.top(),
                                  lldlg.response_area_width as u32,
                                  lldlg.response_area_height as u32);
                    let hotkey = if flags.contains(DLG_HAS_HOTKEY) {
                        // cut off hotkey
                        let hk = dlgstring.chars().next().unwrap();
                        dlgstring = dlgstring[1..].to_owned();
                        Some(hk)
                    } else {
                        None
                    };
                    data.button_element.add(&entity, ButtonElement {
                        responsive_area: responsive_rect,
                        hotkey: hotkey,
                        in_focus: false,
                    });
                }
                _ => {}
            }


        let font_size = if flags.contains(DLG_FONT16) {
            FontSize::Font16
        } else if flags.contains(DLG_FONT14) {
            FontSize::Font14
        } else if flags.contains(DLG_FONT10) {
            FontSize::Font10
        } else if flags.contains(DLG_FONT16X) {
            FontSize::Font16X
        } else {
            FontSize::Font16
        };
        let valign = if flags.contains(DLG_VERTICAL_ALIGNMENT_TOP) {
            VerticalAlignment::Top
        } else if flags.contains(DLG_VERTICAL_ALIGNMENT_MIDDLE) {
            VerticalAlignment::Center
        } else if flags.contains(DLG_VERTICAL_ALIGNMENT_BOTTOM) {
            VerticalAlignment::Bottom
        } else {
            VerticalAlignment::Center
        };
        let text_offset = if (lldlg.text_offset_x != 0) && (lldlg.text_offset_y != 0) {
            Some(Point::new(lldlg.text_offset_x as i32, lldlg.text_offset_y as i32))
        } else {
            None
        };
        match ctrltype {
            ControlType::Button | ControlType::DefaultButton => {
                let halign = HorizontalAlignment::Center;
                data.label_element.add(&entity,
                                       LabelElement {
                                           text: dlgstring,
                                           font_size: font_size,
                                           text_offset: text_offset,
                                           horizontal_alignment: halign,
                                           vertical_alignment: valign,
                                       });
            },
            ControlType::LightupButton => {
                let halign = if flags.contains(DLG_HORIZONTAL_ALIGNMENT_CENTER) ||
                    flags.contains(DLG_HORIZONTAL_ALIGNMENT_CENTER2) {
                        HorizontalAlignment::Center
                    } else if flags.contains(DLG_HORIZONTAL_ALIGNMENT_RIGHT) {
                        HorizontalAlignment::Right
                    } else {
                        HorizontalAlignment::Left
                    };
                data.label_element.add(&entity,
                                       LabelElement {
                                           text: dlgstring,
                                           font_size: font_size,
                                           text_offset: text_offset,
                                           horizontal_alignment: halign,
                                           vertical_alignment: valign,
                                       });
            },
            ControlType::LabelLeftAlign => {
                data.label_element.add(&entity, LabelElement {
                    text: dlgstring,
                    font_size: font_size,
                    text_offset: text_offset,
                    horizontal_alignment: HorizontalAlignment::Left,
                    vertical_alignment: valign,
                });
            },
            ControlType::LabelRightAlign => {
                data.label_element.add(&entity, LabelElement {
                    text: dlgstring,
                    font_size: font_size,
                    text_offset: text_offset,
                    horizontal_alignment: HorizontalAlignment::Right,
                    vertical_alignment: valign,
                });
            },
            ControlType::LabelCenterAlign => {
                data.label_element.add(&entity, LabelElement {
                    text: dlgstring,
                    font_size: font_size,
                    text_offset: text_offset,
                    horizontal_alignment: HorizontalAlignment::Center,
                    vertical_alignment: valign,
                });
            },
            ControlType::Image => {
                gd.pcx_cache.borrow_mut().load(gd, dlgstring.as_str());
                data.img_element.add(&entity, ImageElement {
                    imgpath: dlgstring,
                });
            },
            _ => {}
        }

            // smk overlay(s)
            if lldlg.smk_offset > 0 {
                let mut smkoverlays = Vec::<SMKOverlay>::new();

                // read all smk overlays
                let mut smk_offset = lldlg.smk_offset;
                let mut smk_cache = gd.video_cache.borrow_mut();
                while smk_offset > 0 {
                    file.seek(SeekFrom::Start(smk_offset as u64)).ok();
                    let llstruct = SMKLLStruct::read(file);

                    let smkflags = SMKFlags::from_bits(llstruct.flags).unwrap();
                    let offset = Point::new(llstruct.overlay_x_pos as i32,
                                            llstruct.overlay_y_pos as i32);

                    file.seek(SeekFrom::Start(llstruct.filename as u64)).ok();
                    let smkfile = read_0terminated_string(file);
                    println!(" smk overlay: {}, flags: {:?}, next overlay: {}",
                             smkfile,
                             smkflags,
                             llstruct.overlay_offset);
                    let fcount = smk_cache.get(gd, &smkfile).frames.len();
                    let ol = SMKOverlay {
                        flags: smkflags,
                        smkfile: smkfile,
                        current_frame: 0,
                        frame_count: fcount,
                        offset: offset,
                    };
                    smkoverlays.push(ol);
                    smk_offset = llstruct.overlay_offset;
                }
                println!("read {} smk overlays", smkoverlays.len());

                data.smk_overlays_element.add(&entity, SMKOverlaysElement {
                    overlays: smkoverlays,
                });
            };

        });


        // let responsive_rect = Rect::new(lldlg.response_area_left as i32 + rect.left(),
        //                                 lldlg.response_area_top as i32 + rect.top(),
        //                                 lldlg.response_area_width as u32,
        //                                 lldlg.response_area_height as u32);
        // let dlgstring = if lldlg.string_offset > 0 {
        //     file.seek(SeekFrom::Start(lldlg.string_offset as u64)).ok();
        //     Some(read_0terminated_string(file))
        // } else {
        //     None
        // };
        // let ctrltype = ControlType::from_u32(lldlg.control_type).unwrap();
        // println!("id: {}, string: {:?}, controltype: {:?}",
        //          lldlg.control_id,
        //          dlgstring,
        //          ctrltype);
        // println!(" {:?}", flags);
    }


    pub fn read<T: Read + Seek>(gd: &GameData, file: &mut T) -> Self {
        println!("reading dialog...");
        println!(" reading low level dialog struct.");

        let mut world = World::<DialogSystems>::new();

        // The first entry in all .bin files is setting up the dialog area.
        // The format of it is slightly different, such as the SMK offset
        // becomes the offset to the first control.
        let mainlldlg = DialogLLStruct::read(file);
        assert!(mainlldlg.next_entry == 0);

        if mainlldlg.smk_offset > 0 {
            println!(" reading controls");
            file.seek(SeekFrom::Start(mainlldlg.smk_offset as u64)).ok();
            loop {
                let lldlg = DialogLLStruct::read(file);
                Dialog::ll_dlg_to_entity(gd, &lldlg, file, &mut world);
                let next = lldlg.next_entry;
                if next == 0 {
                    break;
                }
                file.seek(SeekFrom::Start(next as u64)).ok();
            }
        }

        Dialog {
            world: world,
        }
    }
}

/// //////////////////////////////////////

struct MenuView {
    dlg: Dialog,
    mouse_pointer: MousePointer,

    bgd_pcx: String,
}
impl MenuView {
    fn new(gd: &GameData, context: &mut GameContext, short_name: &str, menufile: &str) -> Self {
        let dlg = Dialog::read(gd, &mut gd.open(menufile).unwrap());

        let sn = short_name.to_owned();
        let bgd_pcx = format!("glue/pal{}/backgnd.pcx", sn);
        let mut cache = gd.pcx_cache.borrow_mut();
        let pal = cache.get(gd, &bgd_pcx);
        context.screen.set_palette(&pal.palette.to_sdl()).ok();
        let mp = MousePointer::new(gd, context);
        MenuView {
            dlg: dlg,
            mouse_pointer: mp,
            bgd_pcx: bgd_pcx,
        }
    }
}

impl View for MenuView {
    fn render_layers(&mut self, context: &mut GameContext) {
        self.mouse_pointer.render(&mut context.renderer);
    }
    fn render(&mut self, gd: &GameData, context: &mut GameContext, _: &GameState, _: f64) -> ViewAction {
        if context.events.now.quit || context.events.now.key_escape == Some(true) {
            return ViewAction::Quit;
        }

        // update input
        if let Some((mouse_x, mouse_y)) = context.events.now.mouse_move {
            self.mouse_pointer.update_pos(mouse_x, mouse_y);
            self.dlg.world.systems.input_sys.mouse_pos = Point::new(mouse_x, mouse_y);
        }
        self.dlg.world.systems.input_sys.events = context.events.now;
        //self.mouse_pointer.update();
        process!(self.dlg.world, input_sys);
        match self.dlg.world.systems.input_sys.selected_entry {
            Some(bwid) => {
                println!("selected {}!", bwid);
            },
            _ => {},
        }

        // clear the screen
        context.screen.fill_rect(None, Color::RGB(0, 0, 0)).ok();

        let pcx_cache = gd.pcx_cache.borrow();
        let bgd = pcx_cache.get_ro(&self.bgd_pcx);

        self.dlg.world.update();
        // FIXME: get proper reindexing table
        let reindex = &gd.fontmm_reindex.data;
        let screen_pitch = context.screen.pitch();
        context.screen.with_lock_mut(|buffer: &mut [u8]| {
            render_block(&bgd.data, bgd.header.width as usize, bgd.header.height as usize,
                         0, 0, buffer, screen_pitch as usize);

            let dh = &self.dlg.world.data;
            for e in self.dlg.world.entities() {
                if dh.ui_element.has(&e) {
                    if !dh.ui_element[e].visible {
                        continue;
                    }
                    // draw_rect(buffer, screen_pitch, &dh.ui_element[e].rect, 21);
                }
                if dh.img_element.has(&e) {
                    let rect = &dh.ui_element[e].rect;
                    let pcx = pcx_cache.get_ro(&dh.img_element[e].imgpath);
                    render_block(&pcx.data,
                                 pcx.header.width as usize,
                                 pcx.header.height as usize,
                                 rect.left(), rect.top(),
                                 buffer, screen_pitch as usize);

                }
                if dh.smk_overlays_element.has(&e) {
                    let rect = &dh.ui_element[e].rect;
                    let focused = if dh.button_element.has(&e) {
                        dh.button_element[e].in_focus
                    } else {
                        false
                    };
                    let cache = gd.video_cache.borrow();
                    for ol in &dh.smk_overlays_element[e].overlays {
                        if ol.flags.contains(SMK_SHOW_IF_OVER) && !focused {
                            continue;
                        }
                        let video = cache.get_ro(&ol.smkfile);
                        let frame = &video.frames[ol.current_frame];
                        let pt = rect.top_left() + ol.offset;
                        render_block(frame, video.width, video.height,
                                     pt.x(), pt.y(), buffer, screen_pitch as usize);
                    }

                }
                if dh.label_element.has(&e) {
                    let fnt = gd.font(dh.label_element[e].font_size);
                    let halign = dh.label_element[e].horizontal_alignment.clone();
                    let valign = dh.label_element[e].vertical_alignment.clone();
                    match dh.label_element[e].text_offset {
                        Some(offset) => {
                            let mut rect = dh.ui_element[e].rect;
                            rect.offset(offset.x(), offset.y());
                            fnt.render_text_aligned(dh.label_element[e].text.as_ref(), 1,
                                                    reindex,
                                                    buffer,
                                                    screen_pitch, &rect,
                                                    HorizontalAlignment::Left,
                                                    VerticalAlignment::Top);
                            },
                        None => {
                            fnt.render_text_aligned(dh.label_element[e].text.as_ref(), 1,
                                                    reindex, buffer,
                                                    screen_pitch, &dh.ui_element[e].rect,
                                                    halign,
                                                    valign);
                        }
                    }
                }
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
                                               "mm",
                                               // "rez/gluexpcmpgn.bin"
                                               // "rez/glucmpgn.bin"
                                               "rez/glumain.bin"
                                               // "rez/gamemenu.bin"
                                               // "rez/glugamemode.bin"
                        ))
                    });

}
