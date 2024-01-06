use std::{
    collections::HashMap,
    convert::TryInto,
    iter,
};

use crate::{
    patches::{
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
        PlatformConfig,
        PlatformType,
        BlockConfig,
        HudmemoConfig,
        WaypointConfig,
        CounterConfig,
        SwitchConfig,
        PlayerHintConfig,
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
    ($area:expr, $id:expr, $requested_layer_id:expr, $object_type:ident, $new_property_data:ident, $update_property_data:ident) => {
        let area = $area;
        let id = $id;
        let requested_layer_id = $requested_layer_id;
        let mrea_id = area.mlvl_area.mrea.to_u32().clone();

        // add more layers as needed
        if let Some(requested_layer_id) = requested_layer_id {
            while area.layer_flags.layer_count <= requested_layer_id {
                area.add_layer(b"New Layer\0".as_cstr());
            }
        }

        if let Some(id) = id {
            let scly = area.mrea().scly_section_mut();

            // try to find existing object
            let info = {
                let mut info = None;

                let layer_count = scly.layers.as_mut_vec().len();
                for _layer_id in 0..layer_count {
                    let layer = scly.layers
                        .iter()
                        .nth(_layer_id)
                        .unwrap();

                    let obj = layer.objects
                        .iter()
                        .find(|obj| obj.instance_id & 0x00FFFFFF == id & 0x00FFFFFF);

                    if let Some(obj) = obj {
                        if obj.property_data.object_type() != structs::$object_type::OBJECT_TYPE {
                            panic!("Failed to edit existing object 0x{:X} in room 0x{:X}: Unexpected object type 0x{:X} (expected 0x{:X})", id, mrea_id, obj.property_data.object_type(), structs::$object_type::OBJECT_TYPE);
                        }

                        info = Some((_layer_id as u32, obj.instance_id));
                        break;
                    }
                }

                info
            };

            if let Some(info) = info {
                let (layer_id, _) = info;

                // move and update
                if requested_layer_id.is_some() && requested_layer_id.unwrap() != layer_id {
                    let requested_layer_id = requested_layer_id.unwrap();

                    // clone existing object
                    let mut obj = scly.layers
                        .as_mut_vec()[layer_id as usize]
                        .objects
                        .as_mut_vec()
                        .iter_mut()
                        .find(|obj| obj.instance_id & 0x00FFFFFF == id & 0x00FFFFFF)
                        .unwrap()
                        .clone();

                    // modify it
                    $update_property_data!(obj);
                    
                    // remove original
                    scly.layers
                        .as_mut_vec()[layer_id as usize]
                        .objects
                        .as_mut_vec()
                        .retain(|obj| obj.instance_id & 0x00FFFFFF != id & 0x00FFFFFF);

                    // re-add to target layer
                    scly.layers
                        .as_mut_vec()[requested_layer_id as usize]
                        .objects
                        .as_mut_vec()
                        .push(obj);

                    return Ok(());
                }

                // get mutable reference to existing object
                let obj = scly.layers
                    .as_mut_vec()[layer_id as usize]
                    .objects
                    .as_mut_vec()
                    .iter_mut()
                    .find(|obj| obj.instance_id & 0x00FFFFFF == id & 0x00FFFFFF)
                    .unwrap();

                // update it
                $update_property_data!(obj);

                return Ok(());
            }
        }

        // add new object
        let id = id.unwrap_or(area.new_object_id_from_layer_id(0));
        let scly = area.mrea().scly_section_mut();
        let layers = &mut scly.layers.as_mut_vec();
        let objects = layers[requested_layer_id.unwrap_or(0) as usize].objects.as_mut_vec();
        let property_data = $new_property_data!();
        let property_data: structs::SclyProperty = property_data.into();

        assert!(property_data.object_type() == structs::$object_type::OBJECT_TYPE);

        objects.push(
            structs::SclyObject {
                instance_id: id,
                property_data,
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
    macro_rules! new {
        () => {
            structs::StreamedAudio {
                name: b"mystreamedaudio\0".as_cstr(),
                active: config.active.unwrap_or(true) as u8,
                audio_file_name: string_to_cstr(config.audio_file_name),
                no_stop_on_deactivate: config.no_stop_on_deactivate.unwrap_or(true) as u8,
                fade_in_time: config.fade_in_time.unwrap_or(0.1),
                fade_out_time: config.fade_out_time.unwrap_or(1.5),
                volume: config.volume.unwrap_or(100),
                oneshot: config.oneshot.unwrap_or(0),
                is_music: config.is_music as u8,
            }
        };
    }
    
    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_streamed_audio_mut().unwrap();
    
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

    add_edit_obj_helper!(area, config.id, config.layer, StreamedAudio, new, update);
}

pub fn patch_add_liquid<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    config: &WaterConfig,
    resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
)
-> Result<(), String>
{
    let water_type = WaterType::from_str(config.liquid_type.as_str());

    /* add dependencies to area */
    {
        let deps = water_type.dependencies();
        let deps_iter = deps.iter()
            .map(|&(file_id, fourcc)| structs::Dependency {
                    asset_id: file_id,
                    asset_type: fourcc,
            });
    
        area.add_dependencies(resources, 0, deps_iter);
    }

    let mut water_obj = water_type.to_obj();
    {        
        let water = water_obj.property_data.as_water_mut().unwrap();
        water.position[0] = config.position[0];
        water.position[1] = config.position[1];
        water.position[2] = config.position[2];
        water.scale[0]    = config.scale[0];
        water.scale[1]    = config.scale[1];
        water.scale[2]    = config.scale[2];
    }
    macro_rules! new {
        () => {
            water_obj.property_data
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            $obj.property_data = water_obj.property_data.clone();
        };
    }

    add_edit_obj_helper!(area, config.id, config.layer, Water, new, update);
}

pub fn patch_add_actor_key_frame<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: ActorKeyFrameConfig,
)
    -> Result<(), String>
{
    macro_rules! new {
        () => {
            structs::ActorKeyFrame {
                name: b"my keyframe\0".as_cstr(),
                active: config.active.unwrap_or(true) as u8,
                animation_id: config.animation_id,
                looping: config.looping as u8,
                lifetime: config.lifetime,
                fade_out: config.fade_out,
                total_playback: config.total_playback,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_actor_key_frame_mut().unwrap();

            if let Some(active) = config.active { property_data.active = active as u8 }

            property_data.animation_id = config.animation_id;
            property_data.looping = config.looping as u8;
            property_data.lifetime = config.lifetime;
            property_data.fade_out = config.fade_out;
            property_data.total_playback = config.total_playback;
        };
    }

    add_edit_obj_helper!(area, Some(config.id), config.layer, ActorKeyFrame, new, update);
}

pub fn patch_add_timer<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: TimerConfig,
)
    -> Result<(), String>
{
    macro_rules! new {
        () => {
            structs::Timer {
                name: b"my timer\0".as_cstr(),
                start_time: config.time,
                max_random_add: config.max_random_add.unwrap_or(0.0),
                looping: config.looping.unwrap_or(false) as u8,
                start_immediately: config.start_immediately.unwrap_or(false) as u8,
                active: config.active.unwrap_or(true) as u8,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_timer_mut().unwrap();

            property_data.start_time = config.time;

            if let Some(active           ) = config.active            { property_data.active            = active            as u8 }
            if let Some(max_random_add   ) = config.max_random_add    { property_data.max_random_add    = max_random_add          }
            if let Some(looping          ) = config.looping           { property_data.looping           = looping           as u8 }
            if let Some(start_immediately) = config.start_immediately { property_data.start_immediately = start_immediately as u8 }
        };
    }

    add_edit_obj_helper!(area, Some(config.id), config.layer, Timer, new, update);
}

pub fn patch_add_relay<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: RelayConfig,
)
    -> Result<(), String>
{
    macro_rules! new {
        () => {
            structs::Relay {
                name: b"my relay\0".as_cstr(),
                active: config.active.unwrap_or(true) as u8,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_relay_mut().unwrap();
            if let Some(active) = config.active { property_data.active = active as u8 }
        };
    }

    add_edit_obj_helper!(area, Some(config.id), config.layer, Relay, new, update);
}

pub fn patch_add_spawn_point<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: SpawnPointConfig,
)
    -> Result<(), String>
{
    let spawn_point = {
        let mut spawn_point = structs::SpawnPoint {
            name: b"my spawnpoint\0".as_cstr(),
            position: config.position.into(),
            rotation: config.rotation.unwrap_or([0.0, 0.0, 0.0]).into(),
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
            default_spawn: config.default_spawn.unwrap_or(false) as u8,
            active: config.active.unwrap_or(true) as u8,
            morphed: config.morphed.unwrap_or(false) as u8,
        };

        if let Some(items) = config.items.as_ref() {
            items.update_spawn_point(&mut spawn_point);
        }

        spawn_point
    };

    macro_rules! new {
        () => {
            spawn_point.clone()
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_spawn_point_mut().unwrap();

            property_data.position = config.position.into();

            if let Some(items) = config.items.as_ref() {
                items.update_spawn_point(property_data);
            }

            if let Some(active       ) = config.active        { property_data.active        = active        as u8     }
            if let Some(default_spawn) = config.default_spawn { property_data.default_spawn = default_spawn as u8     }
            if let Some(morphed      ) = config.morphed       { property_data.morphed       = morphed       as u8     }
            if let Some(rotation     ) = config.rotation      { property_data.rotation      = rotation     .   into() }
        };
    }

    add_edit_obj_helper!(area, Some(config.id), config.layer, SpawnPoint, new, update);
}

pub fn patch_add_trigger<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: TriggerConfig,
)
    -> Result<(), String>
{
    macro_rules! new {
        () => {
            structs::Trigger {
                name: b"my trigger\0".as_cstr(),
                position: config.position.into(),
                scale: config.scale.into(),
                damage_info: structs::scly_structs::DamageInfo {
                    weapon_type: config.damage_type.unwrap_or(DamageType::Power) as u32,
                    damage: config.damage_amount.unwrap_or(0.0),
                    radius: 0.0,
                    knockback_power: 0.0
                },
                force: config.force.unwrap_or([0.0, 0.0, 0.0]).into(),
                flags: config.flags.unwrap_or(1),
                active: config.active.unwrap_or(true) as u8,
                deactivate_on_enter: config.deactivate_on_enter.unwrap_or(false) as u8,
                deactivate_on_exit: config.deactivate_on_exit.unwrap_or(false) as u8,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_trigger_mut().unwrap();

            property_data.position = config.position.into();
            property_data.scale = config.scale.into();

            if let Some(active             ) = config.active              { property_data.active                          = active              as u8  }
            if let Some(damage_type        ) = config.damage_type         { property_data.damage_info        .weapon_type = damage_type         as u32 }
            if let Some(damage_amount      ) = config.damage_amount       { property_data.damage_info        .damage      = damage_amount              }
            if let Some(force              ) = config.force               { property_data.force                           = force              .into() }
            if let Some(flags              ) = config.flags               { property_data.flags                           = flags                      }
            if let Some(deactivate_on_enter) = config.deactivate_on_enter { property_data.deactivate_on_enter             = deactivate_on_enter as u8  }
            if let Some(deactivate_on_exit ) = config.deactivate_on_exit  { property_data.deactivate_on_exit              = deactivate_on_exit  as u8  }
        };
    }

    add_edit_obj_helper!(area, config.id, config.layer, Trigger, new, update);
}

pub fn patch_add_special_fn<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: SpecialFunctionConfig,
)
    -> Result<(), String>
{
    let default = "".to_string();
    let unknown0 = config.unknown1.as_ref().unwrap_or(&default);
    let unknown0 = string_to_cstr(unknown0.clone());

    macro_rules! new {
        () => {
            structs::SpecialFunction {
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
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_special_function_mut().unwrap();

            property_data.type_ = config.type_ as u32;
            
            if let Some(position             ) = config.position                       { property_data.position              = position             .into() }
            if let Some(rotation             ) = config.rotation                       { property_data.rotation              = rotation             .into() }
            if let Some(_                    ) = config.unknown1             .as_ref() { property_data.unknown0              = unknown0                     }
            if let Some(unknown2             ) = config.unknown2                       { property_data.unknown1              = unknown2                     }
            if let Some(unknown3             ) = config.unknown3                       { property_data.unknown2              = unknown3                     }
            if let Some(layer_change_room_id ) = config.layer_change_room_id           { property_data.layer_change_room_id  = layer_change_room_id         }
            if let Some(layer_change_layer_id) = config.layer_change_layer_id          { property_data.layer_change_layer_id = layer_change_layer_id        }
            if let Some(item_id              ) = config.item_id                        { property_data.item_id               = item_id               as u32 }
            if let Some(active               ) = config.active                         { property_data.unknown4              = active                as u8  }
            if let Some(unknown6             ) = config.unknown6                       { property_data.unknown5              = unknown6                     }
            if let Some(spinner1             ) = config.spinner1                       { property_data.unknown6              = spinner1                     }
            if let Some(spinner2             ) = config.spinner2                       { property_data.unknown7              = spinner2                     }
            if let Some(spinner3             ) = config.spinner3                       { property_data.unknown8              = spinner3                     }
        };
    }

    add_edit_obj_helper!(area, config.id, config.layer, SpecialFunction, new, update);
}

pub fn patch_add_hudmemo<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    config: HudmemoConfig,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    strg_id: Option<ResId<res_id::STRG>>,
)
    -> Result<(), String>
{
    let memo_type = match config.modal.unwrap_or(false) {
        false => 0,
        true => 1,
    };

    macro_rules! new {
        () => {
            structs::HudMemo {
                name: b"my hudmemo\0".as_cstr(),
                first_message_timer: config.message_time.unwrap_or(4.0),
                unknown: 1,
                memo_type,
                strg: strg_id.unwrap_or(ResId::invalid()),
                active: config.active.unwrap_or(true) as u8,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_hud_memo_mut().unwrap();

            if config.modal.is_some() {
                property_data.memo_type = memo_type;
            }

            if let Some(strg_id) = strg_id { property_data.strg = strg_id }
            if let Some(message_time) = config.message_time { property_data.first_message_timer = message_time}
            if let Some(active) = config.active { property_data.active = active as u8}

        };
    }

    if let Some(strg_id) = strg_id {
        let strg_dep: structs::Dependency = strg_id.into();
        area.add_dependencies(game_resources, 0, iter::once(strg_dep));
    }

    add_edit_obj_helper!(area, Some(config.id), config.layer, HudMemo, new, update);
}

pub fn patch_add_actor_rotate_fn<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: ActorRotateConfig,
)
    -> Result<(), String>
{
    macro_rules! new {
        () => {
            structs::ActorRotate {
                name: b"my actor rotate\0".as_cstr(),
                rotation: config.rotation.into(),
                time_scale: config.time_scale,
                update_actors: config.update_actors as u8,
                update_on_creation: config.update_on_creation as u8,
                update_active: config.update_active as u8,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_actor_rotate_mut().unwrap();

            property_data.rotation           = config.rotation          .into();
            property_data.time_scale         = config.time_scale               ;
            property_data.update_actors      = config.update_actors      as u8 ;
            property_data.update_on_creation = config.update_on_creation as u8 ;
            property_data.update_active      = config.update_active      as u8 ;
        };
    }

    add_edit_obj_helper!(area, config.id, config.layer, ActorRotate, new, update);
}

pub fn patch_add_waypoint<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: WaypointConfig,
)
    -> Result<(), String>
{
    macro_rules! new {
        () => {
            structs::Waypoint {
                name: b"my waypoint\0".as_cstr(),
                position: config.position.unwrap_or([0.0, 0.0, 0.0]).into(),
                rotation: config.rotation.unwrap_or([0.0, 0.0, 0.0]).into(),
                active: config.active.unwrap_or(true) as u8,
                speed: config.speed.unwrap_or(1.0),
                pause: config.pause.unwrap_or(0.0),
                pattern_translate: config.pattern_translate.unwrap_or(0),
                pattern_orient: config.pattern_orient.unwrap_or(0),
                pattern_fit: config.pattern_fit.unwrap_or(0),
                behaviour: config.behaviour.unwrap_or(0),
                behaviour_orient: config.behaviour_orient.unwrap_or(0),
                behaviour_modifiers: config.behaviour_modifiers.unwrap_or(0),
                animation: config.animation.unwrap_or(0),
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_waypoint_mut().unwrap();
            if let Some(position           ) = config.position            {property_data.position            = position.into()    }
            if let Some(rotation           ) = config.rotation            {property_data.rotation            = rotation.into()    }
            if let Some(active             ) = config.active              {property_data.active              = active as u8       }
            if let Some(speed              ) = config.speed               {property_data.speed               = speed              }
            if let Some(pause              ) = config.pause               {property_data.pause               = pause              }
            if let Some(pattern_translate  ) = config.pattern_translate   {property_data.pattern_translate   = pattern_translate  }
            if let Some(pattern_orient     ) = config.pattern_orient      {property_data.pattern_orient      = pattern_orient     }
            if let Some(pattern_fit        ) = config.pattern_fit         {property_data.pattern_fit         = pattern_fit        }
            if let Some(behaviour          ) = config.behaviour           {property_data.behaviour           = behaviour          }
            if let Some(behaviour_orient   ) = config.behaviour_orient    {property_data.behaviour_orient    = behaviour_orient   }
            if let Some(behaviour_modifiers) = config.behaviour_modifiers {property_data.behaviour_modifiers = behaviour_modifiers}
            if let Some(animation          ) = config.animation           {property_data.animation           = animation          }
        };
    }

    add_edit_obj_helper!(area, Some(config.id), config.layer, Waypoint, new, update);
}

pub fn patch_add_counter<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: CounterConfig,
)
    -> Result<(), String>
{
    macro_rules! new {
        () => {
            structs::Counter {
                name: b"my counter\0".as_cstr(),
                start_value: config.start_value.unwrap_or(0),
                max_value: config.max_value.unwrap_or(1),
                auto_reset: config.auto_reset.unwrap_or(false) as u8,
                active: config.active.unwrap_or(true) as u8,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_counter_mut().unwrap();
            if let Some(start_value ) = config.start_value {property_data.start_value = start_value       }
            if let Some(max_value   ) = config.max_value   {property_data.max_value   = max_value         }
            if let Some(auto_reset  ) = config.auto_reset  {property_data.auto_reset  = auto_reset  as u8 }
            if let Some(active      ) = config.active      {property_data.active      = active      as u8 }
        };
    }

    add_edit_obj_helper!(area, Some(config.id), config.layer, Counter, new, update);
}

pub fn patch_add_switch<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: SwitchConfig,
)
    -> Result<(), String>
{
    macro_rules! new {
        () => {
            structs::Switch {
                name: b"my switch\0".as_cstr(),
                active: config.active.unwrap_or(true) as u8,
                open: config.open.unwrap_or(false) as u8,
                auto_close: config.auto_close.unwrap_or(false) as u8,
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_switch_mut().unwrap();
            if let Some(active           ) = config.active           {property_data.active           = active      as u8 }
            if let Some(open             ) = config.open             {property_data.open             = open        as u8 }
            if let Some(auto_close       ) = config.auto_close       {property_data.auto_close       = auto_close  as u8 }
        };
    }

    add_edit_obj_helper!(area, Some(config.id), config.layer, Switch, new, update);
}

pub fn patch_add_player_hint<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: PlayerHintConfig,
)
    -> Result<(), String>
{
    macro_rules! new {
        () => {
            structs::PlayerHint {
                name: b"my playerhint\0".as_cstr(),
                
                position: [0.0, 0.0, 0.0].into(),
                rotation: [0.0, 0.0, 0.0].into(),
                
                active: config.active.unwrap_or(true) as u8,

                data: structs::PlayerHintStruct {
                    unknown1              : config.unknown1              .unwrap_or(false) as u8,
                    unknown2              : config.unknown2              .unwrap_or(false) as u8,
                    extend_target_distance: config.extend_target_distance.unwrap_or(false) as u8,
                    unknown4              : config.unknown4              .unwrap_or(false) as u8,
                    unknown5              : config.unknown5              .unwrap_or(false) as u8,
                    disable_unmorph       : config.disable_unmorph       .unwrap_or(false) as u8,
                    disable_morph         : config.disable_morph         .unwrap_or(false) as u8,
                    disable_controls      : config.disable_controls      .unwrap_or(false) as u8,
                    disable_boost         : config.disable_boost         .unwrap_or(false) as u8,
                    activate_visor_combat : config.activate_visor_combat .unwrap_or(false) as u8,
                    activate_visor_scan   : config.activate_visor_scan   .unwrap_or(false) as u8,
                    activate_visor_thermal: config.activate_visor_thermal.unwrap_or(false) as u8,
                    activate_visor_xray   : config.activate_visor_xray   .unwrap_or(false) as u8,
                    unknown6              : config.unknown6              .unwrap_or(false) as u8,
                    face_object_on_unmorph: config.face_object_on_unmorph.unwrap_or(false) as u8,
                }.into(),

                priority: config.priority.unwrap_or(10),
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_player_hint_mut().unwrap();
            if let Some(active                 ) = config.active                 { property_data.active                      = active                 as u8 }
            if let Some(priority               ) = config.priority               { property_data.priority                    = priority                     }
            if let Some(unknown1               ) = config.unknown1               { property_data.data.unknown1               = unknown1               as u8 }
            if let Some(unknown2               ) = config.unknown2               { property_data.data.unknown2               = unknown2               as u8 }
            if let Some(extend_target_distance ) = config.extend_target_distance { property_data.data.extend_target_distance = extend_target_distance as u8 }
            if let Some(unknown4               ) = config.unknown4               { property_data.data.unknown4               = unknown4               as u8 }
            if let Some(unknown5               ) = config.unknown5               { property_data.data.unknown5               = unknown5               as u8 }
            if let Some(disable_unmorph        ) = config.disable_unmorph        { property_data.data.disable_unmorph        = disable_unmorph        as u8 }
            if let Some(disable_morph          ) = config.disable_morph          { property_data.data.disable_morph          = disable_morph          as u8 }
            if let Some(disable_controls       ) = config.disable_controls       { property_data.data.disable_controls       = disable_controls       as u8 }
            if let Some(disable_boost          ) = config.disable_boost          { property_data.data.disable_boost          = disable_boost          as u8 }
            if let Some(activate_visor_combat  ) = config.activate_visor_combat  { property_data.data.activate_visor_combat  = activate_visor_combat  as u8 }
            if let Some(activate_visor_scan    ) = config.activate_visor_scan    { property_data.data.activate_visor_scan    = activate_visor_scan    as u8 }
            if let Some(activate_visor_thermal ) = config.activate_visor_thermal { property_data.data.activate_visor_thermal = activate_visor_thermal as u8 }
            if let Some(activate_visor_xray    ) = config.activate_visor_xray    { property_data.data.activate_visor_xray    = activate_visor_xray    as u8 }
            if let Some(unknown6               ) = config.unknown6               { property_data.data.unknown6               = unknown6               as u8 }
            if let Some(face_object_on_unmorph ) = config.face_object_on_unmorph { property_data.data.face_object_on_unmorph = face_object_on_unmorph as u8 }
        };
    }

    add_edit_obj_helper!(area, Some(config.id), config.layer, PlayerHint, new, update);
}

pub fn patch_add_platform<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    config: PlatformConfig,
) -> Result<(), String>
{
    let platform_type = {
        match config.platform_type {
            Some(platform_type) => platform_type,
            None => {
                if config.alt_platform.unwrap_or(false) {
                    PlatformType::Snow
                } else {
                    PlatformType::Metal
                }
            }
        }
    };

    let (deps, cmdl, dcln) = {
        match platform_type {
            PlatformType::Snow => {
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
            },
            PlatformType::Metal => {
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
        }
    };

    let deps_iter = deps.iter()
        .map(|&(file_id, fourcc)| structs::Dependency {
            asset_id: file_id,
            asset_type: FourCC::from_bytes(fourcc),
        }
    );
    area.add_dependencies(game_resources,0,deps_iter);

    macro_rules! new {
        () => {
            structs::Platform {
                name: b"myplatform\0".as_cstr(),

                position: config.position.into(),
                rotation: config.rotation.unwrap_or([0.0, 0.0, 0.0]).into(),
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
            }
        };
    }

    macro_rules! update {
        ($obj:expr) => {
            let property_data = $obj.property_data.as_platform_mut().unwrap();

            if config.platform_type.is_some() {
                property_data.cmdl = cmdl;
                property_data.dcln = dcln;
            }

            property_data.position = config.position.into();

            if let Some(rotation) = config.rotation {
                property_data.rotation = rotation.into();
            }
        };
    }

    add_edit_obj_helper!(area, config.id, config.layer, Platform, new, update);
}

pub fn patch_add_block<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    config: BlockConfig,
    old_scale: bool,
) -> Result<(), String>
{
    let texture = config.texture.unwrap_or(GenericTexture::Grass);

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
        config.id,
        config.position,
        config.scale.unwrap_or([1.0, 1.0, 1.0]),
        texture,
        1,
        config.layer,
        config.active.unwrap_or(true),
        old_scale,
    );

    Ok(())
}

pub fn add_block<'r>(
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    id: Option<u32>,
    position: [f32;3],
    scale: [f32;3],
    texture: GenericTexture,
    is_tangible: u8,
    layer: Option<u32>,
    active: bool,
    old_scale: bool,
)
{
    let layer_id = layer.unwrap_or(0);

    let scale = match old_scale {
        true => {
            scale
        },
        false => {
            [scale[0]*0.587, scale[1]*0.587, scale[2]*0.587]
        }
    };

    let actor_id = match id {
        Some(id) => id,
        None => area.new_object_id_from_layer_id(layer_id as usize),
    };

    while area.layer_flags.layer_count <= layer_id {
        area.add_layer(b"New Layer\0".as_cstr());
    }

    let scly = area.mrea().scly_section_mut();
    let objects = &mut scly.layers.as_mut_vec()[layer_id as usize].objects.as_mut_vec();

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
                active: active as u8,
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
