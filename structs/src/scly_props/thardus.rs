use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::{SclyPropertyData};
use crate::scly_props::structs::{ActorParameters, PatternedInfo, DamageInfo, DamageVulnerability, HealthInfo};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Thardus<'r>
{
    #[auto_struct(expect = 44)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub patterned_info: PatternedInfo,
    pub actor_parameters: ActorParameters,
    pub unknown1: u8,
    pub unknown2: u8,
    pub asset_ids: GenericArray<u32, U24>,
    pub values: GenericArray<f32, U6>,
    pub asset_ids2: GenericArray<u32, U6>,
}

use crate::{impl_position, impl_rotation, impl_scale, impl_patterned_info};
impl<'r> SclyPropertyData for Thardus<'r>
{
    const OBJECT_TYPE: u8 = 0x58;
    impl_position!();
    impl_rotation!();
    impl_scale!();
    impl_patterned_info!();

    const SUPPORTS_DAMAGE_INFOS: bool = true;

    fn impl_get_damage_infos(&self) -> Vec<DamageInfo> {
        vec![
            self.patterned_info.contact_damage.clone(),
        ]
    }

    fn impl_set_damage_infos(&mut self, x: Vec<DamageInfo>) {
        self.patterned_info.contact_damage = x[0].clone();
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
