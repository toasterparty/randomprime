use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::res_id:: *;
use crate::{SclyPropertyData};
use crate::scly_props::structs::{ActorParameters, DamageInfo, PatternedInfo, RidleyStruct1, RidleyStruct2};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct RidleyV1<'r>
{
    #[auto_struct(expect = 48)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub patterned_info: PatternedInfo,
    pub actor_params: ActorParameters,

    pub models: GenericArray<ResId<CMDL>, U12>,
    pub particle: ResId<PART>,

    pub unknown0: f32,
    pub unknown1: f32,
    pub unknown2: f32,
    pub unknown3: f32,

    pub wpsc0: u32, // missing ResId<WPSC>
    pub damage_info0: DamageInfo,
    pub ridley_struct_0: RidleyStruct1,
    pub sound0: u32,
    pub wpsc1: u32, // missing ResId<WPSC>
    pub damage_info1: DamageInfo,
    pub ridley_struct_1: RidleyStruct2,
    pub wpsc2: u32, // missing ResId<WPSC>
    pub damage_info2: DamageInfo,
    pub ridley_struct_2: RidleyStruct2,
    pub sound1: u32, // missing ResId<WPSC>
    pub damage_info3: DamageInfo,
    pub ridley_struct_3: RidleyStruct2,
    pub unknown4: f32,
    pub unknown5: f32,
    pub damage_info4: DamageInfo,
    pub unknown6: f32,
    pub damage_info5: DamageInfo,
    pub unknown7: f32,
    pub damage_info6: DamageInfo,
    pub unknown8: f32,
    pub elsc: u32,
    pub unknown9: f32,
    pub sound2: u32,
    pub damage_info7: DamageInfo,
}

impl<'r> SclyPropertyData for RidleyV1<'r>
{
    const OBJECT_TYPE: u8 = 0x7B;
}