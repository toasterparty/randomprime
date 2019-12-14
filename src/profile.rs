use serde_derive::{Serialize,Deserialize};
use std::{
    collections::HashMap,
    fs
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Profile {
    pub mpdrp_version:u8,
    pub seed:u32,
    pub weights:HashMap<String,[i8;4]>,
    pub skip_frigate:bool
}

impl Profile {
    pub fn from_json(json_path:&str) -> Profile {
        let json:&str = &fs::read_to_string(json_path)
                .map_err(|e| format!("Could not read JSON file: {}",e)).unwrap();
        let profile:Profile = serde_json::from_str(json)
                .map_err(|e| format!("Could not parse JSON file: {}",e)).unwrap();
        profile
    }
    pub fn new() -> Profile {
        let mut profile:Profile = Profile {
            mpdrp_version:1,
            seed:0,
            weights: HashMap::new(),
            skip_frigate:true
        };
        profile.weights.insert(String::from("Tallon Overworld"),[25,25,25,25]);
        profile.weights.insert(String::from("Chozo Ruins"),[25,25,25,25]);
        profile.weights.insert(String::from("Magmoor Caverns"),[25,25,25,25]);
        profile.weights.insert(String::from("Phendrana Drifts"),[25,25,25,25]);
        profile.weights.insert(String::from("Phazon Mines"),[25,25,25,25]);
        profile
    }
}
