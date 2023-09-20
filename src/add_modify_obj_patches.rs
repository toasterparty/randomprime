use std::{
    collections::HashMap,
    convert::TryInto,
};

use crate::{
    patches::{
        id_in_use,
        string_to_cstr,
        WaterType,
    },
    patcher::PatcherState,
    mlvl_wrapper,
    patch_config::{
        WaterConfig,
        GenericTexture,
        RelayConfig,
        TimerConfig,
        ActorKeyFrameConfig,
        SpawnPointConfig,
        TriggerConfig,
        DamageType,
        LockOnPoint,
        SpecialFunctionConfig,
        ActorRotateConfig,
        StreamedAudioConfig,
    },
    pickup_meta::PickupType,
    door_meta::DoorType,
};

use resource_info_table::resource_info;

use reader_writer::{
    CStrConversionExtension,
    FourCC,
};

use structs::{res_id, ResId, SclyPropertyData};

macro_rules! add_edit_obj_helper {
    ($area:expr, $config:expr, $object_type:ident, $new_property_data:ident, $update_property_data:ident) => {
        let area = $area;
        let config = $config;
        let mrea_id = area.mlvl_area.mrea.to_u32().clone();

        if let Some(id) = config.id {
            let scly = area.mrea().scly_section_mut();
            let layers = &mut scly.layers.as_mut_vec(); 
    
            // try to find existing object   
            let obj = {
                let mut obj = None;
                for layer in layers.iter_mut() {
                    obj = layer.objects
                        .as_mut_vec()
                        .iter_mut()
                        .find(|obj| obj.instance_id & 0x00FFFFFF == id & 0x00FFFFFF);
    
                    if obj.is_some() {
                        break;
                    }
                }
    
                obj
            };
    
            if let Some(obj) = obj {
                // edit existing object
                if obj.property_data.object_type() != structs::$object_type::OBJECT_TYPE {
                    panic!("Failed to edit existing object 0x{:X} in room 0x{:X}: Unexpected object type 0x{:X} (expected 0x{:X})", id, mrea_id, obj.property_data.object_type(), structs::$object_type::OBJECT_TYPE);
                }

                $update_property_data!(config, obj);
    
                return Ok(());
            }
        }
    
        // add new object
        let id = config.id.unwrap_or(area.new_object_id_from_layer_id(0));
        let scly = area.mrea().scly_section_mut();
        let layers = &mut scly.layers.as_mut_vec();
        let objects = layers[0].objects.as_mut_vec();
        let property_data = $new_property_data!(config);

        objects.push(
            structs::SclyObject {
                instance_id: id,
                property_data: property_data.into(),
                connections: vec![].into(),
            }
        );

        return Ok(());
    };
}

pub fn patch_add_streamed_audio<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: StreamedAudioConfig,
) -> Result<(), String>
{
    macro_rules! streamed_audio_new {
        ($config:expr) => {
            structs::StreamedAudio {
                name: b"mystreamedaudio\0".as_cstr(),
                active: $config.active.unwrap_or(true) as u8,
                audio_file_name: string_to_cstr($config.audio_file_name),
                no_stop_on_deactivate: $config.no_stop_on_deactivate.unwrap_or(true) as u8,
                fade_in_time: $config.fade_in_time.unwrap_or(0.1),
                fade_out_time: $config.fade_out_time.unwrap_or(1.5),
                volume: $config.volume.unwrap_or(100),
                oneshot: $config.oneshot.unwrap_or(0),
                is_music: $config.is_music as u8,
            }
        };
    }
    
    macro_rules! streamed_audio_update {
        ($config:expr, $obj:expr) => {
            let config = $config;
            let obj = $obj;
    
            let property_data = obj.property_data.as_streamed_audio_mut().unwrap();
    
            property_data.audio_file_name = string_to_cstr(config.audio_file_name);
            property_data.is_music = config.is_music as u8;
    
            if let Some(active                ) = config.active                { property_data.active                = active                as u8 }
            if let Some(no_stop_on_deactivate ) = config.no_stop_on_deactivate { property_data.no_stop_on_deactivate = no_stop_on_deactivate as u8 }
            if let Some(fade_in_time          ) = config.fade_in_time          { property_data.fade_in_time          = fade_in_time                }
            if let Some(fade_out_time         ) = config.fade_out_time         { property_data.fade_out_time         = fade_out_time               }
            if let Some(volume                ) = config.volume                { property_data.volume                = volume                      }
            if let Some(oneshot               ) = config.oneshot               { property_data.oneshot               = oneshot                     }
        };
    }

    add_edit_obj_helper!(area, config, StreamedAudio, streamed_audio_new, streamed_audio_update);
}

pub fn patch_add_liquid<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    water_config: &WaterConfig,
    resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
)
-> Result<(), String>
{
    let water_type = WaterType::from_str(water_config.liquid_type.as_str());

    // add dependencies to area //
    let deps = water_type.dependencies();
    let deps_iter = deps.iter()
        .map(|&(file_id, fourcc)| structs::Dependency {
                asset_id: file_id,
                asset_type: fourcc,
        });

    area.add_dependencies(resources, 0, deps_iter);

    let mut water_obj = water_type.to_obj();
    let water = water_obj.property_data.as_water_mut().unwrap();
    water.position[0] = water_config.position[0];
    water.position[1] = water_config.position[1];
    water.position[2] = water_config.position[2];
    water.scale[0]    = water_config.scale[0];
    water.scale[1]    = water_config.scale[1];
    water.scale[2]    = water_config.scale[2];

    // add water to area //
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    layer.objects.as_mut_vec().push(water_obj);

    Ok(())
}

