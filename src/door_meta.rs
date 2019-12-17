use crate::pickup_meta::ScriptObjectLocation;


#[derive(Clone, Debug)]
pub struct DoorLocation {
    pub door_location: ScriptObjectLocation,
    pub door_force_location: ScriptObjectLocation,
    pub door_shield_location: ScriptObjectLocation
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
