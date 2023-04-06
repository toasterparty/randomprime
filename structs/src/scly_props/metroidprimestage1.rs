use auto_struct_macros::auto_struct;

use crate::scly_props::structs::*;
use crate::SclyPropertyData;
use reader_writer::generic_array::GenericArray;
use reader_writer::typenum::*;
use reader_writer::CStr;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct MetroidPrimeStage1<'r> {
    #[auto_struct(expect = 22)]
    prop_count: u32,

    #[auto_struct(expect = 3)]
    pub version: u32,
    pub name: CStr<'r>,
    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub unknown2: u8,
    pub unknown3: f32,
    pub unknown4: f32,
    pub unknown5: f32,
    pub unknown6: u32,
    pub unknown7: u8,
    pub unknown8: u32,
    pub health_info1: HealthInfo,
    pub health_info2: HealthInfo,
    pub unknown9: u32,
    pub exo_structs: GenericArray<ExoStructA, U4>,
    pub unknown10: u32,
    pub unknown11: u32,
    pub exo_struct_b: ExoStructB,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct ExoStructA {
    #[auto_struct(expect = 14)]
    prop_count: u32,
    pub dont_care: GenericArray<f32, U14>,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct ExoStructB {
    #[auto_struct(expect = 29)]
    prop_count: u32,
    // pub unknown: u32,
    pub patterned_info: PatternedInfo,
    pub actor_params: ActorParameters,
    pub unknown2: u32,
    pub shake_datas: GenericArray<CameraShakeData, U3>,
    pub exo_struct_ba: ExoStructBA,
    pub exo_struct_bb: GenericArray<ExoStructBB, U4>,
    pub wpsc1: u32,
    pub damage_info2: DamageInfo, // hecl calls it damage_info2
    pub camera_shake_data1: CameraShakeData,
    pub wpsc2: u32,
    pub damage_info3: DamageInfo, // hecl calls it damage_info3
    pub camera_shake_data2: CameraShakeData,
    pub projectile_info: ExoProjectileInfo,
    pub damage_info4: DamageInfo, // hecl calls it damage_info5
    pub camera_shake_data3: CameraShakeData,
    pub dont_care: GenericArray<u32, U4>,
    pub exo_struct_bc: GenericArray<ExoStructBC, U4>,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct CameraShakeData { // PrimeStruct2
    // #[auto_struct(expect = 8)]
    // prop_count: u32,
    pub use_sfx: u8,
    pub duration: f32,
    pub sfx_dist: f32,
    pub components: GenericArray<CameraShakerComponent, U3>,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct CameraShakerComponent {
    pub use_modulation: u8,
    pub am: CameraShakePoint,
    pub fm: CameraShakePoint,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct CameraShakePoint {
    pub dont_care: GenericArray<f32, U4>,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct ExoStructBA {
    #[auto_struct(expect = 9)]
    prop_count: u32,
    pub parts: GenericArray<u32, U3>,
    pub damage_info: DamageInfo,
    pub unknown4: f32,
    pub unknown5: f32,
    pub txtr: u32,
    pub unknown6: u32,
    pub sound: u32,
    pub part4: u32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct ExoStructBB {
    pub beam_info: BeamInfo,
    pub wpsc: u32,
    pub damage_info1: DamageInfo,
    pub exo_struct_bba: ExoStructBBA,
    pub unknown14: f32,
    pub damage_info2: DamageInfo,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct ExoStructBBA {
    #[auto_struct(expect = 8)]
    prop_count: u32,
    pub dont_care: GenericArray<u32, U8>,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct ExoStructBC {
    #[auto_struct(expect = 4)]
    prop_count: u32,
    pub vulnerability: DamageVulnerability,
    pub beam_color: GenericArray<f32, U4>,
    pub dont_care: GenericArray<u32, U2>,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct ExoProjectileInfo {
    #[auto_struct(expect = 10)]
    prop_count: u32,
    pub part: u32,
    pub damage_info: DamageInfo,
    pub dont_cares1: GenericArray<u32, U4>,
    pub dont_cares2: GenericArray<u8, U4>,
}

use crate::{impl_patterned_info, impl_position, impl_rotation, impl_scale};
impl<'r> SclyPropertyData for MetroidPrimeStage1<'r> {
    const OBJECT_TYPE: u8 = 0x84;

    impl_position!();
    impl_rotation!();
    impl_scale!();
}