pub fn patch_add_actor_key_frame<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    actor_key_frame_config: ActorKeyFrameConfig,
)
    -> Result<(), String>
{
    if id_in_use(area, actor_key_frame_config.id) {
        panic!("id 0x{:X} already in use", actor_key_frame_config.id);
    }

    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    layer.objects.as_mut_vec().push(
        structs::SclyObject {
            instance_id: actor_key_frame_config.id,
            property_data: structs::ActorKeyFrame {
                name: b"my keyframe\0".as_cstr(),
                active: actor_key_frame_config.active.unwrap_or(true) as u8,
                animation_id: actor_key_frame_config.animation_id,
                looping: actor_key_frame_config.looping as u8,
                lifetime: actor_key_frame_config.lifetime,
                fade_out: actor_key_frame_config.fade_out,
                total_playback: actor_key_frame_config.total_playback,
            }.into(),
            connections: vec![].into(),
        },
    );

    Ok(())
}

pub fn patch_add_timer<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    timer_config: TimerConfig,
)
    -> Result<(), String>
{
    if id_in_use(area, timer_config.id) {
        panic!("id 0x{:X} already in use", timer_config.id);
    }

    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    layer.objects.as_mut_vec().push(
        structs::SclyObject {
            instance_id: timer_config.id,
            property_data: structs::Timer {
                name: b"my timer\0".as_cstr(),
                start_time: timer_config.time,
                max_random_add: timer_config.max_random_add.unwrap_or(0.0),
                looping: timer_config.looping.unwrap_or(false) as u8,
                start_immediately: timer_config.start_immediately.unwrap_or(false) as u8,
                active: timer_config.active.unwrap_or(true) as u8,
            }.into(),
            connections: vec![].into(),
        },
    );

    Ok(())
}

pub fn patch_add_relay<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    relay_config: RelayConfig,
)
    -> Result<(), String>
{
    if id_in_use(area, relay_config.id) {
        panic!("id 0x{:X} already in use", relay_config.id);
    }

    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    layer.objects.as_mut_vec().push(
        structs::SclyObject {
            instance_id: relay_config.id,
            property_data: structs::Relay {
                    name: b"my relay\0".as_cstr(),
                    active: relay_config.active.unwrap_or(true) as u8,
                }.into(),
            connections: vec![].into(),
        },
    );

    Ok(())
}

pub fn patch_add_spawn_point<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    spawn_point_config: SpawnPointConfig,
)
    -> Result<(), String>
{
    if id_in_use(area, spawn_point_config.id) {
        panic!("id 0x{:X} already in use", spawn_point_config.id);
    }

    let rotation = spawn_point_config.rotation
        .as_ref()
        .unwrap_or(&[0.0, 0.0, 0.0]);
    let rotation = *rotation;

    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    let mut spawn_point = structs::SclyObject {
            instance_id: spawn_point_config.id,
            property_data: structs::SpawnPoint {
                    name: b"my spawnpoint\0".as_cstr(),
                    position: spawn_point_config.position.into(),
                    rotation: rotation.into(),
                    power: 0,
                    ice: 0,
                    wave: 0,
                    plasma: 0,
                    missiles: 0,
                    scan_visor: 0,
                    bombs: 0,
                    power_bombs: 0,
                    flamethrower: 0,
                    thermal_visor: 0,
                    charge: 0,
                    super_missile: 0,
                    grapple: 0,
                    xray: 0,
                    ice_spreader: 0,
                    space_jump: 0,
                    morph_ball: 0,
                    combat_visor: 0,
                    boost_ball: 0,
                    spider_ball: 0,
                    power_suit: 0,
                    gravity_suit: 0,
                    varia_suit: 0,
                    phazon_suit: 0,
                    energy_tanks: 0,
                    unknown0: 0,
                    health_refill: 0,
                    unknown1: 0,
                    wavebuster: 0,
                    default_spawn: spawn_point_config.default_spawn.unwrap_or(false) as u8,
                    active: spawn_point_config.active.unwrap_or(true) as u8,
                    morphed: spawn_point_config.morphed.unwrap_or(false) as u8,
                }.into(),
            connections: vec![].into(),
        };

    if spawn_point_config.items.is_some() {
        let items = spawn_point_config.items.unwrap();
        items.update_spawn_point(spawn_point.property_data.as_spawn_point_mut().unwrap());
    }

    layer.objects.as_mut_vec().push(spawn_point);

    Ok(())
}

