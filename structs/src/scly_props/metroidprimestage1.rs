use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::{SclyPropertyData};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct MetroidPrimeStage1<'r>
{
    #[auto_struct(expect = 22)]
    pub prop_count: u32,

    pub version: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub dont_care0: GenericArray<u8, U900>,
    pub dont_care1: GenericArray<u8, U900>,
    pub dont_care2: GenericArray<u8, U900>,
    pub dont_care3: GenericArray<u8, U172>,
}

impl<'r> SclyPropertyData for MetroidPrimeStage1<'r>
{
    const OBJECT_TYPE: u8 = 0x84;
}
