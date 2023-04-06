use auto_struct_macros::auto_struct;

use crate::SclyPropertyData;
use reader_writer::generic_array::GenericArray;
use reader_writer::typenum::*;
use reader_writer::CStr;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct RidleyV2<'r> {
    #[auto_struct(expect = 40)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub dont_care: GenericArray<u8, U901>, // I don't care about PAL, sorry
}

use crate::{impl_position, impl_rotation, impl_scale};
impl<'r> SclyPropertyData for RidleyV2<'r> {
    const OBJECT_TYPE: u8 = 0x7B;
    impl_position!();
    impl_rotation!();
    impl_scale!();
}