pub fn patch_add_trigger<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    trigger_config: TriggerConfig,
)
    -> Result<(), String>
{
    let instance_id = {
        if trigger_config.id.is_some() {
            let id = trigger_config.id.unwrap();
            if id_in_use(area, id) {
                panic!("id 0x{:X} already in use", id);
            }

            id
        } else {
            area.new_object_id_from_layer_id(0)
        }
    };

    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    layer.objects.as_mut_vec().push(
        structs::SclyObject {
            instance_id,
            property_data: structs::Trigger {
                name: b"Start Sequence Trigger\0".as_cstr(),
                position: trigger_config.position.into(),
                scale: trigger_config.scale.into(),
                damage_info: structs::scly_structs::DamageInfo {
                    weapon_type: trigger_config.damage_type.unwrap_or(DamageType::Power) as u32,
                    damage: trigger_config.damage_amount.unwrap_or(0.0),
                    radius: 0.0,
                    knockback_power: 0.0
                },
                force: trigger_config.force.unwrap_or([0.0, 0.0, 0.0]).into(),
                flags: trigger_config.flags.unwrap_or(1),
                active: trigger_config.active.unwrap_or(true) as u8,
                deactivate_on_enter: trigger_config.deactivate_on_enter.unwrap_or(false) as u8,
                deactivate_on_exit: trigger_config.deactivate_on_exit.unwrap_or(false) as u8,
            }.into(),
            connections: vec![].into(),
        }
    );

    Ok(())
}

pub fn patch_add_special_fn<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: SpecialFunctionConfig,
)
    -> Result<(), String>
{
    let instance_id = {
        if config.id.is_some() {
            let id = config.id.unwrap();
            if id_in_use(area, id) {
                panic!("id 0x{:X} already in use", id);
            }

            id
        } else {
            area.new_object_id_from_layer_id(0)
        }
    };

    let unknown0 = config.unknown1.unwrap_or("".to_string());
    let unknown0 = string_to_cstr(unknown0);

    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];

    layer.objects.as_mut_vec().push(
        structs::SclyObject {
            instance_id,
            property_data: structs::SpecialFunction {
                name: b"myspecialfun\0".as_cstr(),
                position: config.position.unwrap_or_default().into(),
                rotation: config.rotation.unwrap_or_default().into(),
                type_: config.type_ as u32,
                unknown0: unknown0,
                unknown1: config.unknown2.unwrap_or_default(),
                unknown2: config.unknown3.unwrap_or_default(),
                unknown3: config.unknown4.unwrap_or_default(),
                layer_change_room_id: config.layer_change_room_id.unwrap_or(0xFFFFFFFF),
                layer_change_layer_id: config.layer_change_layer_id.unwrap_or(0xFFFFFFFF),
                item_id: config.item_id.unwrap_or(PickupType::PowerBeam) as u32,
                unknown4: config.active.unwrap_or(true) as u8, // active
                unknown5: config.unknown6.unwrap_or_default(),
                unknown6: config.spinner1.unwrap_or(0xFFFFFFFF),
                unknown7: config.spinner2.unwrap_or(0xFFFFFFFF),
                unknown8: config.spinner3.unwrap_or(0xFFFFFFFF),
            }.into(),
            connections: vec![].into(),
        }
    );

    Ok(())
}

pub fn patch_add_actor_rotate_fn<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: ActorRotateConfig,
)
    -> Result<(), String>
{
    let instance_id = {
        if config.id.is_some() {
            let id = config.id.unwrap();
            if id_in_use(area, id) {
                panic!("id 0x{:X} already in use", id);
            }

            id
        } else {
            area.new_object_id_from_layer_id(0)
        }
    };

    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];

    layer.objects.as_mut_vec().push(
        structs::SclyObject {
            instance_id,
            property_data: structs::ActorRotate {
                name: b"my actor rotate\0".as_cstr(),
                rotation: config.rotation.into(),
                time_scale: config.time_scale,
                update_actors: config.update_actors as u8,
                update_on_creation: config.update_on_creation as u8,
                update_active: config.update_active as u8,
            }.into(),
            connections: vec![].into(),
        }
    );

    Ok(())
}

