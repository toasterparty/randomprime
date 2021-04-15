use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::{ResId, SclyPropertyData};
use crate::res_id:: *;
use crate::scly_props::structs::{ActorParameters, DamageInfo, PatternedInfo};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct MetroidPrimeStage2<'r>
{
    #[auto_struct(expect = 11)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub patterned_info: PatternedInfo,
    pub actor_parameters: ActorParameters,
    pub part1: ResId<PART>,
    pub damage_info: DamageInfo,
    pub elsc: ResId<ELSC>,
    pub unknown: u32,
    pub part2: ResId<PART>,
}

impl<'r> SclyPropertyData for MetroidPrimeStage2<'r>
{
    const OBJECT_TYPE: u8 = 0x83;
}
