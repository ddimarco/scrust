use ecs::Entity;
use ecs::ServiceManager;

use byteorder::{LittleEndian, ByteOrder};

use iscript::{IScript, AnimationType};
use gamedata::GameData;
use render::{render_buffer_with_solid_reindexing, render_buffer_with_transparency_reindexing, render_buffer_solid};
use gamedata::GRPCache;
use iscriptsys::IScriptSteppingSys;

#[derive(Debug)]
pub enum IScriptEntityAction {
    CreateImageUnderlay {
        parent: Entity,
        image_id: u16,
        rel_x: i8,
        rel_y: i8,
    },
    CreateImageOverlay {
        parent: Entity,
        image_id: u16,
        rel_x: i8,
        rel_y: i8,
    },
    CreateSpriteOverlay { sprite_id: u16, x: u16, y: u16 },
    CreateSpriteUnderlay { parent: Option<Entity>, sprite_id: u16, x: u16, y: u16, use_parent_dir: bool },
    RemoveEntity {entity: Entity},
}
/// *****************************************

/// current unit state
pub enum IScriptCurrentUnitState {
    Idle,
    Moving(i32, i32),
}
pub struct IScriptStateElement {
    pub iscript_id: u32,
    /// current position in iscript
    pub pos: usize,
    /// positions stack to jump back after a call
    pub call_stack: Vec<usize>,

    pub waiting_ticks_left: usize,
    // FIXME: signed or unsigned?
    pub rel_x: i8,
    pub rel_y: i8,
    pub direction: u8,
    pub frameset: u16,
    pub visible: bool,
    pub alive: bool,
    pub current_state: IScriptCurrentUnitState,
    pub movement_angle: f32,

    pub map_pos_x: u16,
    pub map_pos_y: u16,
    pub parent_entity: Option<Entity>,
    pub children: Vec<Entity>,
    /// stops iscript interpretation (for opcode IgnoreRest)
    pub paused: bool
}
impl IScriptStateElement {
    pub fn new(iscript: &IScript,
               iscript_id: u32,
               map_x: u16,
               map_y: u16,
               parent_entity: Option<Entity>)
               -> Self {
        let start_pos = {
            let ref iscript_anim_offsets = iscript.id_offsets_map.get(&iscript_id).unwrap();
            // println!("header:");
            // for anim_idx in 0..iscript_anim_offsets.len() {
            //     let anim = AnimationType::from_usize(anim_idx).unwrap();
            //     let pos = iscript_anim_offsets[anim_idx];
            // println!("{:?}: {}", anim, pos);
            // }
            iscript_anim_offsets[AnimationType::Init as usize]
        };
        IScriptStateElement {
            iscript_id: iscript_id,
            pos: start_pos as usize,
            call_stack: Vec::new(),
            waiting_ticks_left: 0,
            visible: true,
            rel_x: 0,
            rel_y: 0,
            frameset: 0,
            direction: 0,
            movement_angle: 0f32,
            alive: true,
            current_state: IScriptCurrentUnitState::Idle,
            map_pos_x: map_x,
            map_pos_y: map_y,
            parent_entity: parent_entity,
            children: Vec::new(),
            paused: false,
        }
    }

