use crate::pickup_meta::ScriptObjectLocation;
use structs::structs::{
    DamageVulnerability,
    ChargedBeams,
    BeamCombos
};


#[derive(Clone, Copy, Debug)]
pub struct DoorLocation {
    pub door_location: ScriptObjectLocation,
    pub door_force_location: ScriptObjectLocation,
    pub door_shield_location: ScriptObjectLocation
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
enum TypeVulnerability {
    DoubleDamage = 0x0,
    Normal = 0x1,
    Reflect = 0x2,
    Immune = 0x3,
    PassThrough = 0x4,
    DirectDouble = 0x5,
    DirectNormal = 0x6,
    DirectImmune = 0x7,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum DoorType {
    Blue,
    Purple,
    White,
    Red
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
