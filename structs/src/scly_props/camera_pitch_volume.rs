use auto_struct_macros::auto_struct;
use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::SclyPropertyData;
use crate::{impl_position, impl_rotation, impl_scale};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct CameraPitchVolume<'r>
{
    #[auto_struct(expect = 8)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>, // "volume"

    pub active: u8,
    pub up_pitch: f32,
    pub down_pitch: f32,
    pub actual_scale: f32, // TODO: scale
}

impl<'r> SclyPropertyData for CameraPitchVolume<'r>
{
    const OBJECT_TYPE: u8 = 0x69;

    impl_position!();
    impl_rotation!();
    impl_scale!();
}
