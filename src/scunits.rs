use std::f32;

use std::rc::Rc;

use ::gamedata::{GameData, GRPCache};
use ::iscript::{AnimationType};
use ::iscriptstate::{IScriptState, IScriptCurrentAction, IScriptEntityAction};

use ::render::{render_buffer_solid, render_buffer_with_transparency_reindexing, render_buffer_with_solid_reindexing};

// TODO: move to utils
pub fn angle2discrete(angle: f32) -> u8 {
    let pi = f32::consts::PI;
    // x+24 % 32 ~ x + 90°
    (((angle+pi)*32. / (2.*pi)).round() + 24.) as u8 % 32
}

pub fn discrete2angle(discrete_angle: u8) -> f32 {
    let pi = f32::consts::PI;
    (((discrete_angle + 8) % 32) as f32 / 32.) * (2.*pi) - pi
}

#[derive(Debug, Clone, Copy)]
pub enum SCImageRemapping {
    Normal,
    OFire,
    GFire,
    BFire,
    BExpl,
    Shadow,
}

// TODO create UnitType structs (to keep all the per-unit information)

pub struct SCImage {
    pub image_id: u16,
    pub grp_id: u32,
    pub player_id: usize,
    iscript_state: IScriptState,
    underlays: Vec<SCImage>,
    overlays: Vec<SCImage>,
    // FIXME: only for units?
    pub commands: Vec<UnitCommands>,
}
pub trait IScriptableTrait {
    fn get_iscript_state<'a>(&'a self) -> &'a IScriptState;
    fn get_iscript_state_mut<'a>(&'a mut self) -> &'a mut IScriptState;
}
impl IScriptableTrait for SCImage {
    fn get_iscript_state<'a>(&'a self) -> &'a IScriptState {
        &self.iscript_state
    }
    fn get_iscript_state_mut<'a>(&'a mut self) -> &'a mut IScriptState {
        &mut self.iscript_state
    }
}
pub trait SCImageTrait: IScriptableTrait {
    fn get_scimg<'a>(&'a self) -> &'a SCImage;
    fn get_scimg_mut<'a>(&'a mut self) -> &'a mut SCImage;
}
impl SCImageTrait for SCImage {
    fn get_scimg<'a>(&'a self) -> &'a SCImage {
        self
    }
    fn get_scimg_mut<'a>(&'a mut self) -> &'a mut SCImage {
        self
    }
}

impl SCImage {
    pub fn new(gd: &Rc<GameData>, image_id: u16, map_x: u16, map_y: u16) -> SCImage {
        let iscript_id = gd.images_dat.iscript_id[image_id as usize];
        let grp_id = gd.images_dat.grp_id[image_id as usize];
        {
            gd.grp_cache.borrow_mut().load(gd, grp_id);
        }

        SCImage {
            image_id: image_id,
            grp_id: grp_id,
            iscript_state: IScriptState::new(&gd, iscript_id, image_id, map_x, map_y),
            // FIXME we probably only need 1 overlay & 1 underlay
            underlays: Vec::<SCImage>::new(),
            overlays: Vec::<SCImage>::new(),
            player_id: 0,
            commands: Vec::<UnitCommands>::new(),
        }
    }

    fn can_turn(&self) -> bool {
        (self.iscript_state.gd.images_dat.graphic_turns[self.image_id as usize] > 0)
    }
    fn draw_flipped(&self) -> bool {
        self.can_turn() && self.iscript_state.direction > 16
    }

    fn frame_idx(&self) -> usize {
        if !self.can_turn() {
            self.iscript_state.frameset as usize
        } else if self.iscript_state.direction > 16 {
            (self.iscript_state.frameset + 32 - self.iscript_state.direction as u16) as usize
        } else {
            (self.iscript_state.frameset + self.iscript_state.direction as u16) as usize
        }
    }

