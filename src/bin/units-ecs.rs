extern crate sdl2;
use sdl2::pixels::Color;

extern crate scrust;
use scrust::gamedata::GameData;
use scrust::{GameContext, GameState, View, ViewAction};

use scrust::iscript::{IScript, AnimationType, OpCode};

use scrust::render::{render_buffer_solid, render_buffer_with_transparency_reindexing, render_buffer_with_solid_reindexing};

extern crate byteorder;
use byteorder::{ReadBytesExt, LittleEndian};
use byteorder::ByteOrder;

extern crate rand;
use rand::Rng;

extern crate num;
use num::FromPrimitive;

#[macro_use]
extern crate ecs;

use ecs::World;
use ecs::DataHelper;
use ecs::system::{EntityProcess, EntitySystem, System};
use ecs::EntityIter;
use ecs::BuildData;
use ecs::ServiceManager;
// use ecs::IndexedEntity;
use ecs::Entity;

/// Public*****************************************

macro_rules! var_read {
    (u8, $gd: ident, $file:expr) => ($file.read_u8($gd));
    (u16, $gd: ident, $file:expr) => ($file.read_u16($gd));
    (u32, $gd: ident, $file:expr) => ($file.read_u32($gd));
}
macro_rules! def_opcodes {
    (
        $self_var:expr,
        $gd:ident,
        $debug:expr,
        $code_var:ident,
        $( $opcode:pat => ( $( $param:ident : $tpe:ident),*)
           $code:block),*
    )
        =>
    {

            match $code_var {
                $(
                    $opcode => {
                        if $debug {
                            print!("op: {:?}(", $code_var);
                        }
                        $(
                            let $param = var_read!($tpe, $gd, $self_var);
                            if $debug {
                                print!("{}: {} = {}, ", stringify!($param),
                                         stringify!($tpe), $param);
                            }
                        )*
                            if $debug {
                                println!(")");
                            }
                            $code
                    }
                ),*

                _ => panic!("unknown opcode: {:?}", $code_var),
            }
    }
}

#[derive(Debug)]
pub enum IScriptEntityAction {
    CreateImageUnderlay { image_id: u16, rel_x: i8, rel_y: i8 },
    CreateImageOverlay { image_id: u16, rel_x: i8, rel_y: i8 },
    CreateSpriteOverlay { sprite_id: u16, x: u16, y: u16 },
    CreateSpriteUnderlay { sprite_id: u16, x: u16, y: u16 },
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
    pub pos: u16,

    pub waiting_ticks_left: usize,
    // FIXME: signed or unsigned?
    pub rel_x: i8,
    pub rel_y: i8,
    pub direction: u8,
    pub frameset: u16,
    pub follow_main_graphic: bool,
    pub visible: bool,
    pub alive: bool,
    pub current_state: IScriptCurrentUnitState,
    pub movement_angle: f32,

    pub map_pos_x: u16,
    pub map_pos_y: u16,
    // parent: Option<ecs::EntityData<'a, UnitComponents>>,
}
impl IScriptStateElement {
    pub fn new(iscript: &IScript,
               iscript_id: u32,
               map_x: u16,
               map_y: u16)
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
            // image_id: image_id,
            pos: start_pos,
            waiting_ticks_left: 0,
            visible: true,
            rel_x: 0,
            rel_y: 0,
            frameset: 0,
            direction: 0,
            movement_angle: 0f32,
            follow_main_graphic: false,
            alive: true,
            current_state: IScriptCurrentUnitState::Idle,
            map_pos_x: map_x,
            map_pos_y: map_y,
        }
    }

    /// reference to iscript animation offsets
    pub fn iscript_anim_offsets<'a>(&self, iscript: &'a IScript) -> &'a Vec<u16> {
        iscript.id_offsets_map.get(&self.iscript_id).unwrap()
    }

    // pub fn set_animation(&mut self, anim: AnimationType) {
    //     self.pos = self.iscript_anim_offsets()[anim as usize];
    // }
    // pub fn is_animation_valid(&self, anim: AnimationType) -> bool {
    //     self.iscript_anim_offsets()[anim as usize] > 0
    // }

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

    // pub fn current_animation(&self) -> AnimationType {
    //     let mut nearest_label = AnimationType::Init;
    //     let mut nearest_dist = 10000;
    //     for lbl_idx in 0..self.iscript_anim_offsets().len() {
    //         let lbl_pos = self.iscript_anim_offsets()[lbl_idx];
    //         if self.pos >= lbl_pos {
    //             let dist = self.pos - lbl_pos;
    //             if dist < nearest_dist {
    //                 nearest_label = AnimationType::from_usize(lbl_idx).unwrap();
    //                 nearest_dist = dist;
    //             }
    //         }
    //     }
    //     nearest_label
    // }

    pub fn anim_count(&self, iscript: &IScript) -> usize {
        self.iscript_anim_offsets(iscript).len()
    }

    fn read_u8(&mut self, iscript: &IScript) -> u8 {
        let val = iscript.data[self.pos as usize];
        self.pos += 1;
        val
    }
    fn read_u16(&mut self, iscript: &IScript) -> u16 {
        let val = LittleEndian::read_u16(&iscript.data[(self.pos as usize)..]);
        self.pos += 2;
        val
    }
    fn read_u32(&mut self, iscript: &IScript) -> u32 {
        let val = LittleEndian::read_u32(&iscript.data[(self.pos as usize)..]);
        self.pos += 4;
        val
    }
}
/// *****************************************