pub fn patch_add_platform<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    position: [f32;3],
    rotation: [f32;3],
    alt_platform: bool,
) -> Result<(), String>
{
    let (deps, cmdl, dcln) = {
        if alt_platform {
            (
                vec![
                    (0xDCDFD386, b"CMDL"),
                    (0x6D412D11, b"DCLN"),
                    (0xEED972E7, b"TXTR"),
                    (0xF1478D6A, b"TXTR"),
                    (0xF89D34EF, b"TXTR"),
                ],
                ResId::<res_id::CMDL>::new(0xDCDFD386),
                ResId::<res_id::DCLN>::new(0x6D412D11),
            )
        } else {
            (
                vec![
                    (0x48DF38A3, b"CMDL"),
                    (0xB2D50628, b"DCLN"),
                    (0x19C17D5C, b"TXTR"),
                    (0x0259F5F6, b"TXTR"),
                    (0x71190250, b"TXTR"),
                    (0xD0BA0FA8, b"TXTR"),
                    (0xF1478D6A, b"TXTR"),
                ],
                ResId::<res_id::CMDL>::new(0x48DF38A3),
                ResId::<res_id::DCLN>::new(0xB2D50628),
            )
        }
    };

    let deps_iter = deps.iter()
        .map(|&(file_id, fourcc)| structs::Dependency {
            asset_id: file_id,
            asset_type: FourCC::from_bytes(fourcc),
        }
    );
    area.add_dependencies(game_resources,0,deps_iter);

    let platform_id = area.new_object_id_from_layer_name("Default");

    let layers = area.mrea().scly_section_mut().layers.as_mut_vec();
    layers[0].objects.as_mut_vec().push(
        structs::SclyObject {
            instance_id: platform_id,
            property_data: structs::Platform {
                name: b"myplatform\0".as_cstr(),

                position: position.into(),
                rotation: rotation.into(),
                scale: [1.0, 1.0, 1.0].into(),
                extent: [0.0, 0.0, 0.0].into(),
                scan_offset: [0.0, 0.0, 0.0].into(),

                cmdl: cmdl,
                ancs: structs::scly_structs::AncsProp {
                    file_id: ResId::invalid(),
                    node_index: 0,
                    default_animation: 0xFFFFFFFF,
                },
                actor_params: structs::scly_structs::ActorParameters {
                    light_params: structs::scly_structs::LightParameters {
                        unknown0: 1,
                        unknown1: 1.0,
                        shadow_tessellation: 0,
                        unknown2: 1.0,
                        unknown3: 20.0,
                        color: [1.0, 1.0, 1.0, 1.0].into(),
                        unknown4: 1,
                        world_lighting: 1,
                        light_recalculation: 1,
                        unknown5: [0.0, 0.0, 0.0].into(),
                        unknown6: 4,
                        unknown7: 4,
                        unknown8: 0,
                        light_layer_id: 0
                    },
                    scan_params: structs::scly_structs::ScannableParameters {
                        scan: ResId::invalid(), // None
                    },
                    xray_cmdl: ResId::invalid(), // None
                    xray_cskr: ResId::invalid(), // None
                    thermal_cmdl: ResId::invalid(), // None
                    thermal_cskr: ResId::invalid(), // None

                    unknown0: 1,
                    unknown1: 1.0,
                    unknown2: 1.0,

                    visor_params: structs::scly_structs::VisorParameters {
                        unknown0: 0,
                        target_passthrough: 0,
                        visor_mask: 15 // Combat|Scan|Thermal|XRay
                    },
                    enable_thermal_heat: 1,
                    unknown3: 0,
                    unknown4: 0,
                    unknown5: 1.0
                },

                unknown1: 5.0,
                active: 1,

                dcln: dcln,

                health_info: structs::scly_structs::HealthInfo {
                    health: 1.0,
                    knockback_resistance: 1.0,
                },
                damage_vulnerability: DoorType::Disabled.vulnerability(),

                detect_collision: 1,
                unknown4: 1.0,
                unknown5: 0,
                unknown6: 200,
                unknown7: 20,
            }.into(),
            connections: vec![].into(),
        }
    );

    Ok(())
}

pub fn patch_add_block<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    position: [f32;3],
    scale: [f32;3],
    texture: GenericTexture,
    // rotation: [f32;3],
) -> Result<(), String>
{
    let deps = vec![
        (texture.cmdl().to_u32(), b"CMDL"),
        (texture.txtr().to_u32(), b"TXTR"),
    ];
    let deps_iter = deps.iter()
        .map(|&(file_id, fourcc)| structs::Dependency {
            asset_id: file_id,
            asset_type: FourCC::from_bytes(fourcc),
        }
    );
    area.add_dependencies(game_resources, 0, deps_iter);

    add_block(
        area,
        position,
        scale,
        texture,
        1,
    );

    Ok(())
}

pub fn add_block<'r>(
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    position: [f32;3],
    scale: [f32;3],
    texture: GenericTexture,
    is_tangible: u8,
)
{
    let actor_id = area.new_object_id_from_layer_name("Default");

    let scly = area.mrea().scly_section_mut();
    let objects = &mut scly.layers.as_mut_vec()[0].objects.as_mut_vec();

    objects.push(
        structs::SclyObject {
            instance_id: actor_id,
            property_data: structs::Actor {
                name: b"myactor\0".as_cstr(),
                position: position.into(),
                rotation: [0.0, 0.0, 0.0].into(),
                scale: scale.into(),
                hitbox: [0.0, 0.0, 0.0].into(),
                scan_offset: [0.0, 0.0, 0.0].into(),
                unknown1: 1.0,
                unknown2: 0.0,
                health_info: structs::scly_structs::HealthInfo {
                    health: 5.0,
                    knockback_resistance: 1.0
                },
                damage_vulnerability: DoorType::Disabled.vulnerability(),
                cmdl: texture.cmdl(),
                ancs: structs::scly_structs::AncsProp {
                    file_id: ResId::invalid(), // None
                    node_index: 0,
                    default_animation: 0xFFFFFFFF, // -1
                },
                actor_params: structs::scly_structs::ActorParameters {
                    light_params: structs::scly_structs::LightParameters {
                        unknown0: 1,
                        unknown1: 1.0,
                        shadow_tessellation: 0,
                        unknown2: 1.0,
                        unknown3: 20.0,
                        color: [1.0, 1.0, 1.0, 1.0].into(),
                        unknown4: 1,
                        world_lighting: 1,
                        light_recalculation: 1,
                        unknown5: [0.0, 0.0, 0.0].into(),
                        unknown6: 4,
                        unknown7: 4,
                        unknown8: 0,
                        light_layer_id: 0
                    },
                    scan_params: structs::scly_structs::ScannableParameters {
                        scan: ResId::invalid(), // None
                    },
                    xray_cmdl: ResId::invalid(), // None
                    xray_cskr: ResId::invalid(), // None
                    thermal_cmdl: ResId::invalid(), // None
                    thermal_cskr: ResId::invalid(), // None

                    unknown0: 1,
                    unknown1: 1.0,
                    unknown2: 1.0,

                    visor_params: structs::scly_structs::VisorParameters {
                        unknown0: 0,
                        target_passthrough: 0,
                        visor_mask: 15 // Combat|Scan|Thermal|XRay
                    },
                    enable_thermal_heat: 1,
                    unknown3: 0,
                    unknown4: 0,
                    unknown5: 1.0
                },
                looping: 1,
                snow: 1,
                solid: is_tangible,
                camera_passthrough: 0,
                active: 1,
                unknown8: 0,
                unknown9: 1.0,
                unknown10: 1,
                unknown11: 0,
                unknown12: 0,
                unknown13: 0
            }.into(),
            connections: vec![].into()
        },
    );
}


