use auto_struct_macros::auto_struct;
use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::SclyPropertyData;
use crate::scly_props::structs::*;
use crate::{impl_position, impl_rotation, impl_scale, impl_patterned_info_with_auxillary};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Geemer<'r>
{
    #[auto_struct(expect = 16)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub patterned_info: PatternedInfo,
    pub actor_params: ActorParameters,

    pub dont_cares: GenericArray<u8, U40>,
}

impl<'r> SclyPropertyData for Geemer<'r>
{
    const OBJECT_TYPE: u8 = 0x70;

    impl_position!();
    impl_rotation!();
    impl_scale!();
    impl_patterned_info_with_auxillary!();
}
