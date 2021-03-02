use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Sound<'r>
{
    #[auto_struct(expect = 20)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub sound_id: u16,
    
    pub unknown1: u8,
    pub unknown2: GenericArray<f32, U3>,
    pub unknown5: u32,
    pub unknown6: u32,
    pub unknown7: u32,
    pub unknown8: u32,
    pub unknown9: u8,
    pub unknown10: u8,
    pub unknown11: u8,
    pub unknown12: u8,
    pub unknown13: u8,
    pub unknown14: u8,
    pub unknown15: u8,
    pub unknown16: u32,
}

impl<'r> SclyPropertyData for Sound<'r>
{
    const OBJECT_TYPE: u8 = 0x9;
}
