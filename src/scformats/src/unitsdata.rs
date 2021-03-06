use std::io::Read;

// TODO: macroify?
use ::utils::{read_vec_u32, read_vec_u16, read_vec_u8};



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
        #[derive(Clone)]
        pub struct $struct_name {
            $(
                pub $name: Vec<$tpe>,
            )*
        }
        impl $struct_name {
            pub fn read(file: &mut Read) -> $struct_name {
                $(
                    let $name = dat_reader!($tpe, file, $count);
                )*

                $struct_name {
                    $( $name: $name, )*
                }
            }

            pub fn print_entry(&self, i: usize) {
                println!("entry {} of {}",
                         i, stringify!($struct_name));
                $(
                    println!(" {}: {}", stringify!($name), self.$name[i]);
                )*
            }
        }

    }
}

dat_struct! (
    ImagesDat
    {
        grp_id:                u32;  999,
        graphic_turns:         u8;   999,
        clickable:             u8;   999,
// Allows running for Iscript animations other than the Initial and
// Death animations. Unchecked, prevents the sprite movement, attack,
// spellcasting etc. If the Movement Control for the corresponding
// flingy.dat entry is set to "Flingy.dat Control", the sprite
// movement WILL take place, but without any animation.
        use_full_iscript:      u8;   999,
        draw_if_cloaked:       u8;   999,
        draw_function:         u8;   999,
        remapping:             u8;   999,
        iscript_id:            u32;  999,
// Overlay used to place the Images.dat entry #424 ("Shield Overlay"),
// if the unit has shields and is hit. [pointer to images.tbl]
        shield_overlay:        u32;  999,
// This one usually controls a part of the attack animation. Except
// for the Bunker (which is hardcoded), the use of this property is
// not specific, but removing it will hang the game if unit's Iscript
// animation calls for the overlay with the opcodes: 0xD(imgoluselo),
// 0xE(imguluselo) or 0x15(sproluselo).[pointer to a LOG\LOL\LOX\LOA
// file in images.tbl]
        attack_overlay:        u32;  999,
// The "Flames/Bleeding" overlay control, dependent on the current HP
// value. If the number of frames of the used GRP file is higher than
// the number of frames of the overlay, the game will crash.[pointer
// to a LOF file in images.tbl]
        damage_overlay:        u32;  999,
// This one is used for various purposes: for "Resource Miners", it
// controls where they "hold" the resources;for the gas-containers, it
// controls the placement of the smoke graphics (iscript connection
// via the "creategasoverlays" (0x38) opcode); for the base-turret
// units, it controls the placement of the turret (also "imgoluselo"
// connection); for the Battlecruiser, it is the location of the
// Yamato Gun graphics.[pointer to a LOS\LOL\LOO\LOA\LOB file in
// images.tbl]
        special_overlay:       u32;  999,
// Complementary to "Lift-off Dust", this one controls the placement
// of the landing dust. Some units (Dropship,Science Vessel)
// originally had this one too, but the idea was abandoned.Also used
// for the "2 in 1 Egg" zerg units, to determine the location where to
// put the 2 spawned units.[pointer to a LOB\LOU file in images.tbl]
        landing_dust_overlay:  u32;  999,
// Complementary to "Landing Dust", this one controls the placement of
// the lifting-off dust. Some units (Dropship, Science Vessel)
// originally had this too, but the idea was abandoned. [pointer to a
// LOD file in images.tbl]
        lift_off_overlay:      u32;  999
    }
);