    pub fn remapping(&self, gd: &GameData) -> SCImageRemapping {
        let idat_draw_func = gd.images_dat.draw_function[self.image_id as usize];
        match idat_draw_func {
            10 => SCImageRemapping::Shadow,
            9 => {
                match gd.images_dat.remapping[self.image_id as usize] {
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
        }
    }

    fn _draw(&self,
             grp_cache: &GRPCache,
             cx: i32,
             cy: i32,
             buffer: &mut [u8],
             buffer_pitch: u32,
             has_parent: bool) {
        if !self.iscript_state.visible {
            return;
        }

        let fridx = self.frame_idx();
        let grp = grp_cache.get_ro(self.grp_id);
        // this seems like a hack
        if fridx >= grp.frames.len() && has_parent {
            println!("WARNING: suspicious frame index");
            return;
        }
        let udata = &grp.frames[fridx];

        let w = grp.header.width as u32;
        let h = grp.header.height as u32;
        let x_center = cx + self.iscript_state.rel_x as i32;
        let y_center = cy + self.iscript_state.rel_y as i32;

        let remap = self.remapping(self.iscript_state.gd.as_ref());
        let reindex = match remap {
            SCImageRemapping::OFire => &self.iscript_state.gd.ofire_reindexing.data,
            SCImageRemapping::BFire => &self.iscript_state.gd.bfire_reindexing.data,
            SCImageRemapping::GFire => &self.iscript_state.gd.gfire_reindexing.data,
            SCImageRemapping::BExpl => &self.iscript_state.gd.bexpl_reindexing.data,
            SCImageRemapping::Shadow => &self.iscript_state.gd.shadow_reindexing,
            SCImageRemapping::Normal => {
                if self.player_id < 11 {
                    let startpt = self.player_id as usize * 256;
                    &self.iscript_state.gd.player_reindexing[startpt..startpt + 256]
                } else {
                    // neutral player (i.e. minerals, critters, etc)
                    &self.iscript_state.gd.player_reindexing[0..256]
                }
            }
        };
        match remap {
            SCImageRemapping::OFire | SCImageRemapping::BFire | SCImageRemapping::GFire |
            SCImageRemapping::BExpl | SCImageRemapping::Shadow => {
                render_buffer_with_transparency_reindexing(udata,
                                                           w,
                                                           h,
                                                           self.draw_flipped(),
                                                           x_center,
                                                           y_center,
                                                           buffer,
                                                           buffer_pitch,
                                                           &reindex);
            }
            SCImageRemapping::Normal => {
                render_buffer_with_solid_reindexing(udata,
                                                    w,
                                                    h,
                                                    self.draw_flipped(),
                                                    x_center,
                                                    y_center,
                                                    buffer,
                                                    buffer_pitch,
                                                    &reindex);
            }
        }


    }

    /// cx, cy: screen coordinates
    pub fn draw(&self,
                grp_cache: &GRPCache,
                cx: i32,
                cy: i32,
                buffer: &mut [u8],
                buffer_pitch: u32) {
        // draw underlays
        for ul in &self.underlays {
            ul._draw(grp_cache, cx, cy, buffer, buffer_pitch, true);
        }
        // draw main image
        self._draw(grp_cache, cx, cy, buffer, buffer_pitch, false);
        // draw overlays
        for ol in &self.overlays {
            ol._draw(grp_cache, cx, cy, buffer, buffer_pitch, true);
        }
    }

    pub fn step(&mut self,
                // just for creating new entities
                gd: &Rc<GameData>)
                -> Option<IScriptEntityAction> {


        // new command handling
        match self.commands.pop() {
            None => {},
            Some(UnitCommands::Move(mx, my)) => {
                self.get_iscript_state_mut().set_animation(AnimationType::Walking);
                // TODO: move current_action into scimage?
                self.get_iscript_state_mut().current_action = IScriptCurrentAction::Moving(mx, my);
            },
            _ => {
                println!("unknown command!");
            }
        }
        // check status of current action
        {
            let iss = self.get_iscript_state_mut();
            match iss.current_action {
                IScriptCurrentAction::Moving(mx, my) => {
                    // FIXME: doesn't work for flying units & scvs
                    // TODO: incorporate movement speed, turn duration, etc
                    let goal_dist = ((mx - iss.map_pos_x as i32) as f32).hypot(
                        (my - iss.map_pos_y as i32) as f32);
                    if goal_dist < 4. {
                        iss.current_action = IScriptCurrentAction::Idle;
                        iss.set_animation(AnimationType::WalkingToIdle);
                    } else {
                        // TODO: proper path planning
                        let (xdiff, ydiff) = ((mx - iss.map_pos_x as i32) as f32,
                                              (my - iss.map_pos_y as i32) as f32);
                        // res: -180 to +180 (in rad)
                        let angle = ydiff.atan2(xdiff);
                        let disc_angle = angle2discrete(angle);
                         // println!("angle: {}, discrete: {}", angle, disc_angle);
                        iss.direction = disc_angle;
                        iss.movement_angle = angle;
                    }
                },
                _ => {},
            }
        }


        // FIXME: death animation for marine: shadow tries to display wrong frameset
        for ul in &mut self.underlays {
            let action = ul.iscript_state._interpret_iscript(Some(&self.iscript_state));
            // assuming they do not create additional under/overlays
            assert!(action.is_none());
        }
        self.underlays.retain(|ref ul| ul.iscript_state.alive);

        let iscript_action = self.iscript_state._interpret_iscript(None);
        for ol in &mut self.overlays {
            let action = ol.iscript_state._interpret_iscript(Some(&self.iscript_state));
            // assuming they do not create additional under/overlays
            assert!(action.is_none());
        }
        self.overlays.retain(|ref ol| ol.iscript_state.alive);

        // create additional entities if necessary
        match iscript_action {
            Some(IScriptEntityAction::CreateImageUnderlay { image_id, rel_x, rel_y }) => {
                let mut underlay = SCImage::new(gd, image_id, 0, 0);
                underlay.iscript_state.rel_x = rel_x;
                underlay.iscript_state.rel_y = rel_y;
                self.underlays.push(underlay);
                None
            }
            Some(IScriptEntityAction::CreateImageOverlay { image_id, rel_x, rel_y }) => {
                let mut overlay = SCImage::new(gd, image_id, 0, 0);
                overlay.iscript_state.rel_x = rel_x;
                overlay.iscript_state.rel_y = rel_y;
                self.overlays.push(overlay);
                None
            }
            _ => iscript_action,
        }
    }
}

/// /////////////////////////////////////

// sprite: additional features:
// - health bar
// - selection circle

pub struct SCSpriteSelectable {
    /// from sprites.dat: length of health bar in pixels
    health_bar: u8,
    // circle_img: u8,
    circle_offset: u8,
    // FIXME: inefficient
    circle_grp_id: u32,

    pub sel_width: u16,
    pub sel_height: u16,
}

pub struct SCSprite {
    pub sprite_id: u16,
    pub img: SCImage,
    pub selectable_data: Option<SCSpriteSelectable>,
}

impl IScriptableTrait for SCSprite {
    fn get_iscript_state<'a>(&'a self) -> &'a IScriptState {
        self.img.get_iscript_state()
    }
    fn get_iscript_state_mut<'a>(&'a mut self) -> &'a mut IScriptState {
        self.img.get_iscript_state_mut()
    }
}
impl SCImageTrait for SCSprite {
    fn get_scimg<'a>(&'a self) -> &'a SCImage {
        &self.img
    }
    fn get_scimg_mut<'a>(&'a mut self) -> &'a mut SCImage {
        &mut self.img
    }
}
pub trait SCSpriteTrait: SCImageTrait {
    fn get_scsprite<'a>(&'a self) -> &'a SCSprite;
    fn get_scsprite_mut<'a>(&'a mut self) -> &'a mut SCSprite;
}
impl SCSpriteTrait for SCSprite {
    fn get_scsprite<'a>(&'a self) -> &'a SCSprite {
        self
    }
    fn get_scsprite_mut<'a>(&'a mut self) -> &'a mut SCSprite {
        self
    }
}
impl SCSprite {
    pub fn new(gd: &Rc<GameData>, sprite_id: u16, map_x: u16, map_y: u16) -> SCSprite {
        let image_id = gd.sprites_dat.image_id[sprite_id as usize];
        let img = SCImage::new(gd, image_id, map_x, map_y);

        let selectable_data = if sprite_id >= 130 {
            let circle_img = gd.sprites_dat.selection_circle_image[(sprite_id - 130) as usize];
            let circle_grp_id = gd.images_dat.grp_id[561 + circle_img as usize];

            let sel_width;
            let sel_height;
            {
                let mut grpcache = gd.grp_cache.borrow_mut();
                grpcache.load(gd, circle_grp_id);
                sel_width = grpcache.get_ro(circle_grp_id).header.width.clone();
                sel_height = grpcache.get_ro(circle_grp_id).header.height.clone();
            }

            Some(SCSpriteSelectable {
                health_bar: gd.sprites_dat.health_bar[(sprite_id - 130) as usize],
                circle_offset: gd.sprites_dat.selection_circle_offset[(sprite_id - 130) as usize],
                circle_grp_id: circle_grp_id,
                sel_width: sel_width,
                sel_height: sel_height,
            })
        } else {
            None
        };
        SCSprite {
            sprite_id: sprite_id,
            img: img,
            selectable_data: selectable_data,
        }
    }