    /// reference to iscript animation offsets
    pub fn iscript_anim_offsets<'a>(&self, iscript: &'a IScript) -> &'a Vec<u16> {
        iscript.id_offsets_map.get(&self.iscript_id).unwrap()
    }

    pub fn set_animation(&mut self, iscript: &IScript, anim: AnimationType) {
        // FIXME: need to also change the animation for its children
        self.waiting_ticks_left = 0;
        self.pos = self.iscript_anim_offsets(iscript)[anim as usize] as usize;
    }
    pub fn is_animation_valid(&self, iscript: &IScript, anim: AnimationType) -> bool {
        self.iscript_anim_offsets(iscript)[anim as usize] > 0
    }

    pub fn set_direction(&mut self, dir: u8) {
        self.direction = dir % 32;
    }
    pub fn turn_cwise(&mut self, units: u8) {
        let new_dir = self.direction + units;
        self.set_direction(new_dir);
    }
    pub fn turn_ccwise(&mut self, units: u8) {
        let new_dir = ((self.direction as i16 - units as i16) + 32) % 32;
        assert!(new_dir >= 0);
        self.set_direction(new_dir as u8);
    }

    pub fn anim_count(&self, iscript: &IScript) -> usize {
        self.iscript_anim_offsets(iscript).len()
    }

    pub fn read_u8(&mut self, iscript: &IScript) -> u8 {
        let val = iscript.data[self.pos as usize];
        self.pos += 1;
        val
    }
    pub fn read_u16(&mut self, iscript: &IScript) -> u16 {
        let val = LittleEndian::read_u16(&iscript.data[(self.pos as usize)..]);
        self.pos += 2;
        val
    }
}
/// *****************************************

/// unit service definition
#[derive(Default)]
pub struct UnitServices {
}
impl UnitServices {
}
impl ServiceManager for UnitServices {}

// component definitions

pub enum SCImageRemapping {
    Normal,
    OFire,
    GFire,
    BFire,
    BExpl,
    Shadow,
}
pub struct SCImageComponent {
    pub image_id: u16,
    pub grp_id: u32,
    pub player_id: usize,
    can_turn: bool,
    remapping: SCImageRemapping,
}
impl SCImageComponent {
    pub fn new(gd: &GameData, image_id: u16) -> Self {
        let grp_id = gd.images_dat.grp_id[image_id as usize];
        {
            gd.grp_cache.borrow_mut().load(gd, grp_id);
        }
        let can_turn = gd.images_dat.graphic_turns[image_id as usize] > 0;

        let draw_func = gd.images_dat.draw_function[image_id as usize];
        let remapping = match draw_func {
            10 => SCImageRemapping::Shadow,
            9 => {
                match gd.images_dat.remapping[image_id as usize] {
                    1 => SCImageRemapping::OFire,
                    2 => SCImageRemapping::GFire,
                    3 => SCImageRemapping::BFire,
                    4 => SCImageRemapping::BExpl,
                    x => {
                        panic!("unknown remapping {}!", x);
                    }
                }
            }
            _ => SCImageRemapping::Normal,
        };

        SCImageComponent {
            image_id: image_id,
            grp_id: grp_id,
            player_id: 0,
            can_turn: can_turn,
            remapping: remapping,
        }
    }

    pub fn reindexing_table<'a>(&self, gd: &'a GameData) -> &'a [u8] {
        match self.remapping {
            SCImageRemapping::OFire => &gd.ofire_reindexing.data,
            SCImageRemapping::BFire => &gd.bfire_reindexing.data,
            SCImageRemapping::GFire => &gd.gfire_reindexing.data,
            SCImageRemapping::BExpl => &gd.bexpl_reindexing.data,
            SCImageRemapping::Shadow => &gd.shadow_reindexing,
            SCImageRemapping::Normal => {
                if self.player_id < 11 {
                    let startpt = self.player_id as usize * 256;
                    &gd.player_reindexing[startpt..startpt + 256]
                } else {
                    // neutral player (i.e. minerals, critters, etc)
                    &gd.player_reindexing[0..256]
                }
            }
        }
    }

    pub fn draw(&self,
            inbuf: &[u8],
            w: u32,
            h: u32,
            flipped: bool,
            cx: i32,
            cy: i32,
            outbuf: &mut [u8],
            outbuf_pitch: u32,
            reindex: &[u8]) {
        match self.remapping {
            SCImageRemapping::OFire | SCImageRemapping::BFire | SCImageRemapping::GFire |
            SCImageRemapping::BExpl | SCImageRemapping::Shadow => {
                render_buffer_with_transparency_reindexing(inbuf,
                                                           w,
                                                           h,
                                                           flipped,
                                                           cx,
                                                           cy,
                                                           outbuf,
                                                           outbuf_pitch,
                                                           &reindex);
            }
            SCImageRemapping::Normal => {
                render_buffer_with_solid_reindexing(inbuf,
                                                    w,
                                                    h,
                                                    flipped,
                                                    cx,
                                                    cy,
                                                    outbuf,
                                                    outbuf_pitch,
                                                    &reindex);
            }
        }
    }

    // TODO: it might make sense to join scimage & iscriptstate
    pub fn frame_idx(&self, iscript_state: &IScriptStateElement) -> usize {
        if !self.can_turn {
            iscript_state.frameset as usize
        } else if iscript_state.direction > 16 {
            (iscript_state.frameset + 32 - iscript_state.direction as u16) as usize
        } else {
            (iscript_state.frameset + iscript_state.direction as u16) as usize
        }
    }

    pub fn draw_flipped(&self, iscript_state: &IScriptStateElement) -> bool {
        self.can_turn && (iscript_state.direction > 16)
    }
}