dat_struct! (
    SpritesDat
    {
        image_id                 :u16  ;517,
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
        sprite_id      :u16  ;209,
        // Maximum speed at which the sprite will move. Measured in pixels-per-frame,
        // but written as "Speed*(320/3)" (rounded up, it's weird, but that's how it
        // works). Works ONLY if "Move Control" is set to "Flingy.dat Control".

        // This is measured in pixels/tick * (320/3). A tick, if you recall, is
        // 1/10th of a second. It's generally more helpful to think of the speed
        // of a unit you want to emulate and look at its Top Speed rather than
        // try and calculate the pixels/frame. Larger numbers are obviously
        // faster.
        top_speed      :u32  ;209,
        // How fast the sprite speeds up or slows down. Added to or subtracted
        // from current speed until it reaches the Top Speed or 0. Measured in
        // pixels-per-frame. Works ONLY if "Move Control" is set to "Flingy.dat
        // Control".
        acceleration   :u16  ;209,
        // Distance from its destination at which the sprite will begin to
        // deccelerate from its Top Speed to a complete halt. Measured in
        // pixels*256.
        halt_distance  :u32  ;209,
        // The distance the sprite requires to "wipe around" to turn to another
        // direction. Works ONLY if "Move Control" is set to "Flingy.dat
        // Control".
        // Smaller numbers cause a unit to "skid" and make more sweeping turns.
        turn_radius    :u8   ;209,
        unused         :u8   ;209,
        // Indicates the mechanism that is used to control the movement of the
        // flingy.dat entry. "Flingy.dat Control" makes use of the Acceleration,
        // Speed, Turn Style and Turn Radius properties, i.e. the values in this
        // editor will be used. "Iscript.bin Control" ignores these properties
        // and follows only the Iscript opcode sequence. "Partially
        // Mobile/Weapon" is used for various weapons sprites, not completely
        // understood.
        // 0: flingy.dat
        // 1: partially mobile, weapon
        // 2: iscript.bin
        move_control   :u8   ;209
    }
);

enum_from_primitive! {
    pub enum WeaponsDamageType {
        Independent = 0,
        Explosive,
        Concussive,
        Normal,
        IgnoreArmor,
    }
}
enum_from_primitive! {
    pub enum WeaponsExplosionType {
        None,
        Normal,
        RadialSplash,
        EnemySplash,
        Lockdown,
        NuclearMissile,
        Parasite,
        Broodlings,
        EMPShockwave,
        Irradiate,
        Ensnare,
        Plague,
        StasisField,
        DarkSwarm,
        Consume,
        YamatoGun,
        Restoration,
        DisruptionWeb,
        CorrosiveAcid,
        MindControl,
        Feedback,
        OpticalFlare,
        Maelstrom,
        Unknown1,
        SplashAir,
    }
}
enum_from_primitive! {
    pub enum WeaponBehavior {
        FlyToTarget,
        FlyToTarget2,
        AppearOnTargetUnit,
        PsionicStorm,
        AppearOnTargetSite,
        AppearOnAttacker,
        AttackAndSelfDestruct,
        Bounce,
        AttackTarget3x3Area,
        GoToMaxRange,
    }
}

