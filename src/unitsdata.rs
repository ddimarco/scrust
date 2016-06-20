use std::io::Read;

extern crate byteorder;
use byteorder::{LittleEndian, ReadBytesExt};

// TODO: macroify?

fn read_vec_u32(file: &mut Read, count: usize) -> Vec<u32> {
    let mut res = Vec::<u32>::with_capacity(count);
    for _ in 0..count {
        let val = file.read_u32::<LittleEndian>().unwrap();
        res.push(val);
    }
    res
}
fn read_vec_u16(file: &mut Read, count: usize) -> Vec<u16> {
    let mut res = Vec::<u16>::with_capacity(count);
    for _ in 0..count {
        let val = file.read_u16::<LittleEndian>().unwrap();
        res.push(val);
    }
    res
}
fn read_vec_u8(file: &mut Read, count: usize) -> Vec<u8> {
    let mut res = Vec::<u8>::with_capacity(count);
    for _ in 0..count {
        let val = file.read_u8().unwrap();
        res.push(val);
    }
    res
}

macro_rules! dat_reader {
    (u32, $file:ident, $count:expr) => (read_vec_u32($file, $count));
    (u16, $file:ident, $count:expr) => (read_vec_u16($file, $count));
    (u8, $file:ident, $count:expr) => (read_vec_u8($file, $count));
}
macro_rules! dat_struct {
    (
        $struct_name:ident {
            $( $name:ident: $tpe:ident; $count:expr),*
        }
    ) => {
        pub struct $struct_name {
            $( pub $name: Vec<$tpe>, )*
        }
        impl $struct_name {
            pub fn read(file: &mut Read) -> $struct_name {
                $( let $name = dat_reader!($tpe, file, $count);)*

                $struct_name {
                    $( $name: $name, )*
                }
            }
        }

    }
}

dat_struct! (
    ImagesDat
    {
        grp_file:              u32;  999,
        graphic_turns:         u8;   999,
        clickable:             u8;   999,
        use_full_iscript:      u8;   999,
        draw_if_cloaked:       u8;   999,
        draw_function:         u8;   999,
        remapping:             u8;   999,
        iscript_id:            u32;  999,
        shield_overlay:        u32;  999,
        attacku_overlay:       u32;  999,
        damage_overlay:        u32;  999,
        special_overlay:       u32;  999,
        landing_dust_overlay:  u32;  999,
        lift_off_overlay:      u32;  999
    }
);

dat_struct! (
    SpritesDat
    {
        image_file               :u16  ;517,
        health_bar               :u8   ;387,
        unknown                  :u8   ;517,
        visible                  :u8   ;517,
        selection_circle_image   :u8   ;387,
        selection_circle_offset  :u8   ;387
    }
);

dat_struct! (
    FlingyDat
    {
        sprite         :u16  ;209,
        top_speed      :u32  ;209,
        acceleration   :u16  ;209,
        halt_distance  :u32  ;209,
        turn_radius    :u8   ;209,
        unused         :u8   ;209,
        move_control   :u8   ;209
    }
);

dat_struct! (
    UnitsDat
    {
        graphics                        :u8   ;228,
        subunit1                        :u16  ;228,
        subunit2                        :u16  ;228,
        infestation                     :u16  ;96,
        construction_animation          :u32  ;228,
        unit_direction                  :u8   ;228,
        shield_enable                   :u8   ;228,
        shield_amount                   :u16  ;228,
        hit_points                      :u32  ;228,
        elevation_level                 :u8   ;228,
        unknown                         :u8   ;228,
        sub_label                       :u8   ;228,
        comp_AI_idle                    :u8   ;228,
        human_AI_idle                   :u8   ;228,
        return_to_idle                  :u8   ;228,
        attack_unit                     :u8   ;228,
        attack_move                     :u8   ;228,
        ground_weapon                   :u8   ;228,
        max_ground_hits                 :u8   ;228,
        air_weapon                      :u8   ;228,
        max_air_hits                    :u8   ;228,
        ai_internal                     :u8   ;228,
        special_ability_flags           :u32  ;228,
        target_acquisition_range        :u8   ;228,
        sight_range                     :u8   ;228,
        armor_upgrade                   :u8   ;228,
        unit_size                       :u8   ;228,
        armor                           :u8   ;228,
        right_click_action              :u8   ;228,
        ready_sound                     :u16  ;106,
        what_sound_start                :u16  ;228,
        what_sound_end                  :u16  ;228,
        piss_sound_start                :u16  ;106,
        piss_sound_end                  :u16  ;106,
        yes_sound_start                 :u16  ;106,
        yes_sound_end                   :u16  ;106,
        star_edit_placement_box_width   :u16  ;228,
        star_edit_elacement_box_height  :u16  ;228,
        addon_horizontal                :u16  ;96,
        addon_vertical                  :u16  ;96,
        unit_size_left                  :u16  ;228,
        unit_size_up                    :u16  ;228,
        unit_size_right                 :u16  ;228,
        unit_size_down                  :u16  ;228,
        portrait                        :u16  ;228,
        mineral_cost                    :u16  ;228,
        vespene_cost                    :u16  ;228,
        build_time                      :u16  ;228,
        unknown1                        :u16  ;228,
        star_edit_group_flags           :u8   ;228,
        supply_provided                 :u8   ;228,
        supply_required                 :u8   ;228,
        space_required                  :u8   ;228,
        space_provided                  :u8   ;228,
        build_score                     :u16  ;228,
        destroy_score                   :u16  ;228,
        unit_map_string                 :u16  ;228,
        broodwar_unit_flag              :u8   ;228,
        star_edit_availability_flags    :u16  ;228
    }
);