pub struct SCSpriteComponent {
    pub sprite_id: u16,
}
pub struct SelectableComponent {
    /// from sprites.dat: length of health bar in pixels
    pub health_bar: u8,
    pub circle_offset: u8,
    pub circle_grp_id: u32,

    pub sel_width: u16,
    pub sel_height: u16,
}
impl SelectableComponent {
    pub fn draw_selection_circle(&self,
                                 grp_cache: &GRPCache,
                                 cx: i32,
                                 cy: i32,
                                 buffer: &mut [u8],
                                 buffer_pitch: u32) {
        let grp = grp_cache.get_ro(self.circle_grp_id);
        render_buffer_solid(&grp.frames[0],
                            grp.header.width as u32,
                            grp.header.height as u32,
                            false,
                            cx,
                            cy + self.circle_offset as i32,
                            buffer,
                            buffer_pitch);
    }


    // FIXME: clipping
    pub fn draw_healthbar(&self, cx: u32, cy: u32, buffer: &mut [u8], buffer_pitch: u32) {
        let boxes = self.health_bar as u32 / 3;
        let box_width = 3;
        if self.health_bar == 0 {
            return;
        }
        let width = 2 + (box_width * boxes) + (boxes - 1);
        let height = 8;

        let mut outpos = ((cy + self.circle_offset as u32) - height / 2) *
            buffer_pitch + (cx - width / 2);
        for y in 0..height {
            for x in 0..width {
                let outer_border = y == 0 || y == height - 1 || x == 0 || x == (width - 1);
                let inner_border = x % (box_width + 1) == 0;
                if inner_border || outer_border {
                    // black
                    buffer[outpos as usize] = 0;
                } else {
                    // green
                    buffer[outpos as usize] = 185;
                }
                outpos += 1;
            }
            outpos += buffer_pitch - width;
        }
    }
}
#[derive(Debug)]
pub enum FlingyMoveControl {
    FlingyDat,
    PartiallyMobile,
    IScriptBin,
}
pub struct SCFlingyComponent {
    pub flingy_id: u16,
    pub move_control: FlingyMoveControl,
}
pub struct SCUnitComponent {
    pub unit_id: u16,
    pub kill_count: usize,
}

// TODO: merge into 1?
pub struct UnderlayComponent {}
pub struct OverlayComponent {}

use ecs::system::LazySystem;
components! {
    #[builder(EntityInit)]
    struct UnitComponents {
        #[hot] iscript_state: IScriptStateElement,
        #[hot] scimage: SCImageComponent,
        #[hot] selectable: SelectableComponent,
        #[hot] scsprite: SCSpriteComponent,
        #[hot] scflingy: SCFlingyComponent,
        #[hot] scunit: SCUnitComponent,

        #[hot] underlay: UnderlayComponent,
        #[hot] overlay: OverlayComponent,
    }
}
systems! {
    struct UnitSystems<UnitComponents, UnitServices> {
        active: {
            iscript_stepping_sys: LazySystem<IScriptSteppingSys>
                = LazySystem::<IScriptSteppingSys>::new(),
        },
        passive: {
        }
    }
}
