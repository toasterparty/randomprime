use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::{SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct OmegaPirate<'r>
{
    #[auto_struct(expect = 45)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub dont_care: GenericArray<u8, U757>,
}

impl<'r> SclyPropertyData for OmegaPirate<'r>
{
    const OBJECT_TYPE: u8 = 0x86;
}
