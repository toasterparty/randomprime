use std::collections::HashMap;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref ROOM_BY_MREA: HashMap<u32, &'static RoomLookup> = {
        let mut map = HashMap::new();
        for room in ROOM_LOOKUP.iter() {
            map.insert(room.mrea_id, room);
        }
        map
    };

    pub static ref ROOM_BY_INTERNAL_ID: HashMap<u32, &'static RoomLookup> = {
        let mut map = HashMap::new();
        for room in ROOM_LOOKUP.iter() {
            map.insert(room.internal_id, room);
        }
        map
    };

    pub static ref ROOM_BY_NAME: HashMap<(String, String), &'static RoomLookup> = {
        let mut map = HashMap::new();
        for room in ROOM_LOOKUP.iter() {
            map.insert((room.region_name.to_string(), room.room_name.to_string()), room);
        }
        map
    };
}

pub struct RoomLookup
{
    pub mrea_id: u32,
    pub internal_id: u32,
    pub _area_index: u32,
    pub layer_count: u32,
    pub region_name: &'static str,
    pub room_name: &'static str,
}

const ROOM_LOOKUP: &[RoomLookup] = &[
    RoomLookup {
        mrea_id: 0xB4B41C48,
        internal_id: 0x2084E568,
        _area_index: 0,
        layer_count: 19,
        region_name: "End Cinema",
        room_name: "End Cinema"
    },
    RoomLookup {
        mrea_id: 0xD1241219,
        internal_id: 0xC34F20FF,
        _area_index: 0,
        layer_count: 5,
        region_name: "Frigate Orpheon",
        room_name: "Exterior Docking Hangar"
    },
    RoomLookup {
        mrea_id: 0x07640602,
        internal_id: 0xAE68F0CF,
        _area_index: 1,
        layer_count: 3,
        region_name: "Frigate Orpheon",
        room_name: "Air Lock"
    },
    RoomLookup {
        mrea_id: 0x624F493A,
        internal_id: 0x5A332264,
        _area_index: 2,
        layer_count: 1,
        region_name: "Frigate Orpheon",
        room_name: "Deck Alpha Access Hall"
    },
    RoomLookup {
        mrea_id: 0xC8971E99,
        internal_id: 0x9A48973E,
        _area_index: 3,
        layer_count: 1,
        region_name: "Frigate Orpheon",
        room_name: "Deck Alpha Mech Shaft"
    },
    RoomLookup {
        mrea_id: 0x20E48216,
        internal_id: 0x68E253D2,
        _area_index: 4,
        layer_count: 2,
        region_name: "Frigate Orpheon",
        room_name: "Emergency Evacuation Area"
    },
    RoomLookup {
        mrea_id: 0xAE1EC8BD,
        internal_id: 0xB0134DF2,
        _area_index: 5,
        layer_count: 2,
        region_name: "Frigate Orpheon",
        room_name: "Connection Elevator to Deck Alpha"
    },
    RoomLookup {
        mrea_id: 0xEE21C026,
        internal_id: 0xB5735576,
        _area_index: 6,
        layer_count: 1,
        region_name: "Frigate Orpheon",
        room_name: "Deck Alpha Umbilical Hall"
    },
    RoomLookup {
        mrea_id: 0xC5DE3C06,
        internal_id: 0xCEE4F1C8,
        _area_index: 7,
        layer_count: 2,
        region_name: "Frigate Orpheon",
        room_name: "Biotech Research Area 2"
    },
    RoomLookup {
        mrea_id: 0xCDE604F0,
        internal_id: 0xA3049BAD,
        _area_index: 8,
        layer_count: 1,
        region_name: "Frigate Orpheon",
        room_name: "Map Facility"
    },
    RoomLookup {
        mrea_id: 0x1055715C,
        internal_id: 0x796AD968,
        _area_index: 9,
        layer_count: 1,
        region_name: "Frigate Orpheon",
        room_name: "Main Ventilation Shaft Section F"
    },
    RoomLookup {
        mrea_id: 0x31C44B23,
        internal_id: 0x95CDA510,
        _area_index: 10,
        layer_count: 1,
        region_name: "Frigate Orpheon",
        room_name: "Connection Elevator to Deck Beta"
    },
    RoomLookup {
        mrea_id: 0x292DDC1C,
        internal_id: 0xE13DEC29,
        _area_index: 11,
        layer_count: 1,
        region_name: "Frigate Orpheon",
        room_name: "Main Ventilation Shaft Section E"
    },
    RoomLookup {
        mrea_id: 0xA8813FB6,
        internal_id: 0xF4160CEA,
        _area_index: 12,
        layer_count: 1,
        region_name: "Frigate Orpheon",
        room_name: "Deck Beta Conduit Hall"
    },
    RoomLookup {
        mrea_id: 0x3E05B8DC,
        internal_id: 0x96F0FF16,
        _area_index: 13,
        layer_count: 1,
        region_name: "Frigate Orpheon",
        room_name: "Main Ventilation Shaft Section D"
    },
    RoomLookup {
        mrea_id: 0x85578E54,
        internal_id: 0x9ABB98C6,
        _area_index: 14,
        layer_count: 2,
        region_name: "Frigate Orpheon",
        room_name: "Biotech Research Area 1"
    },
    RoomLookup {
        mrea_id: 0x5BDC869C,
        internal_id: 0x0AE280EA,
        _area_index: 15,
        layer_count: 1,
        region_name: "Frigate Orpheon",
        room_name: "Main Ventilation Shaft Section C"
    },
    RoomLookup {
        mrea_id: 0x49C59925,
        internal_id: 0x8A1AEE7E,
        _area_index: 16,
        layer_count: 1,
        region_name: "Frigate Orpheon",
        room_name: "Deck Beta Security Hall"
    },
    RoomLookup {
        mrea_id: 0x6ED3231B,
        internal_id: 0x05289664,
        _area_index: 10,
        layer_count: 1,
        region_name: "Frigate Orpheon",
        room_name: "Connection Elevator to Deck Beta"
    },
    RoomLookup {
        mrea_id: 0x093500E4,
        internal_id: 0xDF5E9BC0,
        _area_index: 18,
        layer_count: 1,
        region_name: "Frigate Orpheon",
        room_name: "Subventilation Shaft Section A"
    },
    RoomLookup {
        mrea_id: 0x4CF4E25C,
        internal_id: 0x7D2F93D5,
        _area_index: 19,
        layer_count: 1,
        region_name: "Frigate Orpheon",
        room_name: "Main Ventilation Shaft Section B"
    },
    RoomLookup {
        mrea_id: 0xD16B26D0,
        internal_id: 0x1A2D47EE,
        _area_index: 20,
        layer_count: 2,
        region_name: "Frigate Orpheon",
        room_name: "Biohazard Containment"
    },
    RoomLookup {
        mrea_id: 0xE667B605,
        internal_id: 0x6E0AC070,
        _area_index: 21,
        layer_count: 1,
        region_name: "Frigate Orpheon",
        room_name: "Deck Gamma Monitor Hall"
    },
    RoomLookup {
        mrea_id: 0xC0BBB28A,
        internal_id: 0xDFD494A8,
        _area_index: 22,
        layer_count: 1,
        region_name: "Frigate Orpheon",
        room_name: "Subventilation Shaft Section B"
    },
    RoomLookup {
        mrea_id: 0x758C4F1C,
        internal_id: 0xE578A694,
        _area_index: 23,
        layer_count: 1,
        region_name: "Frigate Orpheon",
        room_name: "Main Ventilation Shaft Section A"
    },
    RoomLookup {
        mrea_id: 0x2CA2A263,
        internal_id: 0xDD087939,
        _area_index: 24,
        layer_count: 1,
        region_name: "Frigate Orpheon",
        room_name: "Deck Beta Transit Hall"
    },
    RoomLookup {
        mrea_id: 0x87452DC1,
        internal_id: 0xB22C4E90,
        _area_index: 25,
        layer_count: 3,
        region_name: "Frigate Orpheon",
        room_name: "Reactor Core"
    },
    RoomLookup {
        mrea_id: 0x6FF0FD62,
        internal_id: 0xBAE54D64,
        _area_index: 26,
        layer_count: 3,
        region_name: "Frigate Orpheon",
        room_name: "Cargo Freight Lift to Deck Gamma"
    },
    RoomLookup {
        mrea_id: 0x3EA190EE,
        internal_id: 0xDE9A4E18,
        _area_index: 27,
        layer_count: 2,
        region_name: "Frigate Orpheon",
        room_name: "Reactor Core Entrance"
    },
    RoomLookup {
        mrea_id: 0x3E6B2BB7,
        internal_id: 0xDBED08BA,
        _area_index: 0,
        layer_count: 2,
        region_name: "Chozo Ruins",
        room_name: "Transport to Tallon Overworld North"
    },
    RoomLookup {
        mrea_id: 0xB7F1952A,
        internal_id: 0x3850211A,
        _area_index: 1,
        layer_count: 3,
        region_name: "Chozo Ruins",
        room_name: "Ruins Entrance"
    },
    RoomLookup {
        mrea_id: 0xD5CDB809,
        internal_id: 0x6656AABE,
        _area_index: 2,
        layer_count: 5,
        region_name: "Chozo Ruins",
        room_name: "Main Plaza"
    },
    RoomLookup {
        mrea_id: 0x560DBE38,
        internal_id: 0xF45C3943,
        _area_index: 3,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Ruined Fountain Access"
    },
    RoomLookup {
        mrea_id: 0xDF746AE0,
        internal_id: 0xADB1CC68,
        _area_index: 4,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Ruined Shrine Access"
    },
    RoomLookup {
        mrea_id: 0x092D89FD,
        internal_id: 0x85C58EC0,
        _area_index: 5,
        layer_count: 3,
        region_name: "Chozo Ruins",
        room_name: "Nursery Access"
    },
    RoomLookup {
        mrea_id: 0x53359457,
        internal_id: 0x32FEAA13,
        _area_index: 6,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Plaza Access"
    },
    RoomLookup {
        mrea_id: 0x2B3F1CEE,
        internal_id: 0xC8F9A1BE,
        _area_index: 7,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Piston Tunnel"
    },
    RoomLookup {
        mrea_id: 0x165A4DE9,
        internal_id: 0xB73FCF79,
        _area_index: 8,
        layer_count: 3,
        region_name: "Chozo Ruins",
        room_name: "Ruined Fountain"
    },
    RoomLookup {
        mrea_id: 0x3C785450,
        internal_id: 0x59593237,
        _area_index: 9,
        layer_count: 5,
        region_name: "Chozo Ruins",
        room_name: "Ruined Shrine"
    },
    RoomLookup {
        mrea_id: 0xCB1E8A0B,
        internal_id: 0x52C34364,
        _area_index: 10,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Eyon Tunnel"
    },
    RoomLookup {
        mrea_id: 0xEF069019,
        internal_id: 0xC4CC2110,
        _area_index: 11,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Vault"
    },
    RoomLookup {
        mrea_id: 0x3F04F304,
        internal_id: 0x9F2FA381,
        _area_index: 12,
        layer_count: 3,
        region_name: "Chozo Ruins",
        room_name: "Training Chamber"
    },
    RoomLookup {
        mrea_id: 0x870B0525,
        internal_id: 0x8F6534FF,
        _area_index: 13,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Arboretum Access"
    },
    RoomLookup {
        mrea_id: 0x282B16B4,
        internal_id: 0xD0BFE7BE,
        _area_index: 14,
        layer_count: 3,
        region_name: "Chozo Ruins",
        room_name: "Meditation Fountain"
    },
    RoomLookup {
        mrea_id: 0x59E0184E,
        internal_id: 0xB98BED3C,
        _area_index: 15,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Tower of Light Access"
    },
    RoomLookup {
        mrea_id: 0xC2576E4D,
        internal_id: 0xDE6AAF50,
        _area_index: 16,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Ruined Nursery"
    },
    RoomLookup {
        mrea_id: 0xA5089191,
        internal_id: 0x2D0D3708,
        _area_index: 17,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Vault Access"
    },
    RoomLookup {
        mrea_id: 0x18D186BB,
        internal_id: 0x678AF25B,
        _area_index: 18,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Training Chamber Access"
    },
    RoomLookup {
        mrea_id: 0x18AB6106,
        internal_id: 0x29AFCCC6,
        _area_index: 19,
        layer_count: 5,
        region_name: "Chozo Ruins",
        room_name: "Arboretum"
    },
    RoomLookup {
        mrea_id: 0x491BFABA,
        internal_id: 0xBF54765D,
        _area_index: 20,
        layer_count: 2,
        region_name: "Chozo Ruins",
        room_name: "Magma Pool"
    },
    RoomLookup {
        mrea_id: 0x0D72F1F7,
        internal_id: 0xA1AE2B03,
        _area_index: 21,
        layer_count: 2,
        region_name: "Chozo Ruins",
        room_name: "Tower of Light"
    },
    RoomLookup {
        mrea_id: 0x1D5E482C,
        internal_id: 0x66D6F156,
        _area_index: 22,
        layer_count: 3,
        region_name: "Chozo Ruins",
        room_name: "Save Station 1"
    },
    RoomLookup {
        mrea_id: 0x46295CA0,
        internal_id: 0x5DDB6EA0,
        _area_index: 23,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "North Atrium"
    },
    RoomLookup {
        mrea_id: 0x8316EDF5,
        internal_id: 0x372F1027,
        _area_index: 24,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Transport to Magmoor Caverns North"
    },
    RoomLookup {
        mrea_id: 0x3D238FCD,
        internal_id: 0x4D30DE7F,
        _area_index: 25,
        layer_count: 2,
        region_name: "Chozo Ruins",
        room_name: "Sunchamber Lobby"
    },
    RoomLookup {
        mrea_id: 0x95F2019E,
        internal_id: 0x9A3822ED,
        _area_index: 26,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Gathering Hall Access"
    },
    RoomLookup {
        mrea_id: 0x11BD63B7,
        internal_id: 0x5225F6B8,
        _area_index: 27,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Tower Chamber"
    },
    RoomLookup {
        mrea_id: 0xE34FD92B,
        internal_id: 0xBBEE2E4F,
        _area_index: 28,
        layer_count: 3,
        region_name: "Chozo Ruins",
        room_name: "Ruined Gallery"
    },
    RoomLookup {
        mrea_id: 0xDE161372,
        internal_id: 0xCF4C7AA5,
        _area_index: 29,
        layer_count: 2,
        region_name: "Chozo Ruins",
        room_name: "Sun Tower"
    },
    RoomLookup {
        mrea_id: 0x3AD2120F,
        internal_id: 0xC5E2F1AC,
        _area_index: 30,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Transport Access North"
    },
    RoomLookup {
        mrea_id: 0x54C40995,
        internal_id: 0xBA2C85C0,
        _area_index: 31,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Sunchamber Access"
    },
    RoomLookup {
        mrea_id: 0x47E73BC5,
        internal_id: 0xE1A2B7A0,
        _area_index: 32,
        layer_count: 3,
        region_name: "Chozo Ruins",
        room_name: "Gathering Hall"
    },
    RoomLookup {
        mrea_id: 0xEA8A4073,
        internal_id: 0x88045571,
        _area_index: 33,
        layer_count: 3,
        region_name: "Chozo Ruins",
        room_name: "Totem Access"
    },
    RoomLookup {
        mrea_id: 0x1A5B2E16,
        internal_id: 0x8FE4082D,
        _area_index: 34,
        layer_count: 3,
        region_name: "Chozo Ruins",
        room_name: "Map Station"
    },
    RoomLookup {
        mrea_id: 0x41CC90EC,
        internal_id: 0x27A3BC01,
        _area_index: 35,
        layer_count: 3,
        region_name: "Chozo Ruins",
        room_name: "Sun Tower Access"
    },
    RoomLookup {
        mrea_id: 0xC8309DF6,
        internal_id: 0xF08D4DB1,
        _area_index: 36,
        layer_count: 5,
        region_name: "Chozo Ruins",
        room_name: "Hive Totem"
    },
    RoomLookup {
        mrea_id: 0x9A0A03EB,
        internal_id: 0xF262C1EA,
        _area_index: 37,
        layer_count: 7,
        region_name: "Chozo Ruins",
        room_name: "Sunchamber"
    },
    RoomLookup {
        mrea_id: 0xEEEC837D,
        internal_id: 0x78796F40,
        _area_index: 38,
        layer_count: 3,
        region_name: "Chozo Ruins",
        room_name: "Watery Hall Access"
    },
    RoomLookup {
        mrea_id: 0xF7D8954E,
        internal_id: 0x266493A1,
        _area_index: 39,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Save Station 2"
    },
    RoomLookup {
        mrea_id: 0x713600E3,
        internal_id: 0x9096A9E4,
        _area_index: 40,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "East Atrium"
    },
    RoomLookup {
        mrea_id: 0x492CBF4A,
        internal_id: 0x42C471B8,
        _area_index: 41,
        layer_count: 3,
        region_name: "Chozo Ruins",
        room_name: "Watery Hall"
    },
    RoomLookup {
        mrea_id: 0x463D0D2E,
        internal_id: 0xAF7193D2,
        _area_index: 42,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Energy Core Access"
    },
    RoomLookup {
        mrea_id: 0x0F403B07,
        internal_id: 0x17F49345,
        _area_index: 43,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Dynamo Access"
    },
    RoomLookup {
        mrea_id: 0xC9D52BBC,
        internal_id: 0x1F1E23E8,
        _area_index: 44,
        layer_count: 3,
        region_name: "Chozo Ruins",
        room_name: "Energy Core"
    },
    RoomLookup {
        mrea_id: 0x04D6C285,
        internal_id: 0xB3E2522B,
        _area_index: 45,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Dynamo"
    },
    RoomLookup {
        mrea_id: 0xEF7EB590,
        internal_id: 0x0C0A9627,
        _area_index: 46,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Burn Dome Access"
    },
    RoomLookup {
        mrea_id: 0xC2715A58,
        internal_id: 0x3F036C33,
        _area_index: 47,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "West Furnace Access"
    },
    RoomLookup {
        mrea_id: 0x4148F7B0,
        internal_id: 0xE559824C,
        _area_index: 48,
        layer_count: 3,
        region_name: "Chozo Ruins",
        room_name: "Burn Dome"
    },
    RoomLookup {
        mrea_id: 0x2E318473,
        internal_id: 0xF2A7E5B2,
        _area_index: 49,
        layer_count: 3,
        region_name: "Chozo Ruins",
        room_name: "Furnace"
    },
    RoomLookup {
        mrea_id: 0x44E528F6,
        internal_id: 0x2B394D67,
        _area_index: 50,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "East Furnace Access"
    },
    RoomLookup {
        mrea_id: 0xD9E78EB0,
        internal_id: 0x479E5576,
        _area_index: 51,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Crossway Access West"
    },
    RoomLookup {
        mrea_id: 0xFB54A0CB,
        internal_id: 0xD4F7F4CC,
        _area_index: 52,
        layer_count: 5,
        region_name: "Chozo Ruins",
        room_name: "Hall of the Elders"
    },
    RoomLookup {
        mrea_id: 0x13FFF119,
        internal_id: 0x289146C1,
        _area_index: 53,
        layer_count: 2,
        region_name: "Chozo Ruins",
        room_name: "Crossway"
    },
    RoomLookup {
        mrea_id: 0x9D516D9D,
        internal_id: 0x4E9ADBB0,
        _area_index: 54,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Reflecting Pool Access"
    },
    RoomLookup {
        mrea_id: 0xE1CE5BD1,
        internal_id: 0x0369134E,
        _area_index: 55,
        layer_count: 3,
        region_name: "Chozo Ruins",
        room_name: "Elder Hall Access"
    },
    RoomLookup {
        mrea_id: 0x675A297F,
        internal_id: 0xF75D445D,
        _area_index: 56,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Crossway Access South"
    },
    RoomLookup {
        mrea_id: 0xE1981EFC,
        internal_id: 0x0439F4EF,
        _area_index: 57,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Elder Chamber"
    },
    RoomLookup {
        mrea_id: 0x361ECAAC,
        internal_id: 0x7C7AEA92,
        _area_index: 58,
        layer_count: 4,
        region_name: "Chozo Ruins",
        room_name: "Reflecting Pool"
    },
    RoomLookup {
        mrea_id: 0x188A23AF,
        internal_id: 0xAF25B033,
        _area_index: 59,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Save Station 3"
    },
    RoomLookup {
        mrea_id: 0xA2F90C53,
        internal_id: 0xA5DF18DB,
        _area_index: 60,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Transport Access South"
    },
    RoomLookup {
        mrea_id: 0xAFEFE677,
        internal_id: 0x01F1977C,
        _area_index: 61,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Antechamber"
    },
    RoomLookup {
        mrea_id: 0xA5FA69A1,
        internal_id: 0xC705A398,
        _area_index: 62,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Transport to Tallon Overworld East"
    },
    RoomLookup {
        mrea_id: 0x236E1B0F,
        internal_id: 0x23F35FE1,
        _area_index: 63,
        layer_count: 1,
        region_name: "Chozo Ruins",
        room_name: "Transport to Tallon Overworld South"
    },
    RoomLookup {
        mrea_id: 0xC00E3781,
        internal_id: 0xB2E861AC,
        _area_index: 0,
        layer_count: 2,
        region_name: "Phendrana Drifts",
        room_name: "Transport to Magmoor Caverns West"
    },
    RoomLookup {
        mrea_id: 0xC4107CD7,
        internal_id: 0xEE36BA52,
        _area_index: 1,
        layer_count: 2,
        region_name: "Phendrana Drifts",
        room_name: "Shoreline Entrance"
    },
    RoomLookup {
        mrea_id: 0xF7285979,
        internal_id: 0x27E29EB5,
        _area_index: 2,
        layer_count: 6,
        region_name: "Phendrana Drifts",
        room_name: "Phendrana Shorelines"
    },
    RoomLookup {
        mrea_id: 0x85D9F399,
        internal_id: 0x47EFDA6F,
        _area_index: 3,
        layer_count: 3,
        region_name: "Phendrana Drifts",
        room_name: "Temple Entryway"
    },
    RoomLookup {
        mrea_id: 0x0581699D,
        internal_id: 0x9E8FA3C4,
        _area_index: 4,
        layer_count: 1,
        region_name: "Phendrana Drifts",
        room_name: "Save Station B"
    },
    RoomLookup {
        mrea_id: 0x3C13643A,
        internal_id: 0xD5972FA5,
        _area_index: 5,
        layer_count: 5,
        region_name: "Phendrana Drifts",
        room_name: "Ruins Entryway"
    },
    RoomLookup {
        mrea_id: 0xC8115292,
        internal_id: 0xBA8FAC6C,
        _area_index: 6,
        layer_count: 5,
        region_name: "Phendrana Drifts",
        room_name: "Plaza Walkway"
    },
    RoomLookup {
        mrea_id: 0x4E85203C,
        internal_id: 0xD4E7624E,
        _area_index: 7,
        layer_count: 4,
        region_name: "Phendrana Drifts",
        room_name: "Ice Ruins Access"
    },
    RoomLookup {
        mrea_id: 0x6655F51E,
        internal_id: 0xCB2D7AE0,
        _area_index: 8,
        layer_count: 4,
        region_name: "Phendrana Drifts",
        room_name: "Chozo Ice Temple"
    },
    RoomLookup {
        mrea_id: 0xB33A0620,
        internal_id: 0xC1E8B524,
        _area_index: 9,
        layer_count: 3,
        region_name: "Phendrana Drifts",
        room_name: "Ice Ruins West"
    },
    RoomLookup {
        mrea_id: 0xDAFCC26F,
        internal_id: 0x5F9D4CD7,
        _area_index: 10,
        layer_count: 3,
        region_name: "Phendrana Drifts",
        room_name: "Ice Ruins East"
    },
    RoomLookup {
        mrea_id: 0xEF674B4C,
        internal_id: 0x997591BC,
        _area_index: 11,
        layer_count: 1,
        region_name: "Phendrana Drifts",
        room_name: "Chapel Tunnel"
    },
    RoomLookup {
        mrea_id: 0xCFB8ABD1,
        internal_id: 0x46F4DE51,
        _area_index: 12,
        layer_count: 1,
        region_name: "Phendrana Drifts",
        room_name: "Courtyard Entryway"
    },
    RoomLookup {
        mrea_id: 0x034D8137,
        internal_id: 0x2987144D,
        _area_index: 13,
        layer_count: 4,
        region_name: "Phendrana Drifts",
        room_name: "Canyon Entryway"
    },
    RoomLookup {
        mrea_id: 0x40C548E9,
        internal_id: 0x9458A4D2,
        _area_index: 14,
        layer_count: 2,
        region_name: "Phendrana Drifts",
        room_name: "Chapel of the Elders"
    },
    RoomLookup {
        mrea_id: 0x1921876D,
        internal_id: 0x4303C1B2,
        _area_index: 15,
        layer_count: 6,
        region_name: "Phendrana Drifts",
        room_name: "Ruined Courtyard"
    },
    RoomLookup {
        mrea_id: 0xA20A7455,
        internal_id: 0xD9BB2AAB,
        _area_index: 16,
        layer_count: 1,
        region_name: "Phendrana Drifts",
        room_name: "Phendrana Canyon"
    },
    RoomLookup {
        mrea_id: 0x5694A06B,
        internal_id: 0xC025E629,
        _area_index: 17,
        layer_count: 1,
        region_name: "Phendrana Drifts",
        room_name: "Save Station A"
    },
    RoomLookup {
        mrea_id: 0xD341D2DB,
        internal_id: 0x64C37415,
        _area_index: 18,
        layer_count: 4,
        region_name: "Phendrana Drifts",
        room_name: "Specimen Storage"
    },
    RoomLookup {
        mrea_id: 0xEAB320CF,
        internal_id: 0x20AC209D,
        _area_index: 19,
        layer_count: 3,
        region_name: "Phendrana Drifts",
        room_name: "Quarantine Access"
    },
    RoomLookup {
        mrea_id: 0xB51FCE29,
        internal_id: 0x14F1AE87,
        _area_index: 20,
        layer_count: 7,
        region_name: "Phendrana Drifts",
        room_name: "Research Entrance"
    },
    RoomLookup {
        mrea_id: 0x05E1962E,
        internal_id: 0x91F87B2D,
        _area_index: 21,
        layer_count: 1,
        region_name: "Phendrana Drifts",
        room_name: "North Quarantine Tunnel"
    },
    RoomLookup {
        mrea_id: 0x83151B33,
        internal_id: 0xAFCAE029,
        _area_index: 22,
        layer_count: 1,
        region_name: "Phendrana Drifts",
        room_name: "Map Station"
    },
    RoomLookup {
        mrea_id: 0x3947E047,
        internal_id: 0xB6355C87,
        _area_index: 23,
        layer_count: 2,
        region_name: "Phendrana Drifts",
        room_name: "Hydra Lab Entryway"
    },
    RoomLookup {
        mrea_id: 0x70181194,
        internal_id: 0xDB4D2B00,
        _area_index: 24,
        layer_count: 5,
        region_name: "Phendrana Drifts",
        room_name: "Quarantine Cave"
    },
    RoomLookup {
        mrea_id: 0x43E4CC25,
        internal_id: 0x0EA3412A,
        _area_index: 25,
        layer_count: 5,
        region_name: "Phendrana Drifts",
        room_name: "Research Lab Hydra"
    },
    RoomLookup {
        mrea_id: 0x0035FDAD,
        internal_id: 0x2821CA0C,
        _area_index: 26,
        layer_count: 1,
        region_name: "Phendrana Drifts",
        room_name: "South Quarantine Tunnel"
    },
    RoomLookup {
        mrea_id: 0x2191A05D,
        internal_id: 0xB74243C2,
        _area_index: 27,
        layer_count: 1,
        region_name: "Phendrana Drifts",
        room_name: "Quarantine Monitor"
    },
    RoomLookup {
        mrea_id: 0x37BBB33C,
        internal_id: 0x7D554BA6,
        _area_index: 28,
        layer_count: 3,
        region_name: "Phendrana Drifts",
        room_name: "Observatory Access"
    },
    RoomLookup {
        mrea_id: 0xDD0B0739,
        internal_id: 0x31D08ACB,
        _area_index: 29,
        layer_count: 1,
        region_name: "Phendrana Drifts",
        room_name: "Transport to Magmoor Caverns South"
    },
    RoomLookup {
        mrea_id: 0x3FB4A34E,
        internal_id: 0x7E5F6217,
        _area_index: 30,
        layer_count: 3,
        region_name: "Phendrana Drifts",
        room_name: "Observatory"
    },
    RoomLookup {
        mrea_id: 0xD695B958,
        internal_id: 0xDD1AC534,
        _area_index: 31,
        layer_count: 1,
        region_name: "Phendrana Drifts",
        room_name: "Transport Access"
    },
    RoomLookup {
        mrea_id: 0x1E48B18F,
        internal_id: 0x665E3028,
        _area_index: 32,
        layer_count: 3,
        region_name: "Phendrana Drifts",
        room_name: "West Tower Entrance"
    },
    RoomLookup {
        mrea_id: 0x715C31EE,
        internal_id: 0xFB01A2DF,
        _area_index: 33,
        layer_count: 1,
        region_name: "Phendrana Drifts",
        room_name: "Save Station D"
    },
    RoomLookup {
        mrea_id: 0xD79EE805,
        internal_id: 0x88EBDFAC,
        _area_index: 34,
        layer_count: 3,
        region_name: "Phendrana Drifts",
        room_name: "Frozen Pike"
    },
    RoomLookup {
        mrea_id: 0xD79D6B9F,
        internal_id: 0x7EA7EBC4,
        _area_index: 35,
        layer_count: 2,
        region_name: "Phendrana Drifts",
        room_name: "West Tower"
    },
    RoomLookup {
        mrea_id: 0x760E731A,
        internal_id: 0x938EA87D,
        _area_index: 36,
        layer_count: 1,
        region_name: "Phendrana Drifts",
        room_name: "Pike Access"
    },
    RoomLookup {
        mrea_id: 0x39C70FB9,
        internal_id: 0x6C4E9E84,
        _area_index: 37,
        layer_count: 2,
        region_name: "Phendrana Drifts",
        room_name: "Frost Cave Access"
    },
    RoomLookup {
        mrea_id: 0x20EA1D30,
        internal_id: 0xF7A085E1,
        _area_index: 38,
        layer_count: 1,
        region_name: "Phendrana Drifts",
        room_name: "Hunter Cave Access"
    },
    RoomLookup {
        mrea_id: 0xB3C33249,
        internal_id: 0xE5F4B66E,
        _area_index: 39,
        layer_count: 8,
        region_name: "Phendrana Drifts",
        room_name: "Control Tower"
    },
    RoomLookup {
        mrea_id: 0xA49B2544,
        internal_id: 0x77AD2910,
        _area_index: 40,
        layer_count: 3,
        region_name: "Phendrana Drifts",
        room_name: "Research Core"
    },
    RoomLookup {
        mrea_id: 0x4C6F7773,
        internal_id: 0xC91D48C5,
        _area_index: 41,
        layer_count: 3,
        region_name: "Phendrana Drifts",
        room_name: "Frost Cave"
    },
    RoomLookup {
        mrea_id: 0x1EC7951A,
        internal_id: 0xD28A57CF,
        _area_index: 42,
        layer_count: 3,
        region_name: "Phendrana Drifts",
        room_name: "Hunter Cave"
    },
    RoomLookup {
        mrea_id: 0x51091931,
        internal_id: 0x0CEA2747,
        _area_index: 43,
        layer_count: 1,
        region_name: "Phendrana Drifts",
        room_name: "East Tower"
    },
    RoomLookup {
        mrea_id: 0xD8E905DD,
        internal_id: 0xCC011016,
        _area_index: 44,
        layer_count: 3,
        region_name: "Phendrana Drifts",
        room_name: "Research Core Access"
    },
    RoomLookup {
        mrea_id: 0xCEDDBA38,
        internal_id: 0x389C60A0,
        _area_index: 45,
        layer_count: 1,
        region_name: "Phendrana Drifts",
        room_name: "Save Station C"
    },
    RoomLookup {
        mrea_id: 0x253E76B3,
        internal_id: 0x4E7934C0,
        _area_index: 46,
        layer_count: 1,
        region_name: "Phendrana Drifts",
        room_name: "Upper Edge Tunnel"
    },
    RoomLookup {
        mrea_id: 0x53801084,
        internal_id: 0x9B3E462B,
        _area_index: 47,
        layer_count: 1,
        region_name: "Phendrana Drifts",
        room_name: "Lower Edge Tunnel"
    },
    RoomLookup {
        mrea_id: 0xCA6CC052,
        internal_id: 0xFF2D6F70,
        _area_index: 48,
        layer_count: 1,
        region_name: "Phendrana Drifts",
        room_name: "Chamber Access"
    },
    RoomLookup {
        mrea_id: 0x89D7A0A6,
        internal_id: 0xFFAA01E4,
        _area_index: 49,
        layer_count: 1,
        region_name: "Phendrana Drifts",
        room_name: "Lake Tunnel"
    },
    RoomLookup {
        mrea_id: 0x98DCC321,
        internal_id: 0x0836FE0A,
        _area_index: 50,
        layer_count: 2,
        region_name: "Phendrana Drifts",
        room_name: "Aether Lab Entryway"
    },
    RoomLookup {
        mrea_id: 0x21B4BFF6,
        internal_id: 0x354889CE,
        _area_index: 51,
        layer_count: 6,
        region_name: "Phendrana Drifts",
        room_name: "Research Lab Aether"
    },
    RoomLookup {
        mrea_id: 0x54DEF128,
        internal_id: 0x8C1B7564,
        _area_index: 52,
        layer_count: 1,
        region_name: "Phendrana Drifts",
        room_name: "Phendrana's Edge"
    },
    RoomLookup {
        mrea_id: 0x49175472,
        internal_id: 0x1B5105C6,
        _area_index: 53,
        layer_count: 4,
        region_name: "Phendrana Drifts",
        room_name: "Gravity Chamber"
    },
    RoomLookup {
        mrea_id: 0xF7C84340,
        internal_id: 0xC0C0FDD4,
        _area_index: 54,
        layer_count: 1,
        region_name: "Phendrana Drifts",
        room_name: "Storage Cave"
    },
    RoomLookup {
        mrea_id: 0x3C9490E5,
        internal_id: 0x60AF3512,
        _area_index: 55,
        layer_count: 1,
        region_name: "Phendrana Drifts",
        room_name: "Security Cave"
    },
    RoomLookup {
        mrea_id: 0xB2701146,
        internal_id: 0x8FF17910,
        _area_index: 0,
        layer_count: 3,
        region_name: "Tallon Overworld",
        room_name: "Landing Site"
    },
    RoomLookup {
        mrea_id: 0x7B143499,
        internal_id: 0x765EB1D3,
        _area_index: 1,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Gully"
    },
    RoomLookup {
        mrea_id: 0xEE209548,
        internal_id: 0xD5A1AABC,
        _area_index: 2,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Canyon Cavern"
    },
    RoomLookup {
        mrea_id: 0x5B4E38F5,
        internal_id: 0xAD1E40D5,
        _area_index: 3,
        layer_count: 3,
        region_name: "Tallon Overworld",
        room_name: "Temple Hall"
    },
    RoomLookup {
        mrea_id: 0xC44E7A07,
        internal_id: 0xE7E45FF6,
        _area_index: 4,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Alcove"
    },
    RoomLookup {
        mrea_id: 0xE76AD711,
        internal_id: 0xB31AA6E7,
        _area_index: 5,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Waterfall Cavern"
    },
    RoomLookup {
        mrea_id: 0x2043C96E,
        internal_id: 0x0CBF54EA,
        _area_index: 6,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Tallon Canyon"
    },
    RoomLookup {
        mrea_id: 0xBDB1FCAC,
        internal_id: 0x663AAED7,
        _area_index: 7,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Temple Security Station"
    },
    RoomLookup {
        mrea_id: 0xB9ABCD56,
        internal_id: 0x5AED0817,
        _area_index: 8,
        layer_count: 5,
        region_name: "Tallon Overworld",
        room_name: "Frigate Crash Site"
    },
    RoomLookup {
        mrea_id: 0x13D96D3D,
        internal_id: 0xBCC6AC1B,
        _area_index: 9,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Transport Tunnel A"
    },
    RoomLookup {
        mrea_id: 0x404804D9,
        internal_id: 0x0F5DA870,
        _area_index: 10,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Root Tunnel"
    },
    RoomLookup {
        mrea_id: 0x234762BE,
        internal_id: 0xBE5CD99F,
        _area_index: 11,
        layer_count: 2,
        region_name: "Tallon Overworld",
        room_name: "Temple Lobby"
    },
    RoomLookup {
        mrea_id: 0xBB158C7E,
        internal_id: 0x1CFE014F,
        _area_index: 12,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Frigate Access Tunnel"
    },
    RoomLookup {
        mrea_id: 0xCEA263E3,
        internal_id: 0x076FA7F2,
        _area_index: 13,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Overgrown Cavern"
    },
    RoomLookup {
        mrea_id: 0x11A02448,
        internal_id: 0x6FD3B9AB,
        _area_index: 14,
        layer_count: 3,
        region_name: "Tallon Overworld",
        room_name: "Transport to Chozo Ruins West"
    },
    RoomLookup {
        mrea_id: 0xBD8C8625,
        internal_id: 0x3C4E9A9E,
        _area_index: 15,
        layer_count: 2,
        region_name: "Tallon Overworld",
        room_name: "Root Cave"
    },
    RoomLookup {
        mrea_id: 0x2398E906,
        internal_id: 0xCD2B0EA2,
        _area_index: 16,
        layer_count: 23,
        region_name: "Tallon Overworld",
        room_name: "Artifact Temple"
    },
    RoomLookup {
        mrea_id: 0x5E0EE592,
        internal_id: 0xD9B5B863,
        _area_index: 17,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Main Ventilation Shaft Section C"
    },
    RoomLookup {
        mrea_id: 0x85CA08AB,
        internal_id: 0xDB04A2D2,
        _area_index: 18,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Transport Tunnel C"
    },
    RoomLookup {
        mrea_id: 0xC7E821BA,
        internal_id: 0x61D4ABA9,
        _area_index: 19,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Transport Tunnel B"
    },
    RoomLookup {
        mrea_id: 0x24F8AFF3,
        internal_id: 0x67A8800F,
        _area_index: 20,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Arbor Chamber"
    },
    RoomLookup {
        mrea_id: 0xAFD4E038,
        internal_id: 0xAA9F1D8D,
        _area_index: 21,
        layer_count: 4,
        region_name: "Tallon Overworld",
        room_name: "Main Ventilation Shaft Section B"
    },
    RoomLookup {
        mrea_id: 0x8A31665E,
        internal_id: 0xB0C789B5,
        _area_index: 22,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Transport to Chozo Ruins East"
    },
    RoomLookup {
        mrea_id: 0x15D6FF8B,
        internal_id: 0x6D105C48,
        _area_index: 23,
        layer_count: 3,
        region_name: "Tallon Overworld",
        room_name: "Transport to Magmoor Caverns East"
    },
    RoomLookup {
        mrea_id: 0x66CBE887,
        internal_id: 0x3FE0F3BF,
        _area_index: 24,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Main Ventilation Shaft Section A"
    },
    RoomLookup {
        mrea_id: 0xEE09629A,
        internal_id: 0x26E887B7,
        _area_index: 25,
        layer_count: 4,
        region_name: "Tallon Overworld",
        room_name: "Reactor Core"
    },
    RoomLookup {
        mrea_id: 0xFB427580,
        internal_id: 0x0F0FFF7C,
        _area_index: 26,
        layer_count: 4,
        region_name: "Tallon Overworld",
        room_name: "Reactor Access"
    },
    RoomLookup {
        mrea_id: 0x37B3AFE6,
        internal_id: 0xA49148B9,
        _area_index: 27,
        layer_count: 4,
        region_name: "Tallon Overworld",
        room_name: "Cargo Freight Lift to Deck Gamma"
    },
    RoomLookup {
        mrea_id: 0xF0594C6D,
        internal_id: 0x7DFD6F83,
        _area_index: 28,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Savestation"
    },
    RoomLookup {
        mrea_id: 0x4A96005E,
        internal_id: 0x293B2A0E,
        _area_index: 29,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Deck Beta Transit Hall"
    },
    RoomLookup {
        mrea_id: 0xAC2C58FE,
        internal_id: 0x58C5CAFF,
        _area_index: 30,
        layer_count: 5,
        region_name: "Tallon Overworld",
        room_name: "Biohazard Containment"
    },
    RoomLookup {
        mrea_id: 0x76F6E356,
        internal_id: 0x5296214E,
        _area_index: 31,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Deck Beta Security Hall"
    },
    RoomLookup {
        mrea_id: 0x5F2EB7B6,
        internal_id: 0x39B3509E,
        _area_index: 32,
        layer_count: 4,
        region_name: "Tallon Overworld",
        room_name: "Biotech Research Area 1"
    },
    RoomLookup {
        mrea_id: 0xC3D44A6E,
        internal_id: 0xEBA417CF,
        _area_index: 33,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Deck Beta Conduit Hall"
    },
    RoomLookup {
        mrea_id: 0xE47228EF,
        internal_id: 0x586931AD,
        _area_index: 34,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Connection Elevator to Deck Beta"
    },
    RoomLookup {
        mrea_id: 0xFFB4A966,
        internal_id: 0x90091C8F,
        _area_index: 35,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Hydro Access Tunnel"
    },
    RoomLookup {
        mrea_id: 0xF47DBE5B,
        internal_id: 0x96F0A70E,
        _area_index: 36,
        layer_count: 3,
        region_name: "Tallon Overworld",
        room_name: "Great Tree Hall"
    },
    RoomLookup {
        mrea_id: 0xC5D6A597,
        internal_id: 0xFFAFEED1,
        _area_index: 37,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Great Tree Chamber"
    },
    RoomLookup {
        mrea_id: 0x1A932F64,
        internal_id: 0xDA7DA040,
        _area_index: 38,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Transport Tunnel D"
    },
    RoomLookup {
        mrea_id: 0xB4FBBEF5,
        internal_id: 0x0081A28C,
        _area_index: 39,
        layer_count: 2,
        region_name: "Tallon Overworld",
        room_name: "Life Grove Tunnel"
    },
    RoomLookup {
        mrea_id: 0x9D330A07,
        internal_id: 0xB4F4A399,
        _area_index: 40,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Transport Tunnel E"
    },
    RoomLookup {
        mrea_id: 0x0CA514F0,
        internal_id: 0x05301E9D,
        _area_index: 41,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Transport to Chozo Ruins South"
    },
    RoomLookup {
        mrea_id: 0x86EB2E02,
        internal_id: 0xDF6B8371,
        _area_index: 42,
        layer_count: 3,
        region_name: "Tallon Overworld",
        room_name: "Life Grove"
    },
    RoomLookup {
        mrea_id: 0x7D106670,
        internal_id: 0xBC2A964C,
        _area_index: 43,
        layer_count: 1,
        region_name: "Tallon Overworld",
        room_name: "Transport to Phazon Mines East"
    },
    RoomLookup {
        mrea_id: 0x430E999C,
        internal_id: 0x2AC6EC36,
        _area_index: 0,
        layer_count: 1,
        region_name: "Phazon Mines",
        room_name: "Transport to Tallon Overworld South"
    },
    RoomLookup {
        mrea_id: 0x68CC7758,
        internal_id: 0x5D41D1EC,
        _area_index: 1,
        layer_count: 2,
        region_name: "Phazon Mines",
        room_name: "Quarry Access"
    },
    RoomLookup {
        mrea_id: 0x643D038F,
        internal_id: 0x42E7D7A1,
        _area_index: 2,
        layer_count: 7,
        region_name: "Phazon Mines",
        room_name: "Main Quarry"
    },
    RoomLookup {
        mrea_id: 0x27A391B7,
        internal_id: 0xD5080F92,
        _area_index: 3,
        layer_count: 2,
        region_name: "Phazon Mines",
        room_name: "Waste Disposal"
    },
    RoomLookup {
        mrea_id: 0x361D41B0,
        internal_id: 0x97FF7B49,
        _area_index: 4,
        layer_count: 1,
        region_name: "Phazon Mines",
        room_name: "Save Station Mines A"
    },
    RoomLookup {
        mrea_id: 0xC7653A92,
        internal_id: 0xA4766825,
        _area_index: 5,
        layer_count: 4,
        region_name: "Phazon Mines",
        room_name: "Security Access A"
    },
    RoomLookup {
        mrea_id: 0x97D2B2F6,
        internal_id: 0xC5A0412D,
        _area_index: 6,
        layer_count: 5,
        region_name: "Phazon Mines",
        room_name: "Ore Processing"
    },
    RoomLookup {
        mrea_id: 0x956F1552,
        internal_id: 0x03417FEF,
        _area_index: 7,
        layer_count: 6,
        region_name: "Phazon Mines",
        room_name: "Mine Security Station"
    },
    RoomLookup {
        mrea_id: 0x4346A747,
        internal_id: 0x8D681DF6,
        _area_index: 8,
        layer_count: 2,
        region_name: "Phazon Mines",
        room_name: "Research Access"
    },
    RoomLookup {
        mrea_id: 0xE39C342B,
        internal_id: 0x3ABA07F2,
        _area_index: 9,
        layer_count: 1,
        region_name: "Phazon Mines",
        room_name: "Storage Depot B"
    },
    RoomLookup {
        mrea_id: 0x26219C01,
        internal_id: 0xDA7A8AB1,
        _area_index: 10,
        layer_count: 1,
        region_name: "Phazon Mines",
        room_name: "Elevator Access A"
    },
    RoomLookup {
        mrea_id: 0xA20201D4,
        internal_id: 0xF364FF62,
        _area_index: 11,
        layer_count: 3,
        region_name: "Phazon Mines",
        room_name: "Security Access B"
    },
    RoomLookup {
        mrea_id: 0x35C5D736,
        internal_id: 0x05C7A002,
        _area_index: 12,
        layer_count: 1,
        region_name: "Phazon Mines",
        room_name: "Storage Depot A"
    },
    RoomLookup {
        mrea_id: 0x8A97BB54,
        internal_id: 0xD3438CDA,
        _area_index: 13,
        layer_count: 6,
        region_name: "Phazon Mines",
        room_name: "Elite Research"
    },
    RoomLookup {
        mrea_id: 0x0146ED43,
        internal_id: 0xE6844CFA,
        _area_index: 14,
        layer_count: 1,
        region_name: "Phazon Mines",
        room_name: "Elevator A"
    },
    RoomLookup {
        mrea_id: 0x8988D1CB,
        internal_id: 0x234D3378,
        _area_index: 15,
        layer_count: 2,
        region_name: "Phazon Mines",
        room_name: "Elite Control Access"
    },
    RoomLookup {
        mrea_id: 0xC50AF17A,
        internal_id: 0x08B55780,
        _area_index: 16,
        layer_count: 6,
        region_name: "Phazon Mines",
        room_name: "Elite Control"
    },
    RoomLookup {
        mrea_id: 0xECEFEA8D,
        internal_id: 0x745FA43F,
        _area_index: 17,
        layer_count: 1,
        region_name: "Phazon Mines",
        room_name: "Maintenance Tunnel"
    },
    RoomLookup {
        mrea_id: 0x90709AAC,
        internal_id: 0xDF54F650,
        _area_index: 18,
        layer_count: 1,
        region_name: "Phazon Mines",
        room_name: "Ventilation Shaft"
    },
    RoomLookup {
        mrea_id: 0xAD2E7EB9,
        internal_id: 0xFC928FE7,
        _area_index: 19,
        layer_count: 4,
        region_name: "Phazon Mines",
        room_name: "Phazon Processing Center"
    },
    RoomLookup {
        mrea_id: 0x3F375ECC,
        internal_id: 0xCA589A4B,
        _area_index: 20,
        layer_count: 5,
        region_name: "Phazon Mines",
        room_name: "Omega Research"
    },
    RoomLookup {
        mrea_id: 0x42C4AAF1,
        internal_id: 0x821A98D5,
        _area_index: 21,
        layer_count: 1,
        region_name: "Phazon Mines",
        room_name: "Transport Access"
    },
    RoomLookup {
        mrea_id: 0xED6DE73B,
        internal_id: 0x7B2D211C,
        _area_index: 22,
        layer_count: 4,
        region_name: "Phazon Mines",
        room_name: "Processing Center Access"
    },
    RoomLookup {
        mrea_id: 0x198FF5DC,
        internal_id: 0x22B9F51A,
        _area_index: 23,
        layer_count: 1,
        region_name: "Phazon Mines",
        room_name: "Map Station Mines"
    },
    RoomLookup {
        mrea_id: 0xF517A1EA,
        internal_id: 0x88466117,
        _area_index: 24,
        layer_count: 4,
        region_name: "Phazon Mines",
        room_name: "Dynamo Access"
    },
    RoomLookup {
        mrea_id: 0xE2C2CF38,
        internal_id: 0x91C144BF,
        _area_index: 25,
        layer_count: 1,
        region_name: "Phazon Mines",
        room_name: "Transport to Magmoor Caverns South"
    },
    RoomLookup {
        mrea_id: 0x3953C353,
        internal_id: 0xEE1EF9A6,
        _area_index: 26,
        layer_count: 6,
        region_name: "Phazon Mines",
        room_name: "Elite Quarters"
    },
    RoomLookup {
        mrea_id: 0xFEA372E2,
        internal_id: 0xFB5299C0,
        _area_index: 27,
        layer_count: 6,
        region_name: "Phazon Mines",
        room_name: "Central Dynamo"
    },
    RoomLookup {
        mrea_id: 0x71343C3F,
        internal_id: 0xA15814C4,
        _area_index: 28,
        layer_count: 3,
        region_name: "Phazon Mines",
        room_name: "Elite Quarters Access"
    },
    RoomLookup {
        mrea_id: 0x5ABEEC20,
        internal_id: 0x7171D8DE,
        _area_index: 29,
        layer_count: 2,
        region_name: "Phazon Mines",
        room_name: "Quarantine Access A"
    },
    RoomLookup {
        mrea_id: 0x7BD5E0BB,
        internal_id: 0xEE54D213,
        _area_index: 30,
        layer_count: 1,
        region_name: "Phazon Mines",
        room_name: "Save Station Mines B"
    },
    RoomLookup {
        mrea_id: 0xBB3AFC4E,
        internal_id: 0xBA16E7C3,
        _area_index: 31,
        layer_count: 6,
        region_name: "Phazon Mines",
        room_name: "Metroid Quarantine B"
    },
    RoomLookup {
        mrea_id: 0xFB051F5A,
        internal_id: 0xC21090E6,
        _area_index: 32,
        layer_count: 7,
        region_name: "Phazon Mines",
        room_name: "Metroid Quarantine A"
    },
    RoomLookup {
        mrea_id: 0x14530779,
        internal_id: 0xF64A8383,
        _area_index: 33,
        layer_count: 1,
        region_name: "Phazon Mines",
        room_name: "Quarantine Access B"
    },
    RoomLookup {
        mrea_id: 0x66D0D003,
        internal_id: 0x587DFD50,
        _area_index: 34,
        layer_count: 1,
        region_name: "Phazon Mines",
        room_name: "Save Station Mines C"
    },
    RoomLookup {
        mrea_id: 0x3FD9D766,
        internal_id: 0x26634F99,
        _area_index: 35,
        layer_count: 1,
        region_name: "Phazon Mines",
        room_name: "Elevator Access B"
    },
    RoomLookup {
        mrea_id: 0xEC47C242,
        internal_id: 0x7EF868E8,
        _area_index: 36,
        layer_count: 2,
        region_name: "Phazon Mines",
        room_name: "Fungal Hall B"
    },
    RoomLookup {
        mrea_id: 0xE87957E0,
        internal_id: 0x0C591FA3,
        _area_index: 37,
        layer_count: 1,
        region_name: "Phazon Mines",
        room_name: "Elevator B"
    },
    RoomLookup {
        mrea_id: 0xB089331E,
        internal_id: 0xD28106BE,
        _area_index: 38,
        layer_count: 1,
        region_name: "Phazon Mines",
        room_name: "Missile Station Mines"
    },
    RoomLookup {
        mrea_id: 0xBBFA4AB3,
        internal_id: 0x0F7D3A4A,
        _area_index: 39,
        layer_count: 1,
        region_name: "Phazon Mines",
        room_name: "Phazon Mining Tunnel"
    },
    RoomLookup {
        mrea_id: 0xDE9D71F5,
        internal_id: 0x586FAD0D,
        _area_index: 40,
        layer_count: 2,
        region_name: "Phazon Mines",
        room_name: "Fungal Hall Access"
    },
    RoomLookup {
        mrea_id: 0x0F5277D1,
        internal_id: 0x3D18BC4A,
        _area_index: 41,
        layer_count: 2,
        region_name: "Phazon Mines",
        room_name: "Fungal Hall A"
    },
    RoomLookup {
        mrea_id: 0x3BEAADC9,
        internal_id: 0x7DC0D75B,
        _area_index: 0,
        layer_count: 2,
        region_name: "Magmoor Caverns",
        room_name: "Transport to Chozo Ruins North"
    },
    RoomLookup {
        mrea_id: 0x6D434F4E,
        internal_id: 0x8E77DC91,
        _area_index: 1,
        layer_count: 1,
        region_name: "Magmoor Caverns",
        room_name: "Burning Trail"
    },
    RoomLookup {
        mrea_id: 0x79784D3D,
        internal_id: 0x989F697A,
        _area_index: 2,
        layer_count: 1,
        region_name: "Magmoor Caverns",
        room_name: "Lake Tunnel"
    },
    RoomLookup {
        mrea_id: 0x09B3E01C,
        internal_id: 0x308AE90A,
        _area_index: 3,
        layer_count: 1,
        region_name: "Magmoor Caverns",
        room_name: "Save Station Magmoor A"
    },
    RoomLookup {
        mrea_id: 0xA4719C6A,
        internal_id: 0x11DDB78C,
        _area_index: 4,
        layer_count: 5,
        region_name: "Magmoor Caverns",
        room_name: "Lava Lake"
    },
    RoomLookup {
        mrea_id: 0xDA2ECB94,
        internal_id: 0xF9CA8A01,
        _area_index: 5,
        layer_count: 2,
        region_name: "Magmoor Caverns",
        room_name: "Pit Tunnel"
    },
    RoomLookup {
        mrea_id: 0xBAD9EDBF,
        internal_id: 0x90D4D2AB,
        _area_index: 6,
        layer_count: 4,
        region_name: "Magmoor Caverns",
        room_name: "Triclops Pit"
    },
    RoomLookup {
        mrea_id: 0x0DCC4BCC,
        internal_id: 0xD9062B28,
        _area_index: 7,
        layer_count: 1,
        region_name: "Magmoor Caverns",
        room_name: "Monitor Tunnel"
    },
    RoomLookup {
        mrea_id: 0xADEF843E,
        internal_id: 0x822830CF,
        _area_index: 8,
        layer_count: 1,
        region_name: "Magmoor Caverns",
        room_name: "Storage Cavern"
    },
    RoomLookup {
        mrea_id: 0x0C57A641,
        internal_id: 0x7368B88C,
        _area_index: 9,
        layer_count: 4,
        region_name: "Magmoor Caverns",
        room_name: "Monitor Station"
    },
    RoomLookup {
        mrea_id: 0x47F2C087,
        internal_id: 0x3B614CF7,
        _area_index: 10,
        layer_count: 1,
        region_name: "Magmoor Caverns",
        room_name: "Transport Tunnel A"
    },
    RoomLookup {
        mrea_id: 0x89A6CB8D,
        internal_id: 0x2A729E1B,
        _area_index: 11,
        layer_count: 1,
        region_name: "Magmoor Caverns",
        room_name: "Warrior Shrine"
    },
    RoomLookup {
        mrea_id: 0x901040DF,
        internal_id: 0x1BADEDDE,
        _area_index: 12,
        layer_count: 1,
        region_name: "Magmoor Caverns",
        room_name: "Shore Tunnel"
    },
    RoomLookup {
        mrea_id: 0xDCA9A28B,
        internal_id: 0x4318F156,
        _area_index: 13,
        layer_count: 1,
        region_name: "Magmoor Caverns",
        room_name: "Transport to Phendrana Drifts North"
    },
    RoomLookup {
        mrea_id: 0xF5EF1862,
        internal_id: 0x8E26574F,
        _area_index: 14,
        layer_count: 1,
        region_name: "Magmoor Caverns",
        room_name: "Fiery Shores"
    },
    RoomLookup {
        mrea_id: 0x3346C676,
        internal_id: 0x7AF80EA5,
        _area_index: 15,
        layer_count: 1,
        region_name: "Magmoor Caverns",
        room_name: "Transport Tunnel B"
    },
    RoomLookup {
        mrea_id: 0x4C3D244C,
        internal_id: 0xB3128CF6,
        _area_index: 16,
        layer_count: 2,
        region_name: "Magmoor Caverns",
        room_name: "Transport to Tallon Overworld West"
    },
    RoomLookup {
        mrea_id: 0xE4A4462E,
        internal_id: 0x5A34AF8C,
        _area_index: 17,
        layer_count: 1,
        region_name: "Magmoor Caverns",
        room_name: "Twin Fires Tunnel"
    },
    RoomLookup {
        mrea_id: 0x4C784BEA,
        internal_id: 0x4D5469E4,
        _area_index: 18,
        layer_count: 3,
        region_name: "Magmoor Caverns",
        room_name: "Twin Fires"
    },
    RoomLookup {
        mrea_id: 0xA73BD0E0,
        internal_id: 0x6547C75A,
        _area_index: 19,
        layer_count: 2,
        region_name: "Magmoor Caverns",
        room_name: "North Core Tunnel"
    },
    RoomLookup {
        mrea_id: 0xC0498676,
        internal_id: 0x59573062,
        _area_index: 20,
        layer_count: 7,
        region_name: "Magmoor Caverns",
        room_name: "Geothermal Core"
    },
    RoomLookup {
        mrea_id: 0x4CC18E5A,
        internal_id: 0x1A2F5E11,
        _area_index: 21,
        layer_count: 1,
        region_name: "Magmoor Caverns",
        room_name: "Plasma Processing"
    },
    RoomLookup {
        mrea_id: 0x70D950B8,
        internal_id: 0x458B6673,
        _area_index: 22,
        layer_count: 1,
        region_name: "Magmoor Caverns",
        room_name: "South Core Tunnel"
    },
    RoomLookup {
        mrea_id: 0x8ABEB3C3,
        internal_id: 0x6D5BE01A,
        _area_index: 23,
        layer_count: 4,
        region_name: "Magmoor Caverns",
        room_name: "Magmoor Workstation"
    },
    RoomLookup {
        mrea_id: 0x046D5649,
        internal_id: 0x04122421,
        _area_index: 24,
        layer_count: 1,
        region_name: "Magmoor Caverns",
        room_name: "Workstation Tunnel"
    },
    RoomLookup {
        mrea_id: 0xD38FD611,
        internal_id: 0x24DE8508,
        _area_index: 25,
        layer_count: 1,
        region_name: "Magmoor Caverns",
        room_name: "Transport Tunnel C"
    },
    RoomLookup {
        mrea_id: 0xEF2F1440,
        internal_id: 0x921FFEDB,
        _area_index: 26,
        layer_count: 1,
        region_name: "Magmoor Caverns",
        room_name: "Transport to Phazon Mines West"
    },
    RoomLookup {
        mrea_id: 0xC1AC9233,
        internal_id: 0xC0201A31,
        _area_index: 27,
        layer_count: 1,
        region_name: "Magmoor Caverns",
        room_name: "Transport to Phendrana Drifts South"
    },
    RoomLookup {
        mrea_id: 0x7F56D921,
        internal_id: 0xEABBF3E8,
        _area_index: 28,
        layer_count: 1,
        region_name: "Magmoor Caverns",
        room_name: "Save Station Magmoor B"
    },
    RoomLookup {
        mrea_id: 0x93668996,
        internal_id: 0x2B878F78,
        _area_index: 0,
        layer_count: 1,
        region_name: "Impact Crater",
        room_name: "Crater Entry Point"
    },
    RoomLookup {
        mrea_id: 0x49CB2363,
        internal_id: 0xB4F7699E,
        _area_index: 1,
        layer_count: 1,
        region_name: "Impact Crater",
        room_name: "Crater Tunnel A"
    },
    RoomLookup {
        mrea_id: 0xBD946AC3,
        internal_id: 0x66F7C0E2,
        _area_index: 2,
        layer_count: 2,
        region_name: "Impact Crater",
        room_name: "Phazon Core"
    },
    RoomLookup {
        mrea_id: 0x4D446C3F,
        internal_id: 0x2C152A9A,
        _area_index: 3,
        layer_count: 1,
        region_name: "Impact Crater",
        room_name: "Crater Missile Station"
    },
    RoomLookup {
        mrea_id: 0x32D5A180,
        internal_id: 0xC8C538AF,
        _area_index: 4,
        layer_count: 2,
        region_name: "Impact Crater",
        room_name: "Crater Tunnel B"
    },
    RoomLookup {
        mrea_id: 0x67156A0D,
        internal_id: 0xE601A86A,
        _area_index: 5,
        layer_count: 1,
        region_name: "Impact Crater",
        room_name: "Phazon Infusion Chamber"
    },
    RoomLookup {
        mrea_id: 0xDADF06C3,
        internal_id: 0xE77A43A7,
        _area_index: 6,
        layer_count: 1,
        region_name: "Impact Crater",
        room_name: "Subchamber One"
    },
    RoomLookup {
        mrea_id: 0x0749DF46,
        internal_id: 0xE7ACE51C,
        _area_index: 7,
        layer_count: 3,
        region_name: "Impact Crater",
        room_name: "Subchamber Two"
    },
    RoomLookup {
        mrea_id: 0x7A3AD91E,
        internal_id: 0xE58D943D,
        _area_index: 8,
        layer_count: 1,
        region_name: "Impact Crater",
        room_name: "Subchamber Three"
    },
    RoomLookup {
        mrea_id: 0xA7AC009B,
        internal_id: 0xE55B3286,
        _area_index: 9,
        layer_count: 1,
        region_name: "Impact Crater",
        room_name: "Subchamber Four"
    },
    RoomLookup {
        mrea_id: 0x77714498,
        internal_id: 0xC5445690,
        _area_index: 10,
        layer_count: 1,
        region_name: "Impact Crater",
        room_name: "Subchamber Five"
    },
    RoomLookup {
        mrea_id: 0x1A666C55,
        internal_id: 0xE420D94B,
        _area_index: 11,
        layer_count: 1,
        region_name: "Impact Crater",
        room_name: "Metroid Prime Lair"
    },
    
    
];