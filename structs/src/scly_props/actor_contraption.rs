use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::{SclyPropertyData};
use crate::scly_props::structs::{DamageVulnerability, AnimationParameters,ActorParameters, HealthInfo, DamageInfo};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct ActorContraption<'r>
{
    #[auto_struct(expect = 15)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub dont_care0: GenericArray<f32, U8>,
    pub health_info: HealthInfo,
    pub damage_vulnerability: DamageVulnerability,
    pub animation_params: AnimationParameters,
    pub actor_params: ActorParameters,
    pub dont_care1: u32,
    pub damage_info: DamageInfo,
    pub dont_care2: u8,
}

impl<'r> SclyPropertyData for ActorContraption<'r>
{
    const OBJECT_TYPE: u8 = 0x6E;
}
