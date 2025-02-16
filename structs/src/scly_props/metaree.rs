use auto_struct_macros::auto_struct;
use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::SclyPropertyData;
use crate::scly_props::structs::*;
use crate::{impl_position, impl_rotation, impl_scale, impl_patterned_info};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Metaree<'r>
{
    #[auto_struct(expect = 12)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    
    pub patterned_info: PatternedInfo,
    pub actor_parameters: ActorParameters,
    pub damage_info: DamageInfo,

    pub dont_cares: GenericArray<f32, U7>,
}

impl<'r> SclyPropertyData for Metaree<'r>
{
    const OBJECT_TYPE: u8 = 0x37;

    impl_position!();
    impl_rotation!();
    impl_scale!();
    impl_patterned_info!();

    const SUPPORTS_DAMAGE_INFOS: bool = true;

    fn impl_get_damage_infos(&self) -> Vec<DamageInfo> {
        vec![
            self.patterned_info.contact_damage.clone(),
            self.damage_info.clone(),
        ]
    }

    fn impl_set_damage_infos(&mut self, x: Vec<DamageInfo>) {
        self.patterned_info.contact_damage = x[0].clone();
        self.damage_info = x[1].clone();
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