dat_struct! (
    UnitsDat
    {
        // called "graphics" earlier
        flingy_id                       :u8   ;228,
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
        comp_ai_idle                    :u8   ;228,
        human_ai_idle                   :u8   ;228,
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

dat_struct! (
    WeaponsDat
    {
// The name of the weapon, displayed when you highlight its
// icon in the control bar. [pointer to stat_txt.tbl]
        label: u16; 130,
 // The main graphics that the weapon uses. 0-Scourge = No
 // graphics.[pointer to flingy.dat]
        graphics: u32; 130,
        unused1: u8; 130,
        target_flags: u16; 130,
        minimum_range: u32; 130,
        maximum_range: u32; 130,
 // The upgrade that will increase the damage dealt by
 // the weapon by the "Bonus" value.
        damage_upgrade: u8; 130,
 // The type of damage the weapon does. Normal, Explosive
 // and Concussive do different amount of damage to units
 // of different Size (Small, Medium or Large): Normal does
 // equal damage to Small, Medium and Large. Explosive does
 // 50% to Small and 75% to Medium. Concussive does 50% to
 // Medium and 25% to Large. Independent deals 1 point of
 // damage every second attack, regardless of target's
 // armor.
        damage_type: u8; 130,
 // Determines how the weapon sprite will "behave" when
 // it attacks the target. Weapon behaviours that
 // "Follow" will track the target as it moves, those
 // that "Don't Follow" will strike the place where the
 // target was at the moment of attack.
        behavior: u8; 130,
 // Time until the weapon is removed if it does not hit a
 // target. 1 game second equals: on Fastest-24, on
 // Faster-21, on Fast-18, on Normal-15, on Slow-12, on
 // Slower-9 and on Slowest-6.
        remove_after: u8; 130,
 // Effect the weapon has on the area around the target
 // after hitting its target. Used to determine
 // different type of spell effects and splash damage.
        explosion_type: u8; 130,
 // Distance from the target at which the weapon
 // will deal 100% of its base damage. Works ONLY
 // if the "Explosion" is set to "Nuclear Missile",
 // "Splash (Radial)", "Splash (Enemy)" or "Splash
 // (Air)".
        inner_splash_range: u16; 130,
        medium_splash_range: u16; 130,
        outer_splash_range: u16; 130,
        damage_amount: u16; 130,
        damage_bonus: u16; 130,
 // "Reload time" - time delay between two attacks.
 // Depends on the game speed used. 1 game second
 // equals: on Fastest-24, on Faster-21, on Fast-18, on
 // Normal-15, on Slow-12, on Slower-9 and on
 // Slowest-6. Value of 0 will crash the game.
        cooldown: u8; 130,
 // Usually, multiple this value by the Damage Amount to
 // get the total damage that is DISPLAYED for the
 // weapon. To a degree also the number of weapons used
 // per attack, but anything other than 2 will result in
 // 1 weapon being used. (e.g. Goliath, Scout and
 // Valkyrie use 2 missiles per attack).
        damage_factor: u8; 130,
 // Angle within which the weapon can be fired without
 // waiting for the unit's graphics to turn. 128 = 180
 // degrees.
        attack_angle: u8; 130,
 // Angle by which the weapon's sprite will spin after it
 // is spawned. 128 = 180 degrees.
        launch_spin: u8; 130,
 // Distance (in pixels) from the front of the attacking
 // unit (depending on which direction it is facing), at
 // which the weapon's sprite will be spawned.
        forward_offset: u8; 130,
 // Distance (in pixels) from the top of the attacking
 // unit, at which the weapon's sprite will be spawned.
        upward_offset: u8; 130,
 // The line displayed when the weapon is to
 // acquire an invalid target (e.g. attacking a
 // Mutalisk with a ground-only weapon, like
 // Flamethrower) [pointer to stat_txt.tbl]
        target_error_message: u16; 130,
 // The icon used for the weapon. [pointer to a frame in
 // unit\cmdbtns\cmdicons.grp]
        icon: u16; 130
    }
);

dat_struct! (
    OrdersDat
    {
// Probably the label of the order. Doesn't do anything - used only
// for convenience.[pointer to stat_txt.tbl?]
        label: u16; 189,
// Order will use the weapons.dat targeting settings.
        use_weapon_targeting: u8; 189,
        unknown1: u32; 189,
// Order's execution can be interrupted by calling another order (e.g.
// movement or something). Does not work for the "Die" order.
        can_be_interrupted: u8; 189,
        unknown2: u8; 189,
// Order execution can be queued using the SHIFT+click combination, up to 16 times.
        can_be_queued: u8; 189,
        unknown3: u32; 189,
// Weapon used by the order to determine the targeting rules, if "Use
// Weapon Targeting" is checked. [pointer to weapons.dat]
        targeting: u8; 189,
// Technology used to determine the energy cost of calling the order. [pointer to techdata.dat]
        energy: u8; 189,
// Unit Iscript animation to use while calling the order.
        animation: u8; 189,
// Determines which icon on the button panel is highlightened when the
// order is being called. If the selected icon is not present, no icon
// is highlightened. 65535 = no highlight at all [pointer to
// cmdicons.grp]
        highlight: u16; 189,
        unknown4: u16; 189,
// Order to be run if the target is obscured by the Fog-of-War. [pointer to orders.dat]
        obscured: u8; 189
    }
);