pub fn patch_lock_on_point<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    config: LockOnPoint,
) -> Result<(), String>
{
    let deps = vec![
        (0xBFE4DAA0, b"CMDL"),
        (0x57C7107D, b"TXTR"),
        (0xE580D665, b"TXTR"),
    ];
    let deps_iter = deps.iter()
        .map(|&(file_id, fourcc)| structs::Dependency {
            asset_id: file_id,
            asset_type: FourCC::from_bytes(fourcc),
        }
    );
    area.add_dependencies(game_resources, 0, deps_iter);

    let is_grapple = config.is_grapple.unwrap_or(false);
    let no_lock = config.no_lock.unwrap_or(false);
    let position = config.position;

    if is_grapple {
        let deps = vec![
            (0x3abe45a6, b"SCAN"),
            (0x191a6881, b"STRG"),
            (0x748c37a5, b"SCAN"),
            (0x50ac3b9a, b"STRG"),
            (0xA482DBD1, b"TXTR"),
            (0xC9A36445, b"TXTR"),
            (0x2702E5E0, b"TXTR"),
            (0x34E79314, b"TXTR"),
            (0x46434ED3, b"TXTR"),
            (0x4F944876, b"TXTR"),
        ];
        let deps_iter = deps.iter()
            .map(|&(file_id, fourcc)| structs::Dependency {
                asset_id: file_id,
                asset_type: FourCC::from_bytes(fourcc),
            }
        );
        area.add_dependencies(game_resources, 0, deps_iter);
    }

    let actor_id = config.id1.unwrap_or(area.new_object_id_from_layer_name("Default"));
    let mut grapple_point_id = 0;
    let mut special_function_id = 0;
    let mut timer_id = 0;
    let mut poi_pre_id = 0;
    let mut poi_post_id = 0;
    let mut damageable_trigger_id = 0;
    let mut add_scan_point = false;

    if is_grapple {
        grapple_point_id = area.new_object_id_from_layer_name("Default");
        add_scan_point = true; // We don't actually need the scan points, just their assets. Could save on objects by making this false via config
        if add_scan_point {
            special_function_id = area.new_object_id_from_layer_name("Default");
            timer_id = area.new_object_id_from_layer_name("Default");
            poi_pre_id = area.new_object_id_from_layer_name("Default");
            poi_post_id = area.new_object_id_from_layer_name("Default");
        }
    } else if !no_lock {
        damageable_trigger_id = config.id2.unwrap_or(area.new_object_id_from_layer_name("Default"));
    }

    let layers = area.mrea().scly_section_mut().layers.as_mut_vec();
    layers[0].objects.as_mut_vec().push(
        structs::SclyObject {
            instance_id: actor_id,
            property_data: structs::Actor {
                name: b"myactor\0".as_cstr(),
                position: position.into(),
                rotation: [0.0, 0.0, 0.0].into(),
                scale: [8.0, 8.0, 8.0].into(),
                hitbox: [0.0, 0.0, 0.0].into(),
                scan_offset: [0.0, 0.0, 0.0].into(),
                unknown1: 1.0,
                unknown2: 0.0,
                health_info: structs::scly_structs::HealthInfo {
                    health: 5.0,
                    knockback_resistance: 1.0
                },
                damage_vulnerability: DoorType::Disabled.vulnerability(),
                cmdl: ResId::<res_id::CMDL>::new(0xBFE4DAA0),
                ancs: structs::scly_structs::AncsProp {
                    file_id: ResId::invalid(),
                    node_index: 0,
                    default_animation: 0xFFFFFFFF,
                },
                actor_params: structs::scly_structs::ActorParameters {
                    light_params: structs::scly_structs::LightParameters {
                        unknown0: 1,
                        unknown1: 1.0,
                        shadow_tessellation: 0,
                        unknown2: 1.0,
                        unknown3: 20.0,
                        color: [1.0, 1.0, 1.0, 1.0].into(),
                        unknown4: 1,
                        world_lighting: 1,
                        light_recalculation: 1,
                        unknown5: [0.0, 0.0, 0.0].into(),
                        unknown6: 4,
                        unknown7: 4,
                        unknown8: 0,
                        light_layer_id: 0
                    },
                    scan_params: structs::scly_structs::ScannableParameters {
                        scan: ResId::invalid(), // None
                    },
                    xray_cmdl: ResId::invalid(), // None
                    xray_cskr: ResId::invalid(), // None
                    thermal_cmdl: ResId::invalid(), // None
                    thermal_cskr: ResId::invalid(), // None

                    unknown0: 1,
                    unknown1: 1.0,
                    unknown2: 1.0,

                    visor_params: structs::scly_structs::VisorParameters {
                        unknown0: 0,
                        target_passthrough: 1,
                        visor_mask: 15 // Combat|Scan|Thermal|XRay
                    },
                    enable_thermal_heat: 1,
                    unknown3: 0,
                    unknown4: 0,
                    unknown5: 1.0
                },
                looping: 1,
                snow: 1,
                solid: 0,
                camera_passthrough: 1,
                active: config.active1.unwrap_or(true) as u8,
                unknown8: 0,
                unknown9: 1.0,
                unknown10: 1,
                unknown11: 0,
                unknown12: 0,
                unknown13: 0
            }.into(),
            connections: vec![].into()
        },
    );

    if is_grapple {
        layers[0].objects.as_mut_vec().push(
            structs::SclyObject {
                instance_id: grapple_point_id,
                property_data: structs::GrapplePoint {
                    name: b"my grapple point\0".as_cstr(),
                    position: [position[0], position[1], position[2] - 0.5].into(),
                    rotation: [0.0, -0.0, 0.0].into(),
                    active: 1,
                    grapple_params: structs::GrappleParams {
                        unknown1: 10.0,
                        unknown2: 10.0,
                        unknown3: 1.0,
                        unknown4: 1.0,
                        unknown5: 1.0,
                        unknown6: 1.0,
                        unknown7: 1.0,
                        unknown8: 45.0,
                        unknown9: 90.0,
                        unknown10: 0.0,
                        unknown11: 0.0,

                        disable_turning: 0,
                    },
                }.into(),
                connections: vec![].into(),
            },
        );

        if add_scan_point {
            layers[0].objects.as_mut_vec().push(
                structs::SclyObject {
                    instance_id: special_function_id,
                    connections: vec![
                        structs::Connection {
                            state: structs::ConnectionState::ZERO,
                            message: structs::ConnectionMsg::DEACTIVATE,
                            target_object_id: poi_pre_id,
                        },
                        structs::Connection {
                            state: structs::ConnectionState::ZERO,
                            message: structs::ConnectionMsg::ACTIVATE,
                            target_object_id: poi_post_id,
                        },
                    ].into(),
                    property_data: structs::SclyProperty::SpecialFunction(
                        Box::new(structs::SpecialFunction {
                            name: b"myspecialfun\0".as_cstr(),
                            position: position.into(),
                            rotation: [0.0, 0.0, 0.0].into(),
                            type_: 5, // inventory activator
                            unknown0: b"\0".as_cstr(),
                            unknown1: 0.0,
                            unknown2: 0.0,
                            unknown3: 0.0,
                            layer_change_room_id: 0xFFFFFFFF,
                            layer_change_layer_id: 0xFFFFFFFF,
                            item_id: 12, // grapple beam
                            unknown4: 1, // active
                            unknown5: 0.0,
                            unknown6: 0xFFFFFFFF,
                            unknown7: 0xFFFFFFFF,
                            unknown8: 0xFFFFFFFF,
                        })
                    ),
                }
            );

            layers[0].objects.as_mut_vec().push(
                structs::SclyObject {
                    instance_id: timer_id,
                    connections: vec![
                        structs::Connection {
                            state: structs::ConnectionState::ZERO,
                            message: structs::ConnectionMsg::ACTION,
                            target_object_id: special_function_id,
                        },
                    ].into(),
                    property_data: structs::Timer {
                        name: b"grapple timer\0".as_cstr(),
                        start_time: 0.02,
                        max_random_add: 0.0,
                        looping: 0,
                        start_immediately: 1,
                        active: 1,
                    }.into(),
                }
            );

            layers[0].objects.as_mut_vec().push(
                structs::SclyObject {
                    instance_id: poi_pre_id,
                    connections: vec![].into(),
                    property_data: structs::SclyProperty::PointOfInterest(
                        Box::new(structs::PointOfInterest {
                            name: b"mypoi\0".as_cstr(),
                            position: [position[0], position[1], position[2] - 0.5].into(),
                            rotation: [0.0, 0.0, 0.0].into(),
                            active: 1,
                            scan_param: structs::scly_structs::ScannableParameters {
                                scan: resource_info!("Grapple Point pre.SCAN").try_into().unwrap(),
                            },
                            point_size: 0.0,
                        })
                    ),
                }
            );

            layers[0].objects.as_mut_vec().push(
                structs::SclyObject {
                    instance_id: poi_post_id,
                    connections: vec![].into(),
                    property_data: structs::SclyProperty::PointOfInterest(
                        Box::new(structs::PointOfInterest {
                            name: b"mypoi\0".as_cstr(),
                            position: [position[0], position[1], position[2] - 0.5].into(),
                            rotation: [0.0, 0.0, 0.0].into(),
                            active: 0,
                            scan_param: structs::scly_structs::ScannableParameters {
                                scan: resource_info!("Grapple Point.SCAN").try_into().unwrap(),
                            },
                            point_size: 0.0,
                        })
                    ),
                }
            );
        }
    } else if !no_lock {
        layers[0].objects.as_mut_vec().push(
            structs::SclyObject {
                instance_id: damageable_trigger_id,
                property_data: structs::DamageableTrigger {
                    name: b"my dtrigger\0".as_cstr(),
                    position: position.into(),
                    scale: [0.001, 0.001, 0.001].into(),
                    health_info: structs::scly_structs::HealthInfo {
                        health: 9999999999.0,
                        knockback_resistance: 1.0
                    },
                    damage_vulnerability: DoorType::Blue.vulnerability(),
                    unknown0: 0,
                    pattern_txtr0: ResId::invalid(),
                    pattern_txtr1: ResId::invalid(),
                    color_txtr: ResId::invalid(),
                    lock_on: 1,
                    active: config.active2.unwrap_or(true) as u8,
                    visor_params: structs::scly_structs::VisorParameters {
                        unknown0: 0,
                        target_passthrough: 0,
                        visor_mask: 15 // Combat|Scan|Thermal|XRay
                    }
                }.into(),
                connections: vec![].into(),
            },
        );
    }

    Ok(())
}