/// unit service definition
#[derive(Default)]
pub struct UnitServices {
    iscript: Option<IScript>,
}
impl UnitServices {
    pub fn load_iscript(&mut self, gd: &GameData) {
        let iscript = IScript::read(&mut gd.open("scripts/iscript.bin").unwrap());
        self.iscript = Some(iscript);
    }
}
impl ServiceManager for UnitServices {}

// component definitions


pub struct IScriptSteppingSys {
    // ugly HACK, bc we cannot borrow refs to service.xyz and dh.yyy in process()
    iscript_copy: Option<IScript>,
}
impl System for IScriptSteppingSys {
    type Components = UnitComponents;
    type Services = UnitServices;
}
impl IScriptSteppingSys {
    fn create_img_underlay(&self, img_id: u16, rel_x: i8, rel_y: i8) {
    }
    fn create_img_overlay(&self, img_id: u16, rel_x: i8, rel_y: i8) {
    }

    fn interpret_iscript(&self,
                         cpy: &IScript,
                         e: ecs::EntityData<UnitComponents>,
                         parent: Option<ecs::EntityData<UnitComponents>>,
                         dh: &mut DataHelper<UnitComponents, UnitServices>) {
        if !dh.iscript_state[e].alive {
            return;
        }
    // FIXME: is waiting actually counted in frames?
        if dh.iscript_state[e].waiting_ticks_left > 0 {
            dh.iscript_state[e].waiting_ticks_left -= 1;
            return;
        }

        let opcode = OpCode::from_u8(dh.iscript_state[e].read_u8(cpy))
            .expect("couldn't read opcode!");
        def_opcodes! (
                dh.iscript_state[e],
                cpy,
                true,
                opcode,

                OpCode::ImgUl => (image_id: u16, rel_x: u8, rel_y: u8) {
                    // shadows and such; img* is associated with the current entity
                    // actions.push(IScriptEntityAction::CreateImageUnderlay {
                    //     image_id: image_id,
                    //     rel_x: rel_x as i8,
                    //     rel_y: rel_y as i8,
                    // });
                    self.create_img_underlay(image_id, rel_x as i8, rel_y as i8);
                },

        OpCode::ImgOl => (image_id: u16, rel_x: u8, rel_y: u8) {
        // e.g. explosions on death
            // actions.push(IScriptEntityAction::CreateImageOverlay {
            //     image_id: image_id,
            //     rel_x: rel_x as i8,
            //     rel_y: rel_y as i8,
            // });
            self.create_img_overlay(image_id, rel_x as i8, rel_y as i8);
        },
        OpCode::SprOl => (sprite_id: u16, rel_x: u8, rel_y: u8) {
        // independent overlay, e.g. scanner sweep
        // FIXME
            println!("--- sprol not implemented yet ---");
            // actions.push(IScriptEntityAction::CreateSpriteOverlay {
            //     sprite_id: sprite_id,
            //     x: (rel_x as u16) + (dh.iscript_state[e].rel_x as u16) +
            //         dh.iscript_state[e].map_pos_x,
            //     y: (rel_y as u16) + (dh.iscript_state[e].rel_y as u16) +
            //         dh.iscript_state[e].map_pos_y,
            // });
        },
        OpCode::LowSprUl => (sprite_id: u16, rel_x: u8, rel_y: u8) {
        // independent underlay, e.g. gore
        // FIXME
            println!("--- lowsprul not implemented yet ---");
            // actions.push(IScriptEntityAction::CreateSpriteUnderlay {
            //     sprite_id: sprite_id,
            //     x: (rel_x as u16) + (dh.iscript_state[e].rel_x as u16) +
            //         dh.iscript_state[e].map_pos_x,
            //     y: (rel_y as u16) + (dh.iscript_state[e].rel_y as u16) +
            //         dh.iscript_state[e].map_pos_y,
            // });
        },

        OpCode::CreateGasOverlays => (overlay_no: u8) {
            let smoke_img_id = 430 + overlay_no as u16;
            // FIXME
            // let overlay_id = gd.images_dat.special_overlay[self.image_id as usize];
            // let (rx, ry) = {
            //     let mut c = gd.lox_cache.borrow_mut();
            //     let lo = c.get(&gd, overlay_id) ;
            //     lo.frames[0].offsets[overlay_no as usize]
            // };
        //     return Some(IScriptEntityAction::CreateImageOverlay {
        //         image_id: smoke_img_id,
        // // FIXME signed or unsigned?
        //         rel_x: rx,
        //         rel_y: ry,
        //     });
        },

        OpCode::PlayFram => (frame: u16) {
        // println!("playfram: {}", frame);
            dh.iscript_state[e].frameset = frame;
        },
        OpCode::PlayFramTile => (frame: u16) {
            println!("--- playframtile not implemented yet ---");
        // FIXME
        },
        OpCode::EngFrame => (frame: u8) {
        // FIXME is this right: same as playfram?
            dh.iscript_state[e].frameset = frame as u16;
        },
        OpCode::FollowMainGraphic => () {
            // assert!(parent.is_some());
            dh.iscript_state[e].follow_main_graphic = true;
        },
        OpCode::EngSet => () {
        // same as FollowMainGraphic
            // assert!(parent.is_some());
            dh.iscript_state[e].follow_main_graphic = true;
        },

        OpCode::Wait => (ticks: u8) {
            dh.iscript_state[e].waiting_ticks_left += ticks as usize;
        },
        OpCode::WaitRand => (minticks: u8, maxticks: u8) {
            let r = ::rand::thread_rng().gen_range(minticks, maxticks+1);
            dh.iscript_state[e].waiting_ticks_left += r as usize;
        },
        OpCode::SigOrder => (signal: u8) {
        // FIXME
            println!("--- not implemented yet ---");
        },
        OpCode::Goto => (target: u16) {
            dh.iscript_state[e].pos = target;
        },
        OpCode::RandCondJmp => (val: u8, target: u16) {
            let r = ::rand::random::<u8>();
            if r < val {
                dh.iscript_state[e].pos = target;
            }
        },
        OpCode::TurnRand => (units: u8) {
            if ::rand::thread_rng().gen_range(0, 100) < 50 {
                dh.iscript_state[e].turn_cwise(units);
            } else {
                dh.iscript_state[e].turn_ccwise(units);
            }
        },
        OpCode::TurnCWise => (units: u8) {
            dh.iscript_state[e].turn_cwise(units);
        },
        OpCode::TurnCCWise => (units: u8) {
            dh.iscript_state[e].turn_ccwise(units);
        },
        OpCode::SetFlDirect => (dir: u8) {
            dh.iscript_state[e].set_direction(dir);
        },
        // FIXME: might be signed bytes?
        OpCode::SetVertPos => (val: u8) {
            dh.iscript_state[e].rel_y = val as i8;
        },
        OpCode::SetHorPos => (val: u8) {
            dh.iscript_state[e].rel_x = val as i8;
        },
        OpCode::Move => (dist: u8) {
            let mut iss = &mut dh.iscript_state[e];
            let fdist = dist as f32;
            let (dx, dy) = (iss.movement_angle.cos() * fdist ,
                            iss.movement_angle.sin() * fdist);
            iss.map_pos_x = (iss.map_pos_x as i32 + dx.round() as i32) as u16;
            iss.map_pos_y = (iss.map_pos_y as i32 + dy.round() as i32) as u16;
        },

        // FIXME sounds
        OpCode::PlaySndBtwn => (val1: u16, val2: u16) {
        },
        OpCode::PlaySnd => (sound_id: u16) {
        },

        OpCode::NoBrkCodeStart => () {
        // FIXME
        },
        OpCode::NoBrkCodeEnd => () {
        // FIXME
        },
        OpCode::TmpRmGraphicStart => () {
        // Sets the current image overlay state to hidden
            println!("tmprmgraphicstart, has parent: {}", parent.is_some());
            dh.iscript_state[e].visible = false;
        },
        OpCode::TmpRmGraphicEnd => () {
        // Sets the current image overlay state to visible
            println!("tmprmgraphicend, has parent: {}", parent.is_some());
            dh.iscript_state[e].visible = true;
        },
        OpCode::Attack => () {
        // FIXME
        },
        OpCode::AttackWith => (weapon: u8) {
        // FIXME
        },
        OpCode::GotoRepeatAttk => () {
        // Signals to StarCraft that after this point, when the unit's cooldown time
        // is over, the repeat attack animation can be called.
        // FIXME
        },
        OpCode::IgnoreRest => () {
        // this causes the script to stop until the next animation is called.
        // FIXME
        },
        OpCode::SetFlSpeed => (speed: u16) {
        // FIXME
        },

        OpCode::End => () {
            dh.iscript_state[e].alive = false;
        }
        );
    }
}
impl EntityProcess for IScriptSteppingSys {
    fn process(&mut self,
               entities: EntityIter<UnitComponents>,
               dh: &mut DataHelper<UnitComponents, UnitServices>) {
        if self.iscript_copy.is_none() {
            let is = &dh.services.iscript;
            let cpy = is.clone();
            self.iscript_copy = cpy;
        }

        // FIXME
        let cpy = self.iscript_copy.as_ref().expect("IScript not loaded!");
        for e in entities {
            // let action = dh.iscript_state[e]._interpret_iscript(cpy, None);
            // println!(" entity {:?}", (**e).id() );

            // println!("entity: {:?}", **e);

            // let index = e.index();
            // let id = e.id();
            // println!("index: {}, id: {}", index, id);

            // let indexed = dh.entities.indexed(e);
            // let entity = **e;
            // println!("{:?}", entity);
            // dh.with_entity_data(&entity, |ent, data| {
            //    println!("relx: {}", data.iscript_state[ent].rel_x);
            // });
            // let ie = dh.entities.indexed(entity);
            // println!("ie: {:?}", ie.iscript_state.rel_x);

            // let ent = Entity(id);
            // let reconstructed_entity = IndexedEntity(index, ent, PhantomData);

            self.interpret_iscript(&cpy, e, None, dh);
        }
    }
}

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
    // FIXME: only for units?
    // pub commands: Vec<UnitCommands>,
    // FIXME we probably only need 1 overlay & 1 underlay?
    underlays: Vec<Entity>,
    overlays: Vec<Entity>,
    can_turn: bool,
    remapping: SCImageRemapping,
}
impl SCImageComponent {
    pub fn new(gd: &GameData, image_id: u16) -> Self {
        // let iscript_id = gd.images_dat.iscript_id[image_id as usize];
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
            underlays: Vec::<Entity>::new(),
            overlays: Vec::<Entity>::new(),
            player_id: 0, // commands: Vec::<UnitCommands>::new(),
            can_turn: can_turn,
            remapping: remapping,
        }
    }

    fn reindexing_table<'a>(&self, gd: &'a GameData) -> &'a [u8] {
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

    fn draw(&self, inbuf: &[u8], w: u32, h: u32, flipped: bool,
            cx: i32, cy: i32, outbuf: &mut [u8], outbuf_pitch: u32, reindex: &[u8]) {
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

    // TODO: might make sense to join scimage & iscriptstate
    fn frame_idx(&self, iscript_state: &IScriptStateElement) -> usize {
        if !self.can_turn {
            iscript_state.frameset as usize
        } else if iscript_state.direction > 16 {
            (iscript_state.frameset + 32 - iscript_state.direction as u16) as usize
        } else {
            (iscript_state.frameset + iscript_state.direction as u16) as usize
        }
    }

    fn draw_flipped(&self, iscript_state: &IScriptStateElement) -> bool {
        self.can_turn && (iscript_state.direction > 16)
    }
}

