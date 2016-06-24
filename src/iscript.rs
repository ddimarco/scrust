use std::io::{Read, Seek, SeekFrom};

use byteorder::{ReadBytesExt, LittleEndian};


use std::collections::HashMap;

pub struct IScript {
    pub id_offsets_map: HashMap<u32, Vec<u16>>,
    pub data: Vec<u8>,
}
impl IScript {
    pub fn read<T: Read + Seek>(file: &mut T) -> IScript {
        // entree type -> number of offsets
        const ISCRIPT_HEADER_TYPES: [usize; 32] = [2, 2, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 14, 14, 16,
                                                   16, 0, 0, 0, 0, 22, 22, 0, 24, 26, 28, 28, 28,
                                                   0, 0, 0, 0];

        // first 2 bytes: offset to entree table
        let entree_offset = file.read_u16::<LittleEndian>().unwrap();
        file.seek(SeekFrom::Start(entree_offset as u64)).ok();

        // read header pairs: maps entree with images.dat id
        let mut header_pairs = Vec::<(u16, u16)>::new();
        loop {
            let script_id = file.read_u16::<LittleEndian>().unwrap();
            // offset to entree
            let header_offset = file.read_u16::<LittleEndian>().unwrap();
            if header_offset == 0 {
                break;
            }
            header_pairs.push((script_id, header_offset));
        }

        let mut id_offsets_map = HashMap::<u32, Vec<u16>>::new();
        // read entrees
        for (script_id, header_offset) in header_pairs {
            file.seek(SeekFrom::Start(header_offset as u64)).ok();
            let marker = file.read_u32::<LittleEndian>().unwrap();
            assert_eq!(marker, 1162888019);

            // pointer section
            let tpe = file.read_u16::<LittleEndian>().unwrap();
            assert!(tpe < 30);
            let _ = file.read_u16::<LittleEndian>().unwrap();

            // how many labels are in this section
            let section_len = ISCRIPT_HEADER_TYPES[tpe as usize];
            let mut offsets = Vec::<u16>::with_capacity(section_len);
            for _ in 0..section_len {
                offsets.push(file.read_u16::<LittleEndian>().unwrap());
            }
            id_offsets_map.insert(script_id as u32, offsets);
        }

        let mut data = Vec::<u8>::new();
        file.seek(SeekFrom::Start(0)).ok();
        file.read_to_end(&mut data).ok();
        IScript {
            id_offsets_map: id_offsets_map,
            data: data,
        }
    }
}

enum_from_primitive! {
#[derive(Debug)]
pub enum AnimationType {
    Init = 0,
    Death,
    GndAttkInit,
    AirAttkInit,
    Unused1,
    GndAttkRpt,
    AirAttkRpt,
    CastSpell,
    GndAttkToIdle,
    AirAttkToIdle,
    Unused2,
    Walking,
    WalkingToIdle,
    SpecialState1,
    SpecialState2,
    AlmostBuilt,
    Built,
    Landing,
    LiftOff,
    IsWorking,
    WorkingToIdle,
    Warpin,
    Unused3,
    StarEditInit,
    Disable,
    Burrow,
    Unburrow,
    Enable,
}
}

enum_from_primitive! {
#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum OpCode {
  PlayFram = 0,
  PlayFramTile, // only possible for buildings, see tilesetdependentiscript
  SetHorPos,
  SetVertPos,
  SetPos,
  Wait,
  WaitRand,
  Goto,
  ImgOl,
  ImgUl,
  ImgOlOrig,
  SwitchUl,
  __0c,
  ImgOlUseLo,
  ImgUlUseLo,
  SprOl,
  HighSprOl,
  LowSprUl,
  UflUnstable,
  SprUlUseLo,
  SprUl,
  SprOlUseLo,
  End,
  SetFlipState,
  PlaySnd,
  PlaySndRand,
  PlaySndBtwn,
  DoMissileDmg,
  AttackMelee,
  FollowMainGraphic,
  RandCondJmp,
  TurnCCWise,
  TurnCWise,
  Turn1CWise,
  TurnRand,
  SetSpawnFrame,
  SigOrder,
  AttackWith,
  Attack,
  CastSpell,
  UseWeapon,
  Move,
  GotoRepeatAttk,
  EngFrame,
  EngSet,
  __2d,
  NoBrkCodeStart,
  NoBrkCodeEnd,
  IgnoreRest,
  AttkShiftProj,
  TmpRmGraphicStart,
  TmpRmGraphicEnd,
  SetFlDirect,
  Call,
  Return,
  SetFlSpeed,
  CreateGasOverlays,
  PwrupCondJmp,
  TrgtRangeCondJmp,
  TrgtArcCondJmp,
  CurDirectCondJmp,
  ImgUlNextId,
  __3e,
  LiftOffCondJmp,
  WarpOverlay,
  OrderDone,
  GrdSprOl,
  __43,
  DoGrdDamage,
}
}
