use auto_struct_macros::auto_struct;
use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::res_id:: *;
use crate::scly_props::structs::*;
use crate::SclyPropertyData;
use crate::scly_props::structs::*;
use crate::{impl_position, impl_rotation, impl_scale, impl_patterned_info};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct FishCloud<'r>
{
    #[auto_struct(expect = 24)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub active: u8,
    pub cmdl: u32,
    pub animation_params: AnimationParameters,
    pub num_boids: u32,
    pub speed: f32, // TODO: speed

    // TODO: don't care
}

impl<'r> SclyPropertyData for FishCloud<'r>
{
    const OBJECT_TYPE: u8 = 0x4F;

    impl_position!();
    impl_rotation!();
    impl_scale!();
}