pub struct SCSpriteComponent {
    pub sprite_id: u16,
}
pub struct SelectableComponent {
    /// from sprites.dat: length of health bar in pixels
    health_bar: u8,
    // circle_img: u8,
    circle_offset: u8,
    // FIXME: inefficient
    circle_grp_id: u32,

    pub sel_width: u16,
    pub sel_height: u16,
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

components! {
    struct UnitComponents {
        #[hot] iscript_state: IScriptStateElement,
        #[hot] scimage: SCImageComponent,
        #[hot] selectable: SelectableComponent,
        #[hot] scsprite: SCSpriteComponent,
        #[hot] scflingy: SCFlingyComponent,
        #[hot] scunit: SCUnitComponent,
    }
}
systems! {
    struct UnitSystems<UnitComponents, UnitServices> {
        active: {
            iscript_stepping_sys: EntitySystem<IScriptSteppingSys>
                = EntitySystem::new(IScriptSteppingSys{iscript_copy: None,},
                                    aspect!(<UnitComponents> all: [iscript_state])),
        },
        passive: {
        }
    }
}

/// *****************************************

struct UnitsECSView {
    world: World<UnitSystems>,
}

impl UnitsECSView {
    fn new(gd: &GameData, context: &mut GameContext) -> UnitsECSView {
        let pal = gd.install_pal.to_sdl();
        context.screen.set_palette(&pal).ok();

        let mut world = World::<UnitSystems>::new();
        world.data.services.load_iscript(gd);

        let image_id = 0;
        let iscript_id = gd.images_dat.iscript_id[image_id];

        world.create_entity(|entity: BuildData<UnitComponents>, data: &mut UnitComponents| {
            data.iscript_state.add(&entity,
                                   IScriptStateElement::new(&gd.iscript, iscript_id,
                                                            // map position
                                                            0, 0));
            data.scimage.add(&entity,
                             SCImageComponent::new(gd, image_id as u16));
        });

        UnitsECSView { world: world }
    }
}
impl View for UnitsECSView {
    fn render(&mut self,
              gd: &GameData,
              context: &mut GameContext,
              _: &GameState,
              _: f64)
              -> ViewAction {
        if context.events.now.quit ||
           context.events.now.is_key_pressed(&sdl2::keyboard::Keycode::Escape) {
            return ViewAction::Quit;
        }
        context.screen.fill_rect(None, Color::RGB(0, 0, 0)).ok();

        self.world.update();

        let grp_cache = gd.grp_cache.borrow();
        let buffer_pitch = context.screen.pitch();
        context.screen.with_lock_mut(|buffer: &mut [u8]| {
            let dh = &self.world.data;
            // NOTE order is random in this loop!
            for e in self.world.entities() {
                // TODO we should remove dead entities instead
                if !dh.iscript_state[e].alive {
                    continue;
                }

                // println!("rendering entity {:?}", **e);
                // every entity is an scimage
                let scimg_comp = &dh.scimage[e];
                let grp = grp_cache.get_ro(scimg_comp.grp_id);
                let fridx = scimg_comp.frame_idx(&dh.iscript_state[e]);
                let draw_flipped = scimg_comp.draw_flipped(&dh.iscript_state[e]);

                scimg_comp.draw(&grp.frames[fridx],
                                grp.header.width as u32,
                                grp.header.height as u32,
                                draw_flipped,
                                200, 200,
                                buffer,
                                buffer_pitch,
                                scimg_comp.reindexing_table(gd)
                );

            }
        });

        ViewAction::None
    }
}

fn main() {
    ::scrust::spawn("units ecs",
                    "/home/dm/.wine/drive_c/StarCraft/",
                    |gd, gc, _| Box::new(UnitsECSView::new(gd, gc)));
}