    // FIXME: clipping
    pub fn draw_healthbar(&self, cx: u32, cy: u32, buffer: &mut [u8], buffer_pitch: u32) {
        match self.selectable_data {
            None => {
                panic!();
            }
            Some(ref selectable) => {
                let boxes = selectable.health_bar as u32 / 3;
                let box_width = 3;
                if selectable.health_bar == 0 {
                    println!("healthbar == 0, not drawing");
                    return;
                }
                let width = 2 + (box_width * boxes) + (boxes - 1);
                let height = 8;

                let mut outpos = ((cy + selectable.circle_offset as u32) - height / 2) *
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
    }


    pub fn draw_selection_circle(&self,
                                 grp_cache: &GRPCache,
                                 cx: i32,
                                 cy: i32,
                                 buffer: &mut [u8],
                                 buffer_pitch: u32) {
        match self.selectable_data {
            Some(ref selectable) => {
                let grp = grp_cache.get_ro(selectable.circle_grp_id);
                render_buffer_solid(&grp.frames[0],
                                                grp.header.width as u32,
                                                grp.header.height as u32,
                                                false,
                                                cx,
                                                cy + selectable.circle_offset as i32,
                                                buffer,
                                                buffer_pitch);
            }
            None => {
                panic!();
            }
        }
    }
}

pub enum UnitCommands {
    Move(i32, i32),
    Attack(u32),
}

#[derive(Debug)]
pub enum FlingyMoveControl {
    FlingyDat,
    PartiallyMobile,
    IScriptBin,
}
pub struct SCFlingy {
    pub flingy_id: u16,
    pub move_control: FlingyMoveControl,
    sprite: SCSprite,
}
impl SCFlingy {
    pub fn new(gd: &Rc<GameData>, flingy_id: u16, map_x: u16, map_y: u16) -> Self {
        let sprite_id = gd.flingy_dat.sprite_id[flingy_id as usize];
        let sprite = SCSprite::new(gd, sprite_id, map_x, map_y);
        let move_control =
            match gd.flingy_dat.move_control[flingy_id as usize] {
                0 => FlingyMoveControl::FlingyDat,
                1 => FlingyMoveControl::PartiallyMobile,
                2 => FlingyMoveControl::IScriptBin,
                _ => unimplemented!(),
            };
        SCFlingy {
            flingy_id: flingy_id,
            move_control: move_control,
            sprite: sprite,
        }
    }

}

impl IScriptableTrait for SCFlingy {
    fn get_iscript_state<'a>(&'a self) -> &'a IScriptState {
        self.sprite.get_iscript_state()
    }
    fn get_iscript_state_mut<'a>(&'a mut self) -> &'a mut IScriptState {
        self.sprite.get_iscript_state_mut()
    }
}
impl SCImageTrait for SCFlingy {
    fn get_scimg<'a>(&'a self) -> &'a SCImage {
        self.sprite.get_scimg()
    }
    fn get_scimg_mut<'a>(&'a mut self) -> &'a mut SCImage {
        self.sprite.get_scimg_mut()
    }
}
impl SCSpriteTrait for SCFlingy {
    fn get_scsprite<'a>(&'a self) -> &'a SCSprite {
        &self.sprite
    }
    fn get_scsprite_mut<'a>(&'a mut self) -> &'a mut SCSprite {
        &mut self.sprite
    }
}

