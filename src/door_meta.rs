use crate::pickup_meta::ScriptObjectLocation;
use structs::structs::{
    DamageVulnerability,
    ChargedBeams,
    BeamCombos
};
use reader_writer::{FourCC};
use serde_derive::{Serialize,Deserialize};


#[derive(Clone, Copy, Debug)]
pub struct DoorLocation {
    pub door_location: ScriptObjectLocation,
    pub door_force_location: ScriptObjectLocation,
    pub door_shield_location: ScriptObjectLocation,
    pub dock_number: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
enum TypeVulnerability {
    Normal = 0x1,
    Reflect = 0x2,
    Immune = 0x3,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum DoorType {
    Blue,
    Purple,
    White,
    Red
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Weights {
    pub tallon_overworld: [u8;4],
    pub chozo_ruins: [u8;4],
    pub magmoor_caverns: [u8;4],
    pub phendrana_drifts: [u8;4],
    pub phazon_mines: [u8;4]
}

pub enum World {
    TallonOverworld,
    ChozoRuins,
    MagmoorCaverns,
    PhendranaDrifts,
    PhazonMines
}

impl World {
    pub fn from_pak(pak_str:&str) -> Option<Self> {
        match pak_str {
            "Metroid2.pak" => Some(World::ChozoRuins),
            "Metroid3.pak" => Some(World::PhendranaDrifts),
            "Metroid4.pak" => Some(World::TallonOverworld),
            "metroid5.pak" => Some(World::PhazonMines),
            "Metroid6.pak" => Some(World::MagmoorCaverns),
            _ => None
        }
    }
}

impl DoorType {

    pub fn shield_cmdl(&self) -> u32 {
        match self {
            DoorType::Blue =>   0x0734977A,
            DoorType::Purple => 0x33188D1B,
            DoorType::White =>  0x59649E9D,
            DoorType::Red =>    0xBBBA1EC7,
        }
    }

    pub fn forcefield_txtr(&self) -> u32 {
        match self {
            DoorType::Blue =>   0x8A7F3683,
            DoorType::Purple => 0xF68DF7F1,
            DoorType::White =>  0xBE4CD99D,
            DoorType::Red =>    0xFC095F6C,
        }
    }

    pub fn dependencies(&self) -> &'static [(u32, FourCC)] {
        match self {
            DoorType::Blue => {
                const DATA: &[(u32,FourCC)] = &[
                    (0x0734977A,FourCC::from_bytes(b"CMDL")),
                    (0x8A7F3683,FourCC::from_bytes(b"TXTR")),
                    (0x88ED4593,FourCC::from_bytes(b"TXTR")), //shield texture
                ];
                DATA
            },
            DoorType::Purple => {
                const DATA: &[(u32,FourCC)] = &[
                    (0x33188D1B,FourCC::from_bytes(b"CMDL")),
                    (0xF68DF7F1,FourCC::from_bytes(b"TXTR")),
                    (0xAB031EA9,FourCC::from_bytes(b"TXTR")), //shield texture
                ];
                DATA
            },
            DoorType::White => {
                const DATA: &[(u32,FourCC)] = &[
                    (0x59649E9D,FourCC::from_bytes(b"CMDL")),
                    (0xBE4CD99D,FourCC::from_bytes(b"TXTR")),
                    (0xF6870C9F,FourCC::from_bytes(b"TXTR")), //shield texture
                ];
                DATA
            },
            DoorType::Red => {
                const DATA: &[(u32,FourCC)] = &[
                    (0xBBBA1EC7,FourCC::from_bytes(b"CMDL")),
                    (0xFC095F6C,FourCC::from_bytes(b"TXTR")),
                    (0x61A6945B,FourCC::from_bytes(b"TXTR")), //shield texture
                ];
                DATA
            },
        }
    }

    pub fn iter() -> impl Iterator<Item = DoorType> {
        [
            DoorType::Blue,
            DoorType::Purple,
            DoorType::White,
            DoorType::Red,
        ].iter().map(|i| *i)
    }

    pub fn vulnerability(&self) -> DamageVulnerability {
        match self {
            DoorType::Blue => DamageVulnerability {
                power: TypeVulnerability::Normal as u32,
                ice: TypeVulnerability::Normal as u32,
                wave: TypeVulnerability::Normal as u32,
                plasma: TypeVulnerability::Normal as u32,
                bomb: TypeVulnerability::Normal as u32,
                power_bomb: TypeVulnerability::Normal as u32,
                missile: TypeVulnerability::Reflect as u32,
                boost_ball: TypeVulnerability::Reflect as u32,
                phazon: TypeVulnerability::Normal as u32,

                enemy_weapon0:TypeVulnerability::Immune as u32,
                enemy_weapon1:TypeVulnerability::Immune as u32,
                enemy_weapon2:TypeVulnerability::Immune as u32,
                enemy_weapon3:TypeVulnerability::Immune as u32,

                unknown_weapon0:TypeVulnerability::Immune as u32,
                unknown_weapon1:TypeVulnerability::Immune as u32,
                unknown_weapon2:TypeVulnerability::Immune as u32,

                charged_beams:ChargedBeams {
                    power:TypeVulnerability::Normal as u32,
                    ice:TypeVulnerability::Normal as u32,
                    wave:TypeVulnerability::Normal as u32,
                    plasma:TypeVulnerability::Normal as u32,
                    phazon:TypeVulnerability::Normal as u32,
                },
                beam_combos:BeamCombos {
                    power:TypeVulnerability::Normal as u32,
                    ice:TypeVulnerability::Normal as u32,
                    wave:TypeVulnerability::Normal as u32,
                    plasma:TypeVulnerability::Normal as u32,
                    phazon:TypeVulnerability::Normal as u32,
                }
            },
            DoorType::Purple => DamageVulnerability {
                power: TypeVulnerability::Reflect as u32,
                ice: TypeVulnerability::Reflect as u32,
                wave: TypeVulnerability::Normal as u32,
                plasma: TypeVulnerability::Reflect as u32,
                bomb: TypeVulnerability::Immune as u32,
                power_bomb: TypeVulnerability::Immune as u32,
                missile: TypeVulnerability::Reflect as u32,
                boost_ball: TypeVulnerability::Reflect as u32,
                phazon: TypeVulnerability::Immune as u32,

                enemy_weapon0:TypeVulnerability::Immune as u32,
                enemy_weapon1:TypeVulnerability::Immune as u32,
                enemy_weapon2:TypeVulnerability::Immune as u32,
                enemy_weapon3:TypeVulnerability::Immune as u32,

                unknown_weapon0:TypeVulnerability::Immune as u32,
                unknown_weapon1:TypeVulnerability::Immune as u32,
                unknown_weapon2:TypeVulnerability::Immune as u32,

                charged_beams:ChargedBeams {
                    power:TypeVulnerability::Reflect as u32,
                    ice:TypeVulnerability::Reflect as u32,
                    wave:TypeVulnerability::Normal as u32,
                    plasma:TypeVulnerability::Reflect as u32,
                    phazon:TypeVulnerability::Reflect as u32,
                },
                beam_combos:BeamCombos {
                    power:TypeVulnerability::Reflect as u32,
                    ice:TypeVulnerability::Reflect as u32,
                    wave:TypeVulnerability::Normal as u32,
                    plasma:TypeVulnerability::Reflect as u32,
                    phazon:TypeVulnerability::Reflect as u32,
                }
            },
            DoorType::White => DamageVulnerability {
                power: TypeVulnerability::Reflect as u32,
                ice: TypeVulnerability::Normal as u32,
                wave: TypeVulnerability::Reflect as u32,
                plasma: TypeVulnerability::Reflect as u32,
                bomb: TypeVulnerability::Immune as u32,
                power_bomb: TypeVulnerability::Immune as u32,
                missile: TypeVulnerability::Reflect as u32,
                boost_ball: TypeVulnerability::Reflect as u32,
                phazon: TypeVulnerability::Immune as u32,

                enemy_weapon0:TypeVulnerability::Immune as u32,
                enemy_weapon1:TypeVulnerability::Immune as u32,
                enemy_weapon2:TypeVulnerability::Immune as u32,
                enemy_weapon3:TypeVulnerability::Immune as u32,

                unknown_weapon0:TypeVulnerability::Immune as u32,
                unknown_weapon1:TypeVulnerability::Immune as u32,
                unknown_weapon2:TypeVulnerability::Immune as u32,


                charged_beams:ChargedBeams {
                    power:TypeVulnerability::Reflect as u32,
                    ice:TypeVulnerability::Normal as u32,
                    wave:TypeVulnerability::Reflect as u32,
                    plasma:TypeVulnerability::Reflect as u32,
                    phazon:TypeVulnerability::Reflect as u32,
                },
                beam_combos:BeamCombos {
                    power:TypeVulnerability::Reflect as u32,
                    ice:TypeVulnerability::Normal as u32,
                    wave:TypeVulnerability::Reflect as u32,
                    plasma:TypeVulnerability::Reflect as u32,
                    phazon:TypeVulnerability::Reflect as u32,
                }
            },
            DoorType::Red => DamageVulnerability {
                power: TypeVulnerability::Reflect as u32,
                ice: TypeVulnerability::Reflect as u32,
                wave: TypeVulnerability::Reflect as u32,
                plasma: TypeVulnerability::Normal as u32,
                bomb: TypeVulnerability::Immune as u32,
                power_bomb: TypeVulnerability::Immune as u32,
                missile: TypeVulnerability::Reflect as u32,
                boost_ball: TypeVulnerability::Reflect as u32,
                phazon: TypeVulnerability::Immune as u32,

                enemy_weapon0:TypeVulnerability::Immune as u32,
                enemy_weapon1:TypeVulnerability::Immune as u32,
                enemy_weapon2:TypeVulnerability::Immune as u32,
                enemy_weapon3:TypeVulnerability::Immune as u32,


                unknown_weapon0:TypeVulnerability::Immune as u32,
                unknown_weapon1:TypeVulnerability::Immune as u32,
                unknown_weapon2:TypeVulnerability::Immune as u32,


                charged_beams:ChargedBeams {
                    power:TypeVulnerability::Reflect as u32,
                    ice:TypeVulnerability::Reflect as u32,
                    wave:TypeVulnerability::Reflect as u32,
                    plasma:TypeVulnerability::Normal as u32,
                    phazon:TypeVulnerability::Reflect as u32,
                },
                beam_combos:BeamCombos {
                    power:TypeVulnerability::Reflect as u32,
                    ice:TypeVulnerability::Reflect as u32,
                    wave:TypeVulnerability::Reflect as u32,
                    plasma:TypeVulnerability::Normal as u32,
                    phazon:TypeVulnerability::Reflect as u32,
                },
            },
        }
    }

    pub fn from_cmdl (cmdl: &u32) -> Option<Self> {
        match cmdl {
            0x0734977A => Some(DoorType::Blue),
            0x33188D1B => Some(DoorType::Purple),
            0x59649E9D => Some(DoorType::White),
            0xBBBA1EC7 => Some(DoorType::Red),
            _ => None,
        }
    }

    pub fn from_txtr (txtr: &u32) -> Option<Self> {
        match txtr {
            0x8A7F3683 => Some(DoorType::Blue),
            0xF68DF7F1 => Some(DoorType::Purple),
            0xBE4CD99D => Some(DoorType::White),
            0xFC095F6C => Some(DoorType::Red),
            _ => None,
        }
    }
}
