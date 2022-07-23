use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use reader_writer::LazyArray;
use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct NewCameraShaker<'r>
{
    #[auto_struct(expect = 15)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub active: u8,
    pub flags: PropertyFlags<'r>,
    pub duration: f32,
    pub sfx_dist: f32,

    pub shakers: GenericArray<CameraShakerComponent<'r>, U3>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct PropertyFlags<'r>
{
    #[auto_struct(derive = flags.len() as u32)]
    pub count: u32,
    #[auto_struct(init = (count as usize, ()))]
    pub flags: LazyArray<'r, u8>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct CameraShakerComponent<'r>
{
    pub flags: PropertyFlags<'r>,
    pub am: CameraShakePoint<'r>,
    pub fm: CameraShakePoint<'r>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct CameraShakePoint<'r>
{
    pub flags: PropertyFlags<'r>,
    pub attack_time: f32,
    pub sustain_time: f32,
    pub duration: f32,
    pub magnitude: f32,
}

impl<'r> SclyPropertyData for NewCameraShaker<'r>
{
    const OBJECT_TYPE: u8 = 0x89;
}