pub trait SCFlingyTrait: SCSpriteTrait {
    fn get_scflingy<'a>(&'a self) -> &'a SCFlingy;
    fn get_scflingy_mut<'a>(&'a mut self) -> &'a mut SCFlingy;
}
impl SCFlingyTrait for SCFlingy {
    fn get_scflingy<'a>(&'a self) -> &'a SCFlingy {
        self
    }
    fn get_scflingy_mut<'a>(&'a mut self) -> &'a mut SCFlingy {
        self
    }
}
pub struct SCUnit {
    pub unit_id: usize,
    // merging flingy and unit for now
    // pub flingy_id: usize,
    // sprite: SCSprite,
    flingy: SCFlingy,
    pub kill_count: usize,
}
impl IScriptableTrait for SCUnit {
    fn get_iscript_state<'a>(&'a self) -> &'a IScriptState {
        self.flingy.get_iscript_state()
    }
    fn get_iscript_state_mut<'a>(&'a mut self) -> &'a mut IScriptState {
        self.flingy.get_iscript_state_mut()
    }
}
impl SCImageTrait for SCUnit {
    fn get_scimg<'a>(&'a self) -> &'a SCImage {
        &self.flingy.get_scimg()
    }
    fn get_scimg_mut<'a>(&'a mut self) -> &'a mut SCImage {
        self.flingy.get_scimg_mut()
    }
}
impl SCSpriteTrait for SCUnit {
    fn get_scsprite<'a>(&'a self) -> &'a SCSprite {
        self.flingy.get_scsprite()
    }
    fn get_scsprite_mut<'a>(&'a mut self) -> &'a mut SCSprite {
        self.flingy.get_scsprite_mut()
    }
}
impl SCFlingyTrait for SCUnit {
    fn get_scflingy<'a>(&'a self) -> &'a SCFlingy {
        &self.flingy
    }
    fn get_scflingy_mut<'a>(&'a mut self) -> &'a mut SCFlingy {
        &mut self.flingy
    }
}

impl SCUnit {
    pub fn new(gd: &Rc<GameData>,
               unit_id: usize,
               map_x: u16,
               map_y: u16,
               player_id: usize)
               -> SCUnit {
        let flingy_id = gd.units_dat.flingy_id[unit_id];
        let mut flingy = SCFlingy::new(gd, flingy_id as u16, map_x, map_y);
        // let sprite_id = gd.flingy_dat.sprite_id[flingy_id as usize];
        // let mut sprite = SCSprite::new(gd, sprite_id, map_x, map_y);
        // sprite.get_scimg_mut().player_id = player_id;
        flingy.get_scimg_mut().player_id = player_id;
        SCUnit {
            unit_id: unit_id as usize,
            flingy: flingy,
            // flingy_id: flingy_id as usize,
            // sprite: sprite,
            kill_count: 0,
        }
    }
}
