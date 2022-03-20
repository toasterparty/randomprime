use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::{SclyPropertyData};
use crate::scly_props::structs::{ActorParameters, AnimationParameters, DamageVulnerability, DamageInfo, PatternedInfo};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Flaahgra<'r>
{
    #[auto_struct(expect = 23)]
    pub prop_count: u32,
 
    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    
    pub patterned_info: PatternedInfo,
    pub actor_params1: ActorParameters,
    pub dont_care: GenericArray<f32, U4>,
    pub damage_vulnerability: DamageVulnerability,
    pub dont_care0: u32,
    pub damage_info1: DamageInfo,
    pub dont_care1: u32,
    pub damage_info2: DamageInfo,
    pub dont_care2: u32,
    pub damage_info3: DamageInfo,
    pub actor_params2: ActorParameters,
    pub dont_care3: f32,
    pub dont_care4: f32,
    pub dont_care5: f32,
    pub anim_params: AnimationParameters,
    pub dont_care6: u32,
}

impl<'r> SclyPropertyData for Flaahgra<'r>
{
    const OBJECT_TYPE: u8 = 0x4D;
}