pub fn patch_add_camera_hint<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    trigger_pos: [f32;3],
    trigger_scale: [f32;3],
    camera_pos: [f32;3],
    camera_rot: [f32;3],
    behavior: u32,
)
-> Result<(), String>
{
    let camear_hint_id = area.new_object_id_from_layer_name("Default");
    let camera_hint_trigger_id = area.new_object_id_from_layer_name("Default");

    let camera_objs = add_camera_hint(
        camear_hint_id,
        camera_hint_trigger_id,
        trigger_pos,
        trigger_scale,
        camera_pos,
        camera_rot,
        behavior,
    );

    area.mrea()
        .scly_section_mut()
        .layers.as_mut_vec()[0]
        .objects
        .as_mut_vec()
        .extend_from_slice(&camera_objs);

    Ok(())
}

pub fn add_camera_hint<'r>(
    camear_hint_id: u32,
    camera_hint_trigger_id: u32,
    trigger_pos: [f32;3],
    trigger_scale: [f32;3],
    camera_pos: [f32;3],
    camera_rot: [f32;3],
    behavior: u32,
) -> Vec<structs::SclyObject<'r>>
{
    let mut objects = Vec::new();

    objects.push(
        structs::SclyObject {
            instance_id: camear_hint_id,
            connections: vec![].into(),
            property_data: structs::SclyProperty::CameraHint(
                Box::new(structs::CameraHint {
                    name: b"CameraHint\0".as_cstr(),
                    position: camera_pos.into(),
                    rotation: camera_rot.into(),
                    active: 1,
                    priority: 8,
                    behavior: behavior,
                    camera_hint_params: structs::CameraHintParameters {
                        calculate_cam_pos: 0,
                        chase_allowed: 0,
                        boost_allowed: 0,
                        obscure_avoidance: 0,
                        volume_collider: 0,
                        apply_immediately: 1,
                        look_at_ball: 1,
                        hint_distance_selection: 0,
                        hint_distance_self_pos: 1,
                        control_interpolation: 0,
                        sinusoidal_interpolation: 0,
                        sinusoidal_interpolation_hintless: 0,
                        clamp_velocity: 0,
                        skip_cinematic: 0,
                        no_elevation_interp: 0,
                        direct_elevation: 0,
                        override_look_dir: 1,
                        no_elevation_vel_clamp: 0,
                        calculate_transform_from_prev_cam: 1,
                        no_spline: 1,
                        unknown21: 0,
                        unknown22: 0,
                    },
                    min_dist: structs::BoolFloat {
                        active: 0,
                        value: 8.0,
                    },
                    max_dist: structs::BoolFloat {
                        active: 0,
                        value: 50.0,
                    },
                    backwards_dist: structs::BoolFloat {
                        active: 0,
                        value: 8.0,
                    },
                    look_at_offset: structs::BoolVec3 {
                        active: 0,
                        value: [0.0, 1.0, 1.0].into(),
                    },
                    chase_look_at_offset: structs::BoolVec3 {
                        active: 0,
                        value: [0.0, 1.0, 1.0].into(),
                    },
                    ball_to_cam: [3.0, 3.0, 3.0].into(),
                    fov: structs::BoolFloat {
                        active: 0,
                        value: 55.0,
                    },
                    attitude_range: structs::BoolFloat {
                        active: 0,
                        value: 90.0,
                    },
                    azimuth_range: structs::BoolFloat {
                        active: 0,
                        value: 90.0,
                    },
                    angle_per_second: structs::BoolFloat {
                        active: 0,
                        value: 120.0,
                    },
                    clamp_vel_range: 10.0,
                    clamp_rot_range: 120.0,
                    elevation: structs::BoolFloat {
                        active: 0,
                        value: 2.7,
                    },
                    interpolate_time: 1.0,
                    clamp_vel_time: 1.0,
                    control_interp_dur: 1.0,
                })
            ),
        }
    );

    objects.push(
        structs::SclyObject {
            instance_id: camera_hint_trigger_id,
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::INCREMENT,
                    target_object_id: camear_hint_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::EXITED,
                    message: structs::ConnectionMsg::DECREMENT,
                    target_object_id: camear_hint_id,
                }
            ].into(),
            property_data: structs::SclyProperty::Trigger(
                Box::new(structs::Trigger {
                    name: b"camerahinttrigger\0".as_cstr(),
                    position: trigger_pos.into(),
                    scale: trigger_scale.into(),
                    damage_info: structs::scly_structs::DamageInfo {
                        weapon_type: 0,
                        damage: 0.0,
                        radius: 0.0,
                        knockback_power: 0.0
                    },
                    force: [0.0, 0.0, 0.0].into(),
                    flags: 1,
                    active: 1,
                    deactivate_on_enter: 0,
                    deactivate_on_exit: 0,
                })
            ),
        }
    );

    // objects.push(
    //     structs::SclyObject {
    //         instance_id: area.new_object_id_from_layer_name("Default"),
    //         connections: vec![
    //             structs::Connection {
    //                 state: structs::ConnectionState::INSIDE,
    //                 message: structs::ConnectionMsg::INCREMENT,
    //                 target_object_id: camear_hint_id,
    //             },
    //             structs::Connection {
    //                 state: structs::ConnectionState::EXITED,
    //                 message: structs::ConnectionMsg::DECREMENT,
    //                 target_object_id: camear_hint_id,
    //             }
    //         ].into(),
    //         property_data: structs::SclyProperty::CameraHintTrigger(
    //             Box::new(structs::CameraHintTrigger {
    //                 name: b"CameraHintTrigger\0".as_cstr(),
    //                 position: spawn_point_position.into(),
    //                 rotation: spawn_point_rotation.into(),
    //                 scale: [10.0, 10.0, 10.0].into(),
    //                 active: 1,
    //                 deactivate_on_enter: 0,
    //                 deactivate_on_exit: 0,
    //             })
    //         ),
    //     }
    // );

    objects
}

