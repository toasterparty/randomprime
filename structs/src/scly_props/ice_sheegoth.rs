use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::{SclyPropertyData};
use crate::scly_props::structs::{ActorParameters, DamageVulnerability, DamageInfo, PatternedInfo};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct IceSheegoth<'r>
{
    #[auto_struct(expect = 37)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub patterned_info: PatternedInfo,
    pub actor_params: ActorParameters,
    pub dont_care0: GenericArray<u32, U6>,
    pub damage_vulnerabilities: GenericArray<DamageVulnerability, U3>,
    pub dont_care1: u32,
    pub damage_info1: DamageInfo,
    pub dont_care2: GenericArray<f32, U4>,
    pub damage_info2: DamageInfo,
    pub dont_care3: GenericArray<f32, U7>,
    pub damage_info3: DamageInfo,
    pub dont_care4: GenericArray<f32, U7>,
    pub dont_care5: u8,
    pub dont_care6: u8,
}

impl<'r> SclyPropertyData for IceSheegoth<'r>
{
    const OBJECT_TYPE: u8 = 0x4B;
}
