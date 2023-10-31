use crate::{
    door_meta::DoorType,
};

/* Public */

pub fn set_position(obj: &mut structs::SclyObject, value: [f32; 3], relative: bool) {
    if !obj.property_data.supports_position() {
        return;
    }

    if relative {
        let x = obj.property_data.get_position();
        obj.property_data.set_position(
            [
                x[0] + value[0],
                x[1] + value[1],
                x[2] + value[2],
            ]
        );
    } else {
        obj.property_data.set_position(value);
    }
}

pub fn set_rotation(obj: &mut structs::SclyObject, value: [f32; 3], relative: bool) {
    if !obj.property_data.supports_rotation() {
        return;
    }

    if relative {
        let x = obj.property_data.get_rotation();
        obj.property_data.set_rotation(
            [
                x[0] + value[0],
                x[1] + value[1],
                x[2] + value[2],
            ]
        );
    } else {
        obj.property_data.set_rotation(value);
    }
}

pub fn set_scale(obj: &mut structs::SclyObject, value: [f32; 3], relative: bool) {
    if !obj.property_data.supports_scale() {
        return;
    }

    if relative {
        let x = obj.property_data.get_scale();
        obj.property_data.set_scale(
            [
                x[0] * value[0],
                x[1] * value[1],
                x[2] * value[2],
            ]
        );
    } else {
        obj.property_data.set_scale(value);
    }
}

pub fn set_patterned_speed(obj: &mut structs::SclyObject, value: f32, index: Option<usize>) {
    let mut data = get_patterned_infos(obj);
    for i in 0..data.len() {
        if should_skip(i, index) { continue; }
        let x = &mut data[i];
            x.speed *= value;
            x.turn_speed *= value;
            x.average_attack_time *= 1.0/value;
            // x.attack_time_variation *= 1.0/value;
            x.damage_wait_time *= 1.0/value;
    }
    set_patterned_infos(obj, data);
}

pub fn set_patterned_size(obj: &mut structs::SclyObject, value: f32, index: Option<usize>) {
    let mut data = get_patterned_infos(obj);
    for i in 0..data.len() {
        if should_skip(i, index) { continue; }
        let x = &mut data[i];
            x.mass *= value;
            x.half_extent *= value;
            x.height *= value;
            x.step_up_height *= value;
            x.min_attack_range *= value;
    }
    set_patterned_infos(obj, data);
}

pub fn set_detection_range(obj: &mut structs::SclyObject, value: f32, index: Option<usize>) {
    let mut data = get_patterned_infos(obj);
    for i in 0..data.len() {
        if should_skip(i, index) { continue; }
        let x = &mut data[i];
            x.detection_range *= value;
            x.detection_height_range *= value;
            x.detection_angle *= value;
            x.player_leash_radius *= value;
            // x.player_leash_time *= value;
            x.leash_radius *= value;
    }
    set_patterned_infos(obj, data);
}

pub fn set_attack_range(obj: &mut structs::SclyObject, value: f32, index: Option<usize>) {
    let mut data = get_patterned_infos(obj);
    for i in 0..data.len() {
        if should_skip(i, index) { continue; }
        let x = &mut data[i];
            x.max_attack_range *= value;
    }
    set_patterned_infos(obj, data);
}

pub fn set_vulnerability(obj: &mut structs::SclyObject, value: DoorType, index: Option<usize>) {
    let mut data = get_vulnerabilities(obj);
    for i in 0..data.len() {
        if should_skip(i, index) { continue; }
        data[i] = value.vulnerability();
    }
    set_vulnerabilities(obj, data);
}

pub fn set_health(obj: &mut structs::SclyObject, value: f32, index: Option<usize>) {
    let mut health_infos = get_health_infos(obj);
    for i in 0..health_infos.len() {
        if should_skip(i, index) { continue; }
        health_infos[i].health *= value;
    }
    set_health_infos(obj, health_infos);
}

pub fn set_damage(obj: &mut structs::SclyObject, value: f32) {
    let mut infos = get_patterned_infos(obj);
    for i in 0..infos.len() {
        let x = &mut infos[i];
            x.x_damage *= value;
    }
    set_patterned_infos(obj, infos);

    let mut damage_infos = get_damage_infos(obj);
    for i in 0..damage_infos.len() {
        damage_infos[i].damage *= value;
        damage_infos[i].knockback_power *= value;
    }
    set_damage_infos(obj, damage_infos);
}

/* Helpers */

fn should_skip(current: usize, check: Option<usize>) -> bool {
    match check {
        Some(x) => x != current,
        None => false,
    }
}

fn get_patterned_infos(obj: &mut structs::SclyObject) -> Vec<structs::scly_structs::PatternedInfo> {
    if !obj.property_data.supports_patterned_infos() {
        Vec::new()
    } else {
        obj.property_data.get_patterned_infos()
    }
}

fn set_patterned_infos(obj: &mut structs::SclyObject, value: Vec<structs::scly_structs::PatternedInfo>) {
    if value.len() > 0 {
        obj.property_data.set_patterned_infos(value);
    }
}

fn get_damage_infos(obj: &mut structs::SclyObject) -> Vec<structs::scly_structs::DamageInfo> {
    if !obj.property_data.supports_damage_infos() {
        Vec::new()
    } else {
        obj.property_data.get_damage_infos()
    }
}

fn set_damage_infos(obj: &mut structs::SclyObject, value: Vec<structs::scly_structs::DamageInfo>) {
    if value.len() > 0 {
        obj.property_data.set_damage_infos(value);
    }
}

fn get_vulnerabilities(obj: &mut structs::SclyObject) -> Vec<structs::scly_structs::DamageVulnerability> {
    if !obj.property_data.supports_vulnerabilities() {
        Vec::new()
    } else {
        obj.property_data.get_vulnerabilities()
    }
}

fn set_vulnerabilities(obj: &mut structs::SclyObject, value: Vec<structs::scly_structs::DamageVulnerability>) {
    if value.len() > 0 {
        obj.property_data.set_vulnerabilities(value);
    }
}

fn get_health_infos(obj: &mut structs::SclyObject) -> Vec<structs::scly_structs::HealthInfo> {
    if !obj.property_data.supports_health_infos() {
        Vec::new()
    } else {
        obj.property_data.get_health_infos()
    }
}

fn set_health_infos(obj: &mut structs::SclyObject, value: Vec<structs::scly_structs::HealthInfo>) {
    if value.len() > 0 {
        obj.property_data.set_health_infos(value);
    }
}