pub fn patch_add_escape_sequence<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    time: f32,
    start_trigger_pos: [f32;3],
    start_trigger_scale: [f32;3],
    stop_trigger_pos: [f32;3],
    stop_trigger_scale: [f32;3],
) -> Result<(), String>
{
    let start_special_function_id = area.new_object_id_from_layer_name("Default");
    let stop_special_function_id = area.new_object_id_from_layer_name("Default");
    let start_sequence_trigger_id = area.new_object_id_from_layer_name("Default");
    let stop_sequence_trigger_id = area.new_object_id_from_layer_name("Default");

    let layers = area.mrea().scly_section_mut().layers.as_mut_vec();
    let objects = layers[0].objects.as_mut_vec();

    objects.push(
        structs::SclyObject {
            instance_id: start_special_function_id,
            connections: vec![].into(),
            property_data: structs::SclyProperty::SpecialFunction(
                Box::new(structs::SpecialFunction {
                    name: b"start escape sequence\0".as_cstr(),
                    position: [0.0, 0.0, 0.0].into(),
                    rotation: [0.0, 0.0, 0.0].into(),
                    type_: 11, // escape sequence
                    unknown0: b"\0".as_cstr(),
                    unknown1: time,
                    unknown2: 0.0,
                    unknown3: 0.0,
                    layer_change_room_id: 0,
                    layer_change_layer_id: 0,
                    item_id: 0,
                    unknown4: 1, // active
                    unknown5: 0.0,
                    unknown6: 0xFFFFFFFF,
                    unknown7: 0xFFFFFFFF,
                    unknown8: 0xFFFFFFFF,
                })
            ),
        }
    );

    objects.push(
        structs::SclyObject {
            instance_id: start_sequence_trigger_id,
            property_data: structs::Trigger {
                name: b"Start Sequence Trigger\0".as_cstr(),
                position: start_trigger_pos.into(),
                scale: start_trigger_scale.into(),
                damage_info: structs::scly_structs::DamageInfo {
                    weapon_type: 0,
                    damage: 0.0,
                    radius: 0.0,
                    knockback_power: 0.0
                },
                force: [0.0, 0.0, 0.0].into(),
                flags: 1,
                active: 1,
                deactivate_on_enter: 0,
                deactivate_on_exit: 0
            }.into(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::EXITED,
                    message: structs::ConnectionMsg::ACTION,
                    target_object_id: start_special_function_id,
                },
            ].into(),
        }
    );

    objects.push(
        structs::SclyObject {
            instance_id: stop_special_function_id,
            connections: vec![].into(),
            property_data: structs::SclyProperty::SpecialFunction(
                Box::new(structs::SpecialFunction {
                    name: b"stop escape sequence\0".as_cstr(),
                    position: [0.0, 0.0, 0.0].into(),
                    rotation: [0.0, 0.0, 0.0].into(),
                    type_: 11, // escape sequence
                    unknown0: b"\0".as_cstr(),
                    unknown1: 0.0, // Set the timer to 0.0, so it stops counting
                    unknown2: 0.0,
                    unknown3: 0.0,
                    layer_change_room_id: 0,
                    layer_change_layer_id: 0,
                    item_id: 0,
                    unknown4: 1, // active
                    unknown5: 0.0,
                    unknown6: 0xFFFFFFFF,
                    unknown7: 0xFFFFFFFF,
                    unknown8: 0xFFFFFFFF,
                })
            ),
        }
    );

    objects.push(
        structs::SclyObject {
            instance_id: stop_sequence_trigger_id,
            property_data: structs::Trigger {
                name: b"stop Sequence Trigger\0".as_cstr(),
                position: stop_trigger_pos.into(),
                scale: stop_trigger_scale.into(),
                damage_info: structs::scly_structs::DamageInfo {
                    weapon_type: 0,
                    damage: 0.0,
                    radius: 0.0,
                    knockback_power: 0.0
                },
                force: [0.0, 0.0, 0.0].into(),
                flags: 1,
                active: 1,
                deactivate_on_enter: 0,
                deactivate_on_exit: 0
            }.into(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::ACTION,
                    target_object_id: stop_special_function_id,
                },
            ].into(),
        }
    );

    Ok(())
}
