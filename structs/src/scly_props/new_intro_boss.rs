use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::res_id:: *;
use crate::{SclyPropertyData};
use crate::scly_props::structs::{ActorParameters, DamageInfo, PatternedInfo};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct NewIntroBoss<'r>
{
    #[auto_struct(expect = 13)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub patterned_info: PatternedInfo,
    pub actor_params: ActorParameters,
    pub min_turn_angle: f32,
    pub weapon_desc: f32,
    pub damage_info: DamageInfo,

    pub particles: GenericArray<ResId<PART>, U2>,
    pub textures: GenericArray<ResId<TXTR>, U2>,
}

impl<'r> SclyPropertyData for NewIntroBoss<'r>
{
    const OBJECT_TYPE: u8 = 0x0E;
}
