use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::SclyPropertyData;
use crate::scly_structs::*;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct RidleyV1<'r>
{
    #[auto_struct(expect = 48)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub patterned_info: PatternedInfo,
    pub actor_params: ActorParameters,
    pub dont_care1: GenericArray<u32, U18>,
    pub damage_info1: DamageInfo,
    pub ridley_struct1_1: RidleyStruct1,
    pub dont_care2: GenericArray<u32, U2>,
    pub damage_info2: DamageInfo,
    pub ridley_struct2_1: RidleyStruct2,
    pub dont_care3: u32,
    pub damage_info3: DamageInfo,
    pub ridley_struct2_2: RidleyStruct2,
    pub dont_care4: u32,
    pub damage_info4: DamageInfo,
    pub ridley_struct2_3: RidleyStruct2,
    pub dont_care5: f32,
    pub dont_care6: f32,
    pub damage_info5: DamageInfo,
    pub dont_care7: f32,
    pub damage_info6: DamageInfo,
    pub dont_care8: f32,
    pub damage_info7: DamageInfo,
    pub dont_care9: f32,
    pub dont_care10: f32,
    pub dont_care11: f32,
    pub dont_care12: f32,
    pub damage_info8: DamageInfo,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct RidleyStruct1
{
    // #[auto_struct(derive = 0)]
    // prop_count: u32,

    pub dont_care: GenericArray<u32, U15>,
    pub color1: GenericArray<f32, U4>,
    pub color2: GenericArray<f32, U4>,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct RidleyStruct2
{
    // #[auto_struct(derive = 0)]
    // prop_count: u32,

    pub dont_care: GenericArray<u32, U8>,
    pub unknown: u8,
}

use crate::{impl_position, impl_rotation, impl_scale, impl_patterned_info};
impl<'r> SclyPropertyData for RidleyV1<'r>
{
    const OBJECT_TYPE: u8 = 0x7B;
    impl_position!();
    impl_rotation!();
    impl_scale!();
    impl_patterned_info!();

    const SUPPORTS_DAMAGE_INFOS: bool = true;

    fn impl_get_damage_infos(&self) -> Vec<DamageInfo> {
        vec![
            self.patterned_info.contact_damage.clone(),
            self.damage_info1.clone(),
            self.damage_info2.clone(),
            self.damage_info3.clone(),
            self.damage_info4.clone(),
            self.damage_info5.clone(),
            self.damage_info6.clone(),
            self.damage_info7.clone(),
            self.damage_info8.clone(),
        ]
    }

    fn impl_set_damage_infos(&mut self, x: Vec<DamageInfo>) {
        self.patterned_info.contact_damage = x[0].clone();
        self.damage_info1 = x[1].clone();
        self.damage_info2 = x[2].clone();
        self.damage_info3 = x[3].clone();
        self.damage_info4 = x[4].clone();
        self.damage_info5 = x[5].clone();
        self.damage_info6 = x[6].clone();
        self.damage_info7 = x[7].clone();
        self.damage_info8 = x[8].clone();
    }

    const SUPPORTS_VULNERABILITIES: bool = true;

    fn impl_get_vulnerabilities(&self) -> Vec<DamageVulnerability> {
        vec![
            self.patterned_info.damage_vulnerability.clone(),
        ]
    }

    fn impl_set_vulnerabilities(&mut self, x: Vec<DamageVulnerability>) {
        self.patterned_info.damage_vulnerability = x[0].clone();
    }

    const SUPPORTS_HEALTH_INFOS: bool = true;

    fn impl_get_health_infos(&self) -> Vec<HealthInfo> {
        vec![
            self.patterned_info.health_info.clone()
        ]
    }

    fn impl_set_health_infos(&mut self, x: Vec<HealthInfo>) {
        self.patterned_info.health_info = x[0].clone();
    }
}
