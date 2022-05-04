use encoding::{
    all::WINDOWS_1252,
    Encoding,
    EncoderTrap,
};

use rand::{
    rngs::StdRng,
    seq::SliceRandom,
    SeedableRng,
    Rng,
};

use crate::patch_config::{
    RunMode,
    ArtifactHintBehavior,
    Visor,
    MapState,
    IsoFormat,
    PickupConfig,
    PatchConfig,
    GameBanner,
    LevelConfig,
    RoomConfig,
    CtwkConfig,
    CutsceneMode,
    DoorConfig,
    WaterConfig,
};

use std::{fs::{self, File}, io::{Read}, path::Path};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::{
    custom_assets::{custom_asset_ids, collect_game_resources, PickupHashKey},
    dol_patcher::DolPatcher,
    ciso_writer::CisoWriter,
    elevators::{Elevator, SpawnRoom, SpawnRoomData, World, is_elevator},
    gcz_writer::GczWriter,
    mlvl_wrapper,
    pickup_meta::{self, PickupType, PickupModel, DoorLocation},
    door_meta::{DoorType, BlastShieldType},
    patcher::{PatcherState, PrimePatcher},
    starting_items::StartingItems,
    txtr_conversions::{
        cmpr_compress,
        cmpr_decompress,
        huerotate_color,
        huerotate_matrix,
        huerotate_in_place,
        POWER_SUIT_TEXTURES,
        VARIA_SUIT_TEXTURES,
        GRAVITY_SUIT_TEXTURES,
        PHAZON_SUIT_TEXTURES,
    },
    GcDiscLookupExtensions,
    extern_assets::ExternPickupModel,
};

use dol_symbol_table::mp1_symbol;
use resource_info_table::{resource_info, ResourceInfo};
use ppcasm::ppcasm;

use reader_writer::{
    generic_array::GenericArray,
    typenum::U3,
    CStrConversionExtension,
    FourCC,
    Reader,
    // Readable,
    Writable,
};
use structs::{res_id, ResId};

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    convert::TryInto,
    ffi::CString,
    fmt,
    io::Write,
    iter,
    mem,
    time::Instant,
};

const ARTIFACT_OF_TRUTH_REQ_LAYER: u32 = 24;

fn artifact_layer_change_template<'r>(instance_id: u32, pickup_kind: u32)
    -> structs::SclyObject<'r>
{
    let layer = if pickup_kind > 29 {
        pickup_kind - 28
    } else {
        assert!(pickup_kind == 29);
        ARTIFACT_OF_TRUTH_REQ_LAYER
    };
    structs::SclyObject {
        instance_id,
        connections: vec![].into(),
        property_data: structs::SpecialFunction::layer_change_fn(
            b"Artifact Layer Switch\0".as_cstr(),
            0xCD2B0EA2,
            layer
        ).into(),
    }
}

fn post_pickup_relay_template<'r>(instance_id: u32, connections: &'static [structs::Connection])
    -> structs::SclyObject<'r>
{
    structs::SclyObject {
        instance_id,
        connections: connections.to_owned().into(),
        property_data: structs::Relay {
            name: b"Randomizer Post Pickup Relay\0".as_cstr(),
            active: 1,
        }.into(),
    }
}

fn build_artifact_temple_totem_scan_strings<R>(
    level_data: &HashMap<String, LevelConfig>,
    rng: &mut R,
    artifact_hints: Option<HashMap<String,String>>,

)
    -> [String; 12]
    where R: Rng
{
    let mut generic_text_templates = [
        "I mean, maybe it'll be in &push;&main-color=#43CD80;{room}&pop;. I forgot, to be honest.\0",
        "I'm not sure where the artifact exactly is, but like, you can try &push;&main-color=#43CD80;{room}&pop;.\0",
        "Hey man, some of the Chozo are telling me that there might be a thing in &push;&main-color=#43CD80;{room}&pop;. Just sayin'.\0",
        "Uhh umm... Where was it...? Uhhh, errr, it's definitely in &push;&main-color=#43CD80;{room}&pop;! I am 100% not totally making it up...\0",
        "Some say it may be in &push;&main-color=#43CD80;{room}&pop;. Others say that you have no business here. Please leave me alone.\0",
        "A buddy and I were drinking and thought 'Hey, wouldn't be crazy if we put it in &push;&main-color=#43CD80;{room}&pop;?' It took both of us just to put it there!\0",
        "So, uhhh, I kind of got lazy and just dropped mine somewhere... Maybe it's in the &push;&main-color=#43CD80;{room}&pop;? Who knows.\0",
        "I was super late and someone had to cover for me. She said she put it in &push;&main-color=#43CD80;{room}&pop;, so you'll just have to trust her.\0",
        "Okay, so this jerk forgets to hide his so I had to hide two. This is literally saving the planet. Anyways, mine is in &push;&main-color=#43CD80;{room}&pop;.\0",
        "To be honest, I don't really remember. I think it was... um... yeah we'll just go with that: It was &push;&main-color=#43CD80;{room}&pop;.\0",
        "Hear the words of Oh Leer, last Chozo of the Artifact Temple. May they serve you... Alright, whatever. It's in &push;&main-color=#43CD80;{room}&pop;.\0",
        "I kind of just played Frisbee with mine. It flew too far and I didn't see where it landed. Somewhere in &push;&main-color=#43CD80;{room}&pop;.\0",
    ];
    generic_text_templates.shuffle(rng);
    let mut generic_templates_iter = generic_text_templates.iter();

    // Where are the artifacts?
    let mut artifact_locations = Vec::<(&str, PickupType)>::new();
    for (_, level) in level_data.iter() {
        for (room_name, room) in level.rooms.iter() {
            if room.pickups.is_none() { continue };
            for pickup in room.pickups.as_ref().unwrap().iter() {
                let pickup_type = PickupType::from_str(&pickup.pickup_type);
                if pickup_type.kind() >= PickupType::ArtifactOfLifegiver.kind() && pickup_type.kind() <= PickupType::ArtifactOfStrength.kind() {
                    artifact_locations.push((&room_name.as_str(), pickup_type));
                }
            }
        }
    }

    // TODO: If there end up being a large number of these, we could use a binary search
    //       instead of searching linearly.
    // XXX It would be nice if we didn't have to use Vec here and could allocated on the stack
    //     instead, but there doesn't seem to be a way to do it that isn't extremely painful or
    //     relies on unsafe code.
    let mut specific_room_templates = [
        ("Artifact Temple", vec!["{pickup} awaits those who truly seek it.\0"]),
    ];
    for rt in &mut specific_room_templates {
        rt.1.shuffle(rng);
    }

    let mut scan_text = [
        String::new(), String::new(), String::new(), String::new(),
        String::new(), String::new(), String::new(), String::new(),
        String::new(), String::new(), String::new(), String::new(),
    ];

    // Shame there isn't a way to flatten tuples automatically
    for (room_name, pt) in artifact_locations.iter() {
        let artifact_id = (pt.kind() - PickupType::ArtifactOfLifegiver.kind()) as usize;
        if scan_text[artifact_id].len() != 0 {
            // If there are multiple of this particular artifact, then we use the first instance
            // for the location of the artifact.
            continue;
        }

        // If there are specific messages for this room, choose one, otherwise choose a generic
        // message.
        let template = specific_room_templates.iter_mut()
            .find(|row| &row.0 == room_name)
            .and_then(|row| row.1.pop())
            .unwrap_or_else(|| generic_templates_iter.next().unwrap());
        let pickup_name = pt.name();
        scan_text[artifact_id] = template.replace("{room}", room_name).replace("{pickup}", pickup_name);
    }

    // Set a default value for any artifacts that we didn't find.
    for i in 0..scan_text.len() {
        if scan_text[i].len() == 0 {
            scan_text[i] = "Artifact not present. This layout may not be completable.\0".to_owned();
        }
    }

    if artifact_hints.is_some() {
        for (artifact_name, hint) in artifact_hints.unwrap() {
            let words: Vec<&str> = artifact_name.split(" ").collect();
            let lastword = words[words.len() - 1];
            let idx = match lastword.trim().to_lowercase().as_str() {
                "lifegiver" => 0,
                "wild"      => 1,
                "world"     => 2,
                "sun"       => 3,
                "elder"     => 4,
                "spirit"    => 5,
                "truth"     => 6,
                "chozo"     => 7,
                "warrior"   => 8,
                "newborn"   => 9,
                "nature"    => 10,
                "strength"  => 11,
                _ => panic!("Error - Unknown artifact - '{}'", artifact_name)
            };
            scan_text[idx] = format!("{}\0",hint.to_owned());
        }
    }

    scan_text
}

fn patch_artifact_totem_scan_strg(res: &mut structs::Resource, text: &str)
    -> Result<(), String>
{
    let strg = res.kind.as_strg_mut().unwrap();
    for st in strg.string_tables.as_mut_vec().iter_mut() {
        let strings = st.strings.as_mut_vec();
        *strings.last_mut().unwrap() = text.to_owned().into();
    }
    Ok(())
}

fn patch_save_banner_txtr(res: &mut structs::Resource)
    -> Result<(), String>
{
    const TXTR_BYTES: &[u8] = include_bytes!("../extra_assets/save_banner.txtr");
    res.compressed = false;
    res.kind = structs::ResourceKind::Unknown(Reader::new(TXTR_BYTES), b"TXTR".into());
    Ok(())
}

fn patch_tournament_winners<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
)
-> Result<(), String>
{
    let frme_id = ResId::<res_id::FRME>::new(0xDCEC3E77);

    let scan_dep: structs::Dependency = custom_asset_ids::TOURNEY_WINNERS_SCAN.into();
    area.add_dependencies(game_resources, 0, iter::once(scan_dep));

    let strg_dep: structs::Dependency = custom_asset_ids::TOURNEY_WINNERS_STRG.into();
    area.add_dependencies(game_resources, 0, iter::once(strg_dep));

    let frme_dep: structs::Dependency = frme_id.into();
    area.add_dependencies(game_resources, 0, iter::once(frme_dep));

    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    let poi = layer.objects.iter_mut()
            .find(|obj| obj.instance_id&0x00FFFFFF == 0x00100340)
            .and_then(|obj| obj.property_data.as_point_of_interest_mut())
            .unwrap();
    poi.scan_param.scan = custom_asset_ids::TOURNEY_WINNERS_SCAN;
    Ok(())
}

fn patch_thermal_conduits_damage_vulnerabilities(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];

    let thermal_conduit_damageable_trigger_obj_ids = [
        0x000F01C8, // ruined courtyard
        0x0028043F, // research core
        0x0015006C, // main ventilation shaft section b
        0x0019002C, // reactor core
        0x00190030, // reactor core
        0x0019002E, // reactor core
        0x00190029, // reactor core
        0x001A006C, // reactor core access
        0x001A006D, // reactor core access
        0x001B008E, // cargo freight lift to deck gamma
        0x001B008F, // cargo freight lift to deck gamma
        0x001B0090, // cargo freight lift to deck gamma
        0x001E01DC, // biohazard containment
        0x001E01E1, // biohazard containment
        0x001E01E0, // biohazard containment
        0x0020002A, // biotech research area 1
        0x00200030, // biotech research area 1
        0x0020002E, // biotech research area 1
        0x0002024C, // main quarry
        0x00170141, // magmoor workstation
        0x00170142, // magmoor workstation
        0x00170143, // magmoor workstation
    ];

    for obj in layer.objects.as_mut_vec().iter_mut() {
        if thermal_conduit_damageable_trigger_obj_ids.contains(&obj.instance_id) {
            let dt = obj.property_data.as_damageable_trigger_mut().unwrap();
            dt.damage_vulnerability = DoorType::Blue.vulnerability();
            dt.health_info.health = 1.0; // single power beam shot
        }
    }

    Ok(())
}

fn is_door_lock<'r>(obj: &structs::SclyObject<'r>) -> bool {
    let actor = obj.property_data.as_actor();

    if actor.is_none() {
        false // non-actors are never door locks
    }
    else {
        let _actor = actor.unwrap();
        _actor.cmdl == 0x5391EDB6 || _actor.cmdl == 0x6E5D6796 // door locks are indentified by their model (check for both horizontal and vertical variants)
    }
}

fn remove_door_locks(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    layer.objects.as_mut_vec().retain(|obj| !is_door_lock(obj));  // keep everything that isn't a door lock

    Ok(())
}

fn patch_morphball_hud(res: &mut structs::Resource)
    -> Result<(), String>
{
    let frme = res.kind.as_frme_mut().unwrap();
    let (jpn_font, jpn_point_scale) = if frme.version == 0 {
        (None, None)
    } else {
        (Some(resource_info!("Deface18B.FONT").try_into().unwrap()), Some([50, 24].into()))
    };
    let widget = frme.widgets.iter_mut()
        .find(|widget| widget.name == b"textpane_bombdigits\0".as_cstr())
        .unwrap();
    // Use the version of Deface18 that has more than just numerical characters for the powerbomb
    // ammo counter
    match &mut widget.kind {
        structs::FrmeWidgetKind::TextPane(textpane) => {
            textpane.font = resource_info!("Deface18B.FONT").try_into().unwrap();
            textpane.jpn_font = jpn_font;
            textpane.jpn_point_scale = jpn_point_scale;
            textpane.word_wrap = 0;
        }
        _ => panic!("Widget \"textpane_bombdigits\" should be a TXPN"),
    }
    widget.origin[0] -= 0.1;

    // We need to shift all of the widgets in the bomb UI left so there's
    // room for the longer powerbomb ammo counter
    const BOMB_UI_WIDGET_NAMES: &[&[u8]] = &[
        b"model_bar",
        b"model_bombbrak0",
        b"model_bombdrop0",
        b"model_bombbrak1",
        b"model_bombdrop1",
        b"model_bombbrak2",
        b"model_bombdrop2",
        b"model_bombicon",
    ];
    for widget in frme.widgets.iter_mut() {
        if !BOMB_UI_WIDGET_NAMES.contains(&widget.name.to_bytes()) {
            continue;
        }
        widget.origin[0] -= 0.325;
    }
    Ok(())
}

fn patch_add_scans_to_savw(
    res: &mut structs::Resource,
    savw_scans_to_add: &Vec<ResId<res_id::SCAN>>,
    savw_scan_logbook_category: &HashMap::<u32, u32>,
    scan_ids_to_remove: &Vec<u32>,
)
    -> Result<(), String>
{
    let savw = res.kind.as_savw_mut().unwrap();
    savw.cinematic_skip_array.as_mut_vec().clear();
    let scan_array = savw.scan_array.as_mut_vec();

    for entry in scan_array.iter_mut() {
        if scan_ids_to_remove.contains(&entry.scan.to_u32()) {
            entry.logbook_category = 0;
        }
    }

    for scan_id in savw_scans_to_add {
        scan_array.push(structs::ScannableObject {
            scan: ResId::<res_id::SCAN>::new(scan_id.to_u32()),
            logbook_category: *savw_scan_logbook_category.get(&scan_id.to_u32()).unwrap(),
        });
    }

    // Danger level is about 5,000
    // println!("size={}", res.size());

    Ok(())
}

fn patch_map_door_icon(
    res: &mut structs::Resource,
    door: DoorLocation,
    map_object_type: u32,
) -> Result<(), String>
{
    if door.door_location.is_none() {
        println!("Warning, no door location to patch map for");
        return Ok(());
    }

    let mapa = res.kind.as_mapa_mut().unwrap();

    let door_icon = mapa.objects.iter_mut()
        .find(|obj| obj.editor_id == door.door_location.as_ref().unwrap().instance_id)
        .unwrap();
    door_icon.type_ = map_object_type;

    Ok(())
}

fn patch_door<'r>(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    door_loc: DoorLocation,
    door_type: Option<DoorType>,
    blast_shield_type: Option<BlastShieldType>,
    door_resources:&HashMap<(u32, FourCC), structs::Resource<'r>>,
) -> Result<(), String> {
    let mrea_id = area.mlvl_area.mrea.to_u32();
    if door_loc.door_force_locations.len() == 0 || door_loc.door_shield_locations.len() == 0 {
        panic!("Tried to change vulnerability/blast shield of door without a damageable shield in room 0x{:X}", mrea_id);
    }

    // Update dependencies based on the upcoming patch(es)
    let mut deps: Vec<(u32, FourCC)> = Vec::new();

    if door_type.is_some() {
        deps.extend_from_slice(&door_type.as_ref().unwrap().dependencies());
    }

    let mut blast_shield_layer_idx = 0;
    if blast_shield_type.is_some() {
        // Create new layer to store the new blast shield //
        area.add_layer(b"Custom Shield Layer\0".as_cstr());
        blast_shield_layer_idx = area.layer_flags.layer_count as usize - 1;

        // Add dependencies
        deps.extend_from_slice(&blast_shield_type.as_ref().unwrap().dependencies());
    }
    let deps_iter = deps.iter()
        .map(|&(file_id, fourcc)| structs::Dependency {
                asset_id: file_id,
                asset_type: fourcc,
        }
    );

    area.add_dependencies(&door_resources, 0, deps_iter);

    let area_internal_id = area.mlvl_area.internal_id;
    let scly = area.mrea().scly_section_mut();
    let layers = &mut scly.layers.as_mut_vec();

    // Add blast shield
    if blast_shield_type.is_some() {
        let door_shield_location = door_loc.door_shield_locations[0];
        let door_shield = layers[door_shield_location.layer as usize].objects.iter_mut()
            .find(|obj| obj.instance_id == door_shield_location.instance_id)
            .and_then(|obj| obj.property_data.as_actor_mut())
            .unwrap();

        let is_vertical = DoorType::from_cmdl(&door_shield.cmdl.to_u32()).unwrap().is_vertical();

        let special_function_id = ps.fresh_instance_id_range.next().unwrap();
        let blast_shield_instance_id = ps.fresh_instance_id_range.next().unwrap();

        let blast_shield_type = blast_shield_type.as_ref().unwrap();

        // Calculate placement //
        let position: GenericArray<f32, U3>;
        let rotation: GenericArray<f32, U3>;
        let scale: GenericArray<f32, U3> = [1.0, 1.5, 1.5].into();
        let hitbox: GenericArray<f32, U3>;
        let scan_offset: GenericArray<f32, U3>;

        if door_shield.rotation[2] >= 45.0 && door_shield.rotation[2] < 135.0 {
            // Leads North
            position    = [door_shield.position[0], door_shield.position[1] + 0.2, door_shield.position[2] - 1.8017].into();
            rotation    = [door_shield.rotation[0], door_shield.rotation[1], door_shield.rotation[2]].into();
            hitbox      = [5.0, 0.5, 4.0].into();
            scan_offset = [0.0, -1.0, 2.0].into();

        } else if (door_shield.rotation[2] >= 135.0 && door_shield.rotation[2] < 225.0) || (door_shield.rotation[2] < -135.0 && door_shield.rotation[2] > -225.0) {
            // Leads East
            position    = [door_shield.position[0] + 0.2, door_shield.position[1], door_shield.position[2] - 1.8017].into();
            rotation    = [door_shield.rotation[0], door_shield.rotation[1], 0.0].into();
            hitbox      = [0.5, 5.0, 4.0].into();
            scan_offset = [-1.0, 0.0, 2.0].into();

        } else if door_shield.rotation[2] >= -135.0 && door_shield.rotation[2] < -45.0 {
            // Leads South
            position    = [door_shield.position[0], door_shield.position[1] - 0.2, door_shield.position[2] - 1.8017].into();
            rotation    = [door_shield.rotation[0], door_shield.rotation[1], door_shield.rotation[2]].into();
            hitbox      = [5.0, 0.5, 4.0].into();
            scan_offset = [0.0, 1.0, 2.0].into();

        } else if door_shield.rotation[2] >= -45.0 && door_shield.rotation[2] < 45.0 {
            // Leads West
            position    = [door_shield.position[0] - 0.2, door_shield.position[1], door_shield.position[2] - 1.8017].into();
            rotation    = [door_shield.rotation[0], door_shield.rotation[1], -179.99].into();
            hitbox      = [0.5, 5.0, 4.0].into();
            scan_offset = [1.0, 0.0, 2.0].into();

        } else {
            assert!(false);
            position    = [0.0, 0.0, 0.0].into();
            rotation    = [0.0, 0.0, 0.0].into();
            hitbox      = [0.0, 0.0, 0.0].into();
            scan_offset = [0.0, 0.0, 0.0].into();
        }

        if is_vertical {
            panic!("Custom Blast Shields cannot be placed on vertical doors");
        }

        // Create new blast shield actor //
        let mut blast_shield = structs::SclyObject {
            instance_id: blast_shield_instance_id,
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::DEAD,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: blast_shield_instance_id,
                },
            ].into(),
            property_data: structs::SclyProperty::Actor(
                Box::new(structs::Actor {
                    name: b"Custom Blast Shield\0".as_cstr(),
                    position,
                    rotation,
                    scale,
                    hitbox,
                    scan_offset,
                    unknown1: 1.0, // mass
                    unknown2: 0.0, // momentum
                    health_info: structs::scly_structs::HealthInfo {
                        health: 1.0,
                        knockback_resistance: 1.0,
                    },
                    damage_vulnerability: blast_shield_type.vulnerability(),
                    cmdl: blast_shield_type.cmdl(),
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
                            color: [1.0, 1.0, 1.0, 1.0].into(), // RGBA
                            unknown4: 1,
                            world_lighting: 1,
                            light_recalculation: 1,
                            unknown5: [0.0, 0.0, 0.0].into(),
                            unknown6: 4,
                            unknown7: 4,
                            unknown8: 0,
                            light_layer_id: 0,
                        },
                        scan_params: structs::scly_structs::ScannableParameters {
                            scan: blast_shield_type.scan(),
                        },
                        xray_cmdl: ResId::invalid(),
                        xray_cskr: ResId::invalid(),
                        thermal_cmdl: ResId::invalid(),
                        thermal_cskr: ResId::invalid(),
                        unknown0: 1,
                        unknown1: 1.0,
                        unknown2: 1.0,
                        visor_params: structs::scly_structs::VisorParameters {
                            unknown0: 0,
                            target_passthrough: 0,
                            visor_mask: 15, // Visor Flags : Combat|Scan|Thermal|XRay
                        },
                        enable_thermal_heat: 0,
                        unknown3: 0,
                        unknown4: 0,
                        unknown5: 1.0,
                    },
                    looping: 1,
                    snow: 1, // immovable
                    solid: 1,
                    camera_passthrough: 0,
                    active: 1,
                    unknown8: 0,
                    unknown9: 1.0,
                    unknown10: 0,
                    unknown11: 0,
                    unknown12: 0,
                    unknown13: 0,
                })
            ),
        };

        // Create Special Function to disable layer once shield is destroyed
        // This is needed because otherwise the shield would re-appear every
        // time the room is loaded
        let special_function = structs::SclyObject {
            instance_id: special_function_id,
            connections: vec![].into(),
            property_data: structs::SclyProperty::SpecialFunction(
                Box::new(structs::SpecialFunction {
                    name: b"myspecialfun\0".as_cstr(),
                    position: [0., 0., 0.].into(),
                    rotation: [0., 0., 0.].into(),
                    type_: 16, // layer change
                    unknown0: b"\0".as_cstr(),
                    unknown1: 0.,
                    unknown2: 0.,
                    unknown3: 0.,
                    layer_change_room_id: area_internal_id,
                    layer_change_layer_id: blast_shield_layer_idx as u32,
                    item_id: 0,
                    unknown4: 1, // active
                    unknown5: 0.,
                    unknown6: 0xFFFFFFFF,
                    unknown7: 0xFFFFFFFF,
                    unknown8: 0xFFFFFFFF,
                })
            ),
        };

        // Activate the layer change when blast shield is destroyed
        blast_shield.connections.as_mut_vec().push(
            structs::Connection {
                state: structs::ConnectionState::DEAD,
                message: structs::ConnectionMsg::DECREMENT,
                target_object_id: special_function_id,
            }
        );

        let mut _break = false;
        for obj in layers[0].objects.as_mut_vec() {
            if _break { break; }
            if obj.property_data.is_door() {
                let connections = obj.connections.clone();
                for conn in connections.iter() {
                    if conn.target_object_id & 0x00FFFFFF == door_shield_location.instance_id & 0x00FFFFFF
                        && conn.message == structs::ConnectionMsg::DEACTIVATE {
                            if obj.property_data.as_door().unwrap().is_morphball_door != 0 {
                                panic!("Custom Blast Shields cannot be placed on morph ball doors");
                            }

                            // Activate the layer change when the door is opened from the other side
                            obj.connections.as_mut_vec().push(
                                structs::Connection {
                                    state: structs::ConnectionState::MAX_REACHED,
                                    message: structs::ConnectionMsg::DECREMENT,
                                    target_object_id: special_function_id,
                                }
                            );

                            // Remove the blast shield when the door is opened from the other side
                            obj.connections.as_mut_vec().push(
                                structs::Connection {
                                    state: structs::ConnectionState::MAX_REACHED,
                                    message: structs::ConnectionMsg::DEACTIVATE,
                                    target_object_id: blast_shield_instance_id,
                                }
                            );

                            _break = true;
                            break;
                        }
                    }
            }
        }

        // Create Gibbs and activate on DEAD //
        // TODO: It's possible, but there's so many goddam dependencies

        // Create camera shake and activate on DEAD //
        // TODO: It's possible, I'm just lazy

        // Create explosion sfx //
        let sound = structs::SclyObject {
            instance_id: ps.fresh_instance_id_range.next().unwrap(),
            connections: vec![].into(),
            property_data: structs::SclyProperty::Sound(
                Box::new(structs::Sound { // copied from main plaza half-pipe
                    name: b"mysound\0".as_cstr(),
                    position: [
                        position[0],
                        position[1],
                        position[2],
                    ].into(),
                    rotation: [0.0,0.0,0.0].into(),
                    sound_id: 3621,
                    active: 1,
                    max_dist: 100.0,
                    dist_comp: 0.2,
                    start_delay: 0.0,
                    min_volume: 20,
                    volume: 127,
                    priority: 127,
                    pan: 64,
                    loops: 0,
                    non_emitter: 0,
                    auto_start: 0,
                    occlusion_test: 0,
                    acoustics: 1,
                    world_sfx: 0,
                    allow_duplicates: 0,
                    pitch: 0,
                })
            )
        };

        // Blast shield triggers explosion sfx when dead //
        blast_shield.connections.as_mut_vec().push(
            structs::Connection {
                state: structs::ConnectionState::DEAD,
                message: structs::ConnectionMsg::PLAY,
                target_object_id: sound.instance_id,
            }
        );

        // Create "You did it" Jingle //
        let streamed_audio = structs::SclyObject {
            instance_id: ps.fresh_instance_id_range.next().unwrap(),
            connections: vec![].into(),
            property_data: structs::SclyProperty::StreamedAudio(
                Box::new(structs::StreamedAudio {
                    name: b"mystreamedaudio\0".as_cstr(),
                    active: 1,
                    audio_file_name: b"/audio/evt_x_event_00.dsp\0".as_cstr(),
                    no_stop_on_deactivate: 0,
                    fade_in_time: 0.0,
                    fade_out_time: 0.0,
                    volume: 92,
                    oneshot: 1,
                    is_music: 1,
                })
            ),
        };

        // Blast shield triggers jingle when dead //
        blast_shield.connections.as_mut_vec().push(
            structs::Connection {
                state: structs::ConnectionState::DEAD,
                message: structs::ConnectionMsg::PLAY,
                target_object_id: streamed_audio.instance_id,
            }
        );

        // add new script objects to layer //
        layers[0].objects.as_mut_vec().push(special_function);
        layers[blast_shield_layer_idx].objects.as_mut_vec().push(streamed_audio);
        layers[blast_shield_layer_idx].objects.as_mut_vec().push(sound);
        layers[blast_shield_layer_idx].objects.as_mut_vec().push(blast_shield);
    }

    // Patch door vulnerability
    if door_type.is_some() {
        let _door_type = door_type.as_ref().unwrap();
        for door_force_location in door_loc.door_force_locations {
            let door_force = layers[door_force_location.layer as usize].objects.iter_mut()
                .find(|obj| obj.instance_id == door_force_location.instance_id)
                .and_then(|obj| obj.property_data.as_damageable_trigger_mut())
                .unwrap();
            door_force.color_txtr = _door_type.forcefield_txtr();
            door_force.damage_vulnerability = _door_type.vulnerability();
        }

        for door_shield_location in door_loc.door_shield_locations {
            let door_shield = layers[door_shield_location.layer as usize].objects.iter_mut()
                .find(|obj| obj.instance_id == door_shield_location.instance_id)
                .and_then(|obj| obj.property_data.as_actor_mut())
                .unwrap();
            door_shield.cmdl = _door_type.shield_cmdl();
        }
    }

    Ok(())
}

// TODO: factor out shared code with modify_pickups_in_mrea
fn patch_add_item<'r>(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    pickup_config: &PickupConfig,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    pickup_hudmemos: &HashMap<PickupHashKey, ResId<res_id::STRG>>,
    pickup_scans: &HashMap<PickupHashKey, (ResId<res_id::SCAN>, ResId<res_id::STRG>)>,
    pickup_hash_key: PickupHashKey,
    skip_hudmemos: bool,
    extern_models: &HashMap<String, ExternPickupModel>,
    shuffle_position: bool,
    seed: u64,
    _no_starting_visor: bool,
) -> Result<(), String>
{
    let mut rng = StdRng::seed_from_u64(seed);
    let room_id = area.mlvl_area.internal_id;

    // Pickup to use for game functionality //
    let pickup_type = PickupType::from_str(&pickup_config.pickup_type);

    let extern_model = if pickup_config.model.is_some() {
        extern_models.get(pickup_config.model.as_ref().unwrap())
    } else {
        None
    };

    // Pickup to use for visuals/hitbox //
    let pickup_model_type: Option<PickupModel> = {
        if pickup_config.model.is_some() {
            let model_name = pickup_config.model.as_ref().unwrap();
            let pmt = PickupModel::from_str(&model_name);
            if pmt.is_none() && !extern_model.is_some() {
                panic!("Unknown Model Type {}", model_name);
            }

            pmt // Some - Native Prime Model
                // None - External Model (e.g. Screw Attack)
        } else {
            Some(PickupModel::from_type(pickup_type)) // No model specified, use pickup type as inspiration
        }
    };

    let pickup_model_type = pickup_model_type.clone().unwrap_or(PickupModel::Nothing);
    let mut pickup_model_data = pickup_model_type.pickup_data();
    if extern_model.is_some() {
        let scale = extern_model.as_ref().unwrap().scale.clone();
        pickup_model_data.scale[0] = pickup_model_data.scale[0]*scale;
        pickup_model_data.scale[1] = pickup_model_data.scale[1]*scale;
        pickup_model_data.scale[2] = pickup_model_data.scale[2]*scale;
        pickup_model_data.cmdl = ResId::<res_id::CMDL>::new(extern_model.as_ref().unwrap().cmdl);
        pickup_model_data.ancs.file_id = ResId::<res_id::ANCS>::new(extern_model.as_ref().unwrap().ancs);
        pickup_model_data.part = ResId::invalid();
        pickup_model_data.ancs.node_index = extern_model.as_ref().unwrap().character;
        pickup_model_data.ancs.default_animation = 0;
        pickup_model_data.actor_params.xray_cmdl = ResId::invalid();
        pickup_model_data.actor_params.xray_cskr = ResId::invalid();
        pickup_model_data.actor_params.thermal_cmdl = ResId::invalid();
        pickup_model_data.actor_params.thermal_cskr = ResId::invalid();
    }

    let new_layer_idx = if area.layer_flags.layer_count < 60 {
        let name = CString::new(format!(
            "Randomizer - Pickup ({:?})", pickup_model_data.name)).unwrap();
        area.add_layer(Cow::Owned(name));
        area.layer_flags.layer_count as usize - 1
    } else {
        0
    };

    // Add hudmemo string as dependency to room //
    let hudmemo_strg: ResId<res_id::STRG> = {
        if pickup_config.hudmemo_text.is_some() {
            *pickup_hudmemos.get(&pickup_hash_key).unwrap()
        } else {
            pickup_type.hudmemo_strg()
        }
    };

    let hudmemo_dep: structs::Dependency = hudmemo_strg.into();
    area.add_dependencies(game_resources, new_layer_idx, iter::once(hudmemo_dep));

    /* Add Model Dependencies */
    // Dependencies are defined externally
    if extern_model.is_some() {
        let deps = extern_model.as_ref().unwrap().dependencies.clone();
        let deps_iter = deps.iter()
            .map(|&(file_id, fourcc)| structs::Dependency {
                asset_id: file_id,
                asset_type: fourcc,
            }
        );
        area.add_dependencies(game_resources, new_layer_idx, deps_iter);
    }
    // If we aren't using an external model, use the dependencies traced by resource_tracing
    else {
        let deps_iter = pickup_model_type
            .dependencies().iter()
            .map(|&(file_id, fourcc)| structs::Dependency {
                asset_id: file_id,
                asset_type: fourcc,
                }
            );
        area.add_dependencies(game_resources, new_layer_idx, deps_iter);
    }

    {
        let frme = ResId::<res_id::FRME>::new(0xDCEC3E77);
        let frme_dep: structs::Dependency = frme.into();
        area.add_dependencies(game_resources, new_layer_idx, iter::once(frme_dep));
    }
    let scan_id = {
        if pickup_config.scan_text.is_some() {
            let (scan, strg) = *pickup_scans.get(&pickup_hash_key).unwrap();

            let scan_dep: structs::Dependency = scan.into();
            area.add_dependencies(game_resources, new_layer_idx, iter::once(scan_dep));

            let strg_dep: structs::Dependency = strg.into();
            area.add_dependencies(game_resources, new_layer_idx, iter::once(strg_dep));

            scan
        }
        else
        {
            let scan_dep: structs::Dependency = pickup_type.scan().into();
            area.add_dependencies(game_resources, new_layer_idx, iter::once(scan_dep));

            let strg_dep: structs::Dependency = pickup_type.scan_strg().into();
            area.add_dependencies(game_resources, new_layer_idx, iter::once(strg_dep));

            pickup_type.scan()
        }
    };

    let curr_increase = {
        if pickup_type == PickupType::Nothing {
            0
        } else {
            if pickup_config.curr_increase.is_some() {
                pickup_config.curr_increase.unwrap()
            } else {
                if pickup_type == PickupType::Missile {
                    5
                } else if pickup_type == PickupType::HealthRefill {
                    50
                } else {
                    1
                }
            }
        }
    };
    let max_increase = {
        if pickup_type == PickupType::Nothing || pickup_type == PickupType::HealthRefill {
            0
        } else {
            pickup_config.max_increase.unwrap_or(curr_increase)
        }
    };
    let kind = {
        if pickup_type == PickupType::Nothing {
            PickupType::HealthRefill.kind()
        } else {
            pickup_type.kind()
        }
    };

    let mut pickup_position = {
        if shuffle_position {
            get_shuffled_position(area, &mut rng)
        } else {
            if pickup_config.position.is_none() {
                panic!("Position is required for additional pickup in room '0x{:X}'", pickup_hash_key.room_id);
            }

            pickup_config.position.unwrap()
        }
    };

    let mut scan_offset = pickup_model_data.scan_offset.clone();

    // If this is the echoes missile expansion model, compensate for the Z offset
    let json_pickup_name = pickup_config.model.as_ref().unwrap_or(&"".to_string()).clone();
    if json_pickup_name.contains(&"prime2_MissileExpansion") || json_pickup_name.contains(&"prime2_UnlimitedMissiles") {
        pickup_position[2] -= 1.2;
        scan_offset[2] += 1.2;
    }

    let mut pickup = structs::Pickup {
        // Location Pickup Data
        // "How is this pickup integrated into the room?"
        name: b"customItem\0".as_cstr(),
        position: pickup_position.into(),
        rotation: [0.0, 0.0, 0.0].into(),
        hitbox: pickup_model_data.hitbox.clone(),
        scan_offset,
        fade_in_timer: 0.0,
        spawn_delay: 0.0,
        disappear_timer: 0.0,
        active: 1,
        drop_rate: 100.0,

        // Type Pickup Data
        // "What does this pickup do?"
        curr_increase,
        max_increase,
        kind,

        // Model Pickup Data
        // "What does this pickup look like?"
        scale: pickup_model_data.scale.clone(),
        cmdl: pickup_model_data.cmdl.clone(),
        ancs: pickup_model_data.ancs.clone(),
        part: pickup_model_data.part.clone(),
        actor_params: pickup_model_data.actor_params.clone(),
    };

    // set the scan file id //
    pickup.actor_params.scan_params.scan = scan_id;

    let mut pickup_obj = structs::SclyObject {
        instance_id: ps.fresh_instance_id_range.next().unwrap(),
        connections: vec![].into(),
        property_data: structs::SclyProperty::Pickup(
            Box::new(pickup)
        )
    };

    let hudmemo = structs::SclyObject {
        instance_id: ps.fresh_instance_id_range.next().unwrap(),
        connections: vec![].into(),
        property_data: structs::SclyProperty::HudMemo(
            Box::new(structs::HudMemo {
                name: b"myhudmemo\0".as_cstr(),
                first_message_timer: {
                    if skip_hudmemos {
                        5.0
                    } else {
                        3.0
                    }
                },
                unknown: 1,
                memo_type: {
                    if skip_hudmemos {
                        0
                    } else {
                        1
                    }
                },
                strg: hudmemo_strg,
                active: 1,
            })
        )
    };

    // Display hudmemo when item is picked up
    pickup_obj.connections.as_mut_vec().push(
        structs::Connection {
            state: structs::ConnectionState::ARRIVED,
            message: structs::ConnectionMsg::SET_TO_ZERO,
            target_object_id: hudmemo.instance_id,
        }
    );

    // create attainment audio
    let attainment_audio = structs::SclyObject {
        instance_id: ps.fresh_instance_id_range.next().unwrap(),
        connections: vec![].into(),
        property_data: structs::SclyProperty::Sound(
            Box::new(structs::Sound { // copied from main plaza half-pipe
                name: b"mysound\0".as_cstr(),
                position: pickup_position.into(),
                rotation: [0.0,0.0,0.0].into(),
                sound_id: 117,
                active: 1,
                max_dist: 50.0,
                dist_comp: 0.2,
                start_delay: 0.0,
                min_volume: 20,
                volume: 127,
                priority: 127,
                pan: 64,
                loops: 0,
                non_emitter: 1,
                auto_start: 0,
                occlusion_test: 0,
                acoustics: 0,
                world_sfx: 0,
                allow_duplicates: 0,
                pitch: 0,
            })
        )
    };

    // Play the sound when item is picked up
    pickup_obj.connections.as_mut_vec().push(
        structs::Connection {
            state: structs::ConnectionState::ARRIVED,
            message: structs::ConnectionMsg::PLAY,
            target_object_id: attainment_audio.instance_id,
        }
    );

    // 2022-02-08 - I had to remove this because there's a bug in the vanilla engine where playerhint -> Scan Visor doesn't holster the weapon
    // // If scan visor, and starting visor is none, then switch to combat and back to scan when obtaining scan
    // let player_hint_id = ps.fresh_instance_id_range.next().unwrap();
    // let player_hint = structs::SclyObject {
    //     instance_id: player_hint_id,
    //         property_data: structs::PlayerHint {
    //         name: b"combat playerhint\0".as_cstr(),
    //         position: [0.0, 0.0, 0.0].into(),
    //         rotation: [0.0, 0.0, 0.0].into(),
    //         unknown0: 1, // active
    //         inner_struct: structs::PlayerHintStruct {
    //             unknowns: [
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 1,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //             ].into(),
    //         }.into(),
    //         unknown1: 10, // priority
    //         }.into(),
    //         connections: vec![].into(),
    // };

    // pickup_obj.connections.as_mut_vec().push(
    //     structs::Connection {
    //         state: structs::ConnectionState::ARRIVED,
    //         message: structs::ConnectionMsg::INCREMENT,
    //         target_object_id: player_hint_id,
    //     }
    // );

    // let player_hint_id_2 = ps.fresh_instance_id_range.next().unwrap();
    // let player_hint_2 = structs::SclyObject {
    //     instance_id: player_hint_id_2,
    //         property_data: structs::PlayerHint {
    //         name: b"combat playerhint\0".as_cstr(),
    //         position: [0.0, 0.0, 0.0].into(),
    //         rotation: [0.0, 0.0, 0.0].into(),
    //         unknown0: 1, // active
    //         inner_struct: structs::PlayerHintStruct {
    //             unknowns: [
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 1,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //             ].into(),
    //         }.into(),
    //         unknown1: 10, // priority
    //         }.into(),
    //         connections: vec![].into(),
    // };

    // let timer_id = ps.fresh_instance_id_range.next().unwrap();
    // let timer = structs::SclyObject {
    //     instance_id: timer_id,
    //     property_data: structs::Timer {
    //         name: b"set-scan\0".as_cstr(),
    //         start_time: 0.5,
    //         max_random_add: 0.0,
    //         reset_to_zero: 0,
    //         start_immediately: 0,
    //         active: 1,
    //     }.into(),
    //     connections: vec![
    //         structs::Connection {
    //             state: structs::ConnectionState::ZERO,
    //             message: structs::ConnectionMsg::INCREMENT,
    //             target_object_id: player_hint_id_2,
    //         },
    //     ].into(),
    // };

    // pickup_obj.connections.as_mut_vec().push(
    //     structs::Connection {
    //         state: structs::ConnectionState::ARRIVED,
    //         message: structs::ConnectionMsg::RESET_AND_START,
    //         target_object_id: timer_id,
    //     }
    // );

    // update MREA layer with new Objects
    let scly = area.mrea().scly_section_mut();
    let layers = scly.layers.as_mut_vec();

    if shuffle_position || *pickup_config.jumbo_scan.as_ref().unwrap_or(&false) {
        let poi_id = ps.fresh_instance_id_range.next().unwrap();
        layers[new_layer_idx].objects.as_mut_vec().push(
            structs::SclyObject {
                instance_id: poi_id,
                connections: vec![].into(),
                property_data: structs::SclyProperty::PointOfInterest(
                    Box::new(structs::PointOfInterest {
                        name: b"mypoi\0".as_cstr(),
                        position: pickup_position.into(),
                        rotation: [0.0, 0.0, 0.0].into(),
                        active: 1,
                        scan_param: structs::scly_structs::ScannableParameters {
                            scan: scan_id,
                        },
                        point_size: 500.0,
                    })
                ),
            }
        );

        pickup_obj.connections.as_mut_vec().push(
            structs::Connection {
                state: structs::ConnectionState::ARRIVED,
                message: structs::ConnectionMsg::DEACTIVATE,
                target_object_id: poi_id,
            }
        );
    }

    // If this is an artifact, create and push change function
    let pickup_kind = pickup_type.kind();
    if pickup_kind >= 29 && pickup_kind <= 40 {
        let instance_id = ps.fresh_instance_id_range.next().unwrap();
        let function = artifact_layer_change_template(instance_id, pickup_kind);
        layers[new_layer_idx].objects.as_mut_vec().push(function);
        pickup_obj.connections.as_mut_vec().push(
            structs::Connection {
                state: structs::ConnectionState::ARRIVED,
                message: structs::ConnectionMsg::INCREMENT,
                target_object_id: instance_id,
            }
        );
    }

    if !pickup_config.respawn.unwrap_or(false) && new_layer_idx != 0 {
        // Create Special Function to disable layer once item is obtained
        // This is needed because otherwise the item would re-appear every
        // time the room is loaded
        let special_function = structs::SclyObject {
            instance_id: ps.fresh_instance_id_range.next().unwrap(),
            connections: vec![].into(),
            property_data: structs::SclyProperty::SpecialFunction(
                Box::new(structs::SpecialFunction {
                    name: b"myspecialfun\0".as_cstr(),
                    position: [0., 0., 0.].into(),
                    rotation: [0., 0., 0.].into(),
                    type_: 16, // layer change
                    unknown0: b"\0".as_cstr(),
                    unknown1: 0.,
                    unknown2: 0.,
                    unknown3: 0.,
                    layer_change_room_id: room_id,
                    layer_change_layer_id: new_layer_idx as u32,
                    item_id: 0,
                    unknown4: 1, // active
                    unknown5: 0.,
                    unknown6: 0xFFFFFFFF,
                    unknown7: 0xFFFFFFFF,
                    unknown8: 0xFFFFFFFF,
                })
            ),
        };

        // Activate the layer change when item is picked up
        pickup_obj.connections.as_mut_vec().push(
            structs::Connection {
                state: structs::ConnectionState::ARRIVED,
                message: structs::ConnectionMsg::DECREMENT,
                target_object_id: special_function.instance_id,
            }
        );

        layers[new_layer_idx].objects.as_mut_vec().push(special_function);
    }

    layers[new_layer_idx].objects.as_mut_vec().push(hudmemo);
    layers[new_layer_idx].objects.as_mut_vec().push(attainment_audio);
    layers[new_layer_idx].objects.as_mut_vec().push(pickup_obj);

    // 2022-02-08 - I had to remove this because there's a bug in the vanilla engine where playerhint -> Scan Visor doesn't holster the weapon
    // if pickup_type == PickupType::ScanVisor && no_starting_visor{
    //     layers[new_layer_idx].objects.as_mut_vec().push(player_hint);
    //     layers[new_layer_idx].objects.as_mut_vec().push(player_hint_2);
    //     layers[new_layer_idx].objects.as_mut_vec().push(timer);
    // }

    Ok(())
}

fn is_area_damage_special_function<'r>(obj: &structs::SclyObject<'r>)
-> bool
{
    let special_function = obj.property_data.as_special_function();

    if special_function.is_none() {
        false
    }
    else {
        special_function.unwrap().type_ == 18 // is area damage type
    }
}

fn patch_deheat_room<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer_count = scly.layers.len();
    for i in 0..layer_count {
        let layer = &mut scly.layers.as_mut_vec()[i];
        layer.objects.as_mut_vec().retain(|obj| !is_area_damage_special_function(obj));
    }

    Ok(())
}

fn patch_superheated_room<'r>(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    heat_damage_per_sec: f32,
)
-> Result<(), String>
{
    let area_damage_special_function = structs::SclyObject
    {
        instance_id: ps.fresh_instance_id_range.next().unwrap(),
        connections: vec![].into(),
        property_data: structs::SclyProperty::SpecialFunction(
            Box::new(
            structs::SpecialFunction
            {
                name: b"SpecialFunction Area Damage-component\0".as_cstr(),
                position: [0., 0., 0.].into(),
                rotation: [0., 0., 0.].into(),
                type_: 18,
                unknown0: b"\0".as_cstr(),
                unknown1: heat_damage_per_sec,
                unknown2: 0.0,
                unknown3: 0.0,
                layer_change_room_id: 4294967295,
                layer_change_layer_id: 4294967295,
                item_id: 0,
                unknown4: 1,
                unknown5: 0.0,
                unknown6: 4294967295,
                unknown7: 4294967295,
                unknown8: 4294967295,
            }
            )
        ),
    };

    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    layer.objects.as_mut_vec().push(area_damage_special_function);
    Ok(())
}

fn is_water<'r>(obj: &structs::SclyObject<'r>) -> bool {
    let water = obj.property_data.as_water();
    water.is_some()
}

fn is_underwater_sound<'r>(obj: &structs::SclyObject<'r>) -> bool {
    let sound = obj.property_data.as_sound();
    if sound.is_none() {
        false // non-sounds are never underwater sounds
    } else {
        sound.unwrap().name.to_str().ok().unwrap().to_string().to_lowercase().contains("underwater") // we define underwater sounds by their name
    }
}

fn patch_remove_water<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer_count = scly.layers.len();
    for i in 0..layer_count {
        let layer = &mut scly.layers.as_mut_vec()[i];
        layer.objects.as_mut_vec().retain(|obj| !is_water(obj));
        layer.objects.as_mut_vec().retain(|obj| !is_underwater_sound(obj));
        // TODO: remove fish and bubbles
    }

    Ok(())
}

#[derive(Copy, Clone, Debug)]
pub enum WaterType {
    Normal,
    Poision,
    Lava
}

impl WaterType
{
    pub fn iter() -> impl Iterator<Item = WaterType> {
        [
            WaterType::Normal,
            WaterType::Poision,
            WaterType::Lava,
        ].iter().map(|i| *i)
    }

    fn dependencies(&self)
    -> Vec<(u32, FourCC)>
    {
        let water_obj = self.to_obj();
        let water = water_obj.property_data.as_water().unwrap();

        let mut deps: Vec<(u32, FourCC)> = Vec::new();
        deps.push((water.txtr1,            FourCC::from_bytes(b"TXTR")));
        deps.push((water.txtr2,            FourCC::from_bytes(b"TXTR")));
        deps.push((water.txtr3,            FourCC::from_bytes(b"TXTR")));
        deps.push((water.txtr4,            FourCC::from_bytes(b"TXTR")));
        deps.push((water.refl_map_txtr,    FourCC::from_bytes(b"TXTR")));
        deps.push((water.txtr6,            FourCC::from_bytes(b"TXTR")));
        deps.push((water.lightmap_txtr,    FourCC::from_bytes(b"TXTR")));
        deps.push((water.small_enter_part, FourCC::from_bytes(b"PART")));
        deps.push((water.med_enter_part,   FourCC::from_bytes(b"PART")));
        deps.push((water.large_enter_part, FourCC::from_bytes(b"PART")));
        deps.push((water.part4,            FourCC::from_bytes(b"PART")));
        deps.push((water.part5,            FourCC::from_bytes(b"PART")));
        deps.retain(|i| i.0 != 0xffffffff && i.0 != 0);
        deps
    }

    fn to_obj<'r>(&self)
    -> structs::SclyObject<'r>
    {
        match self {
            WaterType::Normal  => structs::SclyObject {
                instance_id: 0xFFFFFFFF,
                connections: vec![].into(),
                property_data: structs::SclyProperty::Water(
                    Box::new(structs::Water
                    {
                        name: b"normal water\0".as_cstr(),
                        position: [ 0.0, 0.0, 0.0].into(),
                        scale: [10.0, 10.0, 10.0].into(),
                        damage_info: structs::scly_structs::DamageInfo { weapon_type: 0, damage:  0.0, radius: 0.0, knockback_power: 0.0 },
                        unknown1: [0.0, 0.0, 0.0].into(),
                        unknown2: 2047,
                        unknown3: 0,
                        display_fluid_surface: 1,
                        txtr1: 2003342689,
                        txtr2: 4059883471,
                        txtr3:  351283582,
                        txtr4: 4294967295,
                        refl_map_txtr: 4294967295,
                        txtr6: 1899158552,
                        unknown5: [3.0, 3.0, -4.0].into(),
                        unkown6:  8.0,
                        unkown7: 15.0,
                        unkown8: 15.0,
                        active: 1,
                        fluid_type: 0,
                        unkown11: 0,
                        unkown12: 0.7,
                        fluid_uv_motion:
                            structs::FluidUVMotion
                            {
                                fluid_layer_motion1:
                                structs::FluidLayerMotion
                                    {
                                        fluid_uv_motion: 2,
                                        unknown1: 30.0,
                                        unknown2: 90.0,
                                        unknown3:  0.0,
                                        unknown4:  4.0
                                    },
                                fluid_layer_motion2:
                                structs::FluidLayerMotion
                                    {
                                        fluid_uv_motion: 0,
                                        unknown1: 40.0,
                                        unknown2: -180.0,
                                        unknown3:  0.0,
                                        unknown4: 20.0
                                    },
                                fluid_layer_motion3:
                                structs::FluidLayerMotion
                                    {
                                        fluid_uv_motion: 0,
                                        unknown1: 60.0,
                                        unknown2: 0.0,
                                        unknown3:  0.0,
                                        unknown4: 25.0
                                    },
                                unknown1: 1000.0,
                                unknown2: 0.0
                            },
                        unknown30:  0.0,
                        unknown31:  10.0,
                        unknown32: 1.0,
                        unknown33: 1.0,
                        unknown34: 0.0,
                        unknown35: 90.0,
                        unknown36: 0.0,
                        unknown37: 0.0,
                        unknown38: [1.0, 1.0, 1.0, 1.0].into(),
                        unknown39: [0.411765, 0.670588, 0.831373, 1.0].into(),
                        small_enter_part: 0xffffffff,
                        med_enter_part: 0xffffffff,
                        large_enter_part: 0xffffffff,
                        part4: 0xffffffff,
                        part5: 0xffffffff,
                        sound1: 2499,
                        sound2: 2499,
                        sound3:  463,
                        sound4:  464,
                        sound5:  465,
                        unknown40: 2.4,
                        unknown41: 6,
                        unknown42: 0.0,
                        unknown43: 1.0,
                        unknown44: 0.5,
                        unknown45: 0.8,
                        unknown46: 0.5,
                        unknown47: 0.0,
                        heat_wave_height: 0.0,
                        heat_wave_speed: 1.0,
                        heat_wave_color: [0.596078, 0.752941, 0.819608, 1.0].into(),
                        lightmap_txtr: 4294967295,
                        unknown51: 0.3,
                        unknown52: 0.0,
                        unknown53: 0.0,
                        unknown54: 4294967295,
                        unknown55: 4294967295,
                        crash_the_game: 0
                    })
                ),
            },
            WaterType::Poision => structs::SclyObject {
                instance_id: 0xFFFFFFFF,
                connections: vec![].into(),
                property_data: structs::SclyProperty::Water(
                    Box::new(structs::Water {
                name: b"poision water\0".as_cstr(),
                position: [ 405.3748, -43.92318, 10.530313].into(),
                scale: [13.0, 30.0, 1.0].into(),
                damage_info:
                    structs::scly_structs::DamageInfo { weapon_type: 10,
                    damage: 0.11,
                    radius: 0.0,
                    knockback_power: 0.0
                    },
                unknown1: [0.0, 0.0, 0.0].into(),
                unknown2: 2047,
                unknown3: 0,
                display_fluid_surface: 1,
                txtr1: 2671389366,
                txtr2:  430856216,
                txtr3: 1337209902,
                txtr4: 4294967295,
                refl_map_txtr: 4294967295,
                txtr6: 1899158552,
                unknown5: [3.0, 3.0, -4.0].into(),
                unkown6: 48.0,
                unkown7: 5.0,
                unkown8: 5.0,
                active: 1,
                fluid_type: 1,
                unkown11: 0,
                unkown12: 0.8,
                fluid_uv_motion:
                structs::FluidUVMotion { fluid_layer_motion1: structs::FluidLayerMotion { fluid_uv_motion: 0,
                unknown1: 20.0,
                unknown2:  0.0,
                unknown3: 0.15,
                unknown4: 20.0},
                fluid_layer_motion2:
                structs::FluidLayerMotion { fluid_uv_motion: 0,
                unknown1: 10.0,
                unknown2:  180.0,
                unknown3: 0.15,
                unknown4: 10.0 },
                fluid_layer_motion3: structs::FluidLayerMotion { fluid_uv_motion: 0,
                unknown1: 40.0,
                unknown2: 0.0,
                unknown3: 0.15,
                unknown4: 25.0 },
                unknown1:  100.0,
                unknown2: 0.0 },
                unknown30: 20.0,
                unknown31: 100.0,
                unknown32: 1.0,
                unknown33: 3.0,
                unknown34: 0.0,
                unknown35: 90.0,
                unknown36: 0.0,
                unknown37: 0.0,
                unknown38: [1.0,
                1.0,
                1.0,
                1.0].into(),
                unknown39: [0.619608,
                0.705882,
                0.560784,
                1.0].into(),
                small_enter_part: 0xffffffff,
                med_enter_part:   0xffffffff,
                large_enter_part: 0xffffffff,
                part4: 0xffffffff,
                part5: 0xffffffff,
                sound1: 2499,
                sound2: 2499,
                sound3:  463,
                sound4:  464,
                sound5:  465,
                unknown40: 2.4,
                unknown41: 6,
                unknown42: 0.0,
                unknown43: 1.0,
                unknown44: 0.5,
                unknown45: 0.8,
                unknown46: 1.0,
                unknown47: 0.0,
                heat_wave_height: 0.0,
                heat_wave_speed: 1.0,
                heat_wave_color: [0.784314,
                     1.0,
                0.27451,
                 1.0].into(),
                lightmap_txtr: 4294967295,
                unknown51: 0.3,
                unknown52: 0.0,
                unknown53: 0.0,
                unknown54: 4294967295,
                unknown55: 4294967295,
                crash_the_game: 0
            }))},
            WaterType::Lava    => structs::SclyObject {
                instance_id: 0xFFFFFFFF,
                connections: vec![].into(),
                property_data: structs::SclyProperty::Water(
                    Box::new(structs::Water {
                name: b"lava\0".as_cstr(),
                position: [26.634968,
                -14.81889,
                 0.237813].into(),
                scale: [41.601,
                52.502003,
                7.0010004].into(),
                damage_info: structs::scly_structs::DamageInfo { weapon_type: 11,
                damage:  0.4,
                radius: 0.0,
                knockback_power: 0.0 },
                unknown1: [0.0,
                0.0,
                0.0].into(),
                unknown2: 2047,
                unknown3: 1,
                display_fluid_surface: 1,
                txtr1:  117134624,
                txtr2: 2154768270,
                txtr3: 3598011320,
                txtr4: 1249771730,
                refl_map_txtr: 4294967295,
                txtr6: 4294967295,
                unknown5: [3.0,
                3.0,
                -4.0].into(),
                unkown6: 70.0,
                unkown7: 15.0,
                unkown8: 15.0,
                active: 1,
                fluid_type: 2,
                unkown11: 0,
                unkown12: 0.65,
                fluid_uv_motion: structs::FluidUVMotion { fluid_layer_motion1: structs::FluidLayerMotion { fluid_uv_motion: 0,
                unknown1: 30.0,
                unknown2:  0.0,
                unknown3: 0.15,
                unknown4: 10.0},
                fluid_layer_motion2: structs::FluidLayerMotion { fluid_uv_motion: 0,
                unknown1: 40.0,
                unknown2:  180.0,
                unknown3: 0.15,
                unknown4: 20.0 },
                fluid_layer_motion3: structs::FluidLayerMotion { fluid_uv_motion: 0,
                unknown1: 45.0,
                unknown2: 0.0,
                unknown3: 0.15,
                unknown4: 10.0 },
                unknown1:   70.0,
                unknown2: 0.0 },
                unknown30: 20.0,
                unknown31: 100.0,
                unknown32: 1.0,
                unknown33: 3.0,
                unknown34: 0.0,
                unknown35: 90.0,
                unknown36: 0.0,
                unknown37: 0.0,
                unknown38: [1.0,
                1.0,
                1.0,
                1.0].into(),
                unknown39: [0.631373,
                0.270588,
                0.270588,
                1.0].into(),
                small_enter_part: 0xffffffff,
                med_enter_part: 0xffffffff,
                large_enter_part: 0xffffffff,
                part4: 0xffffffff,
                part5: 0xffffffff,
                sound1: 2412,
                sound2: 2412,
                sound3: 1373,
                sound4: 1374,
                sound5: 1375,
                unknown40: 2.4,
                unknown41: 6,
                unknown42: 0.0,
                unknown43: 1.0,
                unknown44: 0.5,
                unknown45: 0.8,
                unknown46: 0.5,
                unknown47: 1.7,
                heat_wave_height: 1.2,
                heat_wave_speed: 1.0,
                heat_wave_color: [1.0,
                     0.682353,
                0.294118,
                1.0].into(),
                lightmap_txtr: 4294967295,
                unknown51: 0.3,
                unknown52: 0.0,
                unknown53: 0.0,
                unknown54: 4294967295,
                unknown55: 4294967295,
                crash_the_game: 0
            }))},
        }
    }
}

fn collect_liquid_resources<'r>(gc_disc: &structs::GcDisc<'r>)
-> HashMap<(u32, FourCC), structs::Resource<'r>>{
    // Get list of all dependencies needed by liquids //
    let mut looking_for: HashSet<_> = WaterType::iter()
        .flat_map(|pt| pt.dependencies().into_iter())
        .collect();

    // Dependencies read from paks and custom assets will go here //
    let mut found = HashMap::with_capacity(looking_for.len());

    // Iterate through all paks and add add any dependencies to the resource pool //
    for pak_name in pickup_meta::ROOM_INFO.iter().map(|(name, _)| name) { // for all paks

        // get the pak //
        let file_entry = gc_disc.find_file(pak_name).unwrap();
        let pak = match *file_entry.file().unwrap() {
            structs::FstEntryFile::Pak(ref pak) => Cow::Borrowed(pak),
            structs::FstEntryFile::Unknown(ref reader) => Cow::Owned(reader.clone().read(())),
            _ => panic!(),
        };

        // Iterate through all resources in the pak //
        for res in pak.resources.iter() {
            let key = (res.file_id, res.fourcc());
            if looking_for.remove(&key) { // If it's one of our dependencies
                assert!(found.insert(key, res.into_owned()).is_none()); // collect it
            }
        }
    }

    if !looking_for.is_empty()
    {
        println!("error - still looking for {:?}", looking_for);
    }
    assert!(looking_for.is_empty());
    found
}

fn patch_submerge_room<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
)
-> Result<(), String>
{
    let water_type = WaterType::Normal;

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


    let room_origin = {
        let area_transform = area.mlvl_area.area_transform;

        [
            area_transform[3],
            area_transform[7],
            area_transform[11],
        ]
    };

    let bounding_box_untransformed = area.mlvl_area.area_bounding_box;

    // transform bounding box by origin offset provided in area transform   //
    // note that we are assuming the area transformation matrix is identity //
    // on the premise that every door in the game is axis-aligned           //
    let bounding_box_min = [
        room_origin[0] + bounding_box_untransformed[0],
        room_origin[1] + bounding_box_untransformed[1],
        room_origin[2] + bounding_box_untransformed[2],
    ];
    let bounding_box_max = [
        room_origin[0] + bounding_box_untransformed[3],
        room_origin[1] + bounding_box_untransformed[4],
        room_origin[2] + bounding_box_untransformed[5],
    ];

    // The water's size is the difference in min/max //
    water.scale[0] = (bounding_box_max[0] - bounding_box_min[0]).abs();
    water.scale[1] = (bounding_box_max[1] - bounding_box_min[1]).abs();
    water.scale[2] = (bounding_box_max[2] - bounding_box_min[2]).abs();

    // The water is centered in the middle of the bounding box //
    water.position[0] = bounding_box_min[0] + (water.scale[0] / 2.0);
    water.position[1] = bounding_box_min[1] + (water.scale[1] / 2.0);
    water.position[2] = bounding_box_min[2] + (water.scale[2] / 2.0);

    // add water to area //
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    layer.objects.as_mut_vec().push(water_obj);

    Ok(())
}

fn patch_add_liquid<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    water_config: &WaterConfig,
    resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
)
-> Result<(), String>
{
    let water_type = {
        let liquid_type = water_config.liquid_type.to_lowercase();
        if liquid_type == "water" || liquid_type == "normal" {
            WaterType::Normal
        } else if liquid_type == "poison" || liquid_type == "acid" {
            WaterType::Poision
        } else if liquid_type == "lava" || liquid_type == "magma" {
            WaterType::Lava
        } else {
            panic!("Unknown Liquid Type '{}'", liquid_type);
        }
    };

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

fn patch_remove_tangle_weed_scan_point<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    tangle_weed_ids: Vec<u32>,
) -> Result<(), String>
{
    let layer_count = area.layer_flags.layer_count as usize;
    let scly = area.mrea().scly_section_mut();
    let layers = scly.layers.as_mut_vec();

    for i in 0..layer_count {
        for obj in layers[i].objects.as_mut_vec().iter_mut() {
            if tangle_weed_ids.contains(&obj.instance_id) {
                let tangle_weed = obj.property_data.as_snake_weed_swarm_mut().unwrap();
                tangle_weed.actor_params.scan_params.scan = ResId::invalid();
            }
        }
    }

    Ok(())
}

fn patch_add_poi<'r>(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    scan_id: ResId<res_id::SCAN>,
    strg_id: ResId<res_id::STRG>,
    position: [f32;3],
) -> Result<(), String>
{    
    let scly = area.mrea().scly_section_mut();
    let layers = scly.layers.as_mut_vec();
    layers[0].objects.as_mut_vec().push(
        structs::SclyObject {
            instance_id: ps.fresh_instance_id_range.next().unwrap(),
            connections: vec![].into(),
            property_data: structs::SclyProperty::PointOfInterest(
                Box::new(structs::PointOfInterest {
                    name: b"mypoi\0".as_cstr(),
                    position: position.into(),
                    rotation: [0.0, 0.0, 0.0].into(),
                    active: 1,
                    scan_param: structs::scly_structs::ScannableParameters {
                        scan: scan_id,
                    },
                    point_size: 12.0,
                })
            ),
        }
    );

    let frme_id = ResId::<res_id::FRME>::new(0xDCEC3E77);

    let scan_dep: structs::Dependency = scan_id.into();
    area.add_dependencies(game_resources, 0, iter::once(scan_dep));

    let strg_dep: structs::Dependency = strg_id.into();
    area.add_dependencies(game_resources, 0, iter::once(strg_dep));

    let frme_dep: structs::Dependency = frme_id.into();
    area.add_dependencies(game_resources, 0, iter::once(frme_dep));

    Ok(())
}

fn patch_add_scan_actor<'r>(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    position: [f32;3],
    rotation: f32,
)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    scly.layers.as_mut_vec()[0].objects.as_mut_vec().push(
        structs::SclyObject {
            instance_id: ps.fresh_instance_id_range.next().unwrap(),
            connections: vec![].into(),
            property_data: structs::SclyProperty::Actor(
                Box::new(structs::Actor {
                    name: b"Scan Actor\0".as_cstr(),
                    position: position.into(),
                    rotation: [0.0, 90.0, rotation].into(),
                    scale: [1.0, 1.0, 1.0].into(),
                    hitbox: [0.0, 0.0, 0.0].into(),
                    scan_offset: [0.0, 0.0, 0.0].into(),
                    unknown1: 1.0, // mass
                    unknown2: 0.0, // momentum
                    health_info: structs::scly_structs::HealthInfo {
                        health: 5.0,
                        knockback_resistance: 1.0,
                    },
                    damage_vulnerability: DoorType::Disabled.vulnerability(),
                    cmdl: ResId::invalid(),
                    ancs: structs::scly_structs::AncsProp {
                        file_id: ResId::<res_id::ANCS>::new(0x98dab29c), // Scanholo.ANCS
                        node_index: 0,
                        default_animation: 0,
                    },
                    actor_params: structs::scly_structs::ActorParameters {
                        light_params: structs::scly_structs::LightParameters {
                            unknown0: 0,
                            unknown1: 1.0,
                            shadow_tessellation: 0,
                            unknown2: 1.0,
                            unknown3: 20.0,
                            color: [1.0, 1.0, 1.0, 1.0].into(), // RGBA
                            unknown4: 0,
                            world_lighting: 0,
                            light_recalculation: 1,
                            unknown5: [0.0, 0.0, 0.0].into(),
                            unknown6: 4,
                            unknown7: 4,
                            unknown8: 0,
                            light_layer_id: 0,
                        },
                        scan_params: structs::scly_structs::ScannableParameters {
                            scan: ResId::invalid(),
                        },
                        xray_cmdl: ResId::invalid(),
                        xray_cskr: ResId::invalid(),
                        thermal_cmdl: ResId::invalid(),
                        thermal_cskr: ResId::invalid(),
                        unknown0: 1,
                        unknown1: 1.0,
                        unknown2: 1.0,
                        visor_params: structs::scly_structs::VisorParameters {
                            unknown0: 0,
                            target_passthrough: 0,
                            visor_mask: 15, // Visor Flags : Combat|Scan|Thermal|XRay
                        },
                        enable_thermal_heat: 1,
                        unknown3: 0,
                        unknown4: 0,
                        unknown5: 1.0,
                    },
                    looping: 1,
                    snow: 0, // immovable
                    solid: 0,
                    camera_passthrough: 0,
                    active: 1,
                    unknown8: 0,
                    unknown9: 1.0,
                    unknown10: 0,
                    unknown11: 0,
                    unknown12: 0,
                    unknown13: 0,
                })
            ),
        }
    );

    let dep: structs::Dependency = ResId::<res_id::ANCS>::new(0x98DAB29C).into();
    area.add_dependencies(game_resources, 0, iter::once(dep));

    let dep: structs::Dependency = ResId::<res_id::CMDL>::new(0x2A0FA4F9).into();
    area.add_dependencies(game_resources, 0, iter::once(dep)); // AnimatedObjects/Introlevel/scenes/SP_blueHolograms/cooked/Scanholo_bound.CMDL

    let dep: structs::Dependency = ResId::<res_id::TXTR>::new(0x336B78E8).into();
    area.add_dependencies(game_resources, 0, iter::once(dep)); // Worlds/IntroLevel/common_textures/sp_holoanim1C.TXTR

    let dep: structs::Dependency = ResId::<res_id::CSKR>::new(0x41200B2F).into();
    area.add_dependencies(game_resources, 0, iter::once(dep)); // AnimatedObjects/Introlevel/scenes/SP_blueHolograms/cooked/Scanholo_bound.CSKR

    let dep: structs::Dependency = ResId::<res_id::CINF>::new(0xE436418D).into();
    area.add_dependencies(game_resources, 0, iter::once(dep)); // AnimatedObjects/Introlevel/scenes/SP_blueHolograms/cooked/Scanholo_bound.CINF

    let dep: structs::Dependency = ResId::<res_id::ANIM>::new(0xA1ED00B6).into();
    area.add_dependencies(game_resources, 0, iter::once(dep)); // AnimatedObjects/Introlevel/scenes/SP_blueHolograms/cooked/Scanholo_ready.ANIM

    let dep: structs::Dependency = ResId::<res_id::EVNT>::new(0xA7DDBDC4).into();
    area.add_dependencies(game_resources, 0, iter::once(dep)); // AnimatedObjects/Introlevel/scenes/SP_blueHolograms/cooked/Scanholo_ready.EVNT

    Ok(())
}

fn gen_n_pick_closest<R>(n: u32, rng: &mut R, min: f32, max: f32, mid: f32)
-> f32
where R: Rng
{
    assert!(n != 0);
    let mut closest: f32 = 100.1;
    for _ in 0..n {
        let x = rng.gen_range(min, max);
        if f32::abs(x - mid) < f32::abs(closest - mid) {
            closest = x;
        }
    }
    closest
}

fn get_shuffled_position<'r, R>(
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    rng: &mut R,
)
-> [f32; 3]
where R: Rng
{
    let mrea_id = area.mlvl_area.mrea.to_u32();

    // xmin, ymin, zmin,
    // xmax, ymax, zmax,
    let mut bounding_boxes: Vec<[f32; 6]> = Vec::new();
    {
        let mut bounding_box = area.mlvl_area.area_bounding_box.clone();
        let room_origin = {
            let area_transform = area.mlvl_area.area_transform;
            [area_transform[3], area_transform[7], area_transform[11]]
        };
        for i in 0..6 {
            bounding_box[i] = bounding_box[i] + room_origin[i%3];
        }
        bounding_boxes.push(bounding_box.into());
    }

    if mrea_id == 0x2398E906 { // Artifact Temple
        bounding_boxes.clear();
        bounding_boxes.push([
                -410.0, 20.0, -40.0,
                -335.0, 69.0, -17.0,
            ].into()
        );
        bounding_boxes.push([
            -411.429, 67.9626, -14.8928,
            -370.429, 93.9626, -9.8928,
            ].into()
        );
    } else if mrea_id == 0x4148F7B0 { // burn dome
        bounding_boxes.clear();
        bounding_boxes.push([
            565.7892, -27.4683, 30.6111,
            589.7892, 0.5317, 42.6111,
            ].into()
        );
        bounding_boxes.push([
            578.9656, 35.3132, 31.0428,
            598.9656, 44.3132, 37.0428,
            ].into()
        );
        bounding_boxes.push([
            588.6971, 9.1298, 29.8123,
            589.6971, 49.1298, 31.8123,
            ].into()
        );
    }

    let mut offset_xy = 0.0;
    let mut offset_max_z = 0.0;
    if vec![
        0xC44E7A07, // landing site
        0xB2701146, // alcove
        0xB9ABCD56, // fcs
        0x9A0A03EB, // sunchamber
        0xFB54A0CB, // hote
        0xBAD9EDBF, // Triclops pit
        0x3953C353, // Elite Quarters
        0x70181194, // Quarantine Cave
        0xC7E821BA, // ttb
        0x4148F7B0, // burn dome
        0x43E4CC25, // hydra
        0x21B4BFF6, // aether
        ].contains(&mrea_id) {
        offset_xy = 0.1;
        offset_max_z = -0.3;
    }

    // Pick the relative position inside the bounding box
    let x_factor: f32 = gen_n_pick_closest(2, rng, 0.15 + offset_xy, 0.85 - offset_xy, 0.5);
    let y_factor: f32 = gen_n_pick_closest(2, rng, 0.15 + offset_xy, 0.85 - offset_xy, 0.5);
    let z_factor: f32 = gen_n_pick_closest(2, rng, 0.1, 0.8 + offset_max_z, 0.35);

    // Pick a bounding box if multiple are available
    let bounding_box = bounding_boxes.choose(rng).unwrap().clone();
    [
        bounding_box[0] + (bounding_box[3]-bounding_box[0])*x_factor,
        bounding_box[1] + (bounding_box[4]-bounding_box[1])*y_factor,
        bounding_box[2] + (bounding_box[5]-bounding_box[2])*z_factor,
    ]
}

fn modify_pickups_in_mrea<'r>(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    pickup_config: &PickupConfig,
    pickup_location: pickup_meta::PickupLocation,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    pickup_hudmemos: &HashMap<PickupHashKey, ResId<res_id::STRG>>,
    pickup_scans: &HashMap<PickupHashKey, (ResId<res_id::SCAN>, ResId<res_id::STRG>)>,
    pickup_hash_key: PickupHashKey,
    skip_hudmemos: bool,
    hudmemo_delay: f32,
    qol_pickup_scans: bool,
    extern_models: &HashMap<String, ExternPickupModel>,
    shuffle_position: bool,
    seed: u64,
    _no_starting_visor: bool,
)
-> Result<(), String>
{
    let mrea_id = area.mlvl_area.mrea.to_u32();
    let mut rng = StdRng::seed_from_u64(seed);

    let mut position_override: Option<[f32;3]> = None;
    if shuffle_position {
        position_override = Some(get_shuffled_position(area, &mut rng));
    }

    // Pickup to use for game functionality //
    let pickup_type = PickupType::from_str(&pickup_config.pickup_type);

    let extern_model = if pickup_config.model.is_some() {
        extern_models.get(pickup_config.model.as_ref().unwrap())
    } else {
        None
    };

    // Pickup to use for visuals/hitbox //
    let pickup_model_type: Option<PickupModel> = {
        if pickup_config.model.is_some() {
            let model_name = pickup_config.model.as_ref().unwrap();
            let pmt = PickupModel::from_str(&model_name);
            if pmt.is_none() && !extern_model.is_some() {
                panic!("Unkown Model Type {}", model_name);
            }

            pmt // Some - Native Prime Model
                // None - External Model (e.g. Screw Attack)
        } else {
            Some(PickupModel::from_type(pickup_type)) // No model specified, use pickup type as inspiration
        }
    };

    let pickup_model_type = pickup_model_type.clone().unwrap_or(PickupModel::Nothing);
    let mut pickup_model_data = pickup_model_type.pickup_data();
    if extern_model.is_some() {
        let scale = extern_model.as_ref().unwrap().scale.clone();
        pickup_model_data.scale[0] = pickup_model_data.scale[0]*scale;
        pickup_model_data.scale[1] = pickup_model_data.scale[1]*scale;
        pickup_model_data.scale[2] = pickup_model_data.scale[2]*scale;
        pickup_model_data.cmdl = ResId::<res_id::CMDL>::new(extern_model.as_ref().unwrap().cmdl);
        pickup_model_data.ancs.file_id = ResId::<res_id::ANCS>::new(extern_model.as_ref().unwrap().ancs);
        pickup_model_data.part = ResId::invalid();
        pickup_model_data.ancs.node_index = extern_model.as_ref().unwrap().character;
        pickup_model_data.ancs.default_animation = 0;
        pickup_model_data.actor_params.xray_cmdl = ResId::invalid();
        pickup_model_data.actor_params.xray_cskr = ResId::invalid();
        pickup_model_data.actor_params.thermal_cmdl = ResId::invalid();
        pickup_model_data.actor_params.thermal_cskr = ResId::invalid();
    }

    let name = CString::new(format!(
            "Randomizer - Pickup ({:?})", pickup_type.name())).unwrap();
    area.add_layer(Cow::Owned(name));
    let new_layer_idx = area.layer_flags.layer_count as usize - 1;

    let new_layer_2_idx = new_layer_idx + 1;
    if pickup_config.respawn.unwrap_or(false) {
        let name2 = CString::new(format!(
            "Randomizer - Pickup ({:?})", pickup_type.name())).unwrap();
        area.add_layer(Cow::Owned(name2));
        area.layer_flags.flags &= !(1 << new_layer_2_idx); // layer disabled by default
    }

    // Add hudmemo string as dependency to room //
    let hudmemo_strg: ResId<res_id::STRG> = {
        if pickup_config.hudmemo_text.is_some() {
            *pickup_hudmemos.get(&pickup_hash_key).unwrap()
        } else {
            pickup_type.hudmemo_strg()
        }
    };

    let hudmemo_dep: structs::Dependency = hudmemo_strg.into();
    area.add_dependencies(game_resources, new_layer_idx, iter::once(hudmemo_dep));

    /* Add Model Dependencies */
    // Dependencies are defined externally
    if extern_model.is_some() {
        let deps = extern_model.as_ref().unwrap().dependencies.clone();
        let deps_iter = deps.iter()
            .map(|&(file_id, fourcc)| structs::Dependency {
                asset_id: file_id,
                asset_type: fourcc,
            }
        );
        area.add_dependencies(game_resources, new_layer_idx, deps_iter);
    }
    // If we aren't using an external model, use the dependencies traced by resource_tracing
    else {
        let deps_iter = pickup_model_type
            .dependencies().iter()
            .map(|&(file_id, fourcc)| structs::Dependency {
                asset_id: file_id,
                asset_type: fourcc,
                }
            );
        area.add_dependencies(game_resources, new_layer_idx, deps_iter);
    }

    {
        let frme = ResId::<res_id::FRME>::new(0xDCEC3E77);
        let frme_dep: structs::Dependency = frme.into();
        area.add_dependencies(game_resources, new_layer_idx, iter::once(frme_dep));
    }
    let scan_id = {
        if pickup_config.scan_text.is_some() {
            let (scan, strg) = *pickup_scans.get(&pickup_hash_key).unwrap();

            let scan_dep: structs::Dependency = scan.into();
            area.add_dependencies(game_resources, new_layer_idx, iter::once(scan_dep));

            let strg_dep: structs::Dependency = strg.into();
            area.add_dependencies(game_resources, new_layer_idx, iter::once(strg_dep));

            scan
        }
        else
        {
            let scan_dep: structs::Dependency = pickup_type.scan().into();
            area.add_dependencies(game_resources, new_layer_idx, iter::once(scan_dep));

            let strg_dep: structs::Dependency = pickup_type.scan_strg().into();
            area.add_dependencies(game_resources, new_layer_idx, iter::once(strg_dep));

            pickup_type.scan()
        }
    };

    let room_id = area.mlvl_area.internal_id;
    let scly = area.mrea().scly_section_mut();
    let layers = scly.layers.as_mut_vec();

    let mut additional_connections = Vec::new();

    // 2022-02-08 - I had to remove this because there's a bug in the vanilla engine where playerhint -> Scan Visor doesn't holster the weapon
    // if pickup_type == PickupType::ScanVisor && no_starting_visor {

    // // If scan visor, and starting visor is none, then switch to combat and back to scan when obtaining scan
    // let player_hint_id = ps.fresh_instance_id_range.next().unwrap();
    // let player_hint = structs::SclyObject {
    //     instance_id: player_hint_id,
    //         property_data: structs::PlayerHint {
    //         name: b"combat playerhint\0".as_cstr(),
    //         position: [0.0, 0.0, 0.0].into(),
    //         rotation: [0.0, 0.0, 0.0].into(),
    //         unknown0: 1, // active
    //         inner_struct: structs::PlayerHintStruct {
    //             unknowns: [
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 1,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //             ].into(),
    //         }.into(),
    //         unknown1: 10, // priority
    //         }.into(),
    //         connections: vec![].into(),
    // };

    // additional_connections.push(
    //     structs::Connection {
    //         state: structs::ConnectionState::ARRIVED,
    //         message: structs::ConnectionMsg::INCREMENT,
    //         target_object_id: player_hint_id,
    //     }
    // );

    // let player_hint_id_2 = ps.fresh_instance_id_range.next().unwrap();
    // let player_hint_2 = structs::SclyObject {
    //     instance_id: player_hint_id_2,
    //         property_data: structs::PlayerHint {
    //         name: b"combat playerhint\0".as_cstr(),
    //         position: [0.0, 0.0, 0.0].into(),
    //         rotation: [0.0, 0.0, 0.0].into(),
    //         unknown0: 1, // active
    //         inner_struct: structs::PlayerHintStruct {
    //             unknowns: [
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //                 1,
    //                 0,
    //                 0,
    //                 0,
    //                 0,
    //             ].into(),
    //         }.into(),
    //         unknown1: 10, // priority
    //         }.into(),
    //         connections: vec![].into(),
    // };

    // let timer_id = ps.fresh_instance_id_range.next().unwrap();
    // let timer = structs::SclyObject {
    //     instance_id: timer_id,
    //     property_data: structs::Timer {
    //         name: b"set-scan\0".as_cstr(),
    //         start_time: 0.5,
    //         max_random_add: 0.0,
    //         reset_to_zero: 0,
    //         start_immediately: 0,
    //         active: 1,
    //     }.into(),
    //     connections: vec![
    //         structs::Connection {
    //             state: structs::ConnectionState::ZERO,
    //             message: structs::ConnectionMsg::INCREMENT,
    //             target_object_id: player_hint_id_2,
    //         },
    //     ].into(),
    // };

    // additional_connections.push(
    //     structs::Connection {
    //         state: structs::ConnectionState::ARRIVED,
    //         message: structs::ConnectionMsg::RESET_AND_START,
    //         target_object_id: timer_id,
    //     }
    // );


    //     layers[new_layer_idx].objects.as_mut_vec().push(player_hint);
    //     layers[new_layer_idx].objects.as_mut_vec().push(player_hint_2);
    //     layers[new_layer_idx].objects.as_mut_vec().push(timer);
    // }

    // Add a post-pickup relay. This is used to support cutscene-skipping
    let instance_id = ps.fresh_instance_id_range.next().unwrap();
    let mut relay = post_pickup_relay_template(instance_id,
                                            pickup_location.post_pickup_relay_connections);

    additional_connections.push(structs::Connection {
        state: structs::ConnectionState::ARRIVED,
        message: structs::ConnectionMsg::SET_TO_ZERO,
        target_object_id: instance_id,
    });

    // If this is an artifact, insert a layer change function
    let pickup_kind = pickup_type.kind();
    if pickup_kind >= 29 && pickup_kind <= 40 {
        let instance_id = ps.fresh_instance_id_range.next().unwrap();
        let function = artifact_layer_change_template(instance_id, pickup_kind);
        layers[new_layer_idx].objects.as_mut_vec().push(function);
        additional_connections.push(structs::Connection {
            state: structs::ConnectionState::ARRIVED,
            message: structs::ConnectionMsg::INCREMENT,
            target_object_id: instance_id,
        });
    }

    if pickup_config.respawn.unwrap_or(false) {
        // add a special function that activates this pickup
        let special_function_id = ps.fresh_instance_id_range.next().unwrap();
        layers[new_layer_idx].objects.as_mut_vec().push(structs::SclyObject {
            instance_id: special_function_id,
            connections: vec![].into(),
            property_data: structs::SpecialFunction::layer_change_fn(
                b"Enable pickup\0".as_cstr(),
                room_id,
                new_layer_2_idx as u32,
            ).into(),
        });
        layers[new_layer_2_idx].objects.as_mut_vec().push(structs::SclyObject {
            instance_id: ps.fresh_instance_id_range.next().unwrap(),
            property_data: structs::Timer {
                name: b"auto-spawn pickup\0".as_cstr(),
                start_time: 0.001,
                max_random_add: 0.0,
                reset_to_zero: 0,
                start_immediately: 1,
                active: 1,
            }.into(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::ACTIVATE,
                    target_object_id: pickup_location.location.instance_id,
                },
            ].into(),
        });
        additional_connections.push(structs::Connection {
            state: structs::ConnectionState::ARRIVED,
            message: structs::ConnectionMsg::INCREMENT,
            target_object_id: special_function_id
        });
    }

    // Fix chapel IS
    if mrea_id == 0x40C548E9 {
        let trigger_id = ps.fresh_instance_id_range.next().unwrap();
        additional_connections.push(
            structs::Connection {
                state: structs::ConnectionState::ARRIVED,
                message: structs::ConnectionMsg::SET_TO_ZERO,
                target_object_id: 0x000E023A,
            }
        );

        additional_connections.push(
            structs::Connection {
                state: structs::ConnectionState::ARRIVED,
                message: structs::ConnectionMsg::DEACTIVATE,
                target_object_id: trigger_id,
            }
        );

        layers[0].objects.as_mut_vec().push(
            structs::SclyObject {
                instance_id: trigger_id,
                property_data: structs::Trigger {
                    name: b"Trigger\0".as_cstr(),
                    position: [-369.901093, -169.402206, 60.743099].into(),
                    scale: [20.0, 20.0, 5.0].into(),
                    damage_info: structs::scly_structs::DamageInfo {
                        weapon_type: 0,
                        damage: 0.0,
                        radius: 0.0,
                        knockback_power: 0.0
                    },
                    force: [0.0, 0.0, 0.0].into(),
                    flags: 0x1001, // detect morphed+player
                    active: 1,
                    deactivate_on_enter: 0,
                    deactivate_on_exit: 0
                }.into(),
                connections: vec![
                    structs::Connection {
                        state: structs::ConnectionState::INSIDE,
                        message: structs::ConnectionMsg::SET_TO_ZERO,
                        target_object_id: 0x000E023A,
                    }
                ].into()
            }
        );
    }

    let position: [f32; 3];
    let scan_id_out: ResId<res_id::SCAN>;
    {
        let pickup_obj = layers[pickup_location.location.layer as usize].objects.iter_mut()
        .find(|obj| obj.instance_id == pickup_location.location.instance_id)
        .unwrap();
        (position, scan_id_out) = update_pickup(pickup_obj, pickup_type, pickup_model_data, pickup_config, scan_id, position_override);

        if additional_connections.len() > 0 {
            pickup_obj.connections.as_mut_vec().extend_from_slice(&additional_connections);
        }
    }

    if shuffle_position || *pickup_config.jumbo_scan.as_ref().unwrap_or(&false) {
        let poi_id = ps.fresh_instance_id_range.next().unwrap();
        layers[new_layer_idx].objects.as_mut_vec().push(
            structs::SclyObject {
                instance_id: poi_id,
                connections: vec![].into(),
                property_data: structs::SclyProperty::PointOfInterest(
                    Box::new(structs::PointOfInterest {
                        name: b"mypoi\0".as_cstr(),
                        position: position.clone().into(),
                        rotation: [0.0, 0.0, 0.0].into(),
                        active: 1,
                        scan_param: structs::scly_structs::ScannableParameters {
                            scan: scan_id,
                        },
                        point_size: 500.0,
                    })
                ),
            }
        );

        // disable poi
        let special_fn_id = ps.fresh_instance_id_range.next().unwrap();
        layers[new_layer_idx].objects.as_mut_vec().push(structs::SclyObject {
            instance_id: special_fn_id,
            property_data: structs::SpecialFunction::layer_change_fn(
                b"SpecialFunction - remove poi\0".as_cstr(),
                room_id,
                new_layer_idx as u32,
            ).into(),
            connections: vec![].into(),
        });
        additional_connections.push(
            structs::Connection {
                state: structs::ConnectionState::ARRIVED,
                message: structs::ConnectionMsg::DECREMENT,
                target_object_id: special_fn_id,
            }
        );
        additional_connections.push(
            structs::Connection {
                state: structs::ConnectionState::ARRIVED,
                message: structs::ConnectionMsg::DECREMENT,
                target_object_id: poi_id,
            }
        );
        relay.connections.as_mut_vec().push(
            structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::DECREMENT,
                target_object_id: special_fn_id,
            }
        );
        relay.connections.as_mut_vec().push(
            structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::DEACTIVATE,
                target_object_id: poi_id,
            }
        );

        // Always allow cinema in artifact temple
        if mrea_id == 0x2398E906 {
            let trigger = layers[20].objects.iter_mut()
                .find(|obj| obj.instance_id&0x00FFFFFF == 0x00100470)
                .and_then(|obj| obj.property_data.as_trigger_mut())
                .unwrap();
            trigger.active = 1;
        }
    }

    layers[new_layer_idx].objects.as_mut_vec().push(relay);

    // find any overlapping POI that give "helpful" hints to the player and replace their scan text with the items //
    if qol_pickup_scans {
        const EXCLUDE_POI: &[u32] = &[
            0x000200AF, // main plaza tree
            0x00190584, 0x0019039C, // research lab hydra
            0x001F025C, // mqb tank
            0x000D03D9, // Phazon Elite
        ];
        for layer in layers.iter_mut() {
            if mrea_id == 0x2398E906 {
                continue; // Avoid deleting hints
            }
            for obj in layer.objects.as_mut_vec().iter_mut() {
                let obj_id = obj.instance_id&0x00FFFFFF;

                // Make the door in magmoor workstaion passthrough so item is scannable
                // Also the ice in ruins west
                if obj_id == 0x0017016E || obj_id == 0x0017016F || obj_id == 0x00092738
                {
                    let actor = obj.property_data.as_actor_mut().unwrap();
                    actor.actor_params.visor_params.target_passthrough = 1;
                } else if obj.property_data.is_point_of_interest() {
                    let poi = obj.property_data.as_point_of_interest_mut().unwrap();
                    if (
                        f32::abs(poi.position[0] - position[0]) < 6.0 &&
                        f32::abs(poi.position[1] - position[1]) < 6.0 &&
                        f32::abs(poi.position[2] - position[2]) < 3.0 &&
                        !EXCLUDE_POI.contains(&obj_id) &&
                        pickup_location.location.instance_id != 0x002005EA
                       ) || (pickup_location.location.instance_id == 0x0428011c && obj_id == 0x002803CE)  // research core scan
                         || (pickup_location.location.instance_id == 0x00020176 && poi.scan_param.scan == custom_asset_ids::SHORELINES_POI_SCAN) // custom shorelines tower scan
                         || (pickup_location.location.instance_id == 600301 && poi.scan_param.scan == 0x00092837) // Ice Ruins West scan
                         || (pickup_location.location.instance_id == 524406 && poi.scan_param.scan == 0x0008002C) // Ruined Fountain
                         || (pickup_location.location.instance_id == 1179916 && poi.scan_param.scan == 0x9CBB2160) // Vent Shaft
                    {
                        poi.scan_param.scan = scan_id_out;
                    }
                }
            }
        }
    }

    let hudmemo = layers[pickup_location.hudmemo.layer as usize].objects.iter_mut()
        .find(|obj| obj.instance_id ==  pickup_location.hudmemo.instance_id)
        .unwrap();
    // The items in Watery Hall (Charge beam), Research Core (Thermal Visor), and Artifact Temple
    // (Artifact of Truth) should ys have modal hudmenus because a cutscene plays immediately
    // after each item is acquired, and the nonmodal hudmenu wouldn't properly appear.
    update_hudmemo(hudmemo, hudmemo_strg, skip_hudmemos, hudmemo_delay);

    let location = pickup_location.attainment_audio;
    let attainment_audio = layers[location.layer as usize].objects.iter_mut()
        .find(|obj| obj.instance_id ==  location.instance_id)
        .unwrap();
    update_attainment_audio(attainment_audio, pickup_type);
    Ok(())
}

fn update_pickup(
    pickup_obj: &mut structs::SclyObject,
    pickup_type: PickupType,
    pickup_model_data: structs::Pickup,
    pickup_config: &PickupConfig,
    scan_id: ResId<res_id::SCAN>,
    position_override: Option<[f32;3]>,
) -> ([f32; 3], ResId<res_id::SCAN>)
{
    let pickup = pickup_obj.property_data.as_pickup_mut().unwrap();
    let mut original_pickup = pickup.clone();

    if pickup_config.position.is_some() {
        original_pickup.position = pickup_config.position.unwrap().into();
    }

    if position_override.is_some() {
        original_pickup.position = position_override.unwrap().into();
    }

    let original_aabb = pickup_meta::aabb_for_pickup_cmdl(original_pickup.cmdl).unwrap();
    let new_aabb = pickup_meta::aabb_for_pickup_cmdl(pickup_model_data.cmdl).unwrap_or(
        pickup_meta::aabb_for_pickup_cmdl(PickupModel::EnergyTank.pickup_data().cmdl).unwrap()
    );
    let original_center = calculate_center(original_aabb, original_pickup.rotation,
                                            original_pickup.scale);
    let new_center = calculate_center(new_aabb, pickup_model_data.rotation,
                                        pickup_model_data.scale);

    let curr_increase = {
        if pickup_type == PickupType::Nothing {
            0
        } else {
            if pickup_config.curr_increase.is_some() {
                pickup_config.curr_increase.unwrap()
            } else {
                if pickup_type == PickupType::Missile {
                    5
                } else if pickup_type == PickupType::HealthRefill {
                    50
                } else {
                    1
                }
            }
        }
    };
    let max_increase = {
        if pickup_type == PickupType::Nothing || pickup_type == PickupType::HealthRefill {
            0
        } else {
            pickup_config.max_increase.unwrap_or(curr_increase)
        }
    };
    let kind = {
        if pickup_type == PickupType::Nothing {
            PickupType::HealthRefill.kind()
        } else {
            pickup_type.kind()
        }
    };

    // The pickup needs to be repositioned so that the center of its model
    // matches the center of the original.
    let mut position = [
        original_pickup.position[0] - (new_center[0] - original_center[0]),
        original_pickup.position[1] - (new_center[1] - original_center[1]),
        original_pickup.position[2] - (new_center[2] - original_center[2]),
    ];

    let mut scan_offset = [
        original_pickup.scan_offset[0] + (new_center[0] - original_center[0]),
        original_pickup.scan_offset[1] + (new_center[1] - original_center[1]),
        original_pickup.scan_offset[2] + (new_center[2] - original_center[2]),
    ];

    // If this is the echoes missile expansion model, compensate for the Z offset
    let json_pickup_name = pickup_config.model.as_ref().unwrap_or(&"".to_string()).clone();
    if json_pickup_name.contains(&"prime2_MissileExpansion") || json_pickup_name.contains(&"prime2_UnlimitedMissiles") {
        position[2] -= 1.2;
        scan_offset[2] += 1.2;
    }

    *pickup = structs::Pickup {
        // Location Pickup Data
        // "How is this pickup integrated into the room?"
        name: original_pickup.name,
        position: position.into(),
        rotation: pickup_model_data.rotation.clone().into(),
        hitbox: original_pickup.hitbox,
        scan_offset: scan_offset.into(),
        fade_in_timer:  original_pickup.fade_in_timer,
        spawn_delay: original_pickup.spawn_delay,
        disappear_timer: original_pickup.disappear_timer,
        active: original_pickup.active,
        drop_rate: original_pickup.drop_rate,

        // Type Pickup Data
        // "What does this pickup do?"
        curr_increase,
        max_increase,
        kind,

        // Model Pickup Data
        // "What does this pickup look like?"
        scale: pickup_model_data.scale,
        cmdl: pickup_model_data.cmdl.clone(),
        ancs: pickup_model_data.ancs.clone(),
        part: pickup_model_data.part.clone(),
        actor_params: pickup_model_data.actor_params.clone(),
    };

    // Should we use non-default scan id? //
    pickup.actor_params.scan_params.scan = scan_id;

    (position, pickup.actor_params.scan_params.scan)
}

fn update_hudmemo(
    hudmemo: &mut structs::SclyObject,
    hudmemo_strg: ResId<res_id::STRG>,
    skip_hudmemos: bool,
    hudmemo_delay: f32,
)
{
    let hudmemo = hudmemo.property_data.as_hud_memo_mut().unwrap();
    hudmemo.strg = hudmemo_strg;
    if hudmemo_delay != 0.0 {
        hudmemo.first_message_timer = hudmemo_delay;
    }

    if skip_hudmemos {
        hudmemo.memo_type = 0;
        hudmemo.first_message_timer = 5.0;
    }
}

fn update_attainment_audio(
    attainment_audio: &mut structs::SclyObject,
    pickup_type: PickupType,
)
{
    let attainment_audio = attainment_audio.property_data.as_streamed_audio_mut().unwrap();
    let bytes = pickup_type.attainment_audio_file_name().as_bytes();
    attainment_audio.audio_file_name = bytes.as_cstr();
}

fn calculate_center(aabb: [f32; 6], rotation: GenericArray<f32, U3>, scale: GenericArray<f32, U3>)
    -> [f32; 3]
{
    let start = [aabb[0], aabb[1], aabb[2]];
    let end = [aabb[3], aabb[4], aabb[5]];

    let mut position = [0.; 3];
    for i in 0..3 {
        position[i] = (start[i] + end[i]) / 2. * scale[i];
    }

    rotate(position, [rotation[0], rotation[1], rotation[2]], [0.; 3])
}

fn rotate(mut coordinate: [f32; 3], mut rotation: [f32; 3], center: [f32; 3])
    -> [f32; 3]
{
    // Shift to the origin
    for i in 0..3 {
        coordinate[i] -= center[i];
        rotation[i] = rotation[i].to_radians();
    }

    for i in 0..3 {
        let original = coordinate;
        let x = (i + 1) % 3;
        let y = (i + 2) % 3;
        coordinate[x] = original[x] * rotation[i].cos() - original[y] * rotation[i].sin();
        coordinate[y] = original[x] * rotation[i].sin() + original[y] * rotation[i].cos();
    }

    // Shift back to original position
    for i in 0..3 {
        coordinate[i] += center[i];
    }
    coordinate
}

fn patch_samus_actor_size<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    player_size: f32,
) -> Result<(), String>
{
    let mrea_id = area.mlvl_area.mrea.to_u32();
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec() {
        for obj in layer.objects.as_mut_vec() {
            if obj.property_data.is_player_actor() {
                let player_actor = obj.property_data.as_player_actor_mut().unwrap();
                player_actor.scale[0] = player_actor.scale[0]*player_size;
                player_actor.scale[1] = player_actor.scale[1]*player_size;
                player_actor.scale[2] = player_actor.scale[2]*player_size;
            }

            if mrea_id == 0xb4b41c48
            {
                if obj.property_data.is_actor()
                {
                    let actor = obj.property_data.as_actor_mut().unwrap();
                    if actor.name.to_str().unwrap().contains(&"Samus")
                    {
                        actor.scale[0] = actor.scale[0]*player_size;
                        actor.scale[1] = actor.scale[1]*player_size;
                        actor.scale[2] = actor.scale[2]*player_size;
                    }
                }

                // for the end movie, go the extra mile and tilt the cameras down
                if player_size < 0.75
                {
                    if obj.property_data.is_camera()
                    {
                        let camera = obj.property_data.as_camera_mut().unwrap();
                        let name = camera.name.to_str().unwrap().to_lowercase();
                        if name.contains(&"buttons") {
                            camera.rotation[0] = -2.0;
                        } else if name.contains(&"camera4") {
                            camera.rotation[0] = -5.0;
                        }
                    }

                    if vec![0x000004AF, 0x000004A4, 0x00000461, 0x00000477, 0x00000476, 0x00000474, 0x00000479, 0x00000478, 0x00000473, 0x0000045B].contains(&(obj.instance_id&0x0000FFFF))
                    {
                        let waypoint = obj.property_data.as_waypoint_mut().unwrap();
                        waypoint.position[2] = waypoint.position[2] - 2.2;
                    }
                }
            }
        }
    }
    Ok(())
}

fn patch_elevator_actor_size<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    player_size: f32,
) -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec().iter_mut() {
        for obj in layer.objects.as_mut_vec().iter_mut() {
            if !obj.property_data.is_world_transporter() { continue; }
            let wt = obj.property_data.as_world_transporter_mut().unwrap();
            wt.player_scale[0] = wt.player_scale[0]*player_size;
            wt.player_scale[1] = wt.player_scale[1]*player_size;
            wt.player_scale[2] = wt.player_scale[2]*player_size;
        }
    }

    Ok(())
}

fn make_elevators_patch<'a>(
    patcher: &mut PrimePatcher<'_, 'a>,
    level_data: &HashMap<String, LevelConfig>,
    auto_enabled_elevators: bool,
    player_size: f32,
    force_vanilla_layout: bool,
)
-> (bool, bool)
{
    for (pak_name, rooms) in pickup_meta::ROOM_INFO.iter() {
        for room_info in rooms.iter() {
            patcher.add_scly_patch(
                (pak_name.as_bytes(), room_info.room_id.to_u32()),
                move |ps, area| patch_elevator_actor_size(ps, area, player_size),
            );
        }
    }

    if force_vanilla_layout {
        return (false, false);
    }

    let mut skip_frigate = true;
    let mut skip_ending_cinematic = false;
    for (_, level) in level_data.iter() {
        for (elevator_name, destination_name) in level.transports.iter() {

            // special cases, handled elsewhere
            if vec!["Frigate Escape Cutscene", "Essence Dead Cutscene"].contains(&(elevator_name.as_str())) {
                continue;
            }

            let elv = Elevator::from_str(&elevator_name);
            if elv.is_none() {
                panic!("Failed to parse elevator '{}'", elevator_name);
            }
            let elv = elv.unwrap();
            let dest = SpawnRoomData::from_str(destination_name);

            if dest.mlvl == World::FrigateOrpheon.mlvl() {
                skip_frigate = false;
            }

            if dest.mrea == SpawnRoom::EndingCinematic.spawn_room_data().mrea {
                skip_ending_cinematic = true;
            }

            patcher.add_scly_patch((elv.pak_name.as_bytes(), elv.mrea), move |ps, area| {
                let scly = area.mrea().scly_section_mut();
                for layer in scly.layers.iter_mut() {
                    let obj = layer.objects.iter_mut()
                        .find(|obj| obj.instance_id == elv.scly_id);
                    if let Some(obj) = obj {
                        let wt = obj.property_data.as_world_transporter_mut().unwrap();
                        wt.mrea = ResId::new(dest.mrea);
                        wt.mlvl = ResId::new(dest.mlvl);
                        wt.volume = 0; // Turning off the wooshing sound
                    }
                }

                if auto_enabled_elevators {
                    // Auto enable the elevator
                    let layer = &mut scly.layers.as_mut_vec()[0];
                    let mr_id = layer.objects.iter()
                        .find(|obj| obj.property_data.as_memory_relay()
                            .map(|mr| mr.name == b"Memory Relay - dim scan holo\0".as_cstr())
                            .unwrap_or(false)
                        )
                        .map(|mr| mr.instance_id);

                    if let Some(mr_id) = mr_id {
                        layer.objects.as_mut_vec().push(structs::SclyObject {
                            instance_id: ps.fresh_instance_id_range.next().unwrap(),
                            property_data: structs::Timer {
                                name: b"Auto enable elevator\0".as_cstr(),

                                start_time: 0.001,
                                max_random_add: 0f32,
                                reset_to_zero: 0,
                                start_immediately: 1,
                                active: 1,
                            }.into(),
                            connections: vec![
                                structs::Connection {
                                    state: structs::ConnectionState::ZERO,
                                    message: structs::ConnectionMsg::ACTIVATE,
                                    target_object_id: mr_id,
                                },
                            ].into(),
                        });
                    }
                }

                Ok(())
            });

            let dest_world_name = {
                if dest.mlvl == World::FrigateOrpheon.mlvl() {
                    "Frigate"
                } else if dest.mlvl == World::TallonOverworld.mlvl() {
                    "Tallon Overworld"
                } else if dest.mlvl == World::ChozoRuins.mlvl() {
                    "Chozo Ruins"
                } else if dest.mlvl == World::MagmoorCaverns.mlvl() {
                    "Magmoor Caverns"
                } else if dest.mlvl == World::PhendranaDrifts.mlvl() {
                    "Phendrana Drifts"
                } else if dest.mlvl == World::PhazonMines.mlvl() {
                    "Phazon Mines"
                } else if dest.mlvl == World::ImpactCrater.mlvl() {
                    "Impact Crater"
                } else if dest.mlvl == 0x13d79165 {
                    "Credits"
                } else {
                    panic!("unhandled mlvl destination - {}", dest.mlvl)
                }
            };

            let mut is_dest_elev = false;
            for elv in Elevator::iter() {
                if elv.elevator_data().mrea == dest.mrea {
                    is_dest_elev = true;
                    break;
                }
            }

            let room_dest_name = {
                if dest.mlvl == 0x13d79165 {
                    "End of Game".to_string()
                } else if is_dest_elev {
                    dest.name.replace('\0', "\n")
                } else {
                    format!("{} - {}", dest_world_name, dest.name.replace('\0', "\n"))
                }
            };
            let hologram_name = {
                if dest.mlvl == 0x13d79165 {
                    "End of Game".to_string()
                } else if is_dest_elev {
                    dest.name.replace('\0', " ")
                } else {
                    format!("{} - {}", dest_world_name, dest.name.replace('\0', " "))
                }
            };
            let control_name = hologram_name.clone();

            patcher.add_resource_patch((&[elv.pak_name.as_bytes()], elv.room_strg, b"STRG".into()), move |res| {
                let string = format!("Transport to {}\u{0}", room_dest_name);
                let strg = structs::Strg::from_strings(vec![string]);
                res.kind = structs::ResourceKind::Strg(strg);
                Ok(())
            });
            patcher.add_resource_patch((&[elv.pak_name.as_bytes()], elv.hologram_strg, b"STRG".into()), move |res| {
                let string = format!(
                    "Access to &main-color=#FF3333;{} &main-color=#89D6FF;granted. Please step into the hologram.\u{0}",
                    hologram_name,
                );
                let strg = structs::Strg::from_strings(vec![string]);
                res.kind = structs::ResourceKind::Strg(strg);
                Ok(())
            });
            patcher.add_resource_patch((&[elv.pak_name.as_bytes()], elv.control_strg, b"STRG".into()), move |res| {
                let string = format!(
                    "Transport to &main-color=#FF3333;{}&main-color=#89D6FF; active.\u{0}",
                    control_name,
                );
                let strg = structs::Strg::from_strings(vec![string]);
                res.kind = structs::ResourceKind::Strg(strg);
                Ok(())
            });
        }
    }

    (skip_frigate, skip_ending_cinematic)
}

fn patch_post_pq_frigate(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
) -> Result<(), String>
{
    let room_id = area.mlvl_area.mrea.to_u32();
    let layer_count = area.layer_flags.layer_count as usize;
    let layers = area.mrea().scly_section_mut().layers.as_mut_vec();
    for i in 0..layer_count {
        layers[i].objects.as_mut_vec().retain(|obj|
            !vec![
                0x00010074, 0x00010070, 0x00010072, 0x00010071, 0x00010073, 0x00010009, // Air Lock
                0x000E003B, 0x000E0025, 0x000E00CF, 0x000E0095, // Biotech 1
                0x0003000D, 0x0003000C, // Mech Shaft
                0x000500AF, 0x000500AE, 0x000500B1, 0x0005013F, // Alpha Elevator
            ].contains(&(obj.instance_id&0x00FFFFFF))
        );
    }
    let hatch = layers[0].objects.iter_mut()
        .find(|obj| obj.instance_id&0x00FFFFFF == 0x00010064);
    if hatch.is_some() {
        let hatch = hatch.unwrap();
        for conn in hatch.connections.as_mut_vec().iter_mut() {
            if conn.message == structs::ConnectionMsg::DEACTIVATE {
                conn.message = structs::ConnectionMsg::ACTIVATE;
            }
        }
    }

    // Air lock
    if layer_count > 1 {
        let trigger = layers[1].objects.iter_mut()
            .find(|obj| obj.instance_id&0x00FFFFFF == 0x000E003A);
        if trigger.is_some() {
            let trigger = trigger.unwrap();
            for conn in trigger.connections.as_mut_vec().iter_mut() {
                if vec![
                    0x000E0122, // 0x000E0079, 0x000E0078,
                ].contains(&(conn.target_object_id & 0x00FFFFFF)) {
                    conn.message = structs::ConnectionMsg::ACTIVATE;
                }
            }
        }
    }

    let cfldg_trigger = layers[0].objects.iter_mut()
        .find(|obj| obj.instance_id&0x00FFFFFF == 0x001A00B7);
    if cfldg_trigger.is_some() {
        let cfldg_trigger = cfldg_trigger.unwrap();
        cfldg_trigger.connections.as_mut_vec().push(
            structs::Connection {
                state: structs::ConnectionState::INSIDE,
                message: structs::ConnectionMsg::SET_TO_MAX,
                target_object_id: 0x001A011D,
            }
        );
        cfldg_trigger.connections.as_mut_vec().push(
            structs::Connection {
                state: structs::ConnectionState::INSIDE,
                message: structs::ConnectionMsg::SET_TO_ZERO,
                target_object_id: 0x001A00D3,
            }
        );
        cfldg_trigger.connections.as_mut_vec().push(
            structs::Connection {
                state: structs::ConnectionState::INSIDE,
                message: structs::ConnectionMsg::SET_TO_ZERO,
                target_object_id: 0x001A00D4,
            }
        );
        cfldg_trigger.connections.as_mut_vec().push(
            structs::Connection {
                state: structs::ConnectionState::INSIDE,
                message: structs::ConnectionMsg::SET_TO_ZERO,
                target_object_id: 0x001A00FB,
            }
        );
        cfldg_trigger.connections.as_mut_vec().push(
            structs::Connection {
                state: structs::ConnectionState::ENTERED,
                message: structs::ConnectionMsg::RESET_AND_START,
                target_object_id: 0x001A005D,
            }
        );
        let trigger_property_data = cfldg_trigger.property_data.as_trigger_mut().unwrap();
        trigger_property_data.position = [185.410889, -233.339539, -86.378212].into();
        trigger_property_data.flags = 0x1000; // detect morphed player
    }

    // reactor core entrance
    if room_id == 0x3ea190ee {
        layers[0].objects.as_mut_vec().push(
            structs::SclyObject {
                instance_id: _ps.fresh_instance_id_range.next().unwrap(),
                property_data: structs::Trigger {
                    name: b"Trigger\0".as_cstr(),
                    position: [184.816299, -263.740845, -86.882622].into(),
                    scale: [1.5, 1.5, 1.5].into(),
                    damage_info: structs::scly_structs::DamageInfo {
                        weapon_type: 0,
                        damage: 0.0,
                        radius: 0.0,
                        knockback_power: 0.0
                    },
                    force: [0.0, 0.0, 0.0].into(),
                    flags: 0x1000, // detect morphed
                    active: 1,
                    deactivate_on_enter: 0,
                    deactivate_on_exit: 0
                }.into(),
                connections: vec![
                    structs::Connection {
                        state: structs::ConnectionState::INSIDE,
                        message: structs::ConnectionMsg::SET_TO_MAX,
                        target_object_id: 0x001B0002,
                    },
                    structs::Connection {
                        state: structs::ConnectionState::INSIDE,
                        message: structs::ConnectionMsg::SET_TO_ZERO,
                        target_object_id: 0x001B0001,
                    },
                    structs::Connection {
                        state: structs::ConnectionState::ENTERED,
                        message: structs::ConnectionMsg::SET_TO_ZERO,
                        target_object_id: 0x001B007F,
                    },
                    structs::Connection {
                        state: structs::ConnectionState::ENTERED,
                        message: structs::ConnectionMsg::RESET_AND_START,
                        target_object_id: 0x001B0041,
                    }
                ].into()
            }
        );
    }

    Ok(())
}

fn patch_add_circle_platform<'r>(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    position: [f32;3],
) -> Result<(), String>
{
    let deps = vec![
        (0x48DF38A3, b"CMDL"),
        (0xB2D50628, b"DCLN"),
        (0x19C17D5C, b"TXTR"),
        (0x0259F5F6, b"TXTR"),
        (0x71190250, b"TXTR"),
        (0xD0BA0FA8, b"TXTR"),
        (0xF1478D6A, b"TXTR"),
    ];
    let deps_iter = deps.iter()
        .map(|&(file_id, fourcc)| structs::Dependency {
            asset_id: file_id,
            asset_type: FourCC::from_bytes(fourcc),
        }
    );
    area.add_dependencies(game_resources,0,deps_iter);

    let layers = area.mrea().scly_section_mut().layers.as_mut_vec();
    layers[0].objects.as_mut_vec().push(
        structs::SclyObject {
            instance_id: ps.fresh_instance_id_range.next().unwrap(),
            property_data: structs::Platform {
                name: b"myplatform\0".as_cstr(),

                position: position.into(),
                rotation: [0.0, 0.0, -90.0].into(),
                scale: [1.0, 1.0, 1.0].into(),
                extent: [0.0, 0.0, 0.0].into(),
                scan_offset: [0.0, 0.0, 0.0].into(),

                cmdl: ResId::<res_id::CMDL>::new(0x48DF38A3),
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
                        world_lighting: 3,
                        light_recalculation: 1,
                        unknown5: [0.0, 0.0, 0.0].into(),
                        unknown6: 4,
                        unknown7: 4,
                        unknown8: 1,
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

                dcln: ResId::<res_id::DCLN>::new(0xB2D50628),

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

fn patch_add_block<'r>(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    position: [f32;3],
    scale: [f32;3],
    // rotation: [f32;3],
) -> Result<(), String>
{
    let deps = vec![
        (0x27D0663B, b"CMDL"),
        (0xFF6F41A6, b"TXTR"),
    ];
    let deps_iter = deps.iter()
        .map(|&(file_id, fourcc)| structs::Dependency {
            asset_id: file_id,
            asset_type: FourCC::from_bytes(fourcc),
        }
    );
    area.add_dependencies(game_resources,0,deps_iter);

    let layers = area.mrea().scly_section_mut().layers.as_mut_vec();
    layers[0].objects.as_mut_vec().push(
        structs::SclyObject {
            instance_id: ps.fresh_instance_id_range.next().unwrap(),
            property_data: structs::Actor {
                name: b"myactor\0".as_cstr(),
                position: position.into(),
                rotation: [180.0, 0.0, 0.0].into(),
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
                cmdl: resource_info!("27D0663B.CMDL").try_into().unwrap(),
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
                solid: 1,
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

    Ok(())
}

fn patch_lock_on_point<'r>(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    position: [f32;3],
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
    area.add_dependencies(game_resources,0,deps_iter);

    let layers = area.mrea().scly_section_mut().layers.as_mut_vec();
    layers[0].objects.as_mut_vec().push(
        structs::SclyObject {
            instance_id: ps.fresh_instance_id_range.next().unwrap(),
            property_data: structs::Platform {
                name: b"myplatform\0".as_cstr(),

                position: position.into(),
                rotation: [0.0, 0.0, 0.0].into(),
                scale: [8.0, 8.0, 8.0].into(),
                extent: [0.0, 0.0, 0.0].into(),
                scan_offset: [0.0, 0.0, 0.0].into(),

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

                unknown1: 1.0,
                active: 1,

                dcln: ResId::invalid(),

                health_info: structs::scly_structs::HealthInfo {
                    health: 1.0,
                    knockback_resistance: 1.0,
                },
                damage_vulnerability: DoorType::Disabled.vulnerability(),

                detect_collision: 0,
                unknown4: 1.0,
                unknown5: 0,
                unknown6: 200,
                unknown7: 20,
            }.into(),
            connections: vec![].into(),
        }
    );

    layers[0].objects.as_mut_vec().push(
        structs::SclyObject {
            instance_id: ps.fresh_instance_id_range.next().unwrap(),
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
                active: 1,
                visor_params: structs::scly_structs::VisorParameters {
                    unknown0: 0,
                    target_passthrough: 0,
                    visor_mask: 15 // Combat|Scan|Thermal|XRay
                }
            }.into(),
            connections: vec![].into(),
        },
    );

    Ok(())
}

fn patch_ambient_lighting<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    scale: f32,
) -> Result<(), String>
{
    let lights: &mut structs::Lights = area.mrea().lights_section_mut();
    for light in lights.light_layers.as_mut_vec() {
        if light.light_type != 0x0 { // local ambient
            continue;
        }

        light.brightness *= scale;
    }

    Ok(())
}

// fn patch_add_orange_light<'r>(
//     ps: &mut PatcherState,
//     area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
//     game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
//     position: [f32;3],
//     scale: [f32;3],
// ) -> Result<(), String>
// {
//     let deps = vec![
//         (0xB4A658C3, b"PART"),
//     ];
//     let deps_iter = deps.iter()
//         .map(|&(file_id, fourcc)| structs::Dependency {
//             asset_id: file_id,
//             asset_type: FourCC::from_bytes(fourcc),
//         }
//     );
//     area.add_dependencies(game_resources,0,deps_iter);

//     let layers = area.mrea().scly_section_mut().layers.as_mut_vec();
//     layers[0].objects.as_mut_vec().push(
//         structs::SclyObject {
//             instance_id: ps.fresh_instance_id_range.next().unwrap(),
//             property_data: structs::scly_props::Effect {
//                 name: b"my effect\0".as_cstr(),

//                 position: position.into(),
//                 rotation: [0.0, 0.0, 0.0].into(),
//                 scale: scale.into(),
//                 part: resource_info!("B4A658C3.PART").try_into().unwrap(),
//                 elsc: ResId::invalid(),
//                 hot_in_thermal: 0,
//                 no_timer_unless_area_occluded: 0,
//                 rebuild_systems_on_active: 0,
//                 active: 1,
//                 use_rate_inverse_cam_dist: 0,
//                 rate_inverse_cam_dist: 5.0,
//                 rate_inverse_cam_dist_rate: 0.5,
//                 duration: 0.2,
//                 dureation_reset_while_visible: 0.1,
//                 use_rate_cam_dist_range: 0,
//                 rate_cam_dist_range_min: 20.0,
//                 rate_cam_dist_range_max: 30.0,
//                 rate_cam_dist_range_far_rate: 0.0,
//                 combat_visor_visible: 1,
//                 thermal_visor_visible: 1,
//                 xray_visor_visible: 1,
//                 die_when_systems_done: 0,
//                 light_params: structs::scly_structs::LightParameters {
//                     unknown0: 1,
//                     unknown1: 1.0,
//                     shadow_tessellation: 0,
//                     unknown2: 1.0,
//                     unknown3: 20.0,
//                     color: [1.0, 1.0, 1.0, 1.0].into(),
//                     unknown4: 0,
//                     world_lighting: 1,
//                     light_recalculation: 1,
//                     unknown5: [0.0, 0.0, 0.0].into(),
//                     unknown6: 4,
//                     unknown7: 4,
//                     unknown8: 0,
//                     light_layer_id: 0
//                 },
//             }.into(),
//             connections: vec![].into()
//         },
//     );

//     Ok(())
// }

fn patch_disable_item_loss(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
) -> Result<(), String>
{
    let layer = area.mrea().scly_section_mut().layers.iter_mut().next().unwrap();
    let camera = layer.objects.iter_mut()
        .find(|obj| obj.instance_id&0x00FFFFFF == 0x00050115)
        .unwrap();
    for conn in camera.connections.as_mut_vec().iter_mut() {
        if conn.message == structs::ConnectionMsg::RESET {
            conn.message = structs::ConnectionMsg::SET_TO_ZERO;
        }
    }
    Ok(())
}

fn patch_landing_site_cutscene_triggers(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
) -> Result<(), String>
{
    // XXX I'd like to do this some other way than inserting a timer to trigger
    //     the memory relay, but I couldn't figure out how to make the memory
    //     relay default to on/enabled.
    let layer = area.mrea().scly_section_mut().layers.iter_mut().next().unwrap();
    let timer_id = ps.fresh_instance_id_range.next().unwrap();
    for obj in layer.objects.iter_mut() {
        if obj.instance_id == 427 {
            obj.connections.as_mut_vec().push(structs::Connection {
                state: structs::ConnectionState::ACTIVE,
                message: structs::ConnectionMsg::DEACTIVATE,
                target_object_id: timer_id,
            });
        }
        if obj.instance_id == 221 {
            obj.property_data.as_trigger_mut().unwrap().active = 0;
        }
    }
    layer.objects.as_mut_vec().push(structs::SclyObject {
        instance_id: timer_id,
        property_data: structs::Timer {
            name: b"Cutscene fixup timer\0".as_cstr(),

            start_time: 0.001,
            max_random_add: 0f32,
            reset_to_zero: 0,
            start_immediately: 1,
            active: 1,
        }.into(),
        connections: vec![
            structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::ACTIVATE,
                target_object_id: 323,// "Memory Relay Set For Load"
            },
            structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::ACTIVATE,
                target_object_id: 427,// "Memory Relay Ship"
            },
            structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::ACTIVATE,
                target_object_id: 484,// "Effect_BaseLights"
            },
            structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::ACTIVATE,
                target_object_id: 463,// "Actor Save Station Beam"
            },
        ].into(),
    });
    Ok(())
}

fn patch_ending_scene_straight_to_credits(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
) -> Result<(), String>
{
    let layer = area.mrea().scly_section_mut().layers.iter_mut().next().unwrap();
    let trigger = layer.objects.iter_mut()
        .find(|obj| obj.instance_id == 1103) // "Trigger - Start this Beatch"
        .unwrap();
    trigger.connections.as_mut_vec().push(structs::Connection {
        state: structs::ConnectionState::ENTERED,
        message: structs::ConnectionMsg::ACTION,
        target_object_id: 1241, // "SpecialFunction-edngame"
    });
    Ok(())
}

fn patch_arboretum_vines(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
) -> Result<(), String>
{
    let layers = area.mrea().scly_section_mut().layers.as_mut_vec();
    let weeds = layers[1].objects.iter_mut()
        .find(|obj| obj.instance_id&0x00FFFFFF == 0x00130135)
        .unwrap().clone();

    layers[0].objects.as_mut_vec().push(weeds.clone());
    layers[1].objects.as_mut_vec().retain(|obj|
        obj.instance_id&0x00FFFFFF != 0x00130135
    );

    Ok(())
}

fn patch_teleporter_destination<'r>(
    area: &mut mlvl_wrapper::MlvlArea,
    spawn_room: SpawnRoomData,
)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let wt = scly.layers.iter_mut()
        .flat_map(|layer| layer.objects.iter_mut())
        .find(|obj| obj.property_data.is_world_transporter())
        .and_then(|obj| obj.property_data.as_world_transporter_mut())
        .unwrap();
    wt.mlvl = ResId::new(spawn_room.mlvl);
    wt.mrea = ResId::new(spawn_room.mrea);
    Ok(())
}

fn patch_add_load_trigger<'r>(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    position: [f32;3],
    scale: [f32;3],
    dock_num: u32,
)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    
    // Collect all docks in this room
    let mut docks: HashMap<u32, u32> = HashMap::new(); // <dock num, instance id>
    for obj in layer.objects.as_mut_vec() {
        if !obj.property_data.is_dock() {
            continue;
        }

        let dock = obj.property_data.as_dock().unwrap();
        docks.insert(dock.dock_index, obj.instance_id);
    }

    let mut connections: Vec::<structs::Connection> = Vec::new();

    for (dock, instance_id) in docks {
        if dock == dock_num {
            connections.push(
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::SET_TO_MAX,
                    target_object_id: instance_id,
                }
            );
        } else {
            connections.push(
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                    target_object_id: instance_id,
                }
            );
        }
    }

    layer.objects.as_mut_vec().push(
        structs::SclyObject {
            instance_id: ps.fresh_instance_id_range.next().unwrap(),
            property_data: structs::Trigger {
                name: b"Trigger\0".as_cstr(),
                position: position.into(),
                scale: scale.into(),
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
            connections: connections.into(),
        }    
    );

    Ok(())
}

fn fix_artifact_of_truth_requirements(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: &PatchConfig,
) -> Result<(), String>
{
    let level_data: HashMap<String, LevelConfig> = config.level_data.clone();
    let artifact_temple_layer_overrides = config.artifact_temple_layer_overrides.clone().unwrap_or(HashMap::new());

    // Create a new layer that will be toggled on when the Artifact of Truth is collected
    let truth_req_layer_id = area.layer_flags.layer_count;
    area.add_layer(b"Randomizer - Got Artifact 1\0".as_cstr());

    // What is the item at artifact temple?
    let at_pickup_kind = {
        let mut _at_pickup_kind = 0; // nothing item if unspecified
        if level_data.contains_key(World::TallonOverworld.to_json_key()) {
            let rooms = &level_data.get(World::TallonOverworld.to_json_key()).unwrap().rooms;
            if rooms.contains_key("Artifact Temple") {
                let artifact_temple_pickups = &rooms.get("Artifact Temple").unwrap().pickups;
                if artifact_temple_pickups.is_some() {
                    let artifact_temple_pickups = artifact_temple_pickups.as_ref().unwrap();
                    if artifact_temple_pickups.len() != 0 {
                        _at_pickup_kind = PickupType::from_str(&artifact_temple_pickups[0].pickup_type).kind();
                    }
                }
            }
        }
        _at_pickup_kind
    };

    for i in 0..12 {
        let layer_number = if i == 0 {
            truth_req_layer_id
        } else {
            i + 1
        };
        let kind = i + 29;

        let exists = {
            let mut _exists = false;
            for (_, level) in level_data.iter() {
                if _exists {break;}
                for (_, room) in level.rooms.iter() {
                    if _exists {break;}
                    if room.pickups.is_none() { continue };
                    for pickup in room.pickups.as_ref().unwrap().iter() {
                        let pickup = PickupType::from_str(&pickup.pickup_type);
                        if pickup.kind() == kind {
                            _exists = true; // this artifact is placed somewhere in this world
                            break;
                        }
                    }
                }
            }

            for (key, value) in &artifact_temple_layer_overrides {
                let artifact_name = match kind {
                    33 => "lifegiver",
                    32 => "wild",
                    38 => "world",
                    37 => "sun",
                    31 => "elder",
                    39 => "spirit",
                    29 => "truth",
                    35 => "chozo",
                    34 => "warrior",
                    40 => "newborn",
                    36 => "nature",
                    30 => "strength",
                    _ => panic!("Unhandled artifact idx - '{}'", i),
                };

                if key.to_lowercase().contains(&artifact_name) {
                    _exists = _exists || *value; // if value is true, override
                    break;
                }
            }
            _exists
        };

        if exists && at_pickup_kind != kind {
            // If the artifact exists,
            // and it is not the artifact at the Artifact Temple
            // or it's placed in another player's game (multi-world)
            // THEN mark this layer as inactive. It will be activated when the item is collected.
            area.layer_flags.flags &= !(1 << layer_number);
        } else {
            // Either the artifact doesn't exist or it does and it is in the Artifact Temple, so
            // mark this layer as active. In the former case, it needs to always be active since it
            // will never be collect and in the latter case it needs to be active so the Ridley
            // fight can start immediately if its the last artifact collected.
            area.layer_flags.flags |= 1 << layer_number;
        }
    }

    let scly = area.mrea().scly_section_mut();

    // A relay on the new layer is created and connected to "Relay Show Progress 1"
    let new_relay_instance_id = ps.fresh_instance_id_range.next().unwrap();
    let new_relay = structs::SclyObject {
        instance_id: new_relay_instance_id,
        connections: vec![
            structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::SET_TO_ZERO,
                target_object_id: 1048869,
            },
        ].into(),
        property_data: structs::Relay {
            name: b"Relay Show Progress1\0".as_cstr(),
            active: 1,
        }.into(),
    };
    scly.layers.as_mut_vec()[truth_req_layer_id as usize].objects.as_mut_vec().push(new_relay);

    // An existing relay is disconnected from "Relay Show Progress 1" and connected
    // to the new relay
    let relay = scly.layers.as_mut_vec()[1].objects.iter_mut()
        .find(|i| i.instance_id == 68158836).unwrap();
    relay.connections.as_mut_vec().retain(|i| i.target_object_id != 1048869);
    relay.connections.as_mut_vec().push(structs::Connection {
        state: structs::ConnectionState::ZERO,
        message: structs::ConnectionMsg::SET_TO_ZERO,
        target_object_id: new_relay_instance_id,
    });
    Ok(())
}

fn patch_artifact_hint_availability(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    hint_behavior: ArtifactHintBehavior,
) -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    const HINT_RELAY_OBJS: &[u32] = &[
        68157732,
        68157735,
        68157738,
        68157741,
        68157744,
        68157747,
        68157750,
        68157753,
        68157756,
        68157759,
        68157762,
        68157765,
    ];
    match hint_behavior {
        ArtifactHintBehavior::Default => (),
        ArtifactHintBehavior::All => {
            // Unconditionaly connect the hint relays directly to the relay that triggers the hints
            // to conditionally.
            let obj = scly.layers.as_mut_vec()[0].objects.iter_mut()
                .find(|obj| obj.instance_id == 1048956) // "Relay One Shot Out"
                .unwrap();
            obj.connections.as_mut_vec().extend(HINT_RELAY_OBJS.iter().map(|id| {
                structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                    target_object_id: *id,
                }
            }));
        },
        ArtifactHintBehavior::None => {
            // Remove relays that activate artifact hint objects
            scly.layers.as_mut_vec()[1].objects.as_mut_vec()
                .retain(|obj| !HINT_RELAY_OBJS.contains(&obj.instance_id));
        },
    }
    Ok(())
}

fn patch_sun_tower_prevent_wild_before_flaahgra(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea
) -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let idx = scly.layers.as_mut_vec()[0].objects.iter_mut()
        .position(|obj| obj.instance_id == 0x001d015b)
        .unwrap();
    let sunchamber_layer_change_trigger = scly.layers.as_mut_vec()[0].objects.as_mut_vec().remove(idx);
    *scly.layers.as_mut_vec()[1].objects.as_mut_vec() = vec![sunchamber_layer_change_trigger];
    Ok(())
}


fn patch_sunchamber_prevent_wild_before_flaahgra(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea
) -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let enable_sun_tower_layer_id = ps.fresh_instance_id_range.next().unwrap();
    scly.layers.as_mut_vec()[1].objects.as_mut_vec().push(structs::SclyObject {
        instance_id: enable_sun_tower_layer_id,
        connections: vec![].into(),
        property_data: structs::SpecialFunction::layer_change_fn(
            b"Enable Sun Tower Layer Change Trigger\0".as_cstr(),
            0xcf4c7aa5,
            1,
        ).into(),
    });
    let flaahgra_dead_relay = scly.layers.as_mut_vec()[1].objects.iter_mut()
        .find(|obj| obj.instance_id == 0x42500D4)
        .unwrap();
    flaahgra_dead_relay.connections.as_mut_vec().push(structs::Connection {
        state: structs::ConnectionState::ZERO,
        message: structs::ConnectionMsg::INCREMENT,
        target_object_id: enable_sun_tower_layer_id,
    });

    Ok(())
}

fn patch_essence_cinematic_skip_whitescreen(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
) -> Result<(), String>
{
    let timer_furashi_id = 0xB00E9;
    let camera_filter_key_frame_flash_id = 0xB011B;
    let timer_flashddd_id = 0xB011D;
    let special_function_cinematic_skip_id = 0xB01DC;

    let layer = area.mrea().scly_section_mut().layers.iter_mut().next().unwrap();
    let special_function_cinematic_skip_obj = layer.objects.iter_mut()
        .find(|obj| obj.instance_id == special_function_cinematic_skip_id) // "SpecialFunction Cineamtic Skip"
        .unwrap();
    special_function_cinematic_skip_obj.connections.as_mut_vec().extend_from_slice(
        &[
            structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::STOP,
                target_object_id: timer_furashi_id, // "Timer - furashi"
            },
            structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::DECREMENT,
                target_object_id: camera_filter_key_frame_flash_id, // "Camera Filter Keyframe Flash"
            },
            structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::STOP,
                target_object_id: timer_flashddd_id, // "Timer Flashddd"
            },
        ]);
    Ok(())
}

fn patch_essence_cinematic_skip_nomusic(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
) -> Result<(), String>
{
    let streamed_audio_essence_battle_theme_id = 0xB019E;
    let special_function_cinematic_skip_id = 0xB01DC;

    let layer = area.mrea().scly_section_mut().layers.iter_mut().next().unwrap();
    layer.objects.iter_mut()
        .find(|obj| obj.instance_id == special_function_cinematic_skip_id) // "SpecialFunction Cineamtic Skip"
        .unwrap()
        .connections
        .as_mut_vec().push(
            structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::PLAY,
                target_object_id: streamed_audio_essence_battle_theme_id, // "StreamedAudio Crater Metroid Prime Stage 2 SW"
            });
    Ok(())
}

fn patch_research_lab_hydra_barrier<'r>(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[3];

    let obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == 202965810)
        .unwrap();
    let actor = obj.property_data.as_actor_mut().unwrap();
    actor.actor_params.visor_params.target_passthrough = 1;
    Ok(())
}

fn patch_lab_aether_cutscene_trigger(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    version: Version,
) -> Result<(), String>
{
    let layer_num = if version == Version::NtscUTrilogy || version == Version::NtscJTrilogy || version == Version::PalTrilogy || version == Version::Pal || version == Version::NtscJ {
        4
    } else {
        5
    };
    let cutscene_trigger_id = 0x330317 + (layer_num << 26);
    let scly = area.mrea().scly_section_mut();
    let trigger = scly.layers.as_mut_vec()[layer_num as usize]
        .objects.iter_mut()
        .find(|obj| obj.instance_id == cutscene_trigger_id)
        .and_then(|obj| obj.property_data.as_trigger_mut())
        .unwrap();
    trigger.active = 0;

    Ok(())
}

fn patch_research_lab_aether_exploding_wall<'r>(
    ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea
)
    -> Result<(), String>
{
    // The room we're actually patching is Research Core..
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];

    let id = ps.fresh_instance_id_range.next().unwrap();
    let obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == 2622568)
        .unwrap();
    obj.connections.as_mut_vec().push(structs::Connection {
        state: structs::ConnectionState::ZERO,
        message: structs::ConnectionMsg::DECREMENT,
        target_object_id: id,
    });

    layer.objects.as_mut_vec().push(structs::SclyObject {
        instance_id: id,
        property_data: structs::SpecialFunction::layer_change_fn(
            b"SpecialFunction - Remove Research Lab Aether wall\0".as_cstr(),
            0x354889CE,
            3,
        ).into(),
        connections: vec![].into(),
    });
    Ok(())
}

fn patch_research_lab_aether_exploding_wall_2<'r>(
    _ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea
)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];

    // Alert Edward via trigger in lower area instead of relying on gameplay
    let trigger = layer.objects.iter_mut().find(|obj| obj.instance_id&0x00FFFFFF == 0x003302CE).unwrap();
    trigger.connections.as_mut_vec().push(structs::Connection {
        state: structs::ConnectionState::ENTERED,
        message: structs::ConnectionMsg::RESET_AND_START,
        target_object_id: 0x003300D7, // Timer to ALERT Edward
    });

    Ok(())
}

fn patch_observatory_2nd_pass_solvablility<'r>(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[2];

    let iter = layer.objects.as_mut_vec().iter_mut()
        .filter(|obj| obj.instance_id == 0x81E0460 || obj.instance_id == 0x81E0461);
    for obj in iter {
        obj.connections.as_mut_vec().push(structs::Connection {
            state: structs::ConnectionState::DEATH_RATTLE,
            message: structs::ConnectionMsg::INCREMENT,
            target_object_id: 0x1E02EA,// Counter - dead pirates active panel
        });
    }

    Ok(())
}

fn patch_observatory_1st_pass_softlock<'r>(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    // 0x041E0001 => starting at save station will allow us to kill pirates before the lock is active
    // 0x041E0002 => doing reverse lab will allow us to kill pirates before the lock is active
    const LOCK_DOOR_TRIGGER_IDS: &[u32] = &[
                        0x041E0381,
                        0x041E0001,
                        0x041E0002,
                    ];

    let enable_lock_relay_id = 0x041E037A;

    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[1];
    layer.objects.iter_mut()
        .find(|obj| obj.instance_id == LOCK_DOOR_TRIGGER_IDS[0])
        .unwrap()
        .connections.as_mut_vec().extend_from_slice(
            &[
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: LOCK_DOOR_TRIGGER_IDS[1],
                },
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: LOCK_DOOR_TRIGGER_IDS[2],
                },
            ]
        );

    layer.objects.as_mut_vec().extend_from_slice(&[
        structs::SclyObject {
            instance_id: LOCK_DOOR_TRIGGER_IDS[1],
            property_data: structs::Trigger {
                name: b"Trigger\0".as_cstr(),
                position: [-71.301552, -941.337952, 129.976822].into(),
                scale: [10.516006, 6.079956, 7.128998].into(),
                damage_info: structs::scly_structs::DamageInfo {
                    weapon_type: 0,
                    damage: 0.0,
                    radius: 0.0,
                    knockback_power: 0.0
                },
                force: [0.0, 0.0, 0.0].into(),
                flags: 1,
                active: 1,
                deactivate_on_enter: 1,
                deactivate_on_exit: 0
            }.into(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: LOCK_DOOR_TRIGGER_IDS[0],
                },
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: LOCK_DOOR_TRIGGER_IDS[2],
                },
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                    target_object_id: enable_lock_relay_id,
                },
            ].into()
        },
        structs::SclyObject {
            instance_id: LOCK_DOOR_TRIGGER_IDS[2],
            property_data: structs::Trigger {
                name: b"Trigger\0".as_cstr(),
                position: [-71.301552, -853.694336, 129.976822].into(),
                scale: [10.516006, 6.079956, 7.128998].into(),
                damage_info: structs::scly_structs::DamageInfo {
                    weapon_type: 0,
                    damage: 0.0,
                    radius: 0.0,
                    knockback_power: 0.0
                },
                force: [0.0, 0.0, 0.0].into(),
                flags: 1,
                active: 1,
                deactivate_on_enter: 1,
                deactivate_on_exit: 0
            }.into(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: LOCK_DOOR_TRIGGER_IDS[0],
                },
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: LOCK_DOOR_TRIGGER_IDS[1],
                },
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                    target_object_id: enable_lock_relay_id,
                },
            ].into()
        },
    ]);

    Ok(())
}

fn patch_main_ventilation_shaft_section_b_door<'r>(
    ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea
)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];

    layer.objects.as_mut_vec().push(structs::SclyObject {
        instance_id: ps.fresh_instance_id_range.next().unwrap(),
        property_data: structs::Trigger {
            name: b"Trigger_DoorOpen-component\0".as_cstr(),
            position: [31.232622, 442.69165, -64.20529].into(),
            scale: [6.0, 17.0, 6.0].into(),
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
                state: structs::ConnectionState::INSIDE,
                message: structs::ConnectionMsg::SET_TO_ZERO,
                target_object_id: 1376367,
            },
        ].into(),
    });
    Ok(())
}

fn make_main_plaza_locked_door_two_ways(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];

    let trigger_dooropen_id = 0x20007;
    let timer_doorclose_id = 0x20008;
    let actor_doorshield_id = 0x20004;
    let relay_unlock_id = 0x20159;
    let trigger_doorunlock_id = 0x2000F;
    let door_id = 0x20060;
    let trigger_remove_scan_target_locked_door_id = 0x202B8;
    let scan_target_locked_door_id = 0x202F4;
    let relay_notice_ineffective_weapon_id = 0x202FD;

    layer.objects.as_mut_vec().extend_from_slice(&[
        structs::SclyObject {
            instance_id: trigger_doorunlock_id,
            property_data: structs::DamageableTrigger {
                name: b"Trigger_DoorUnlock\0".as_cstr(),
                position: [152.232117, 86.451134, 24.472418].into(),
                scale: [0.25, 4.5, 4.0].into(),
                health_info: structs::scly_structs::HealthInfo {
                    health: 1.0,
                    knockback_resistance: 1.0
                },
                damage_vulnerability: structs::scly_structs::DamageVulnerability {
                    power: 1,           // Normal
                    ice: 1,             // Normal
                    wave: 1,            // Normal
                    plasma: 1,          // Normal
                    bomb: 1,            // Normal
                    power_bomb: 1,      // Normal
                    missile: 2,         // Reflect
                    boost_ball: 2,      // Reflect
                    phazon: 1,          // Normal
                    enemy_weapon0: 3,   // Immune
                    enemy_weapon1: 2,   // Reflect
                    enemy_weapon2: 2,   // Reflect
                    enemy_weapon3: 2,   // Reflect
                    unknown_weapon0: 2, // Reflect
                    unknown_weapon1: 2, // Reflect
                    unknown_weapon2: 1, // Normal
                    charged_beams: structs::scly_structs::ChargedBeams {
                        power: 1,       // Normal
                        ice: 1,         // Normal
                        wave: 1,        // Normal
                        plasma: 1,      // Normal
                        phazon: 1       // Normal
                    },
                    beam_combos: structs::scly_structs::BeamCombos {
                        power: 2,       // Reflect
                        ice: 2,         // Reflect
                        wave: 2,        // Reflect
                        plasma: 2,      // Reflect
                        phazon: 1       // Normal
                    }
                },
                unknown0: 3, // Render Side : East
                pattern_txtr0: resource_info!("testb.TXTR").try_into().unwrap(),
                pattern_txtr1: resource_info!("testb.TXTR").try_into().unwrap(),
                color_txtr: resource_info!("blue.TXTR").try_into().unwrap(),
                lock_on: 0,
                active: 1,
                visor_params: structs::scly_structs::VisorParameters {
                    unknown0: 0,
                    target_passthrough: 0,
                    visor_mask: 15 // Combat|Scan|Thermal|XRay
                }
            }.into(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::REFLECTED_DAMAGE,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                    target_object_id: relay_notice_ineffective_weapon_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::DEAD,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: actor_doorshield_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::MAX_REACHED,
                    message: structs::ConnectionMsg::ACTIVATE,
                    target_object_id: actor_doorshield_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::DEAD,
                    message: structs::ConnectionMsg::ACTIVATE,
                    target_object_id: trigger_dooropen_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::DEAD,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                    target_object_id: door_id,
                },
            ].into(),
        },

        structs::SclyObject {
            instance_id: relay_unlock_id,
            property_data: structs::Relay {
                    name: b"Relay_Unlock\0".as_cstr(),
                    active: 1,
                }.into(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::ACTIVATE,
                    target_object_id: actor_doorshield_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::ACTIVATE,
                    target_object_id: trigger_doorunlock_id,
                },
            ].into(),
        },

        structs::SclyObject {
            instance_id: trigger_dooropen_id,
            property_data: structs::Trigger {
                name: b"Trigger_DoorOpen\0".as_cstr(),
                position: [149.35614, 86.567917, 26.471249].into(),
                scale: [5.0, 5.0, 8.0].into(),
                damage_info: structs::scly_structs::DamageInfo {
                    weapon_type: 0,
                    damage: 0.0,
                    radius: 0.0,
                    knockback_power: 0.0
                },
                force: [0.0, 0.0, 0.0].into(),
                flags: 1,
                active: 0,
                deactivate_on_enter: 0,
                deactivate_on_exit: 0
            }.into(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::INSIDE,
                    message: structs::ConnectionMsg::OPEN,
                    target_object_id: door_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::INSIDE,
                    message: structs::ConnectionMsg::RESET_AND_START,
                    target_object_id: timer_doorclose_id,
                },
            ].into(),
        },

        structs::SclyObject {
            instance_id: actor_doorshield_id,
            property_data: structs::Actor {
                name: b"Actor_DoorShield\0".as_cstr(),
                position: [151.951187, 86.462578, 24.503178].into(),
                rotation: [0.0, 0.0, 0.0].into(),
                scale: [1.0, 1.0, 1.0].into(),
                hitbox: [0.0, 0.0, 0.0].into(),
                scan_offset: [0.0, 0.0, 0.0].into(),
                unknown1: 1.0,
                unknown2: 0.0,
                health_info: structs::scly_structs::HealthInfo {
                    health: 5.0,
                    knockback_resistance: 1.0
                },
                damage_vulnerability: structs::scly_structs::DamageVulnerability {
                    power: 1,           // Normal
                    ice: 1,             // Normal
                    wave: 1,            // Normal
                    plasma: 1,          // Normal
                    bomb: 1,            // Normal
                    power_bomb: 1,      // Normal
                    missile: 1,         // Normal
                    boost_ball: 1,      // Normal
                    phazon: 1,          // Normal
                    enemy_weapon0: 2,   // Reflect
                    enemy_weapon1: 2,   // Reflect
                    enemy_weapon2: 2,   // Reflect
                    enemy_weapon3: 2,   // Reflect
                    unknown_weapon0: 2, // Reflect
                    unknown_weapon1: 2, // Reflect
                    unknown_weapon2: 0, // Double Damage
                    charged_beams: structs::scly_structs::ChargedBeams {
                        power: 1,       // Normal
                        ice: 1,         // Normal
                        wave: 1,        // Normal
                        plasma: 1,      // Normal
                        phazon: 0       // Double Damage
                    },
                    beam_combos: structs::scly_structs::BeamCombos {
                        power: 1,       // Normal
                        ice: 1,         // Normal
                        wave: 1,        // Normal
                        plasma: 1,      // Normal
                        phazon: 0       // Double Damage
                    }
                },
                cmdl: resource_info!("blueShield_v1.CMDL").try_into().unwrap(),
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
                    unknown4: 1,
                    unknown5: 1.0
                },
                looping: 1,
                snow: 1,
                solid: 0,
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

        structs::SclyObject {
            instance_id: timer_doorclose_id,
            property_data: structs::Timer {
                name: b"Timer_DoorClose\0".as_cstr(),
                start_time: 0.25,
                max_random_add: 0.0,
                reset_to_zero: 1,
                start_immediately: 0,
                active: 1
            }.into(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::CLOSE,
                    target_object_id: door_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: trigger_dooropen_id,
                },
            ].into(),
        },
    ]);

    let locked_door_scan = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == scan_target_locked_door_id)
        .and_then(|obj| obj.property_data.as_point_of_interest_mut())
        .unwrap();
    locked_door_scan.active = 0;
    locked_door_scan.scan_param.scan = ResId::invalid(); // None

    let locked_door = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == door_id)
        .and_then(|obj| obj.property_data.as_door_mut())
        .unwrap();
    locked_door.ancs.file_id = resource_info!("newmetroiddoor.ANCS").try_into().unwrap();
    locked_door.ancs.default_animation = 2;
    locked_door.projectiles_collide = 0;

    let trigger_remove_scan_target_locked_door_and_etank = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == trigger_remove_scan_target_locked_door_id)
        .and_then(|obj| obj.property_data.as_trigger_mut())
        .unwrap();
    trigger_remove_scan_target_locked_door_and_etank.active = 0;

    layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == door_id)
        .unwrap()
        .connections
        .as_mut_vec()
        .extend_from_slice(
            &[
                structs::Connection {
                    state: structs::ConnectionState::OPEN,
                    message: structs::ConnectionMsg::ACTIVATE,
                    target_object_id: trigger_dooropen_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::OPEN,
                    message: structs::ConnectionMsg::START,
                    target_object_id: timer_doorclose_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::CLOSED,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: trigger_dooropen_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::OPEN,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: trigger_doorunlock_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::OPEN,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: actor_doorshield_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::CLOSED,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                    target_object_id: relay_unlock_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::MAX_REACHED,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: actor_doorshield_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::MAX_REACHED,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: trigger_doorunlock_id,
                },
            ]
        );

    Ok(())
}

fn patch_arboretum_invisible_wall(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
) -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    layer.objects.as_mut_vec().retain(|obj| obj.instance_id != 0x1302AA);

    Ok(())
}

fn patch_cutscene_force_phazon_suit<'r>
(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layers = &mut scly.layers.as_mut_vec();
    let obj = layers[1].objects.as_mut_vec().iter_mut().find(|obj| obj.instance_id & 0x00FFFFFF == 0x001A02AF);
    if obj.is_none() {
        return Ok(()); // The actor isn't there for major cutscene skips
    }
    let obj = obj.unwrap();
    let player_actor: &mut structs::PlayerActor = obj.property_data.as_player_actor_mut().unwrap();
    player_actor.player_actor_params.unknown0 = 0;

    Ok(())
}

fn patch_remove_ids<'r>
(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    remove_ids: Vec<u32>,
)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layers = &mut scly.layers.as_mut_vec();
    for layer in layers.iter_mut() {
        layer.objects.as_mut_vec().retain(|obj| !remove_ids.contains(&(obj.instance_id&0x00FFFFFF)));
    }
    Ok(())
}

fn patch_remove_doors<'r>
(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layers = &mut scly.layers.as_mut_vec();
    for layer in layers.iter_mut() {
        for obj in layer.objects.as_mut_vec() {
            if !obj.property_data.is_door() {continue;}
            let door = obj.property_data.as_door_mut().unwrap();
            door.position[2] -= 1000.0;
        }
    }
    Ok(())
}

fn patch_transform_bounding_box<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    offset: [f32;3],
    scale: [f32;3],
)
-> Result<(), String>
{
    let bb = area.mlvl_area.area_bounding_box;
    let size: [f32;3] = [
        (bb[3] - bb[0]).abs(),
        (bb[4] - bb[1]).abs(),
        (bb[5] - bb[2]).abs(),
    ];

    area.mlvl_area.area_bounding_box[0] = bb[0] + offset[0] + (size[0]*0.5 - (size[0]*0.5)*scale[0]);
    area.mlvl_area.area_bounding_box[1] = bb[1] + offset[1] + (size[1]*0.5 - (size[1]*0.5)*scale[1]);
    area.mlvl_area.area_bounding_box[2] = bb[2] + offset[2] + (size[2]*0.5 - (size[2]*0.5)*scale[2]);
    area.mlvl_area.area_bounding_box[3] = bb[3] + offset[0] - (size[0]*0.5 - (size[0]*0.5)*scale[0]);
    area.mlvl_area.area_bounding_box[4] = bb[4] + offset[1] - (size[1]*0.5 - (size[1]*0.5)*scale[1]);
    area.mlvl_area.area_bounding_box[5] = bb[5] + offset[2] - (size[2]*0.5 - (size[2]*0.5)*scale[2]);

    Ok(())
}

fn patch_spawn_point_position<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    new_position: [f32; 3],
    relative_position: bool,
    force_default: bool,
)
-> Result<(), String>
{
    let room_id = area.mlvl_area.mrea.to_u32();
    let scly = area.mrea().scly_section_mut();
    let layer_count = scly.layers.len();
    for i in 0..layer_count {
        let layer = &mut scly.layers.as_mut_vec()[i];
        for obj in layer.objects.as_mut_vec().iter_mut() {
            if !obj.property_data.is_spawn_point() {continue;}
            if obj.instance_id & 0xFF000000 == 0xDE000000 { continue; } // don't move spawn points placed by this program

            let spawn_point = obj.property_data.as_spawn_point_mut().unwrap();
            if spawn_point.default_spawn == 0 && !force_default {
                continue;
            }

            if relative_position {
                spawn_point.position[0] = spawn_point.position[0] + new_position[0];
                spawn_point.position[1] = spawn_point.position[1] + new_position[1];
                spawn_point.position[2] = spawn_point.position[2] + new_position[2];
            } else {
                spawn_point.position = new_position.into();
            }

            if force_default {
                spawn_point.default_spawn = 1;
            }

            break; // only patch one spawn point
        }
    }

    if room_id == 0xF517A1EA {
        // find/copy the spawn point //
        let spawn_point = scly.layers.as_mut_vec()[3].objects.as_mut_vec().iter_mut().find(|obj| obj.property_data.is_spawn_point()).unwrap().clone();
        // delete the original in the shitty layer //
        scly.layers.as_mut_vec()[3].objects.as_mut_vec().retain(|obj| !obj.property_data.is_spawn_point());
        // write the copied spawn point to the default layer //
        scly.layers.as_mut_vec()[0].objects.as_mut_vec().push(spawn_point);
    }

    Ok(())
}

fn patch_fix_pca_crash(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    // find the loading trigger and enable it
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec() {
        for obj in layer.objects.as_mut_vec() {
            if obj.property_data.is_trigger() {
                let trigger = obj.property_data.as_trigger_mut().unwrap();
                if trigger.name.to_str().unwrap().contains(&"eliteboss") {
                    trigger.active = 1;
                }
            }
        }
    }

    Ok(())
}

fn patch_backwards_lower_mines_pca(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    // remove from scripting layers
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec() {
        layer.objects.as_mut_vec().retain(|obj| !obj.property_data.is_platform());
    }

    // remove from level/area dependencies (this wasn't a necessary excercise, but it's nice to know how to do)
    let deps_to_remove: Vec<u32> = vec![
        0x744572a0, 0xBF19A105, 0x0D3BB9B1, // cmdl
        0x3cfa9c1c, 0x165B2898, // dcln
        0x122D9D74, 0x245EEA17, 0x71A63C95, 0x7351A073, 0x8229E1A3, 0xDD3931E2, // txtr
        0xBA2E99E8, 0xD03D1FF3, 0xE6D3D35E, 0x4185C16A, 0xEFE6629B, // txtr
    ];
    for dep_array in area.mlvl_area.dependencies.deps.as_mut_vec() {
        dep_array.as_mut_vec().retain(|dep| !deps_to_remove.contains(&dep.asset_id));
    }

    Ok(())
}

fn patch_backwards_lower_mines_eqa(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec() {
        layer.objects.as_mut_vec().retain(|obj| !obj.property_data.is_platform());
    }

    Ok(())
}

fn patch_backwards_lower_mines_mqb(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[2];
    let obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id&0x00FFFFFF == 0x001F0018)
        .unwrap();
    let actor = obj.property_data.as_actor_mut().unwrap();
    actor.actor_params.visor_params.target_passthrough = 1;
    Ok(())
}

fn patch_backwards_lower_mines_mqa(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    let obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id&0x00FFFFFF == 0x00200214) // metriod aggro trigger
        .unwrap();
    obj.connections.as_mut_vec().push(
        structs::Connection {
            state: structs::ConnectionState::ENTERED,
            message: structs::ConnectionMsg::SET_TO_ZERO,
            target_object_id: 0x00200464, // Relay One Shot In
        },
    );
    Ok(())
}

fn patch_backwards_lower_mines_elite_control(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[1];
    let obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id&0x00FFFFFF == 0x00100086)
        .unwrap();
    let actor = obj.property_data.as_actor_mut().unwrap();
    actor.actor_params.visor_params.target_passthrough = 1;
    Ok(())
}

fn patch_main_quarry_barrier(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[4];

    let forcefield_actor_obj_id = 0x100201DA;
    let turn_off_barrier_special_function_obj_id = 0x202B5;
    let turn_off_barrier_trigger_obj_id = 0x1002044D;

    layer.objects.as_mut_vec().push(
        structs::SclyObject {
            instance_id: turn_off_barrier_trigger_obj_id,
            property_data: structs::Trigger {
                name: b"Trigger - Disable Main Quarry barrier\0".as_cstr(),
                position: [82.412056, 9.354454, 2.807631].into(),
                scale: [10.0, 5.0, 7.0].into(),
                damage_info: structs::scly_structs::DamageInfo {
                    weapon_type: 0,
                    damage: 0.0,
                    radius: 0.0,
                    knockback_power: 0.0
                },
                force: [0.0, 0.0, 0.0].into(),
                flags: 1,
                active: 1,
                deactivate_on_enter: 1,
                deactivate_on_exit: 0
            }.into(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: forcefield_actor_obj_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::DECREMENT,
                    target_object_id: turn_off_barrier_special_function_obj_id,
                },
            ].into(),
        }
    );

    Ok(())
}

fn patch_main_quarry_door_lock_0_02<'r>(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    layer.objects.as_mut_vec().retain(|obj| obj.instance_id != 132563);
    Ok(())
}

fn patch_geothermal_core_door_lock_0_02<'r>(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    layer.objects.as_mut_vec().retain(|obj| obj.instance_id != 1311646);
    Ok(())
}

fn patch_hive_totem_boss_trigger_0_02(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[1];
    let trigger_obj_id = 0x4240140;

    let trigger_obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == trigger_obj_id)
        .and_then(|obj| obj.property_data.as_trigger_mut())
        .unwrap();
    trigger_obj.position = [94.571053, 301.616028, 0.344905].into();
    trigger_obj.scale = [6.052994, 24.659973, 7.878154].into();

    Ok(())
}

fn patch_ruined_courtyard_thermal_conduits(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    version: Version,
) -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    let thermal_conduit_damageable_trigger_obj_id = 0xF01C8;
    let thermal_conduit_actor_obj_id = 0xF01C7;
    let debris_generator_obj_id = 0xF01DD;
    let thermal_conduit_cover_actor_obj_id = 0xF01D9;

    layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == thermal_conduit_damageable_trigger_obj_id)
        .and_then(|obj| obj.property_data.as_damageable_trigger_mut())
        .unwrap()
        .active = 1;

    if version == Version::NtscU0_02 {
        layer.objects.as_mut_vec().iter_mut()
            .find(|obj| obj.instance_id == thermal_conduit_actor_obj_id)
            .and_then(|obj| obj.property_data.as_actor_mut())
            .unwrap()
            .active = 1;
    } else if version == Version::NtscJ || version == Version::Pal || version == Version::NtscUTrilogy || version == Version::NtscJTrilogy || version == Version::PalTrilogy {
        layer.objects.as_mut_vec().iter_mut()
            .find(|obj| obj.instance_id == debris_generator_obj_id)
            .unwrap()
            .connections
            .as_mut_vec()
            .push(
                structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: thermal_conduit_cover_actor_obj_id,
                }
            );

        let flags = &mut area.layer_flags.flags;
        *flags |= 1 << 6; // Turn on "Thermal Target"
    }

    Ok(())
}

fn patch_geothermal_core_destructible_rock_pal(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];

    let platform_obj_id = 0x1403AE;
    let scan_target_platform_obj_id = 0x1403B4;
    let actor_blocker_collision_id = 0x1403B5;

    let platform_obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == platform_obj_id)
        .and_then(|obj| obj.property_data.as_platform_mut())
        .unwrap();
    platform_obj.active = 0;

    let scan_target_platform_obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == scan_target_platform_obj_id)
        .and_then(|obj| obj.property_data.as_point_of_interest_mut())
        .unwrap();
    scan_target_platform_obj.active = 0;

    let actor_blocker_collision_obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == actor_blocker_collision_id)
        .and_then(|obj| obj.property_data.as_actor_mut())
        .unwrap();
    actor_blocker_collision_obj.active = 0;

    Ok(())
}

fn patch_ore_processing_door_lock_0_02(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];

    let actor_door_lock_obj_id = 0x6036A;
    let pb_inv_check_timer_obj_id = 0x6036C;
    let pb_inv_check_spec_func_obj_id = 0x60368;

    layer.objects.as_mut_vec().retain(|obj| obj.instance_id != actor_door_lock_obj_id &&
                                            obj.instance_id != pb_inv_check_timer_obj_id &&
                                            obj.instance_id != pb_inv_check_spec_func_obj_id);

    Ok(())
}

fn patch_ore_processing_destructible_rock_pal(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];

    let platform_obj_id = 0x60372;
    let scan_target_platform_obj_id = 0x60378;
    let actor_blocker_collision_id = 0x60379;

    let platform_obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == platform_obj_id)
        .and_then(|obj| obj.property_data.as_platform_mut())
        .unwrap();
    platform_obj.active = 0;

    let scan_target_platform_obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == scan_target_platform_obj_id)
        .and_then(|obj| obj.property_data.as_point_of_interest_mut())
        .unwrap();
    scan_target_platform_obj.active = 0;

    let actor_blocker_collision_obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == actor_blocker_collision_id)
        .and_then(|obj| obj.property_data.as_actor_mut())
        .unwrap();
    actor_blocker_collision_obj.active = 0;

    Ok(())
}

// Removes all cameras and spawn point repositions in the area
// igoring any provided exlcuded script objects.
// Additionally, shortens any specified timers to 0-ish seconds
// When deciding which objects to patch, the most significant
// byte is ignored
fn patch_remove_cutscenes(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    timers_to_zero: Vec<u32>,
    mut skip_ids: Vec<u32>,
    use_timers_instead_of_relay: bool,
)
    -> Result<(), String>
{
    let room_id = area.mlvl_area.mrea;
    let layer_count = area.layer_flags.layer_count as usize;
    let scly = area.mrea().scly_section_mut();

    let mut camera_ids = Vec::<u32>::new();
    let mut spawn_point_ids = Vec::<u32>::new();

    let mut elevator_orientation = [0.0, 0.0, 0.0];

    for i in 0..layer_count {
        let layer = &mut scly.layers.as_mut_vec()[i];

        for obj in layer.objects.iter() {
            // If this is an elevator cutscene taking the player up, don't skip it //
            // (skipping it can cause sounds to persist in an annoying fashion)    //
            if is_elevator(room_id.to_u32())
            {
                if obj.property_data.is_camera() {
                    let camera = obj.property_data.as_camera().unwrap();
                    let name = camera.name.clone().into_owned().to_owned().to_str().unwrap().to_string().to_lowercase();
                    if name.contains(&"leaving") {
                        skip_ids.push(obj.instance_id & 0x00FFFFFF);
                    }
                }
                if obj.property_data.is_player_actor() {
                    let player_actor = obj.property_data.as_player_actor().unwrap();
                    let name = player_actor.name.clone().into_owned().to_owned().to_str().unwrap().to_string().to_lowercase();
                    if name.contains(&"leaving") {
                        skip_ids.push(obj.instance_id & 0x00FFFFFF);
                    }
                }
            }

            // Get a list of all camera instance ids
            if !skip_ids.contains(&(obj.instance_id & 0x00FFFFFF))
            && obj.property_data.is_camera() {
                camera_ids.push(obj.instance_id & 0x00FFFFFF);
            }

            // Get a list of all spawn point ids
            if !skip_ids.contains(&(obj.instance_id & 0x00FFFFFF)) && obj.property_data.is_spawn_point()
            && (room_id != 0xf7285979 || i == 4) { // don't patch spawn points in shorelines except for ridley
                spawn_point_ids.push(obj.instance_id & 0x00FFFFFF);
            }

            if obj.property_data.is_player_actor() {
                let rotation = obj.property_data.as_player_actor().unwrap().rotation;
                elevator_orientation[0] = rotation[0];
                elevator_orientation[1] = rotation[1];
                elevator_orientation[2] = rotation[2];
            }
        }
    }

    let mut id0 = 0xFFFFFFFF;
    if room_id == 0x0749DF46 || room_id == 0x7A3AD91E {
        id0 = ps.fresh_instance_id_range.next().unwrap();

        let target_object_id = {
            if room_id == 0x0749DF46 { // subchamber 2
                0x0007000B
            } else { // subchamber 3
                0x00080016
            }
        };

        // add a timer to turn activate prime
        scly.layers.as_mut_vec()[0].objects.as_mut_vec().push(structs::SclyObject {
            instance_id: id0,
            property_data: structs::Timer {
                name: b"activate-prime\0".as_cstr(),
                start_time: 1.0,
                max_random_add: 0.0,
                reset_to_zero: 0,
                start_immediately: 0,
                active: 1,
            }.into(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::START,
                    target_object_id,
                },
            ].into(),
        });
    }

    // for each layer
    for i in 0..layer_count {
        let layer = &mut scly.layers.as_mut_vec()[i];
        let mut objs_to_add = Vec::<structs::SclyObject>::new();

        // for each object in the layer
        for obj in layer.objects.as_mut_vec() {
            let obj_id = obj.instance_id & 0x00FFFFFF; // remove uper encoding byte

            // If this is an elevator cutscene skip, orient the player towards the door
            if is_elevator(room_id.to_u32()) && obj.property_data.is_spawn_point() {
                obj.property_data.as_spawn_point_mut().unwrap().rotation = elevator_orientation.into();
            }

            // If it's a cutscene-related timer, make it nearly instantaneous
            if obj.property_data.is_timer() {
                let timer = obj.property_data.as_timer_mut().unwrap();

                if timers_to_zero.contains(&obj_id) {
                    if obj_id == 0x0008024E {
                        timer.start_time = 3.0; // chozo ice temple hands
                    } else {
                        timer.start_time = 0.1;
                    }
                }
            }

            // for each connection in that object
            for connection in obj.connections.as_mut_vec().iter_mut() {
                // if this object sends messages to a camera, change the message to be
                // appropriate for a relay
                if camera_ids.contains(&(connection.target_object_id & 0x00FFFFFF)) {
                    if connection.message == structs::ConnectionMsg::ACTIVATE {
                        connection.message = structs::ConnectionMsg::SET_TO_ZERO;
                    }
                }
            }

            // remove every connection to a spawn point, effectively removing all repositions
            obj.connections.as_mut_vec().retain(|conn|
                !spawn_point_ids.contains(&(conn.target_object_id & 0x00FFFFFF)) ||
                conn.target_object_id&0xFF000000 == 0xDE000000 // keep objects that were added via this program
            );

            // if the object is a camera, create a relay with the same id
            if camera_ids.contains(&obj_id) {
                let mut relay = {
                    structs::SclyObject {
                        instance_id: obj.instance_id,
                        connections: obj.connections.clone(),
                        property_data: structs::SclyProperty::Relay(Box::new(
                            structs::Relay {
                                name: b"camera-relay\0".as_cstr(),
                                active: 1,
                            }
                        ))
                    }
                };

                let shot_duration = {
                    if timers_to_zero.contains(&obj_id) {
                        0.1
                    } else {
                        let camera = obj.property_data.as_camera_mut();
                        if camera.is_some() {
                            camera.unwrap().shot_duration
                        } else {
                            // this is when shit gets double patched
                            // println!("object 0x{:X} in room 0x{:X} isn't actually a camera", room_id.to_u32(), obj_id);
                            0.1
                        }
                    }
                };

                let timer_id = ps.fresh_instance_id_range.next().unwrap();
                let mut timer = structs::SclyObject {
                    instance_id: timer_id,
                    property_data: structs::Timer {
                        name: b"cutscene-replacement\0".as_cstr(),
                        start_time: shot_duration,
                        max_random_add: 0.0,
                        reset_to_zero: 0,
                        start_immediately: 0,
                        active: 1,
                    }.into(),
                    connections: vec![].into(),
                };

                let relay_connections = relay.connections.as_mut_vec();
                for connection in relay_connections.iter_mut() {
                    if connection.state == structs::ConnectionState::INACTIVE && use_timers_instead_of_relay {
                        timer.connections.as_mut_vec().push(structs::Connection {
                            state: structs::ConnectionState::ZERO,
                            message: connection.message,
                            target_object_id: connection.target_object_id,
                        });
                    } else if connection.state == structs::ConnectionState::ACTIVE || (connection.state == structs::ConnectionState::INACTIVE && !use_timers_instead_of_relay) {
                        connection.state = structs::ConnectionState::ZERO;
                    }
                }

                if use_timers_instead_of_relay {
                    relay_connections.push(structs::Connection {
                        state: structs::ConnectionState::ZERO,
                        message:  structs::ConnectionMsg::RESET_AND_START,
                        target_object_id: timer_id,
                    });

                    relay_connections.retain(|conn| conn.state != structs::ConnectionState::INACTIVE);
                    objs_to_add.push(timer);
                }

                objs_to_add.push(relay);
            }

            if obj_id == 0x000B00ED { // first essence camera
                let camera = obj.property_data.as_camera_mut().unwrap();
                camera.shot_duration = 1.5;
            }

            if skip_ids.contains(&obj_id) {
                continue;
            }

            // Special handling for specific rooms //
            if obj_id == 0x00250123 { // flaahgra death cutscene (first camera)
                // teleport the player at end of shot (4.0s), this is long enough for
                // the water to change from acid to water, thus granting pre-floaty
                obj.connections.as_mut_vec().push(structs::Connection {
                    state: structs::ConnectionState::INACTIVE,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                    target_object_id: 0x04252FC0, // spawn point by item
                });
            } else if obj_id == 0x001E027E { // observatory scan
                // just cut out all the confusion by having the scan always active
                obj.property_data.as_point_of_interest_mut().unwrap().active = 1;
            } else if obj_id == 0x00170153 && !skip_ids.contains(&obj_id) { // magmoor workstation cutscene (power activated)
                // play this cutscene, but only for a second
                // this is to allow players to get floaty jump without having red mist
                obj.property_data.as_camera_mut().unwrap().shot_duration = 3.3;
            } else if obj_id == 0x00070062 { // subchamber 2 trigger
                // When the player enters the room (properly), start the fight
                obj.connections.as_mut_vec().push(structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::RESET_AND_START,
                    target_object_id: id0, // timer
                });
                let trigger = obj.property_data.as_trigger_mut().unwrap();
                trigger.scale[2] = 8.0;
                trigger.position[2] = trigger.position[2] - 11.7;
                trigger.deactivate_on_enter = 1;
            } else if obj_id == 0x00080058 { // subchamber 3 trigger
                // When the player enters the room (properly), start the fight
                obj.connections.as_mut_vec().push(structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::RESET_AND_START,
                    target_object_id: id0, // timer
                });
                let trigger = obj.property_data.as_trigger_mut().unwrap();
                trigger.scale[2] = 8.0;
                trigger.position[2] = trigger.position[2] - 11.7;
                trigger.deactivate_on_enter = 1;
            } else if obj_id == 0x0009005A { // subchamber 4 trigger
                // When the player enters the room (properly), start the fight
                obj.connections.as_mut_vec().push(structs::Connection {
                    state: structs::ConnectionState::INSIDE, // inside, because it's possible to beat exo to this trigger
                    message: structs::ConnectionMsg::START,
                    target_object_id: 0x00090013, // metroid prime
                });
                if obj.property_data.is_trigger() {
                    let trigger = obj.property_data.as_trigger_mut().unwrap();
                    trigger.scale[2] = 5.0;
                    trigger.position[2] = trigger.position[2] - 11.7;
                }
            } else if obj_id == 0x001201AB { // ventillation shaft end timer
                // Disable gas at end of cutscene, not beggining
                obj.connections.as_mut_vec().push(structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: 0x001200C2, // gas damage trigger
                });
            } else if vec![0x001200B8, 0x001200B7, 0x001200B6, 0x001200B5, 0x001200B4, 0x001200B2].contains(&obj_id) { // vent shaft puffer
                // increment the dead puffer counter if killed by anything
                obj.connections.as_mut_vec().push(structs::Connection {
                    state: structs::ConnectionState::DEAD,
                    message: structs::ConnectionMsg::INCREMENT,
                    target_object_id: 0x00120094, // dead puffer counter
                });
            } else if obj_id == 0x00120060 { // kill puffer trigger
                // the puffers will increment the counter instead of me, the kill trigger
                obj.connections.as_mut_vec().retain(|_conn| false);
            } else if obj_id == 0x001B065F { // central dynamo collision blocker
                // the power bomb rock collision should not extend beyond the door
                let actor = obj.property_data.as_actor_mut().unwrap();
                actor.hitbox[1] = 0.4;
                actor.position[1] = actor.position[1] - 0.8;
            } else if obj_id == 0x0002023E { // main plaza turn crane left relay
                // snap the crane immediately so fast players don't fall through the intangible animation
                obj.connections.as_mut_vec().push(structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::ACTIVATE,
                    target_object_id: 0x0002001F, // platform
                });

                // set to left relay
                obj.connections.as_mut_vec().push(structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                    target_object_id: 0x0002025B,
                });
            } else if obj_id == 0x00130141 { // arboretum disable gate timer
                // Disable glowey gate marks with gate
                for target_object_id in vec![0x00130119, 0x00130118, 0x0013011F, 0x0013011E] { // glowy symbols
                    obj.connections.as_mut_vec().push(structs::Connection {
                        state: structs::ConnectionState::ZERO,
                        message: structs::ConnectionMsg::DEACTIVATE,
                        target_object_id,
                    });
                }
            } else if obj_id == 0x001A04B8 || obj_id == 0x001A04C5 { // Elite Quarters Pickup(s)
                let pickup = obj.property_data.as_pickup_mut().unwrap();
                pickup.position[2] = pickup.position[2] + 2.0; // Move up so it's more obvious

                // The pickup should display hudmemo instead of OP
                obj.connections.as_mut_vec().push(structs::Connection {
                    state: structs::ConnectionState::ARRIVED,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                    target_object_id: 0x001A0348,
                });
                // The pickup should unlock lift instead of OP
                obj.connections.as_mut_vec().push(structs::Connection {
                    state: structs::ConnectionState::ARRIVED,
                    message: structs::ConnectionMsg::DECREMENT,
                    target_object_id: 0x001A03D9,
                });
                // The pickup should unlock doors instead of OP
                obj.connections.as_mut_vec().push(structs::Connection {
                    state: structs::ConnectionState::ARRIVED,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                    target_object_id: 0x001A0328,
                });

            } else if obj_id == 0x001A0126 { // Omega Pirate
                obj.connections.as_mut_vec().retain(|conn| !vec![
                    0x001A03D9, // elevator shield
                    0x001A0328, // door unlock relay
                    ].contains(&(conn.target_object_id & 0x00FFFFFF))
                );
            }
            // unlock the artifact temple forcefield when memory relay is flipped, not when ridley dies
            else if obj_id == 0x00100101 { // ridley
                obj.connections.as_mut_vec().retain(|conn| !vec![
                    0x00100112, // forcefield
                    ].contains(&(conn.target_object_id & 0x00FFFFFF))
                );
            }
            else if obj_id == 0x0010028F { // end of ridley death cine relay
                obj.connections.as_mut_vec().push(structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::DECREMENT,
                    target_object_id: 0x00100112, // forcefield gate
                });
            }

            // ball triggers can be mean sometimes when not in the saftey of a cutscene, tone it down from 40 to 10
            if obj.property_data.is_ball_trigger() && room_id != 0xEF069019 {
                let ball_trigger = obj.property_data.as_ball_trigger_mut().unwrap();
                ball_trigger.force = 10.0;
            }
        }

        // add all relays
        for obj in objs_to_add.iter() {
            layer.objects.as_mut_vec().push(obj.clone());
        }

        // remove all cutscene related objects from layer
        if room_id == 0xf7285979 && i != 4 // the ridley cutscene is okay
        {
            // special shorelines handling
            let shorelines_triggers = vec![
                0x00020155, // intro cutscene
                0x000201F4, // shorelines tower cutscene
            ];

            layer.objects.as_mut_vec().retain(|obj|
                skip_ids.contains(&(&obj.instance_id & 0x00FFFFFF)) || // except for exluded objects
                !(shorelines_triggers.contains(&(&obj.instance_id & 0x00FFFFFF)))
            );
        }
        else if room_id == 0xb4b41c48 { // keep the cinematic stuff in end cinema
            layer.objects.as_mut_vec().retain(|obj| !obj.property_data.is_camera());
        }
        else
        {
            layer.objects.as_mut_vec().retain(|obj|
                skip_ids.contains(&(&obj.instance_id & 0x00FFFFFF)) || // except for exluded objects
                !(
                    obj.property_data.is_camera() ||
                    obj.property_data.is_camera_filter_keyframe() ||
                    obj.property_data.is_camera_blur_keyframe() ||
                    obj.property_data.is_player_actor() ||
                    vec![0x0018028E, 0x001802A1, 0x0018025C, 0x001800CC, 0x00180212, 0x00020473, 0x00070521, 0x001A034A, 0x001A04C2, 0x001A034B].contains(&(obj.instance_id&0x00FFFFFF)) || // thardus death sounds + thardus corpse + main quarry, security station playerhint, post OP death timer for hudmemo, Elite Quarters Control Disablers
                    (obj.property_data.is_special_function() && obj.property_data.as_special_function().unwrap().type_ == 0x18) // "show billboard"
                )
            );
        }
    }

    Ok(())
}

/**
 * Patch is actually for QAA
 *
 */
fn patch_fix_central_dynamo_crash(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec() {
        layer.objects.as_mut_vec().retain(|obj| !vec![0x45].contains(&obj.property_data.object_type()));
    }

    Ok(())
}

fn patch_main_quarry_door_lock_pal(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[7];

    let locked_door_actor_obj_id = 0x1c0205db;

    let locked_door_actor_obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == locked_door_actor_obj_id)
        .and_then(|obj| obj.property_data.as_actor_mut())
        .unwrap();
    locked_door_actor_obj.active = 0;

    Ok(())
}

fn patch_mines_security_station_soft_lock<'r>(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec().iter_mut() {
        for obj in layer.objects.as_mut_vec().iter_mut() {
            if (obj.instance_id & 0x00FFFFFF) != 0x0007033F {
                continue;
            }
            let trigger = obj.property_data.as_trigger_mut().unwrap();
            trigger.scale[0] = 50.0;
            trigger.scale[1] = 100.0;
            trigger.scale[2] = 40.0;
        }
    }

    Ok(())
}

fn patch_research_core_access_soft_lock(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();

    const DRONE_IDS: &[u32] = &[
                        0x082C006C,
                        0x082C0124,
                    ];
    const RELAY_ENABLE_LOCK_IDS: &[u32] = &[
                        0x082C00CF,
                        0x082C010E,
                    ];
    let trigger_alert_drones_id = 0x082C00CD;

    let trigger_alert_drones_obj = scly.layers.as_mut_vec()[2].objects.iter_mut()
        .find(|i| i.instance_id == trigger_alert_drones_id).unwrap();
    trigger_alert_drones_obj.connections.as_mut_vec().retain(|i| i.target_object_id != RELAY_ENABLE_LOCK_IDS[0] && i.target_object_id != RELAY_ENABLE_LOCK_IDS[1]);

    for drone_id in DRONE_IDS {
        scly.layers.as_mut_vec()[2].objects.iter_mut()
            .find(|i| i.instance_id == *drone_id).unwrap()
            .connections.as_mut_vec().extend_from_slice(
                &[
                    structs::Connection {
                        state: structs::ConnectionState::ZERO,
                        message: structs::ConnectionMsg::SET_TO_ZERO,
                        target_object_id: RELAY_ENABLE_LOCK_IDS[0],
                    },
                    structs::Connection {
                        state: structs::ConnectionState::ZERO,
                        message: structs::ConnectionMsg::SET_TO_ZERO,
                        target_object_id: RELAY_ENABLE_LOCK_IDS[1],
                    },
                ]
            );
    }

    Ok(())
}

fn patch_hive_totem_softlock<'r>(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    let trigger = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id & 0x00FFFFFF == 0x002400CA)
        .unwrap();
    trigger.property_data.as_trigger_mut().unwrap().scale[1] = 60.0;

    Ok(())
}

fn patch_gravity_chamber_stalactite_grapple_point<'r>(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];

    // Remove the object that turns off the stalactites layer
    layer.objects.as_mut_vec().retain(|obj| obj.instance_id != 3473722);

    Ok(())
}

fn patch_heat_damage_per_sec<'a>(patcher: &mut PrimePatcher<'_, 'a>, heat_damage_per_sec: f32)
{
    const HEATED_ROOMS: &[ResourceInfo] = &[
        resource_info!("06_grapplegallery.MREA"),
        resource_info!("00a_lava_connect.MREA"),
        resource_info!("11_over_muddywaters_b.MREA"),
        resource_info!("00b_lava_connect.MREA"),
        resource_info!("14_over_magdolitepits.MREA"),
        resource_info!("00c_lava_connect.MREA"),
        resource_info!("09_over_monitortower.MREA"),
        resource_info!("00d_lava_connect.MREA"),
        resource_info!("09_lava_pickup.MREA"),
        resource_info!("00e_lava_connect.MREA"),
        resource_info!("12_over_fieryshores.MREA"),
        resource_info!("00f_lava_connect.MREA"),
        resource_info!("00g_lava_connect.MREA"),
    ];

    for heated_room in HEATED_ROOMS.iter() {
        patcher.add_scly_patch((*heated_room).into(), move |_ps, area| {
            let scly = area.mrea().scly_section_mut();
            let layer = &mut scly.layers.as_mut_vec()[0];
            layer.objects.iter_mut()
                .filter_map(|obj| obj.property_data.as_special_function_mut())
                .filter(|sf| sf.type_ == 18) // Is Area Damage function
                .for_each(|sf| sf.unknown1 = heat_damage_per_sec);
            Ok(())
        });
    }
}

fn patch_save_station_for_warp_to_start<'r>(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    spawn_room: SpawnRoomData,
    version: Version,
    warp_to_start_delay_s: f32,
) -> Result<(), String>
{
    let mut warp_to_start_delay_s = warp_to_start_delay_s;
    if warp_to_start_delay_s < 3.0 {
        warp_to_start_delay_s = 3.0
    }

    area.add_dependencies(
        &game_resources,
        0,
        iter::once(custom_asset_ids::WARPING_TO_START_STRG.into())
    );
    area.add_dependencies(
        &game_resources,
        0,
        iter::once(custom_asset_ids::WARPING_TO_START_DELAY_STRG.into())
    );

    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    let world_transporter_id = ps.fresh_instance_id_range.next().unwrap();
    let timer_id = ps.fresh_instance_id_range.next().unwrap();
    let hudmemo_id = ps.fresh_instance_id_range.next().unwrap();
    let player_hint_id = ps.fresh_instance_id_range.next().unwrap();

    // Add world transporter leading to starting room
    layer.objects
         .as_mut_vec()
         .push(structs::SclyObject {
            instance_id: world_transporter_id,
            property_data: structs::WorldTransporter::warp(
                spawn_room.mlvl,
                spawn_room.mrea,
                "Warp to Start",
                resource_info!("Deface14B_O.FONT").try_into().unwrap(),
                ResId::new(custom_asset_ids::WARPING_TO_START_STRG.to_u32()),
                version == Version::Pal
            ).into(),
            connections: vec![].into(),
        });

    // Add timer to delay warp (can crash if player warps too quickly)
    layer.objects
         .as_mut_vec()
         .push(structs::SclyObject {
            instance_id: timer_id,
            property_data: structs::Timer {
                name: b"Warp to start delay\0".as_cstr(),

                start_time: warp_to_start_delay_s,
                max_random_add: 0.0,
                reset_to_zero: 0,
                start_immediately: 0,
                active: 1,
            }.into(),
            connections: vec![
                structs::Connection {
                    target_object_id: world_transporter_id,
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                },
            ].into(),
        });

    // Inform the player that they are about to be warped
    layer.objects
        .as_mut_vec()
        .push(structs::SclyObject {
           instance_id: hudmemo_id,
           property_data: structs::HudMemo {
                name: b"Warping hudmemo\0".as_cstr(),

                first_message_timer: warp_to_start_delay_s,
                unknown: 1,
                memo_type: 0,
                strg: custom_asset_ids::WARPING_TO_START_DELAY_STRG,
                active: 1,
            }.into(),
           connections: vec![].into(),
       });

    // Stop the player from moving
    layer.objects
        .as_mut_vec()
        .push(structs::SclyObject {
           instance_id: player_hint_id,
           property_data: structs::PlayerHint {

            name: b"Warping playerhint\0".as_cstr(),

            position: [0.0, 0.0, 0.0].into(),
            rotation: [0.0, 0.0, 0.0].into(),

            unknown0: 1, // active

            inner_struct: structs::PlayerHintStruct {
                unknowns: [
                    0,
                    0,
                    0,
                    0,
                    0,
                    1, // disable
                    1, // disable
                    1, // disable
                    1, // disable
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                ].into(),
            }.into(),

            unknown1: 10, // priority
           }.into(),
           connections: vec![].into(),
    });

    for obj in layer.objects.iter_mut() {
        if let Some(sp_function) = obj.property_data.as_special_function_mut() {
            if sp_function.type_ == 7 { // Is Save Station function
                obj.connections
                   .as_mut_vec()
                   .push(structs::Connection {
                        target_object_id: timer_id,
                        state: structs::ConnectionState::RETREAT,
                        message: structs::ConnectionMsg::RESET_AND_START,
                    });
                obj.connections
                    .as_mut_vec()
                    .push(structs::Connection {
                         target_object_id: hudmemo_id,
                         state: structs::ConnectionState::RETREAT,
                         message: structs::ConnectionMsg::SET_TO_ZERO,
                     });
                obj.connections
                    .as_mut_vec()
                    .push(structs::Connection {
                        target_object_id: player_hint_id,
                        state: structs::ConnectionState::RETREAT,
                        message: structs::ConnectionMsg::INCREMENT,
                    });
            }
        }
    }

    Ok(())
}

fn patch_memorycard_strg(res: &mut structs::Resource, version: Version) -> Result<(), String>
{
    if version == Version::NtscJ {
        let strings = res.kind.as_strg_mut().unwrap()
            .string_tables
            .as_mut_vec()
            .iter_mut()
            .find(|table| table.lang == b"JAPN".into())
            .unwrap()
            .strings
            .as_mut_vec();

        let s = strings.iter_mut()
            .nth(8)
            .unwrap();
        *s = "スロットAのメモリーカードに\nデータをセーブしますか？\n&image=SI,0.70,0.68,46434ED3; + &image=SI,0.70,0.68,08A2E4B9; キーを押したまま、「いいえ」を選択して開始ルームにワープします。\u{0}".to_string().into();
    } else {
        let strings = res.kind.as_strg_mut().unwrap()
            .string_tables
            .as_mut_vec()
            .iter_mut()
            .find(|table| table.lang == b"ENGL".into())
            .unwrap()
            .strings
            .as_mut_vec();

        let s = strings.iter_mut()
            .find(|s| *s == "Save progress to Memory Card in Slot A?\u{0}")
            .unwrap();
        *s = "Save progress to Memory Card in Slot A?\nHold &image=SI,0.70,0.68,46434ED3; + &image=SI,0.70,0.68,08A2E4B9; while choosing No to warp to starting room.\u{0}".to_string().into();
    }

    Ok(())
}

fn patch_main_strg(res: &mut structs::Resource, version: Version, msg: &str) -> Result<(), String>
{
    if version == Version::NtscJ {
        let strings_jpn = res.kind.as_strg_mut().unwrap()
            .string_tables
            .as_mut_vec()
            .iter_mut()
            .find(|table| table.lang == b"JAPN".into())
            .unwrap()
            .strings
            .as_mut_vec();

        let s = strings_jpn.iter_mut()
            .nth(37)
            .unwrap();
        *s = "&main-color=#FFFFFF;エクストラ\u{0}".to_string().into();
        strings_jpn.push(format!("{}\0", msg).into());
    }

    let strings = res.kind.as_strg_mut().unwrap()
        .string_tables
        .as_mut_vec()
        .iter_mut()
        .find(|table| table.lang == b"ENGL".into())
        .unwrap()
        .strings
        .as_mut_vec();

    let s = strings.iter_mut()
        .find(|s| *s == "Metroid Fusion Connection Bonuses\u{0}")
        .unwrap();
    *s = "Extras\u{0}".to_string().into();
    strings.push(format!("{}\0", msg).into());

    Ok(())
}

fn patch_main_menu(res: &mut structs::Resource) -> Result<(), String>
{
    let frme = res.kind.as_frme_mut().unwrap();

    let (jpn_font, jpn_point_scale) = if frme.version == 0 {
        (None, None)
    } else {
        (Some(ResId::new(0xC29C51F1)), Some([237, 35].into()))
    };

    frme.widgets.as_mut_vec().push(structs::FrmeWidget {
        name: b"textpane_identifier\0".as_cstr(),
        parent: b"kGSYS_HeadWidgetID\0".as_cstr(),
        use_anim_controller: 0,
        default_visible: 1,
        default_active: 1,
        cull_faces: 0,
        color: [1.0, 1.0, 1.0, 1.0].into(),
        model_draw_flags: 2,
        kind: structs::FrmeWidgetKind::TextPane(
            structs::TextPaneWidget {
                x_dim: 10.455326,
                z_dim: 1.813613,
                scale_center: [
                    -5.227663,
                    0.0,
                    -0.51,
                ].into(),
                font: resource_info!("Deface14B_O.FONT").try_into().unwrap(),
                word_wrap: 0,
                horizontal: 1,
                justification: 0,
                vertical_justification: 0,
                fill_color: [1.0, 1.0, 1.0, 1.0].into(),
                outline_color: [0.0, 0.0, 0.0, 1.0].into(),
                block_extent: [213.0, 38.0].into(),
                jpn_font,
                jpn_point_scale,
            },
        ),
        worker_id: None,
        origin: [9.25, 1.500001, 0.0].into(),
        basis: [
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0,
        ].into(),
        rotation_center: [0.0, 0.0, 0.0].into(),
        unknown0: 0,
        unknown1: 0,
    });

    let mut shadow_widget = frme.widgets.as_mut_vec().last().unwrap().clone();
    shadow_widget.name = b"textpane_identifierb\0".as_cstr();
    let tp = match &mut shadow_widget.kind {
        structs::FrmeWidgetKind::TextPane(tp) => tp,
        _ => unreachable!(),
    };
    tp.fill_color = [0.0, 0.0, 0.0, 0.4].into();
    tp.outline_color = [0.0, 0.0, 0.0, 0.2].into();
    shadow_widget.origin[0] -= -0.235091;
    shadow_widget.origin[1] -= -0.104353;
    shadow_widget.origin[2] -= 0.176318;

    frme.widgets.as_mut_vec().push(shadow_widget);

    Ok(())
}

fn patch_credits(
    res: &mut structs::Resource,
    version: Version,
    config: &PatchConfig,
    level_data: &HashMap<String, LevelConfig>,
)
    -> Result<(), String>
{
    let mut output = "\n\n\n\n\n\n\n".to_string();

    if version == Version::NtscJ {
        output = format!("&line-extra-space=16;&font=5D696116;{}", output);
    }

    if config.credits_string.is_some() {
        output = format!("{}{}", output, config.credits_string.as_ref().unwrap());
    } else {
        output = format!(
            "{}{}",
            output,
            concat!(
                "&push;&font=C29C51F1;&main-color=#89D6FF;",
                "Major Item Locations",
                "&pop;",
            ).to_owned()
        );

        use std::fmt::Write;
        const PICKUPS_TO_PRINT: &[PickupType] = &[
            PickupType::ScanVisor,
            PickupType::ThermalVisor,
            PickupType::XRayVisor,
            PickupType::VariaSuit,
            PickupType::GravitySuit,
            PickupType::PhazonSuit,
            PickupType::MorphBall,
            PickupType::BoostBall,
            PickupType::SpiderBall,
            PickupType::MorphBallBomb,
            PickupType::PowerBomb,
            PickupType::ChargeBeam,
            PickupType::SpaceJumpBoots,
            PickupType::GrappleBeam,
            PickupType::SuperMissile,
            PickupType::Wavebuster,
            PickupType::IceSpreader,
            PickupType::Flamethrower,
            PickupType::WaveBeam,
            PickupType::IceBeam,
            PickupType::PlasmaBeam
        ];

        for pickup_type in PICKUPS_TO_PRINT {
            let room_name = {
                let mut _room_name = String::new();
                for (_, level) in level_data.iter() {
                    for (room_name, room) in level.rooms.iter() {
                        if room.pickups.is_none() { continue };
                        for pickup_info in room.pickups.as_ref().unwrap().iter() {
                            if PickupType::from_str(pickup_type.name()) == PickupType::from_str(&pickup_info.pickup_type) {
                                _room_name = room_name.to_string();
                                break;
                            }
                        }
                    }
                }

                if _room_name.len() == 0 {
                    _room_name = "<Not Present>".to_string();
                }

                _room_name
            };
            let pickup_name = pickup_type.name();
            write!(output, "\n\n{}: {}", pickup_name, room_name).unwrap();
        }
    }
    output = format!("{}{}", output, "\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\0");
    if version == Version::NtscJ {
        let output_jpn = format!("{}", output);
        res.kind.as_strg_mut().unwrap().string_tables
            .as_mut_vec()
            .iter_mut()
            .find(|table| table.lang == b"JAPN".into())
            .unwrap()
            .strings
            .as_mut_vec()
            .push(output_jpn.into());
    }
    res.kind.as_strg_mut().unwrap().string_tables
        .as_mut_vec()
        .iter_mut()
        .find(|table| table.lang == b"ENGL".into())
        .unwrap()
        .strings
        .as_mut_vec()
        .push(output.into());
    Ok(())
}

fn patch_starting_pickups<'r>(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    starting_items: &StartingItems,
    show_starting_memo: bool,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
) -> Result<(), String>
{
    let room_id = area.mlvl_area.internal_id;
    let layer_count = area.mrea().scly_section_mut().layers.as_mut_vec().len() as u32;

    if show_starting_memo {
        // Turn on "Randomizer - Starting Items popup Layer"
        area.layer_flags.flags |= 1 << layer_count;
        area.add_layer(b"Randomizer - Starting Items popup Layer\0".as_cstr());
    }

    let scly = area.mrea().scly_section_mut();

    let timer_starting_items_popup_id = ps.fresh_instance_id_range.next().unwrap();
    let hud_memo_starting_items_popup_id = ps.fresh_instance_id_range.next().unwrap();
    let special_function_starting_items_popup_id = ps.fresh_instance_id_range.next().unwrap();

    for layer in scly.layers.iter_mut() {
        for obj in layer.objects.iter_mut() {
            if let Some(spawn_point) = obj.property_data.as_spawn_point_mut() {
                starting_items.update_spawn_point(spawn_point);
            }
        }
    }

    if show_starting_memo {
        scly.layers.as_mut_vec()[layer_count as usize].objects.as_mut_vec().extend_from_slice(
            &[
                structs::SclyObject {
                    instance_id: timer_starting_items_popup_id,
                    property_data: structs::Timer {
                        name: b"Starting Items popup timer\0".as_cstr(),

                        start_time: 0.025,
                        max_random_add: 0f32,
                        reset_to_zero: 0,
                        start_immediately: 1,
                        active: 1,
                    }.into(),
                    connections: vec![
                        structs::Connection {
                            state: structs::ConnectionState::ZERO,
                            message: structs::ConnectionMsg::SET_TO_ZERO,
                            target_object_id: hud_memo_starting_items_popup_id,
                        },
                        structs::Connection {
                            state: structs::ConnectionState::ZERO,
                            message: structs::ConnectionMsg::DECREMENT,
                            target_object_id: special_function_starting_items_popup_id,
                        },
                    ].into(),
                },
                structs::SclyObject {
                    instance_id: hud_memo_starting_items_popup_id,
                    connections: vec![].into(),
                    property_data: structs::HudMemo {
                        name: b"Starting Items popup hudmemo\0".as_cstr(),

                        first_message_timer: 0.5,
                        unknown: 1,
                        memo_type: 1,
                        strg: custom_asset_ids::STARTING_ITEMS_HUDMEMO_STRG,
                        active: 1,
                    }.into(),
                },
                structs::SclyObject {
                    instance_id: special_function_starting_items_popup_id,
                    connections: vec![].into(),
                    property_data: structs::SpecialFunction::layer_change_fn(
                        b"Disable Starting Items popup Layer\0".as_cstr(),
                        room_id,
                        layer_count,
                    ).into(),
                },
            ]
        );

        area.add_dependencies(
            &game_resources,
            0,
            iter::once(custom_asset_ids::STARTING_ITEMS_HUDMEMO_STRG.into())
        );
    }
    Ok(())
}

include!("../compile_to_ppc/patches_config.rs");
fn create_rel_config_file(
    spawn_room: SpawnRoomData,
    quickplay: bool,
) -> Vec<u8>
{
    let config = RelConfig {
        quickplay_mlvl: if quickplay { spawn_room.mlvl } else { 0xFFFFFFFF },
        quickplay_mrea: if quickplay { spawn_room.mrea } else { 0xFFFFFFFF },
    };
    let mut buf = vec![0; mem::size_of::<RelConfig>()];
    ssmarshal::serialize(&mut buf, &config).unwrap();
    buf
}

fn patch_dol<'r>(
    file: &mut structs::FstEntryFile,
    spawn_room: SpawnRoomData,
    version: Version,
    config: &PatchConfig,
    remove_ball_color: bool,
    smoother_teleports: bool,
    skip_splash_screens: bool,
) -> Result<(), String>
{
    if version == Version::NtscUTrilogy || version == Version::NtscJTrilogy || version == Version::PalTrilogy {
        return Ok(())
    }

    macro_rules! symbol_addr {
        ($sym:tt, $version:expr) => {
            {
                let s = mp1_symbol!($sym);
                match &$version {
                    Version::NtscU0_00    => s.addr_0_00,
                    Version::NtscU0_01    => s.addr_0_01,
                    Version::NtscU0_02    => s.addr_0_02,
                    Version::NtscK        => s.addr_kor,
                    Version::NtscJ        => s.addr_jap,
                    Version::Pal          => s.addr_pal,
                    Version::NtscUTrilogy => unreachable!(),
                    Version::NtscJTrilogy => unreachable!(),
                    Version::PalTrilogy   => unreachable!(),
                }.unwrap_or_else(|| panic!("Symbol {} unknown for version {}", $sym, $version))
            }
        }
    }

    // new text section for code caves or rel loader
    // skip 0x103c0 bytes after toc register
    let new_text_section_start = symbol_addr!("OSArenaHi", version);
    let mut new_text_section_end = new_text_section_start;
    let mut new_text_section = vec![];

    let reader = match *file {
        structs::FstEntryFile::Unknown(ref reader) => reader.clone(),
        _ => panic!(),
    };

    let mut dol_patcher = DolPatcher::new(reader);
    if version == Version::Pal || version == Version::NtscJ {
        dol_patcher
            .patch(symbol_addr!("aMetroidprime", version), b"randomprime\0"[..].into())?;
    } else {
        dol_patcher
            .patch(symbol_addr!("aMetroidprimeA", version), b"randomprime A\0"[..].into())?
            .patch(symbol_addr!("aMetroidprimeB", version), b"randomprime B\0"[..].into())?;
    }

    let normal_is_default_patch = ppcasm!(symbol_addr!("ActivateNewGamePopup__19SNewFileSelectFrameFv", version) + 0x3C, {
            li      r4, 2;
    });
    dol_patcher.ppcasm_patch(&normal_is_default_patch)?;

    if remove_ball_color {
        let colors = b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00".to_vec();
        dol_patcher.patch(symbol_addr!("skBallInnerGlowColors"  , version), colors.clone().into())?;
        dol_patcher.patch(symbol_addr!("BallAuxGlowColors"      , version), colors.clone().into())?;
        dol_patcher.patch(symbol_addr!("BallTransFlashColors"   , version), colors.clone().into())?;
        dol_patcher.patch(symbol_addr!("BallSwooshColors"       , version), colors.clone().into())?;
        dol_patcher.patch(symbol_addr!("BallSwooshColorsJaggy"  , version), colors.clone().into())?;
        dol_patcher.patch(symbol_addr!("BallSwooshColorsCharged", version), colors.clone().into())?;
        dol_patcher.patch(symbol_addr!("BallGlowColors"         , version), colors.clone().into())?;
    } else if config.suit_colors.is_some() {
        let suit_colors = config.suit_colors.as_ref().unwrap();
        let mut colors: Vec<Vec<u8>> = Vec::new();
        colors.push(vec![0xc2, 0x7e, 0x10, 0x66, 0xc4, 0xff, 0x60, 0xff, 0x90, 0x33, 0x33, 0xff, 0xff, 0x80, 0x80, 0x00, 0x9d, 0xb6, 0xd3, 0xf1, 0x00, 0x60, 0x33, 0xff, 0xfb, 0x98, 0x21]); // skBallInnerGlowColors
        colors.push(vec![0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xd5, 0x19, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]); // BallAuxGlowColors
        colors.push(vec![0xc2, 0x7e, 0x10, 0x66, 0xc4, 0xff, 0x60, 0xff, 0x90, 0x33, 0x33, 0xff, 0xff, 0x20, 0x20, 0x00, 0x9d, 0xb6, 0xd3, 0xf1, 0x00, 0xa6, 0x86, 0xd8, 0xfb, 0x98, 0x21]); // BallTransFlashColors
        colors.push(vec![0xC2, 0x8F, 0x17, 0x70, 0xD4, 0xFF, 0x6A, 0xFF, 0x8A, 0x3D, 0x4D, 0xFF, 0xC0, 0x00, 0x00, 0x00, 0xBE, 0xDC, 0xDF, 0xFF, 0x00, 0xC4, 0x9E, 0xFF, 0xFF, 0x9A, 0x22]); // BallSwooshColors
        colors.push(vec![0xFF, 0xCC, 0x00, 0xFF, 0xCC, 0x00, 0xFF, 0xCC, 0x00, 0xFF, 0xCC, 0x00, 0xFF, 0xD5, 0x19, 0xFF, 0xCC, 0x00, 0xFF, 0xCC, 0x00, 0xFF, 0xCC, 0x00, 0xFF, 0xCC, 0x00]); // BallSwooshColorsJaggy
        colors.push(vec![0xFF, 0xE6, 0x00, 0xFF, 0xE6, 0x00, 0xFF, 0xE6, 0x00, 0xFF, 0xE6, 0x00, 0xFF, 0x80, 0x20, 0xFF, 0xE6, 0x00, 0xFF, 0xE6, 0x00, 0xFF, 0xE6, 0x00, 0xFF, 0xE6, 0x00]); // BallSwooshColorsCharged
        colors.push(vec![0xc2, 0x7e, 0x10, 0x66, 0xc4, 0xff, 0x6c, 0xff, 0x61, 0x33, 0x33, 0xff, 0xff, 0x20, 0x20, 0x00, 0x9d, 0xb6, 0xd3, 0xf1, 0x00, 0xa6, 0x86, 0xd8, 0xfb, 0x98, 0x21]); // BallGlowColors

        for i in 0..colors.len() {
            for j in 0..9 {
                let angle = if vec![0].contains(&j) && suit_colors.power_deg.is_some() {
                    suit_colors.power_deg.clone().unwrap()
                } else if vec![1, 2].contains(&j)  && suit_colors.varia_deg.is_some() {
                    suit_colors.varia_deg.clone().unwrap()
                } else if vec![3].contains(&j)  && suit_colors.gravity_deg.is_some() {
                    suit_colors.gravity_deg.clone().unwrap()
                } else if vec![4].contains(&j)  && suit_colors.phazon_deg.is_some() {
                    suit_colors.phazon_deg.clone().unwrap()
                } else {
                    0
                };

                let angle = angle % 360;
                if angle == 0 {
                    continue;
                }
                let matrix = huerotate_matrix(angle as f32);

                let r_idx = j*3;
                let g_idx = r_idx+1;
                let b_idx = r_idx+2;

                let new_rgb = huerotate_color(matrix, colors[i][r_idx], colors[i][g_idx], colors[i][b_idx]);
                colors[i][r_idx] = new_rgb[0];
                colors[i][g_idx] = new_rgb[1];
                colors[i][b_idx] = new_rgb[2];
            }
        }

        let mut i = 0;
        dol_patcher.patch(symbol_addr!("skBallInnerGlowColors"  , version), colors[i].clone().into())?; i+=1;
        dol_patcher.patch(symbol_addr!("BallAuxGlowColors"      , version), colors[i].clone().into())?; i+=1;
        dol_patcher.patch(symbol_addr!("BallTransFlashColors"   , version), colors[i].clone().into())?; i+=1;
        dol_patcher.patch(symbol_addr!("BallSwooshColors"       , version), colors[i].clone().into())?; i+=1;
        dol_patcher.patch(symbol_addr!("BallSwooshColorsJaggy"  , version), colors[i].clone().into())?; i+=1;
        dol_patcher.patch(symbol_addr!("BallSwooshColorsCharged", version), colors[i].clone().into())?; i+=1;
        dol_patcher.patch(symbol_addr!("BallGlowColors"         , version), colors[i].clone().into())?;
    }

    if config.starting_visor != Visor::Combat {
        let visor = config.starting_visor as u16;
        let no_starting_visor = !config.starting_items.combat_visor && !config.starting_items.scan_visor && !config.starting_items.thermal_visor && !config.starting_items.xray;

        // If no visors, spawn into scan visor without transitioning (spawn without scan GUI)
        if no_starting_visor {
            let scan_visor = Visor::Scan as u16;
            let default_visor_patch = ppcasm!(symbol_addr!("__ct__12CPlayerStateFv", version) + 0x68, {
                    li      r0, scan_visor;
                    stw     r0, 0x14(r31); // currentVisor
                    stw     r0, 0x18(r31); // transitioningVisor
            });
            dol_patcher.ppcasm_patch(&default_visor_patch)?;
            let default_visor_patch = ppcasm!(symbol_addr!("__ct__12CPlayerStateFR12CInputStream", version) + 0x70, {
                    li      r0, scan_visor;
                    stw     r0, 0x14(r30); // currentVisor
                    stw     r0, 0x18(r30); // transitioningVisor
            });
            dol_patcher.ppcasm_patch(&default_visor_patch)?;
            // spawn after elevator
            let default_visor_patch = ppcasm!(symbol_addr!("ResetVisor__12CPlayerStateFv", version), {
                    li      r0, scan_visor;
                    stw     r0, 0x14(r3); // currentVisor
                    stw     r0, 0x18(r3); // transitioningVisor
                    nop;
                    nop;
            });
            dol_patcher.ppcasm_patch(&default_visor_patch)?;
        // Otherwise, spawn mid-transition into default visor
        } else {
            // spawn on game initalization
            let default_visor_patch = ppcasm!(symbol_addr!("__ct__12CPlayerStateFv", version) + 0x68, {
                    li      r0, visor;
                    stw     r6, 0x14(r31); // currentVisor = combat
                    stw     r0, 0x18(r31); // transitioningVisor
            });
            dol_patcher.ppcasm_patch(&default_visor_patch)?;
            let default_visor_patch = ppcasm!(symbol_addr!("__ct__12CPlayerStateFR12CInputStream", version) + 0x70, {
                    li      r0, visor;
                    stw     r5, 0x14(r30); // currentVisor = combat
                    stw     r0, 0x18(r30); // transitioningVisor
            });
            dol_patcher.ppcasm_patch(&default_visor_patch)?;
            // spawn after elevator
            let default_visor_patch = ppcasm!(symbol_addr!("ResetVisor__12CPlayerStateFv", version), {
                    li      r0, 0;
                    stw     r0, 0x14(r3); // currentVisor = combat
                    li      r0, visor;
                    stw     r0, 0x18(r3); // transitioningVisor
                    nop;
            });
            dol_patcher.ppcasm_patch(&default_visor_patch)?;
        }

        let visor_item = match config.starting_visor {
            Visor::Combat  => 17,
            Visor::Scan    => 5,
            Visor::Thermal => 9,
            Visor::XRay    => 13,
        };

        // If scan visor or no visor
        if config.starting_visor == Visor::Scan || no_starting_visor
        {
            // 2022-02-08 - I had to remove this because there's a bug in the vanilla engine where playerhint -> Scan Visor doesn't holster the weapon
            // if no_starting_visor {
            //     // Do not check for combat visor in inventory when switching to it
            //     let default_visor_patch = ppcasm!(symbol_addr!("SetAreaPlayerHint__7CPlayerFRC17CScriptPlayerHintRC13CStateManager", version) + 0x120, {
            //         nop;
            //     });
            //     dol_patcher.ppcasm_patch(&default_visor_patch)?;

            //     // Don't holster weapon when grappling
            //     let default_visor_patch = ppcasm!(symbol_addr!("UpdateGrappleState__7CPlayerFRC11CFinalInputR13CStateManager", version) + (0x8017a998 - 0x8017A668), {
            //             nop;
            //     });
            //     dol_patcher.ppcasm_patch(&default_visor_patch)?;
            // }

            // spawn with weapon holstered instead of drawn
            let patch_offset = if version == Version::Pal || version == Version::NtscJ {
                0x3bc
            } else {
                0x434
            };
            let default_visor_patch = ppcasm!(symbol_addr!("__ct__7CPlayerF9TUniqueIdRC12CTransform4fRC6CAABoxUi9CVector3fffffRC13CMaterialList", version) + patch_offset, {
                    li      r0, 0; // r0 = holstered
            });
            dol_patcher.ppcasm_patch(&default_visor_patch)?;

            // stop gun from being drawn after unmorphing
            let (patch_offset, patch_offset2) = if version == Version::Pal || version == Version::NtscJ {
                (0x79c, 0x7a8)
            } else {
                (0x7c8, 0x7d4)
            };
            let default_visor_patch = ppcasm!(symbol_addr!("TransitionFromMorphBallState__7CPlayerFR13CStateManager", version) + patch_offset, {
                    nop;
            });
            dol_patcher.ppcasm_patch(&default_visor_patch)?;
            let default_visor_patch = ppcasm!(symbol_addr!("TransitionFromMorphBallState__7CPlayerFR13CStateManager", version) + patch_offset2, {
                    nop;
            });
            dol_patcher.ppcasm_patch(&default_visor_patch)?;

            // stop gun from being drawn after unmorphing
            let (patch_offset, patch_offset2) = if version == Version::Pal || version == Version::NtscJ {
                (0x14c, 0x158)
            } else {
                (0x1a4, 0x1b0)
            };
            let default_visor_patch = ppcasm!(symbol_addr!("LeaveMorphBallState__7CPlayerFR13CStateManager", version) + patch_offset, {
                    nop;
            });
            dol_patcher.ppcasm_patch(&default_visor_patch)?;
            let default_visor_patch = ppcasm!(symbol_addr!("LeaveMorphBallState__7CPlayerFR13CStateManager", version) + patch_offset2, {
                    nop;
            });
            dol_patcher.ppcasm_patch(&default_visor_patch)?;

            // do not change visors after unmorphing
            let patch_offset = if version == Version::Pal || version == Version::NtscJ {
                0xb0
            } else {
                0x108
            };
            let default_visor_patch = ppcasm!(symbol_addr!("EnterMorphBallState__7CPlayerFR13CStateManager", version) + patch_offset, {
                    nop;
                    nop;
                    nop;
            });
            dol_patcher.ppcasm_patch(&default_visor_patch)?;
        } else {
            let (patch_offset, patch_offset2) = if version == Version::Pal || version == Version::NtscJ {
                (0xdc, 0xf0)
            } else {
                (0xe8, 0xfc)
            };

            // When pressing a or y in in scan visor, check for and switch to default visor instead of combat
            let default_visor_patch = ppcasm!(symbol_addr!("UpdateVisorState__7CPlayerFRC11CFinalInputfR13CStateManager", version) + patch_offset, {
                    li r4, visor_item;
            });
            dol_patcher.ppcasm_patch(&default_visor_patch)?;
            let default_visor_patch = ppcasm!(symbol_addr!("UpdateVisorState__7CPlayerFRC11CFinalInputfR13CStateManager", version) + patch_offset2, {
                    li r4, visor;
            });
            dol_patcher.ppcasm_patch(&default_visor_patch)?;

            let patch_offset = if version == Version::Pal || version == Version::NtscJ {
                0xb0
            } else {
                0x108
            };

            let default_visor_patch = ppcasm!(symbol_addr!("EnterMorphBallState__7CPlayerFR13CStateManager", version) + patch_offset, {
                    nop;
                    nop;
                    nop;
            });
            dol_patcher.ppcasm_patch(&default_visor_patch)?;
        }
    }

    let beam = config.starting_beam as u16;
    let default_beam_patch = ppcasm!(symbol_addr!("__ct__12CPlayerStateFv", version) + 0x58, {
            li      r0, beam;
            stw     r0, 0x8(r31); // currentBeam
    });
    dol_patcher.ppcasm_patch(&default_beam_patch)?;

    if skip_splash_screens {
        let splash_scren_patch = ppcasm!(symbol_addr!("__ct__13CSplashScreenFQ213CSplashScreen13ESplashScreen", version) + 0x70, {
                nop;
        });
        dol_patcher.ppcasm_patch(&splash_scren_patch)?;
    }

    // let boost_on_spider = ppcasm!(symbol_addr!("ComputeBoostBallMovement__10CMorphBallFRC11CFinalInputRC13CStateManagerf", version) + (0x800f4454 - 0x800f43ac), {
    //         nop;
    // });
    // dol_patcher.ppcasm_patch(&boost_on_spider)?;

    // let bouncy_beam_patch = ppcasm!(symbol_addr!("Explode__17CEnergyProjectileFRC9CVector3fRC9CVector3f29EWeaponCollisionResponseTypesR13CStateManagerRC20CDamageVulnerability9TUniqueId", version) + (0x80214cb4 - 0x80214bf8), {
    //         nop;
    // });
    // dol_patcher.ppcasm_patch(&bouncy_beam_patch)?;

    if smoother_teleports {
        // Do not holster arm cannon
        let better_teleport_patch = ppcasm!(symbol_addr!("Teleport__7CPlayerFRC12CTransform4fR13CStateManagerb", version) + 0x31C, {
                nop;
        });
        dol_patcher.ppcasm_patch(&better_teleport_patch)?;
        // NTSC-U 0-00 (0x80017690 - 0x8001766c)
        let better_teleport_patch = ppcasm!(symbol_addr!("SetSpawnedMorphBallState__7CPlayerFQ27CPlayer21EPlayerMorphBallStateR13CStateManager", version) + 0x24, {
                nop; // SetCameraState
        });
        dol_patcher.ppcasm_patch(&better_teleport_patch)?;
        // NTSC-U 0-00 (0x80017770 - 0x8001766c)
        let better_teleport_patch = ppcasm!(symbol_addr!("SetSpawnedMorphBallState__7CPlayerFQ27CPlayer21EPlayerMorphBallStateR13CStateManager", version) + 0x104, {
                nop; // ForceGunOrientation
        });
        dol_patcher.ppcasm_patch(&better_teleport_patch)?;
        // NTSC-U 0-00 (0x80017764 - 0x8001766c)
        let better_teleport_patch = ppcasm!(symbol_addr!("SetSpawnedMorphBallState__7CPlayerFQ27CPlayer21EPlayerMorphBallStateR13CStateManager", version) + 0xf8, {
                nop; // DrawGun
        });
        dol_patcher.ppcasm_patch(&better_teleport_patch)?;
        // let better_teleport_patch = ppcasm!(symbol_addr!("LeaveMorphBallState__7CPlayerFR13CStateManager", version) + (0x80282ec0 - 0x80282d1c), {
        //         nop; // ForceGunOrientation
        // });
        // dol_patcher.ppcasm_patch(&better_teleport_patch)?;
        // let better_teleport_patch = ppcasm!(symbol_addr!("LeaveMorphBallState__7CPlayerFR13CStateManager", version) + (0x80282ecc - 0x80282d1c), {
        //         nop; // DrawGun
        // });
        // dol_patcher.ppcasm_patch(&better_teleport_patch)?;
    }

    if config.automatic_crash_screen {
        let patch_offset = if version == Version::NtscU0_00 {
            0xEC
        } else {
            0x120
        };
        let automatic_crash_patch = ppcasm!(symbol_addr!("CrashScreenControllerPollBranch", version) + patch_offset, {
                nop;
        });
        dol_patcher.ppcasm_patch(&automatic_crash_patch)?;
    }

    let cinematic_skip_patch = ppcasm!(symbol_addr!("ShouldSkipCinematic__22CScriptSpecialFunctionFR13CStateManager", version), {
            li      r3, 0x1;
            blr;
    });
    dol_patcher.ppcasm_patch(&cinematic_skip_patch)?;

    // stop doors from communicating with their partner
    // let open_door_patch = ppcasm!(symbol_addr!("OpenDoor__11CScriptDoorF9TUniqueIdR13CStateManager", version) + (0x8007ec70 - 0x8007ea64), {
    //     nop;
    // });
    // dol_patcher.ppcasm_patch(&open_door_patch)?;

    // pattern 50801f38 981f???? 881f???? 5080177a 981f???? 83e1
    if version == Version::Pal {
        let unlockables_default_ctor_patch = ppcasm!(symbol_addr!("__ct__14CSystemOptionsFv", version) + 0x1dc, {
            li      r6, 100;
            stw     r6, 0x80(r31);
            lis     r6, 0xF7FF;
            stw     r6, 0x84(r31);
        });
        dol_patcher.ppcasm_patch(&unlockables_default_ctor_patch)?;
    } else if version == Version::NtscJ {
        let unlockables_default_ctor_patch = ppcasm!(symbol_addr!("__ct__14CSystemOptionsFv", version) + 0x1bc, {
            li      r6, 100;
            stw     r6, 0x664(r31);
            lis     r6, 0xF7FF;
            stw     r6, 0x668(r31);
        });
        dol_patcher.ppcasm_patch(&unlockables_default_ctor_patch)?;
    } else {
        let unlockables_default_ctor_patch = ppcasm!(symbol_addr!("__ct__14CSystemOptionsFv", version) + 0x194, {
            li      r6, 100;
            stw     r6, 0xcc(r3);
            lis     r6, 0xF7FF;
            stw     r6, 0xd0(r3);
        });
        dol_patcher.ppcasm_patch(&unlockables_default_ctor_patch)?;
    };

    if version == Version::Pal {
        let unlockables_read_ctor_patch = ppcasm!(symbol_addr!("__ct__14CSystemOptionsFRC12CInputStream", version) + 0x330, {
            li      r6, 100;
            stw     r6, 0x80(r28);
            lis     r6, 0xF7FF;
            stw     r6, 0x84(r28);
            mr      r3, r29;
            li      r4, 2;
        });
        dol_patcher.ppcasm_patch(&unlockables_read_ctor_patch)?;
    } else if version == Version::NtscJ {
        let unlockables_read_ctor_patch = ppcasm!(symbol_addr!("__ct__14CSystemOptionsFRC12CInputStream", version) + 0x310, {
            li      r6, 100;
            stw     r6, 0x664(r29);
            lis     r6, 0xF7FF;
            stw     r6, 0x668(r29);
            mr      r3, r30;
            li      r4, 2;
        });
        dol_patcher.ppcasm_patch(&unlockables_read_ctor_patch)?;
    } else {
        let unlockables_read_ctor_patch = ppcasm!(symbol_addr!("__ct__14CSystemOptionsFRC12CInputStream", version) + 0x308, {
            li      r6, 100;
            stw     r6, 0xcc(r28);
            lis     r6, 0xF7FF;
            stw     r6, 0xd0(r28);
            mr      r3, r29;
            li      r4, 2;
        });
        dol_patcher.ppcasm_patch(&unlockables_read_ctor_patch)?;
    };

    if version != Version::Pal && version != Version::NtscJ {
        let missile_hud_formating_patch = ppcasm!(symbol_addr!("SetNumMissiles__20CHudMissileInterfaceFiRC13CStateManager", version) + 0x14, {
                b          skip;
            fmt:
                .asciiz b"%03d/%03d";

            skip:
                stw        r30, 40(r1);// var_8(r1);
                mr         r30, r3;
                stw        r4, 8(r1);// var_28(r1)

                lwz        r6, 4(r30);

                mr         r5, r4;

                lis        r4, fmt@h;
                addi       r4, r4, fmt@l;

                addi       r3, r1, 12;// arg_C

                nop; // crclr      cr6;
                bl         { symbol_addr!("sprintf", version) };

                addi       r3, r1, 20;// arg_14;
                addi       r4, r1, 12;// arg_C
        });
        dol_patcher.ppcasm_patch(&missile_hud_formating_patch)?;
    }

    if config.qol_cosmetic {
        let powerbomb_hud_formating_patch = ppcasm!(symbol_addr!("SetBombParams__17CHudBallInterfaceFiiibbb", version) + 0x2c, {
                b skip;
            fmt:
                .asciiz b"%d/%d";// %d";
                nop;
            skip:
                mr         r6, r27;
                mr         r5, r28;
                lis        r4, fmt@h;
                addi       r4, r4, fmt@l;
                addi       r3, r1, 12;// arg_C;
                nop; // crclr      cr6;
                bl         { symbol_addr!("sprintf", version) };

        });
        dol_patcher.ppcasm_patch(&powerbomb_hud_formating_patch)?;
    }

    if version == Version::Pal || version == Version::NtscJ {
        let level_select_mlvl_upper_patch = ppcasm!(symbol_addr!("__sinit_CFrontEndUI_cpp", version) + 0x0c, {
                lis         r3, {spawn_room.mlvl}@h;
        });
        dol_patcher.ppcasm_patch(&level_select_mlvl_upper_patch)?;

        let level_select_mlvl_lower_patch = ppcasm!(symbol_addr!("__sinit_CFrontEndUI_cpp", version) + 0x18, {
                addi        r0, r3, {spawn_room.mlvl}@l;
        });
        dol_patcher.ppcasm_patch(&level_select_mlvl_lower_patch)?;
    } else {
        let level_select_mlvl_upper_patch = ppcasm!(symbol_addr!("__sinit_CFrontEndUI_cpp", version) + 0x04, {
                lis         r4, {spawn_room.mlvl}@h;
        });
        dol_patcher.ppcasm_patch(&level_select_mlvl_upper_patch)?;

        let level_select_mlvl_lower_patch = ppcasm!(symbol_addr!("__sinit_CFrontEndUI_cpp", version) + 0x10, {
                addi        r0, r4, {spawn_room.mlvl}@l;
        });
        dol_patcher.ppcasm_patch(&level_select_mlvl_lower_patch)?;
    }

    let level_select_mrea_idx_patch = ppcasm!(symbol_addr!("__ct__11CWorldStateFUi", version) + 0x10, {
            li          r0, { spawn_room.mrea_idx };
    });
    dol_patcher.ppcasm_patch(&level_select_mrea_idx_patch)?;

    if config.nonvaria_heat_damage {
        let heat_damage_patch = ppcasm!(symbol_addr!("ThinkAreaDamage__22CScriptSpecialFunctionFfR13CStateManager", version) + 0x4c, {
                lwz     r4, 0xdc(r4);
                nop;
                subf    r0, r6, r5;
                cntlzw  r0, r0;
                nop;
        });
        dol_patcher.ppcasm_patch(&heat_damage_patch)?;
    }

    if config.staggered_suit_damage {
        let (patch_offset, jump_offset) = if version == Version::Pal || version == Version::NtscJ {
            (0x11c, 0x1b8)
        } else {
            (0x128, 0x1c4)
        };

        let staggered_suit_damage_patch = ppcasm!(symbol_addr!("ApplyLocalDamage__13CStateManagerFRC9CVector3fRC9CVector3fR6CActorfRC11CWeaponMode", version) + patch_offset, {
                lwz     r3, 0x8b8(r25);
                lwz     r3, 0(r3);
                lwz     r4, 220(r3);
                lwz     r5, 212(r3);
                addc    r4, r4, r5;
                lwz     r5, 228(r3);
                addc    r4, r4, r5;
                rlwinm  r4, r4, 2, 0, 29;
                lis     r6, data@h;
                addi    r6, r6, data@l;
                lfsx     f0, r4, r6;
                b       { symbol_addr!("ApplyLocalDamage__13CStateManagerFRC9CVector3fRC9CVector3fR6CActorfRC11CWeaponMode", version) + jump_offset };
            data:
                .float 0.0;
                .float 0.1;
                .float 0.2;
                .float 0.5;
        });
        dol_patcher.ppcasm_patch(&staggered_suit_damage_patch)?;
    }

    for (pickup_type, value) in &config.item_max_capacity {
        let capacity_patch = ppcasm!(symbol_addr!("CPlayerState_PowerUpMaxValues", version) + pickup_type.kind() * 4, {
            .long *value;
        });
        dol_patcher.ppcasm_patch(&capacity_patch)?;
    }

    // set etank capacity and base health
    let etank_capacity = config.etank_capacity as f32;
    let base_health = etank_capacity - 1.0;
    let etank_capacity_base_health_patch = ppcasm!(symbol_addr!("g_EtankCapacity", version), {
        .float etank_capacity;
        .float base_health;
    });
    dol_patcher.ppcasm_patch(&etank_capacity_base_health_patch)?;

    if version == Version::NtscU0_02 || version == Version::Pal || version == Version::NtscJ {
        let players_choice_scan_dash_patch = ppcasm!(symbol_addr!("SidewaysDashAllowed__7CPlayerCFffRC11CFinalInputR13CStateManager", version) + 0x3c, {
                b       { symbol_addr!("SidewaysDashAllowed__7CPlayerCFffRC11CFinalInputR13CStateManager", version) + 0x54 };
        });
        dol_patcher.ppcasm_patch(&players_choice_scan_dash_patch)?;
    }

    if config.map_default_state != MapState::Default {
        let is_mapped_patch = ppcasm!(symbol_addr!("IsMapped__13CMapWorldInfoCF7TAreaId", version), {
            li      r3, 0x1;
            blr;
        });
        dol_patcher.ppcasm_patch(&is_mapped_patch)?;
        if config.map_default_state == MapState::Visited {
            let is_area_visited_patch = ppcasm!(symbol_addr!("IsAreaVisited__13CMapWorldInfoCF7TAreaId", version), {
                li      r3, 0x1;
                blr;
            });
            dol_patcher.ppcasm_patch(&is_area_visited_patch)?;
        }
    }

    // Update default game options to match user's prefrence
    {
        /* define default defaults (lol) */
        let mut screen_brightness : u32  = 4    ;
        let mut screen_offset_x   : i32  = 0    ;
        let mut screen_offset_y   : i32  = 0    ;
        let mut screen_stretch    : i32  = 0    ;
        let mut sound_mode        : u32  = 1    ;
        let mut sfx_volume        : u32  = 0x7f ;
        let mut music_volume      : u32  = 0x7f ;
        let mut visor_opacity     : u32  = 0xff ;
        let mut helmet_opacity    : u32  = 0xff ;
        let mut hud_lag           : bool = true ;
        let mut reverse_y_axis    : bool = false;
        let mut rumble            : bool = true ;
        let mut swap_beam_controls: bool = false;
        let hints                 : bool = false;

        /* Update with user-defined defaults */
        if config.default_game_options.is_some()
        {
            let default_game_options = config.default_game_options.clone().unwrap();
            if default_game_options.screen_brightness  .is_some() { screen_brightness  = default_game_options.screen_brightness  .unwrap(); }
            if default_game_options.screen_offset_x    .is_some() { screen_offset_x    = default_game_options.screen_offset_x    .unwrap(); }
            if default_game_options.screen_offset_y    .is_some() { screen_offset_y    = default_game_options.screen_offset_y    .unwrap(); }
            if default_game_options.screen_stretch     .is_some() { screen_stretch     = default_game_options.screen_stretch     .unwrap(); }
            if default_game_options.sound_mode         .is_some() { sound_mode         = default_game_options.sound_mode         .unwrap(); }
            if default_game_options.sfx_volume         .is_some() { sfx_volume         = default_game_options.sfx_volume         .unwrap(); }
            if default_game_options.music_volume       .is_some() { music_volume       = default_game_options.music_volume       .unwrap(); }
            if default_game_options.visor_opacity      .is_some() { visor_opacity      = default_game_options.visor_opacity      .unwrap(); }
            if default_game_options.helmet_opacity     .is_some() { helmet_opacity     = default_game_options.helmet_opacity     .unwrap(); }
            if default_game_options.hud_lag            .is_some() { hud_lag            = default_game_options.hud_lag            .unwrap(); }
            if default_game_options.reverse_y_axis     .is_some() { reverse_y_axis     = default_game_options.reverse_y_axis     .unwrap(); }
            if default_game_options.rumble             .is_some() { rumble             = default_game_options.rumble             .unwrap(); }
            if default_game_options.swap_beam_controls .is_some() { swap_beam_controls = default_game_options.swap_beam_controls .unwrap(); }
            // Users may not change default hint state
        }

        /* Aggregate bit fields */
        let mut bit_flags: u32 =  0x00;
        if  hud_lag            { bit_flags |= 1    << 7; }
        if  reverse_y_axis     { bit_flags |= 1    << 6; }
        if  rumble             { bit_flags |= 1    << 5; }
        if  swap_beam_controls { bit_flags |= 1    << 4; }
        if  hints              { bit_flags |= 1    << 3; }

        /* Replace reset to default function */
        let default_game_options_patch = ppcasm!(symbol_addr!("ResetToDefaults__12CGameOptionsFv", version) + 9 * 4, {
            li         r0, screen_brightness;
            stw        r0, 0x48(r3);
            li         r0, screen_offset_x;
            stw        r0, 0x4C(r3);
            li         r0, screen_offset_y;
            stw        r0, 0x50(r3);
            li         r0, screen_stretch;
            stw        r0, 0x54(r3);
            li         r0, sfx_volume;
            stw        r0, 0x58(r3);
            li         r0, music_volume;
            stw        r0, 0x5C(r3);
            li         r0, sound_mode;
            stw        r0, 0x44(r3);
            li         r0, visor_opacity;
            stw        r0, 0x60(r3);
            li         r0, helmet_opacity;
            stw        r0, 0x64(r3);
            li         r0, bit_flags;
            stb        r0, 0x68(r3);
            nop;
            nop;
            nop;
            nop;
            nop;
        });
        dol_patcher.ppcasm_patch(&default_game_options_patch)?;
    }

    // Multiworld focused patches
    if config.multiworld_dol_patches {
        // IncrPickUp's switch array for UnknownItem1 to actually give stuff
        let incr_pickup_switch_patch = ppcasm!(symbol_addr!("IncrPickUpSwitchCaseData", version) + 21 * 4, {
            .long symbol_addr!("IncrPickUp__12CPlayerStateFQ212CPlayerState9EItemTypei", version) + 25 * 4;
        });
        dol_patcher.ppcasm_patch(&incr_pickup_switch_patch)?;

        // Remove DecrPickUp checks for the correct item types
        let decr_pickup_patch = ppcasm!(symbol_addr!("DecrPickUp__12CPlayerStateFQ212CPlayerState9EItemTypei", version) + 5 * 4, {
            nop;
            nop;
            nop;
            nop;
            nop;
            nop;
            nop;
        });
        dol_patcher.ppcasm_patch(&decr_pickup_patch)?;
    }

    if let Some(update_hint_state_replacement) = &config.update_hint_state_replacement {
        dol_patcher.patch(symbol_addr!("UpdateHintState__13CStateManagerFf", version), Cow::from(update_hint_state_replacement.clone()))?;
    }

    // Add rel loader to the binary
    let (rel_loader_bytes, rel_loader_map_str) = match version {
        Version::NtscU0_00 => {
            let loader_bytes = rel_files::REL_LOADER_100;
            let map_str = rel_files::REL_LOADER_100_MAP;
            (loader_bytes, map_str)
        },
        Version::NtscU0_01 => {
            let loader_bytes = rel_files::REL_LOADER_101;
            let map_str = rel_files::REL_LOADER_101_MAP;
            (loader_bytes, map_str)
        },
        Version::NtscU0_02 => {
            let loader_bytes = rel_files::REL_LOADER_102;
            let map_str = rel_files::REL_LOADER_102_MAP;
            (loader_bytes, map_str)
        },
        Version::NtscK => {
            let loader_bytes = rel_files::REL_LOADER_KOR;
            let map_str = rel_files::REL_LOADER_KOR_MAP;
            (loader_bytes, map_str)
        },
        Version::NtscJ => {
            let loader_bytes = rel_files::REL_LOADER_JAP;
            let map_str = rel_files::REL_LOADER_JAP_MAP;
            (loader_bytes, map_str)
        },
        Version::Pal => {
            let loader_bytes = rel_files::REL_LOADER_PAL;
            let map_str = rel_files::REL_LOADER_PAL_MAP;
            (loader_bytes, map_str)
        },
        Version::NtscUTrilogy => unreachable!(),
        Version::NtscJTrilogy => unreachable!(),
        Version::PalTrilogy => unreachable!(),
    };

    let mut rel_loader = rel_loader_bytes.to_vec();
    let rel_loader_padding_size = ((rel_loader.len() + 3) & !3) - rel_loader.len();
    rel_loader.extend([0; 4][..rel_loader_padding_size].iter().copied());

    let rel_loader_map = dol_linker::parse_symbol_table(
        "extra_assets/rel_loader_1.0?.bin.map".as_ref(),
        rel_loader_map_str.lines().map(|l| Ok(l.to_owned())),
    ).map_err(|e| e.to_string())?;

    let rel_loader_size = rel_loader.len() as u32;
    new_text_section.extend(rel_loader);

    dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("PPCSetFpIEEEMode", version), {
        b      { rel_loader_map["rel_loader_hook"] };
    }))?;

    new_text_section_end = new_text_section_end + rel_loader_size;

    if config.warp_to_start
    {
        let handle_no_to_save_msg_patch = ppcasm!(symbol_addr!("ThinkSaveStation__22CScriptSpecialFunctionFfR13CStateManager", version) + 0x54, {
                b         { new_text_section_end };
        });
        dol_patcher.ppcasm_patch(&handle_no_to_save_msg_patch)?;

        let warp_to_start_patch = ppcasm!(new_text_section_end, {
                lis       r14, {symbol_addr!("g_Main", version)}@h;
                addi      r14, r14, {symbol_addr!("g_Main", version)}@l;
                lwz       r14, 0x0(r14);
                lwz       r14, 0x164(r14);
                lwz       r14, 0x34(r14);
                lbz       r0, 0x86(r14);
                cmpwi     r0, 0;
                beq       { new_text_section_end + 0x34 };
                lbz       r0, 0x89(r14);
                cmpwi     r0, 0;
                beq       { new_text_section_end + 0x34 };
                li        r4, 12;
                b         { new_text_section_end + 0x38 };
                li        r4, 9;
                andi      r14, r14, 0;
                b         { symbol_addr!("ThinkSaveStation__22CScriptSpecialFunctionFfR13CStateManager", version) + 0x58 };
        });

        new_text_section_end = new_text_section_end + warp_to_start_patch.encoded_bytes().len() as u32;
        new_text_section.extend(warp_to_start_patch.encoded_bytes());
    }

    // TO-DO :
    // Disable spring ball on Trilogy if config.spring_ball is set to false
    if config.spring_ball {
        // call compute spring ball movement
        let call_compute_spring_ball_movement_patch = ppcasm!(symbol_addr!("ComputeBallMovement__10CMorphBallFRC11CFinalInputR13CStateManagerf", version) + 0x2c, {
                bl         { new_text_section_end };
        });
        dol_patcher.ppcasm_patch(&call_compute_spring_ball_movement_patch)?;

        // rewrote as tuple to make it cleaner
        let (velocity_offset, movement_state_offset, attached_actor_offset, energy_drain_offset, out_of_water_ticks_offset, surface_restraint_type_offset) = if version == Version::NtscU0_00 || version == Version::NtscU0_01 || version == Version::NtscK
        {
            (0x138, 0x258, 0x26c, 0x274, 0x2b0, 0x2ac)
        }
        else
        {
            (0x148, 0x268, 0x27c, 0x284, 0x2c0, 0x2bc)
        };

        let spring_ball_patch = ppcasm!(new_text_section_end, {
                // stack init
                stwu      r1, -0x20(r1);
                mflr      r0;
                stw       r0, 0x20(r1);
                fmr       f15, f1;
                stw       r31, 0x1c(r1);
                stw       r30, 0x18(r1);
                mr        r30, r5;
                stw       r29, 0x14(r1);
                mr        r29, r4;
                stw       r28, 0x10(r1);
                mr        r28, r3;

                // function body
                lwz       r14, 0x84c(r30);
                lwz       r15, 0x8b8(r30);
                lis       r16, data@h;
                addi      r16, r16, data@l;
                lfs       f1, 0x40(r14);
                stfs      f1, 0x00(r16);
                lfs       f1, 0x50(r14);
                stfs      f1, 0x04(r16);
                lfs       f1, 0x60(r14);
                stfs      f1, 0x08(r16);
                lwz       r0, 0x0c(r16);
                cmplwi    r0, 0;
                bgt       { new_text_section_end + 0x12c };
                lwz       r0, { movement_state_offset }(r14);
                cmplwi    r0, 0;
                beq       { new_text_section_end + 0x80 };
                b         { new_text_section_end + 0x12c };
                cmplwi    r0, 4;
                bne       { new_text_section_end + 0x12c };
                lwz       r0, { out_of_water_ticks_offset }(r14);
                cmplwi    r0, 2;
                bne       { new_text_section_end + 0x8c };
                lwz       r0, { surface_restraint_type_offset }(r14);
                b         { new_text_section_end + 0x90 };
                li        r0, 4;
                cmplwi    r0, 7;
                beq       { new_text_section_end + 0x12c };
                mr        r3, r28;
                bl        { symbol_addr!("IsMovementAllowed__10CMorphBallCFv", version) };
                cmplwi    r3, 0;
                beq       { new_text_section_end + 0x12c };
                lwz       r3, 0x0(r15);
                li        r4, 6;
                bl        { symbol_addr!("HasPowerUp__12CPlayerStateCFQ212CPlayerState9EItemType", version) };
                cmplwi    r3, 0;
                beq       { new_text_section_end + 0x12c };
                lhz       r0, { attached_actor_offset }(r14);
                cmplwi    r0, 65535;
                bne       { new_text_section_end + 0x12c };
                addi      r3, r14, { energy_drain_offset };
                bl        { symbol_addr!("GetEnergyDrainIntensity__18CPlayerEnergyDrainCFv", version) };
                fcmpu     cr0, f1, f14;
                bgt       { new_text_section_end + 0x12c };
                lwz       r0, 0x187c(r28);
                cmplwi    r0, 0;
                bne       { new_text_section_end + 0x12c };
                lfs       f1, 0x14(r29);
                fcmpu     cr0, f1, f14;
                ble       { new_text_section_end + 0x12c };
                lfs       f16, { velocity_offset }(r14);
                lfs       f17, { velocity_offset + 4 }(r14);
                mr        r3, r14;
                mr        r4, r16;
                mr        r5, r30;
                bl        { symbol_addr!("BombJump__7CPlayerFRC9CVector3fR13CStateManager", version) };
                stfs      f16, { velocity_offset }(r14);
                stfs      f17, { velocity_offset + 4 }(r14);
                mr        r3, r14;
                li        r4, 4;
                mr        r5, r29;
                bl        { symbol_addr!("SetMoveState__7CPlayerFQ27NPlayer20EPlayerMovementStateR13CStateManager", version) };
                li        r3, 40;
                stw       r3, 0x0c(r16);
                b         { new_text_section_end + 0x140 };
                lwz       r3, 0x0c(r16);
                cmplwi    r3, 0;
                beq       { new_text_section_end + 0x140 };
                addi      r3, r3, -1;
                stw       r3, 0x0c(r16);

                // call compute boost ball movement
                mr        r3, r28;
                mr        r4, r29;
                mr        r5, r30;
                fmr       f1, f15;
                bl        { symbol_addr!("ComputeBoostBallMovement__10CMorphBallFRC11CFinalInputRC13CStateManagerf", version) };

                // stack deinit
                lwz       r0, 0x20(r1);
                fmr       f1, f15;
                fmr       f15, f14;
                fmr       f16, f14;
                fmr       f17, f14;
                lwz       r31, 0x1c(r1);
                lwz       r30, 0x18(r1);
                lwz       r29, 0x14(r1);
                lwz       r28, 0x10(r1);
                mtlr      r0;
                addi      r1, r1, 0x20;
                blr;
            data:
                .float 0.0;
                .float 0.0;
                .float 0.0;
                .long 0;
        });

        new_text_section_end = new_text_section_end + spring_ball_patch.encoded_bytes().len() as u32;
        new_text_section.extend(spring_ball_patch.encoded_bytes());
    }

    let bytes_needed = ((new_text_section.len() + 31) & !31) - new_text_section.len();
    new_text_section.extend([0; 32][..bytes_needed].iter().copied());
    dol_patcher.add_text_segment(new_text_section_start, Cow::Owned(new_text_section))?;

    // move the ram after the newly added sections (if there are any)
    dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("OSInit", version) + 0xe0, {
            lis        r3, { new_text_section_end + 0x10000 }@h;
        }))?;

    dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("OSInit", version) + 0x118, {
            lis        r3, { new_text_section_end + 0x10000 }@h;
        }))?;

    *file = structs::FstEntryFile::ExternalFile(Box::new(dol_patcher));
    Ok(())
}

fn empty_frigate_pak<'r>(file: &mut structs::FstEntryFile)
    -> Result<(), String>
{
    // To reduce the amount of data that needs to be copied, empty the contents of the pak
    let pak = match file {
        structs::FstEntryFile::Pak(pak) => pak,
        _ => unreachable!(),
    };

    // XXX This is a workaround for a bug in some versions of Nintendont.
    //     The details can be found in a comment on issue #5.
    let res = crate::custom_assets::build_resource_raw(
        0,
        structs::ResourceKind::External(vec![0; 64], b"XXXX".into())
    );
    pak.resources = iter::once(res).collect();
    Ok(())
}

fn patch_ctwk_game(res: &mut structs::Resource, ctwk_config: &CtwkConfig)
    -> Result<(), String>
{
    let mut ctwk = res.kind.as_ctwk_mut().unwrap();
    let mut ctwk_game = match &mut ctwk {
        structs::Ctwk::CtwkGame(i) => i,
        _ => panic!("Failed to map res=0x{:X} as CtwkGame", res.file_id),
    };

    ctwk_game.press_start_delay = 0.001;

    if ctwk_config.fov.is_some() {
        ctwk_game.fov = ctwk_config.fov.unwrap();
    }

    if ctwk_config.hardmode_damage_mult.is_some() {
        ctwk_game.hardmode_damage_mult = ctwk_config.hardmode_damage_mult.unwrap();
    }

    if ctwk_config.hardmode_weapon_mult.is_some() {
        ctwk_game.hardmode_weapon_mult = ctwk_config.hardmode_weapon_mult.unwrap();
    }

    if ctwk_config.underwater_fog_distance.is_some() {
        let underwater_fog_distance = ctwk_config.underwater_fog_distance.unwrap();
        ctwk_game.water_fog_distance_base = ctwk_game.water_fog_distance_base*underwater_fog_distance;
        ctwk_game.water_fog_distance_range = ctwk_game.water_fog_distance_range*underwater_fog_distance;
        ctwk_game.gravity_water_fog_distance_base = ctwk_game.gravity_water_fog_distance_base*underwater_fog_distance;
        ctwk_game.gravity_water_fog_distance_range = ctwk_game.gravity_water_fog_distance_range*underwater_fog_distance;
    }

    Ok(())
}

fn patch_ctwk_player(res: &mut structs::Resource, ctwk_config: &CtwkConfig)
-> Result<(), String>
{
    let mut ctwk = res.kind.as_ctwk_mut().unwrap();
    let mut ctwk_player = match &mut ctwk {
        structs::Ctwk::CtwkPlayer(i) => i,
        _ => panic!("Failed to map res=0x{:X} as CtwkPlayer", res.file_id),
    };

    if ctwk_config.player_size.is_some() {
        let player_size = ctwk_config.player_size.unwrap();
        ctwk_player.player_height = ctwk_player.player_height*player_size;
        ctwk_player.player_xy_half_extent = ctwk_player.player_xy_half_extent*player_size;
        ctwk_player.step_up_height = ctwk_player.step_up_height*player_size;
        ctwk_player.step_down_height = ctwk_player.step_down_height*player_size;
    }

    if ctwk_config.step_up_height.is_some() {
        ctwk_player.step_up_height = ctwk_player.step_up_height*ctwk_config.step_up_height.unwrap();
    }

    if ctwk_config.morph_ball_size.is_some() {
        ctwk_player.player_ball_half_extent = ctwk_player.player_ball_half_extent*ctwk_config.morph_ball_size.unwrap();
    }

    if ctwk_config.easy_lava_escape.unwrap_or(false) {
        ctwk_player.lava_jump_factor = 100.0;
        ctwk_player.lava_ball_jump_factor = 100.0;
    }

    if ctwk_config.move_while_scan.unwrap_or(false) {
        ctwk_player.scan_freezes_game = 0;
    }

    if ctwk_config.scan_range.is_some() {
        let scan_range = ctwk_config.scan_range.unwrap();

        ctwk_player.scanning_range = scan_range;

        if scan_range > ctwk_player.scan_max_lock_distance {
            ctwk_player.scan_max_lock_distance = scan_range;
        }

        if scan_range > ctwk_player.scan_max_target_distance {
            ctwk_player.scan_max_target_distance = scan_range;
        }
    }

    if ctwk_config.bomb_jump_height.is_some() {
        ctwk_player.bomb_jump_height = ctwk_player.bomb_jump_height*ctwk_config.bomb_jump_height.unwrap();
    }

    if ctwk_config.bomb_jump_radius.is_some() {
        ctwk_player.bomb_jump_radius = ctwk_player.bomb_jump_radius*ctwk_config.bomb_jump_radius.unwrap();
    }

    if ctwk_config.grapple_beam_speed.is_some() {
        ctwk_player.grapple_beam_speed = ctwk_player.grapple_beam_speed*ctwk_config.grapple_beam_speed.unwrap();
    }

    if ctwk_config.aim_assist_angle.is_some() {
        let aim_assist_angle = ctwk_config.aim_assist_angle.unwrap();
        ctwk_player.aim_assist_vertical_angle = aim_assist_angle;
        ctwk_player.aim_assist_horizontal_angle = aim_assist_angle;
    }

    if ctwk_config.gravity.is_some() {
        ctwk_player.normal_grav_accel = ctwk_player.normal_grav_accel*ctwk_config.gravity.unwrap();
    }

    if ctwk_config.ice_break_timeout.is_some() {
        ctwk_player.frozen_timeout = ctwk_config.ice_break_timeout.unwrap();
    }

    if ctwk_config.ice_break_jump_count.is_some() {
        ctwk_player.ice_break_jump_count = ctwk_config.ice_break_jump_count.unwrap();
    }

    if ctwk_config.ice_break_jump_count.is_some() {
        ctwk_player.ice_break_jump_count = ctwk_config.ice_break_jump_count.unwrap();
    }

    if ctwk_config.ground_friction.is_some() {
        ctwk_player.translation_friction[0] = ctwk_player.translation_friction[0]*ctwk_config.ground_friction.unwrap();
    }

    if ctwk_config.coyote_frames.is_some() {
        ctwk_player.allowed_ledge_time = (ctwk_config.coyote_frames.unwrap() as f32)*(1.0/60.0);
    }

    if ctwk_config.move_during_free_look.unwrap_or(false) {
        ctwk_player.move_during_free_look = 1;
    }

    if ctwk_config.recenter_after_freelook.unwrap_or(false) {
        ctwk_player.freelook_turns_player = 0;
    }

    if ctwk_config.toggle_free_look.unwrap_or(false) {
        ctwk_player.hold_buttons_for_free_look = 0;
    }

    if ctwk_config.two_buttons_for_free_look.unwrap_or(false) {
        ctwk_player.two_buttons_for_free_look = 1;
    }

    if ctwk_config.disable_dash.unwrap_or(false) {
        ctwk_player.dash_enabled = 0;
    }

    if ctwk_config.varia_damage_reduction.is_some() {
        ctwk_player.varia_damage_reduction = ctwk_player.varia_damage_reduction*ctwk_config.varia_damage_reduction.unwrap();
    }

    if ctwk_config.gravity_damage_reduction.is_some() {
        ctwk_player.gravity_damage_reduction = ctwk_player.gravity_damage_reduction*ctwk_config.gravity_damage_reduction.unwrap();
    }

    if ctwk_config.phazon_damage_reduction.is_some() {
        ctwk_player.phazon_damage_reduction = ctwk_player.phazon_damage_reduction*ctwk_config.phazon_damage_reduction.unwrap();
    }

    if ctwk_config.max_speed.is_some() {
        let max_speed = ctwk_config.max_speed.unwrap();
        ctwk_player.translation_max_speed[0] = ctwk_player.translation_max_speed[0]*max_speed;
        ctwk_player.translation_max_speed[1] = ctwk_player.translation_max_speed[1]*max_speed;
        ctwk_player.translation_max_speed[2] = ctwk_player.translation_max_speed[2]*max_speed;
        ctwk_player.translation_max_speed[3] = ctwk_player.translation_max_speed[3]*max_speed;
        ctwk_player.translation_max_speed[4] = ctwk_player.translation_max_speed[4]*max_speed;
        ctwk_player.translation_max_speed[5] = ctwk_player.translation_max_speed[5]*max_speed;
        ctwk_player.translation_max_speed[6] = ctwk_player.translation_max_speed[6]*max_speed;
        ctwk_player.translation_max_speed[7] = ctwk_player.translation_max_speed[7]*max_speed;
    }

    if ctwk_config.max_acceleration.is_some() {
        let max_acceleration = ctwk_config.max_acceleration.unwrap();
        ctwk_player.translation_max_speed[0] = ctwk_player.translation_max_speed[0]*max_acceleration;
        ctwk_player.translation_max_speed[1] = ctwk_player.translation_max_speed[1]*max_acceleration;
        ctwk_player.translation_max_speed[2] = ctwk_player.translation_max_speed[2]*max_acceleration;
        ctwk_player.translation_max_speed[3] = ctwk_player.translation_max_speed[3]*max_acceleration;
        ctwk_player.translation_max_speed[4] = ctwk_player.translation_max_speed[4]*max_acceleration;
        ctwk_player.translation_max_speed[5] = ctwk_player.translation_max_speed[5]*max_acceleration;
        ctwk_player.translation_max_speed[6] = ctwk_player.translation_max_speed[6]*max_acceleration;
        ctwk_player.translation_max_speed[7] = ctwk_player.translation_max_speed[7]*max_acceleration;
    }

    if ctwk_config.space_jump_impulse.is_some() {
        ctwk_player.double_jump_impulse = ctwk_player.double_jump_impulse*ctwk_config.space_jump_impulse.unwrap();
    }
    if ctwk_config.vertical_space_jump_accel.is_some() {
        ctwk_player.vertical_double_jump_accel = ctwk_player.vertical_double_jump_accel*ctwk_config.vertical_space_jump_accel.unwrap();
    }
    if ctwk_config.horizontal_space_jump_accel.is_some() {
        ctwk_player.horizontal_double_jump_accel = ctwk_player.horizontal_double_jump_accel*ctwk_config.horizontal_space_jump_accel.unwrap();
    }

    if ctwk_config.allowed_jump_time.is_some() {
        ctwk_player.allowed_jump_time = ctwk_player.allowed_jump_time*ctwk_config.allowed_jump_time.unwrap();
    }
    if ctwk_config.allowed_space_jump_time.is_some() {
        ctwk_player.allowed_double_jump_time = ctwk_player.allowed_double_jump_time*ctwk_config.allowed_space_jump_time.unwrap();
    }
    if ctwk_config.min_space_jump_window.is_some() {
        ctwk_player.min_double_jump_window = ctwk_player.min_double_jump_window*ctwk_config.min_space_jump_window.unwrap();
    }
    if ctwk_config.max_space_jump_window.is_some() {
        ctwk_player.max_double_jump_window = ctwk_player.max_double_jump_window*ctwk_config.max_space_jump_window.unwrap();
    }
    if ctwk_config.min_jump_time.is_some() {
        ctwk_player.min_jump_time = ctwk_player.min_jump_time*ctwk_config.min_jump_time.unwrap();
    }
    if ctwk_config.min_space_jump_time.is_some() {
        ctwk_player.min_double_jump_time = ctwk_player.min_double_jump_time*ctwk_config.min_space_jump_time.unwrap();
    }
    if ctwk_config.falling_space_jump.is_some() {
        ctwk_player.falling_double_jump = {
            if ctwk_config.falling_space_jump.unwrap() {
                1
            } else {
                0
            }
        };
    }
    if ctwk_config.impulse_space_jump.is_some() {
        ctwk_player.impulse_double_jump = {
            if ctwk_config.impulse_space_jump.unwrap() {
                1
            } else {
                0
            }
        };
    }

    if ctwk_config.eye_offset.is_some() {
        ctwk_player.eye_offset = ctwk_player.eye_offset*ctwk_config.eye_offset.unwrap();
    }

    if ctwk_config.turn_speed.is_some() {
        let turn_speed = ctwk_config.turn_speed.unwrap();
        ctwk_player.turn_speed_multiplier = ctwk_player.turn_speed_multiplier*turn_speed;
        ctwk_player.free_look_turn_speed_multiplier = ctwk_player.free_look_turn_speed_multiplier*turn_speed;
        // there might be others
    }

    Ok(())
}

fn patch_ctwk_player_gun(res: &mut structs::Resource, ctwk_config: &CtwkConfig)
-> Result<(), String>
{
    let mut ctwk = res.kind.as_ctwk_mut().unwrap();
    let ctwk_player_gun = match &mut ctwk {
        structs::Ctwk::CtwkPlayerGun(i) => i,
        _ => panic!("Failed to map res=0x{:X} as CtwkPlayerGun", res.file_id),
    };

    if ctwk_config.gun_position.is_some() {
        let gun_position = ctwk_config.gun_position.unwrap();
        ctwk_player_gun.gun_position[0] = ctwk_player_gun.gun_position[0] + gun_position[0];
        ctwk_player_gun.gun_position[1] = ctwk_player_gun.gun_position[1] + gun_position[1];
        ctwk_player_gun.gun_position[2] = ctwk_player_gun.gun_position[2] + gun_position[2];
    }

    if ctwk_config.gun_damage.is_some() {
        let gun_damage = ctwk_config.gun_damage.unwrap();
        ctwk_player_gun.missile.damage = ctwk_player_gun.missile.damage*gun_damage;
        for i in 0..ctwk_player_gun.beams.len() {
            ctwk_player_gun.beams[i].normal.damage = ctwk_player_gun.beams[i].normal.damage*gun_damage;
            ctwk_player_gun.beams[i].charged.damage = ctwk_player_gun.beams[i].charged.damage*gun_damage;
            ctwk_player_gun.combos[i].damage = ctwk_player_gun.combos[i].damage*gun_damage;
        }
    }

    if ctwk_config.gun_cooldown.is_some() {
        let gun_cooldown = ctwk_config.gun_cooldown.unwrap();
        for i in 0..ctwk_player_gun.beams.len() {
            ctwk_player_gun.beams[i].cool_down = ctwk_player_gun.beams[i].cool_down*gun_cooldown;
        }
    }
    Ok(())
}

fn patch_ctwk_ball(res: &mut structs::Resource, ctwk_config: &CtwkConfig)
-> Result<(), String>
{
    let mut ctwk = res.kind.as_ctwk_mut().unwrap();

    let ctwk_ball = match &mut ctwk {
        structs::Ctwk::CtwkBall(i) => i,
        _ => panic!("Failed to map res=0x{:X} as CtwkBall", res.file_id),
    };

    if ctwk_config.max_translation_accel.is_some() {
        ctwk_ball.max_translation_accel[0] = ctwk_ball.max_translation_accel[0]*ctwk_config.max_translation_accel.unwrap();
        ctwk_ball.max_translation_accel[1] = ctwk_ball.max_translation_accel[1]*ctwk_config.max_translation_accel.unwrap();
        ctwk_ball.max_translation_accel[2] = ctwk_ball.max_translation_accel[2]*ctwk_config.max_translation_accel.unwrap();
        ctwk_ball.max_translation_accel[3] = ctwk_ball.max_translation_accel[3]*ctwk_config.max_translation_accel.unwrap();
        ctwk_ball.max_translation_accel[4] = ctwk_ball.max_translation_accel[4]*ctwk_config.max_translation_accel.unwrap();
        ctwk_ball.max_translation_accel[5] = ctwk_ball.max_translation_accel[5]*ctwk_config.max_translation_accel.unwrap();
        ctwk_ball.max_translation_accel[6] = ctwk_ball.max_translation_accel[6]*ctwk_config.max_translation_accel.unwrap();
        ctwk_ball.max_translation_accel[7] = ctwk_ball.max_translation_accel[7]*ctwk_config.max_translation_accel.unwrap();
    }
    if ctwk_config.translation_friction.is_some() {
        ctwk_ball.translation_friction[0] = ctwk_ball.translation_friction[0]*ctwk_config.translation_friction.unwrap();
        ctwk_ball.translation_friction[1] = ctwk_ball.translation_friction[1]*ctwk_config.translation_friction.unwrap();
        ctwk_ball.translation_friction[2] = ctwk_ball.translation_friction[2]*ctwk_config.translation_friction.unwrap();
        ctwk_ball.translation_friction[3] = ctwk_ball.translation_friction[3]*ctwk_config.translation_friction.unwrap();
        ctwk_ball.translation_friction[4] = ctwk_ball.translation_friction[4]*ctwk_config.translation_friction.unwrap();
        ctwk_ball.translation_friction[5] = ctwk_ball.translation_friction[5]*ctwk_config.translation_friction.unwrap();
        ctwk_ball.translation_friction[6] = ctwk_ball.translation_friction[6]*ctwk_config.translation_friction.unwrap();
        ctwk_ball.translation_friction[7] = ctwk_ball.translation_friction[7]*ctwk_config.translation_friction.unwrap();
    }
    if ctwk_config.translation_max_speed.is_some() {
        ctwk_ball.translation_max_speed[0] = ctwk_ball.translation_max_speed[0]*ctwk_config.translation_max_speed.unwrap();
        ctwk_ball.translation_max_speed[1] = ctwk_ball.translation_max_speed[1]*ctwk_config.translation_max_speed.unwrap();
        ctwk_ball.translation_max_speed[2] = ctwk_ball.translation_max_speed[2]*ctwk_config.translation_max_speed.unwrap();
        ctwk_ball.translation_max_speed[3] = ctwk_ball.translation_max_speed[3]*ctwk_config.translation_max_speed.unwrap();
        ctwk_ball.translation_max_speed[4] = ctwk_ball.translation_max_speed[4]*ctwk_config.translation_max_speed.unwrap();
        ctwk_ball.translation_max_speed[5] = ctwk_ball.translation_max_speed[5]*ctwk_config.translation_max_speed.unwrap();
        ctwk_ball.translation_max_speed[6] = ctwk_ball.translation_max_speed[6]*ctwk_config.translation_max_speed.unwrap();
        ctwk_ball.translation_max_speed[7] = ctwk_ball.translation_max_speed[7]*ctwk_config.translation_max_speed.unwrap();
    }
    if ctwk_config.ball_forward_braking_accel.is_some() {
        ctwk_ball.ball_forward_braking_accel[0] = ctwk_ball.ball_forward_braking_accel[0]*ctwk_config.ball_forward_braking_accel.unwrap();
        ctwk_ball.ball_forward_braking_accel[1] = ctwk_ball.ball_forward_braking_accel[1]*ctwk_config.ball_forward_braking_accel.unwrap();
        ctwk_ball.ball_forward_braking_accel[2] = ctwk_ball.ball_forward_braking_accel[2]*ctwk_config.ball_forward_braking_accel.unwrap();
        ctwk_ball.ball_forward_braking_accel[3] = ctwk_ball.ball_forward_braking_accel[3]*ctwk_config.ball_forward_braking_accel.unwrap();
        ctwk_ball.ball_forward_braking_accel[4] = ctwk_ball.ball_forward_braking_accel[4]*ctwk_config.ball_forward_braking_accel.unwrap();
        ctwk_ball.ball_forward_braking_accel[5] = ctwk_ball.ball_forward_braking_accel[5]*ctwk_config.ball_forward_braking_accel.unwrap();
        ctwk_ball.ball_forward_braking_accel[6] = ctwk_ball.ball_forward_braking_accel[6]*ctwk_config.ball_forward_braking_accel.unwrap();
        ctwk_ball.ball_forward_braking_accel[7] = ctwk_ball.ball_forward_braking_accel[7]*ctwk_config.ball_forward_braking_accel.unwrap();
    }
    if ctwk_config.ball_gravity.is_some() {
        ctwk_ball.ball_gravity = ctwk_ball.ball_gravity*ctwk_config.ball_gravity.unwrap();
    }
    if ctwk_config.ball_water_gravity.is_some() {
        ctwk_ball.ball_water_gravity = ctwk_ball.ball_water_gravity*ctwk_config.ball_water_gravity.unwrap();
    }
    if ctwk_config.boost_drain_time.is_some() {
        ctwk_ball.boost_drain_time = ctwk_ball.boost_drain_time*ctwk_config.boost_drain_time.unwrap();
    }
    if ctwk_config.boost_min_charge_time.is_some() {
        ctwk_ball.boost_min_charge_time = ctwk_ball.boost_min_charge_time*ctwk_config.boost_min_charge_time.unwrap();
    }
    if ctwk_config.boost_min_rel_speed_for_damage.is_some() {
        ctwk_ball.boost_min_rel_speed_for_damage = ctwk_ball.boost_min_rel_speed_for_damage*ctwk_config.boost_min_rel_speed_for_damage.unwrap();
    }
    if ctwk_config.boost_charge_time0.is_some() {
        ctwk_ball.boost_charge_time0 = ctwk_ball.boost_charge_time0*ctwk_config.boost_charge_time0.unwrap();
    }
    if ctwk_config.boost_charge_time1.is_some() {
        ctwk_ball.boost_charge_time1 = ctwk_ball.boost_charge_time1*ctwk_config.boost_charge_time1.unwrap();
    }
    if ctwk_config.boost_charge_time2.is_some() {
        ctwk_ball.boost_charge_time2 = ctwk_ball.boost_charge_time2*ctwk_config.boost_charge_time2.unwrap();
    }
    if ctwk_config.boost_incremental_speed0.is_some() {
        ctwk_ball.boost_incremental_speed0 = ctwk_ball.boost_incremental_speed0*ctwk_config.boost_incremental_speed0.unwrap();
    }
    if ctwk_config.boost_incremental_speed1.is_some() {
        ctwk_ball.boost_incremental_speed1 = ctwk_ball.boost_incremental_speed1*ctwk_config.boost_incremental_speed1.unwrap();
    }
    if ctwk_config.boost_incremental_speed2.is_some() {
        ctwk_ball.boost_incremental_speed2 = ctwk_ball.boost_incremental_speed2*ctwk_config.boost_incremental_speed2.unwrap();
    }

    Ok(())
}

fn patch_subchamber_five_nintendont_fix<'r>(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
)
-> Result<(), String>
{
    let layers = &mut area.mrea().scly_section_mut().layers.as_mut_vec();
    let trigger = layers[0].objects.as_mut_vec()
        .iter_mut()
        .find(|obj| obj.instance_id & 0x00FFFFFF == 0x000A0017)
        .unwrap();
    let trigger_data = trigger.property_data.as_trigger_mut().unwrap();
    let position = trigger_data.position.clone();
    let scale = trigger_data.scale.clone();
    trigger_data.position[1] = -265.4421;

    layers[0].objects.as_mut_vec().push(
        structs::SclyObject {
            instance_id: ps.fresh_instance_id_range.next().unwrap(),
            connections: vec![].into(),
            property_data: structs::SclyProperty::Trigger(
                Box::new(structs::Trigger {
                    name: b"push\0".as_cstr(),
                    position,
                    scale,
                    damage_info: structs::scly_structs::DamageInfo {
                        weapon_type: 0,
                        damage: 0.0,
                        radius: 0.0,
                        knockback_power: 0.0
                    },
                    force: [0.0, -1000.0, 0.0].into(),
                    flags: 0x2001, // apply force, detect player
                    active: 1,
                    deactivate_on_enter: 0,
                    deactivate_on_exit: 0,
                })
            ),
        }
    );
    Ok(())
}

fn patch_final_boss_permadeath<'r>(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
)
-> Result<(), String>
{
    let mrea_id = area.mlvl_area.mrea.to_u32().clone();
    if mrea_id == 0x1A666C55 { // lair
        let deps = vec![
            (0x12771AF0, b"CMDL"),
            (0xA6114429, b"TXTR"),
        ];
        let deps_iter = deps.iter()
            .map(|&(file_id, fourcc)| structs::Dependency {
                asset_id: file_id,
                asset_type: FourCC::from_bytes(fourcc),
            }
        );
        area.add_dependencies(game_resources, 0, deps_iter);
        area.add_dependencies(
            &game_resources, 0,
            iter::once(custom_asset_ids::WARPING_TO_OTHER_STRG.into())
        );
    }
    let layer_count = area.mrea().scly_section_mut().layers.len();
    area.add_layer(b"Disable Bosses Layer\0".as_cstr());
    if mrea_id != 0x1A666C55
    {
        area.layer_flags.flags &= !(1 << layer_count); // layer disabled by default
    }
    let layers = &mut area.mrea().scly_section_mut().layers.as_mut_vec();
    let mut objs_to_remove = Vec::<u32>::new();
    for i in 0..layer_count {
        for obj in layers[i].objects.as_mut_vec() {
            if  (
                    obj.property_data.is_actor() ||
                    obj.property_data.is_camera() ||
                    obj.property_data.is_platform() ||
                    obj.property_data.is_trigger() ||
                    obj.property_data.is_spawn_point() ||
                    obj.property_data.object_type() == 0x83
                )
                && !vec![
                    0x00050014, 0x0005000E,

                    ].contains(&obj.instance_id)
            {
                objs_to_remove.push(obj.instance_id.clone());
            }
        }
    }

    // Allocate list of ids
    let destinations = if mrea_id == 0xA7AC009B {
        vec![3858868330, 3883549607, 3886867740, 3851260989, 3847959174]
    } else {
        vec![3827358027]
    };

    let mut special_function_ids = Vec::<u32>::new();
    for _ in 0..destinations.len() {
        special_function_ids.push(ps.fresh_instance_id_range.next().unwrap());
    }

    let actor_id = ps.fresh_instance_id_range.next().unwrap();
    let trigger_id = ps.fresh_instance_id_range.next().unwrap();

    if mrea_id == 0x1A666C55 { // lair
        let essence = layers[0].objects.as_mut_vec()
            .iter_mut()
            .find(|obj| obj.instance_id & 0x00FFFFFF == 0x000B0082)
            .unwrap()
            .clone();
        layers[1].objects.as_mut_vec().push(essence.clone());
        layers[0].objects.as_mut_vec().retain(|obj| obj.instance_id & 0x00FFFFFF != 0x000B0082);
        layers[0].objects.as_mut_vec().push(structs::SclyObject {
            instance_id: actor_id,
            property_data: structs::Actor {
                name: b"actor\0".as_cstr(),
                position: [52.0, -298.0, -375.5].into(),
                rotation: [0.0, 0.0, 0.0].into(),
                scale: [1.0, 1.0, 1.0].into(),
                hitbox: [0.0, 0.0, 0.0].into(),
                scan_offset: [0.0, 0.0, 0.0].into(),
                unknown1: 1.0,
                unknown2: 0.0,
                health_info: structs::scly_structs::HealthInfo {
                    health: 5.0,
                    knockback_resistance: 1.0
                },
                damage_vulnerability: DoorType::Blue.vulnerability(),
                cmdl: ResId::<res_id::CMDL>::new(0x12771AF0),
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
                    unknown4: 1,
                    unknown5: 1.0
                },
                looping: 1,
                snow: 1,
                solid: 0,
                camera_passthrough: 0,
                active: 1,
                unknown8: 0,
                unknown9: 1.0,
                unknown10: 0,
                unknown11: 0,
                unknown12: 0,
                unknown13: 0
            }.into(),
            connections: vec![].into()
            }
        );

        let hudmemo_id = ps.fresh_instance_id_range.next().unwrap();
        let player_hint_id = ps.fresh_instance_id_range.next().unwrap();

        // Inform the player that they are about to be warped
        layers[0].objects
            .as_mut_vec()
            .push(structs::SclyObject {
               instance_id: hudmemo_id,
               property_data: structs::HudMemo {
                    name: b"Warping hudmemo\0".as_cstr(),

                    first_message_timer: 6.5,
                    unknown: 1,
                    memo_type: 0,
                    strg: custom_asset_ids::WARPING_TO_OTHER_STRG,
                    active: 1,
                }.into(),
               connections: vec![].into(),
            }
        );
        // Stop the player from moving
        layers[0].objects
            .as_mut_vec()
            .push(structs::SclyObject {
               instance_id: player_hint_id,
               property_data: structs::PlayerHint {

                name: b"Warping playerhint\0".as_cstr(),

                position: [0.0, 0.0, 0.0].into(),
                rotation: [0.0, 0.0, 0.0].into(),

                unknown0: 1, // active

                inner_struct: structs::PlayerHintStruct {
                    unknowns: [
                        0,
                        0,
                        0,
                        0,
                        0,
                        1, // disable
                        1, // disable
                        1, // disable
                        1, // disable
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                    ].into(),
                }.into(),

                unknown1: 10, // priority
               }.into(),
               connections: vec![].into(),
        });
        // Warp the player when entered
        layers[0].objects.as_mut_vec().push(
            structs::SclyObject {
                instance_id: trigger_id,
                connections: vec![
                    structs::Connection {
                        state: structs::ConnectionState::ENTERED,
                        message: structs::ConnectionMsg::RESET_AND_START,
                        target_object_id: 0x000B0183, // teleport timer
                    },
                    structs::Connection {
                        state: structs::ConnectionState::ENTERED,
                        message: structs::ConnectionMsg::INCREMENT,
                        target_object_id: player_hint_id,
                    },
                    structs::Connection {
                        state: structs::ConnectionState::ENTERED,
                        message: structs::ConnectionMsg::SET_TO_ZERO,
                        target_object_id: hudmemo_id,
                    },
                    structs::Connection {
                        state: structs::ConnectionState::ENTERED,
                        message: structs::ConnectionMsg::DECREMENT,
                        target_object_id: special_function_ids[0],
                    },
                ].into(),
                property_data: structs::SclyProperty::Trigger(
                    Box::new(structs::Trigger {
                        name: b"warp\0".as_cstr(),
                        position: [52.0, -298.0, -373.0].into(),
                        scale: [3.0, 3.0, 6.0].into(),
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

        // unload the previous room when entered
        layers[0].objects.as_mut_vec().push(
            structs::SclyObject {
                instance_id: ps.fresh_instance_id_range.next().unwrap(),
                connections: vec![
                    structs::Connection {
                        state: structs::ConnectionState::INSIDE,
                        message: structs::ConnectionMsg::SET_TO_ZERO,
                        target_object_id: 0x000B0173, // dock
                    },
                ].into(),
                property_data: structs::SclyProperty::Trigger(
                    Box::new(structs::Trigger {
                        name: b"unload subchamber five\0".as_cstr(),
                        position: [44.219898, -286.196686, -350.0].into(),
                        scale: [100.0, 100.0, 130.0].into(),
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

        // Deactivate warp while essence is alive
        layers[1].objects.as_mut_vec().push(
            structs::SclyObject {
                instance_id: ps.fresh_instance_id_range.next().unwrap(),
                property_data: structs::Timer {
                    name: b"remove warp\0".as_cstr(),
                    start_time: 0.1,
                    max_random_add: 0.0,
                    reset_to_zero: 0,
                    start_immediately: 1,
                    active: 1,
                }.into(),
                connections: vec![
                    structs::Connection {
                        message: structs::ConnectionMsg::DEACTIVATE,
                        state: structs::ConnectionState::ZERO,
                        target_object_id: actor_id,
                    },
                    structs::Connection {
                        message: structs::ConnectionMsg::DEACTIVATE,
                        state: structs::ConnectionState::ZERO,
                        target_object_id: trigger_id,
                    },
                ].into(),
            }
        );
    }

    let mut connections = Vec::new();
    for target_object_id in objs_to_remove {
        if target_object_id == 0x0006003D {
            connections.push(structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::ACTIVATE,
                target_object_id,
            });
        } else {
            connections.push(structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::DEACTIVATE,
                target_object_id,
            });
        }
    }

    // if mrea_id != 0x1A666C55
    {
        layers[1].objects.as_mut_vec().push(
            structs::SclyObject {
                instance_id: ps.fresh_instance_id_range.next().unwrap(),
                property_data: structs::Timer {
                    name: b"remove boss\0".as_cstr(),
                    start_time: 0.1,
                    max_random_add: 0.0,
                    reset_to_zero: 0,
                    start_immediately: 1,
                    active: 1,
                }.into(),
                connections: connections.into(),
            }
        );
    }

    // Boss deaths
    if mrea_id == 0xA7AC009B
    || mrea_id == 0x1A666C55
    {
        // Add special functions
        for i in 0..destinations.len() {
            layers[0].objects.as_mut_vec().push(
                structs::SclyObject {
                    instance_id: special_function_ids[i],
                    property_data: structs::SpecialFunction::layer_change_fn(
                        b"SpecialFunction - Bosses Stay Dead\0".as_cstr(),
                        destinations[i],
                        1,
                    ).into(),
                    connections: vec![].into(),
                }
            );
        }

        // Add connections to post-death relay
        for layer_idx in 0..layer_count {
            for obj in layers[layer_idx].objects.as_mut_vec() {
                if
                       obj.instance_id & 0x0000FFFF == 0x00000022
                    || obj.property_data.object_type() == 0x83
                    || obj.instance_id & 0x00FFFFFF == 0x000B00EC
                    || obj.instance_id & 0x00FFFFFF == 0x000B01B0
                    || obj.instance_id & 0x00FFFFFF == 0x000B01B6
                { // post-death relay
                    for i in 0..destinations.len() {
                        let (state, message) = if (mrea_id == 0x1A666C55) && i == 0 { // lair->lair
                            (structs::ConnectionState::ZERO, structs::ConnectionMsg::DECREMENT)
                        } else {
                            (structs::ConnectionState::ZERO, structs::ConnectionMsg::INCREMENT)
                        };

                        obj.connections.as_mut_vec().push(structs::Connection {
                            state,
                            message,
                            target_object_id: special_function_ids[i],
                        });
                    }
                }
            }
        }

        let mut _connections = Vec::new();
        for i in 0..destinations.len() {
            let (state, message) = if (mrea_id == 0x1A666C55) && (i == 0) { // lair->lair
                (structs::ConnectionState::ZERO, structs::ConnectionMsg::DECREMENT)
            } else {
                (structs::ConnectionState::ZERO, structs::ConnectionMsg::INCREMENT)
            };

            _connections.push(structs::Connection {
                state,
                message,
                target_object_id: special_function_ids[i],
            });
        }
        layers[0].objects.as_mut_vec().push(
            structs::SclyObject {
                instance_id: ps.fresh_instance_id_range.next().unwrap(),
                property_data: structs::Timer {
                    name: b"change layer\0".as_cstr(),
                    start_time: 0.1,
                    max_random_add: 0.0,
                    reset_to_zero: 0,
                    start_immediately: 1,
                    active: 1,
                }.into(),
                connections: _connections.into(),
            }
        );
    }
    Ok(())
}

fn patch_combat_hud_color(res: &mut structs::Resource, ctwk_config: &CtwkConfig)
-> Result<(), String>
{
    if ctwk_config.hud_color.is_none() {
        return Ok(());
    }

    let mut new_color: [f32;3] = ctwk_config.hud_color.as_ref().unwrap().clone();
    let mut max_new = new_color[0];
    if new_color[1] > max_new { max_new = new_color[1]; }
    if new_color[2] > max_new { max_new = new_color[2]; }
    if max_new < 0.0001 {
        new_color = [1.0, 1.0, 1.0];
    }

    let frme = res.kind.as_frme_mut().unwrap();
    for widget in frme.widgets.as_mut_vec().iter_mut()
    {
        let old_color = widget.color.clone();
        if old_color[0]-old_color[1] > -0.1 && old_color[0]-old_color[1] < 0.1
            && old_color[0]-old_color[2] > -0.1 && old_color[0]-old_color[2] < 0.1
            && old_color[1]-old_color[2] > -0.1 && old_color[1]-old_color[2] < 0.1
        {
            continue;
        }

        let mut max_original = old_color[0];
        if old_color[1] > max_original { max_original = old_color[1]; }
        if old_color[2] > max_original { max_original = old_color[2]; }
        let scale = max_original / max_new;
        let new_color_scaled = [new_color[0]*scale, new_color[1]*scale, new_color[2]*scale, old_color[3]];
        widget.color = new_color_scaled.into();
    }

    Ok(())
}

fn patch_ctwk_gui_colors(res: &mut structs::Resource, ctwk_config: &CtwkConfig)
-> Result<(), String>
{
    let mut ctwk = res.kind.as_ctwk_mut().unwrap();
    let ctwk_gui_colors = match &mut ctwk {
        structs::Ctwk::CtwkGuiColors(i) => i,
        _ => panic!("Failed to map res=0x{:X} as CtwkGuiColors", res.file_id),
    };

    if ctwk_config.hud_color.is_some() {
        let mut new_color = ctwk_config.hud_color.unwrap();
        let mut max_new = new_color[0];
        if new_color[1] > max_new { max_new = new_color[1]; }
        if new_color[2] > max_new { max_new = new_color[2]; }
        if max_new < 0.0001 {
            new_color = [1.0, 1.0, 1.0];
        }

        for i in 0..112 {
            // Skip black/white/gray
            let old_color = ctwk_gui_colors.colors[i as usize];
            if old_color[0]-old_color[1] > -0.1 && old_color[0]-old_color[1] < 0.1
                && old_color[0]-old_color[2] > -0.1 && old_color[0]-old_color[2] < 0.1
                && old_color[1]-old_color[2] > -0.1 && old_color[1]-old_color[2] < 0.1
                && i != 10 && i != 11 // Visor/Beam menu
            {
                continue;
            }

            let mut max_original = old_color[0];
            if old_color[1] > max_original { max_original = old_color[1]; }
            if old_color[2] > max_original { max_original = old_color[2]; }
            let scale = max_original / max_new;

            // Scale new color up or down to approximate original, preserve alpha
            let mut new_color_scaled = [new_color[0]*scale, new_color[1]*scale, new_color[2]*scale, old_color[3]];
            if i == 10 || i == 11 { // beam/visor menus should be partially colored
                let diff = [
                    old_color[0] - new_color_scaled[0],
                    old_color[1] - new_color_scaled[1],
                    old_color[2] - new_color_scaled[2],
                ];
                let diff_scale = 0.65;
                new_color_scaled[0] = new_color_scaled[0] + diff[0]*diff_scale;
                new_color_scaled[1] = new_color_scaled[1] + diff[1]*diff_scale;
                new_color_scaled[2] = new_color_scaled[2] + diff[2]*diff_scale;
            } else if i == 96 || i == 97 { // critical scans should be distinguishable
                let diff = [
                    (1.0 - new_color_scaled[0]) - new_color_scaled[0],
                    (1.0 - new_color_scaled[1]) - new_color_scaled[1],
                    (1.0 - new_color_scaled[2]) - new_color_scaled[2],
                ];
                let diff_scale = 0.65;
                new_color_scaled[0] = new_color_scaled[0] + diff[0]*diff_scale;
                new_color_scaled[1] = new_color_scaled[1] + diff[1]*diff_scale;
                new_color_scaled[2] = new_color_scaled[2] + diff[2]*diff_scale;
            }
            ctwk_gui_colors.colors[i as usize] = new_color_scaled.into();
        }

        for i in 0..5 {
            let i = i as usize;
            for j in 0..7 {
                let j = j as usize;
                let old_color = ctwk_gui_colors.visor_colors[i][j];
                if old_color[0]-old_color[1] > -0.1 && old_color[0]-old_color[1] < 0.1
                    && old_color[0]-old_color[2] > -0.1 && old_color[0]-old_color[2] < 0.1
                    && old_color[1]-old_color[2] > -0.1 && old_color[1]-old_color[2] < 0.1
                {
                    continue;
                }

                let mut max_original = old_color[0];
                if old_color[1] > max_original { max_original = old_color[1]; }
                if old_color[2] > max_original { max_original = old_color[2]; }
                let scale = max_original / max_new;

                // Scale new color up or down to approximate original, preserve alpha
                ctwk_gui_colors.visor_colors[i][j] = [new_color[0]*scale, new_color[1]*scale, new_color[2]*scale, old_color[3]].into();
            }
        }
    }

    Ok(())
}

fn patch_move_item_loss_scan<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer_count = scly.layers.len();
    for i in 0..layer_count {
        let layer = &mut scly.layers.as_mut_vec()[i];
        for obj in layer.objects.as_mut_vec() {
            let mut _poi = obj.property_data.as_point_of_interest_mut();
            if _poi.is_some() {
                let poi = _poi.unwrap();
                poi.position[1] = poi.position[1] + 2.0;
            }
        }
    }

    Ok(())
}

// fn patch_remove_visor_changer<'r>(
//     _ps: &mut PatcherState,
//     area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
// )
// -> Result<(), String>
// {
//     let scly = area.mrea().scly_section_mut();
//     let layer_count = scly.layers.len();
//     for i in 0..layer_count {
//         let layer = &mut scly.layers.as_mut_vec()[i];
//         for obj in layer.objects.as_mut_vec() {
//             let mut _player_hint = obj.property_data.as_player_hint_mut();
//             if _player_hint.is_some() {
//                 let player_hint = _player_hint.unwrap();
//                 player_hint.inner_struct.unknowns[9]  = 0; // Never switch to combat visor
//                 player_hint.inner_struct.unknowns[10] = 0; // Never switch to scan visor
//                 player_hint.inner_struct.unknowns[11] = 0; // Never switch to thermal visor
//                 player_hint.inner_struct.unknowns[12] = 0; // Never switch to xray visor
//             }
//         }
//     }

//     Ok(())
// }

fn is_blast_shield<'r>(obj: &structs::SclyObject<'r>) -> bool {
    if !obj.property_data.is_actor() {
        return false;
    }
    obj.property_data.as_actor().unwrap().cmdl == 0xEFDFFB8C
}

fn is_blast_shield_poi<'r>(obj: &structs::SclyObject<'r>) -> bool {
    if !obj.property_data.is_point_of_interest() {
        return false;
    }
    obj.property_data.as_point_of_interest().unwrap().scan_param.scan.to_u32() == 0x05f56f9d
}

fn patch_remove_blast_shields<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer_count = scly.layers.len();
    for i in 0..layer_count {
        let layer = &mut scly.layers.as_mut_vec()[i];
        layer.objects.as_mut_vec().retain(|obj| !is_blast_shield(&obj) && !is_blast_shield_poi(&obj));
    }

    Ok(())
}

fn patch_remove_control_disabler<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer_count = scly.layers.len();
    for i in 0..layer_count {
        let layer = &mut scly.layers.as_mut_vec()[i];
        for obj in layer.objects.as_mut_vec() {
            let mut _player_hint = obj.property_data.as_player_hint_mut();
            if _player_hint.is_some() {
                let player_hint = _player_hint.unwrap();
                player_hint.inner_struct.unknowns[5] = 0; // always enable unmorphing
                player_hint.inner_struct.unknowns[6] = 0; // always enable morphing
                player_hint.inner_struct.unknowns[7] = 0; // always enable controls
                player_hint.inner_struct.unknowns[8] = 0; // always enable boost
            }
        }
    }

    Ok(())
}

fn patch_add_dock_teleport<'r>(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    source_position: [f32;3],
    source_scale: [f32;3],
    destination_dock_num: u32,
    dest_position: Option<[f32;3]>,
    mrea_idx: Option<u32>,
)
-> Result<(), String>
{
    let mrea_id = area.mlvl_area.mrea.to_u32();

    // Update the list of attached areas to use the new area instead of the old one
    let attached_areas: &mut reader_writer::LazyArray<'r, u16> = &mut area.mlvl_area.attached_areas;
    if mrea_idx.is_some() {
        let idx = mrea_idx.unwrap() as u16;
        attached_areas.as_mut_vec().push(idx);
        area.mlvl_area.attached_area_count += 1;
    }

    let layer = &mut area.mrea().scly_section_mut().layers.as_mut_vec()[0];
    let spawn_point_id = ps.fresh_instance_id_range.next().unwrap();

    // find the destination dock
    let mut found = false;
    let mut dock_position: GenericArray<f32, U3> = [0.0, 0.0, 0.0].into();

    if dest_position.is_some() {
        dock_position = dest_position.unwrap().into();
    } else {
        for obj in layer.objects.as_mut_vec() {
            if obj.property_data.is_dock() {
                let dock = obj.property_data.as_dock_mut().unwrap();

                // Remove all auto-loads in this room
                dock.load_connected = 0;

                // Find the specified dock
                if dock.dock_index == destination_dock_num {
                    found = true;
                    dock_position = dock.position.clone();
                }
            }
        }

        if !found {
            panic!("failed to find dock #{} in room 0x{:X}", destination_dock_num, mrea_id)
        }
    }

    // Check for vanilla door connection via proximity
    if  f32::abs(source_position[0] - dock_position[0]) < 5.0 &&
        f32::abs(source_position[1] - dock_position[1]) < 5.0 &&
        f32::abs(source_position[2] - dock_position[2]) < 5.0
    {
        return Ok(()); // No teleport needed
    }

    // Find the nearest door
    let mut is_frigate_door = false;
    let mut is_ceiling_door = false;
    let mut is_floor_door = false;
    let mut is_square_frigate_door = false;
    let mut is_morphball_door = false;
    let mut door_id: u32 = 0;

    let mut door_rotation: GenericArray<f32, U3> = [0.0, 0.0, 0.0].into();
    let mut disable_ids: Vec<u32> = vec![];
    for obj in layer.objects.as_mut_vec() {
        if !obj.property_data.is_door() {
            continue;
        }

        let door = obj.property_data.as_door().unwrap();
        if  f32::abs(door.position[0] - dock_position[0]) > 5.0 ||
            f32::abs(door.position[1] - dock_position[1]) > 5.0 ||
            f32::abs(door.position[2] - dock_position[2]) > 5.0 {
            continue;
        }

        door_id = obj.instance_id;
        for conn in obj.connections.as_mut_vec().iter() {
            if conn.state == structs::ConnectionState::MAX_REACHED && conn.message == structs::ConnectionMsg::DEACTIVATE {
                disable_ids.push(conn.target_object_id);
            }
        }

        door_rotation = door.rotation.clone();
        is_frigate_door = door.ancs.file_id == 0xfafb5784;
        is_ceiling_door = door.ancs.file_id == 0xf57dd484 && door_rotation[0] > -90.0 && door_rotation[0] < 90.0;
        is_floor_door = door.ancs.file_id == 0xf57dd484 && door_rotation[0] < -90.0 && door_rotation[0] > -270.0;
        is_square_frigate_door = door.ancs.file_id == 0x26CCCB48;
        is_morphball_door = door.is_morphball_door != 0;
    }

    if mrea_id == 0xC9D52BBC && destination_dock_num == 0 { // energy core
        is_morphball_door = true; // it's technically not actually a morph ball door
    }

    let mut spawn_point_position = dock_position.clone();
    let mut spawn_point_rotation = [0.0, 0.0, 0.0];
    let mut door_offset = 3.0;
    let mut vertical_offset = -2.0;

    if is_frigate_door {
        door_offset = -3.0;
        vertical_offset = -2.0;
        spawn_point_rotation[2] = 180.0;
    } else if is_ceiling_door {
        door_offset = 0.0;
        vertical_offset = -5.0;
    } else if is_floor_door {
        door_offset = 2.5;
        vertical_offset = 1.5;
    } else if is_square_frigate_door {
        spawn_point_rotation[2] += 90.0;
    } else if is_morphball_door {
        vertical_offset = 0.0;
        door_offset = 4.0;
    }

    if mrea_id == 0xF5EF1862 && is_morphball_door { // fiery shores0 
        vertical_offset = -5.0;
        door_offset = 0.0;
    }

    if mrea_id == 0x89A6CB8D && is_morphball_door { // warrior shrine
        vertical_offset = 3.0;
        door_offset = 2.0;
        is_morphball_door = false;
    }

    if (mrea_id == 0xB4FBBEF5 || mrea_id == 0x86EB2E02) && is_morphball_door { // life grove + tunnel
        vertical_offset = -1.5;
    }

    if mrea_id == 0x3F04F304 && is_morphball_door { // training chamber
        door_offset = 2.0;
    }

    if mrea_id == 0x2B3F1CEE { // piston tunnel
        door_offset = 2.0;
        vertical_offset = -1.0;
    }

    if door_rotation[2] >= 45.0 && door_rotation[2] < 135.0 {
        // Leads North (Y+)
        spawn_point_position[1] = spawn_point_position[1] - door_offset;
        spawn_point_rotation[2] += 180.0;
    } else if (door_rotation[2] >= 135.0 && door_rotation[2] < 225.0) || (door_rotation[2] < -135.0 && door_rotation[2] > -225.0) {
        // Leads East (X+)
        spawn_point_position[0] = spawn_point_position[0] + door_offset;
        spawn_point_rotation[2] += 270.0;
    } else if door_rotation[2] >= -135.0 && door_rotation[2] < -45.0 {
        // Leads South (Y-)
        spawn_point_position[1] = spawn_point_position[1] + door_offset;
        spawn_point_rotation[2] += 0.0;
    } else if door_rotation[2] >= -45.0 && door_rotation[2] < 45.0 {
        // Leads West (X-)
        spawn_point_position[0] = spawn_point_position[0] - door_offset;
        spawn_point_rotation[2] += 90.0;
    }
    spawn_point_position[2] = spawn_point_position[2] + vertical_offset;

    // Insert a camera hint trigger to prevent the camera from getting slammed into the wall of the departure room
    // except for LGT because it already has a trigger and training chamber because it's goofy
    if is_morphball_door && mrea_id != 0xB4FBBEF5 && mrea_id != 0x3F04F304 {
        let camear_hint_id = ps.fresh_instance_id_range.next().unwrap();
        layer.objects.as_mut_vec().push(
            structs::SclyObject {
                instance_id: camear_hint_id,
                connections: vec![].into(),
                property_data: structs::SclyProperty::CameraHint(
                    Box::new(structs::CameraHint {
                        name: b"CameraHint\0".as_cstr(),
                        position: spawn_point_position.into(),
                        rotation: spawn_point_rotation.into(),
                        active: 1,
                        priority: 8,
                        behavior: 2,
                        camera_hint_params: structs::CameraHintParameters {
                            calculate_cam_pos: 0,
                            chase_allowed: 0,
                            boost_allowed: 0,
                            obscure_avoidance: 0,
                            volume_collider: 0,
                            apply_immediately: 1,
                            look_at_ball: 0,
                            hint_distance_selection: 0,
                            hint_distance_self_pos: 0,
                            control_interpolation: 1,
                            sinusoidal_interpolation: 0,
                            sinusoidal_interpolation_hintless: 0,
                            clamp_velocity: 0,
                            skip_cinematic: 0,
                            no_elevation_interp: 0,
                            direct_elevation: 0,
                            override_look_dir: 0,
                            no_elevation_vel_clamp: 0,
                            calculate_transform_from_prev_cam: 0,
                            no_spline: 0,
                            unknown21: 0,
                            unknown22: 0,
                        },
                        min_dist: structs::BoolFloat {
                            active: 0,
                            value: 6.0,
                        },
                        max_dist: structs::BoolFloat {
                            active: 0,
                            value: 6.0,
                        },
                        backwards_dist: structs::BoolFloat {
                            active: 0,
                            value: 6.0,
                        },
                        look_at_offset: structs::BoolVec3 {
                            active: 0,
                            value: [0.0, 1.0, 1.0].into(),
                        },
                        chase_look_at_offset: structs::BoolVec3 {
                            active: 0,
                            value: [0.0, 1.0, 1.0].into(),
                        },
                        ball_to_cam: [1.0, 1.0, 1.0].into(),
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

        layer.objects.as_mut_vec().push(
            structs::SclyObject {
                instance_id: ps.fresh_instance_id_range.next().unwrap(),
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
                        position: spawn_point_position.into(),
                        scale: [4.0, 4.0, 3.0].into(),
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

        // layer.objects.as_mut_vec().push(
        //     structs::SclyObject {
        //         instance_id: ps.fresh_instance_id_range.next().unwrap(),
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
    }

    // Insert a spawn point in-bounds
    layer.objects.as_mut_vec().push(
        structs::SclyObject {
            instance_id: spawn_point_id,
            connections: vec![].into(),
            property_data: structs::SclyProperty::SpawnPoint(
                Box::new(structs::SpawnPoint {
                    name: b"dockspawnpoint\0".as_cstr(),
                    position: spawn_point_position.into(),
                    rotation: spawn_point_rotation.into(),
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
                    default_spawn: 0,
                    active: 1,
                    morphed: is_morphball_door as u8,
                })
            ),
        }
    );

    // Thin out the trigger so that you can't touch it through the door
    let mut thinnest = 0;
    if source_scale[1] < source_scale[thinnest] {
        thinnest = 1;
    }
    if source_scale[2] < source_scale[thinnest] {
        thinnest = 2;
    }
    let mut source_scale = source_scale.clone();
    source_scale[thinnest] = 0.1;

    let timer_id = ps.fresh_instance_id_range.next().unwrap();

    // Insert a trigger at the previous room which sends the player to the freshly created spawn point
    let mut connections: Vec::<structs::Connection> = Vec::new();
    if door_id != 0 {
        connections.push(
            structs::Connection {
                state: structs::ConnectionState::ENTERED,
                message: structs::ConnectionMsg::RESET_AND_START,
                target_object_id: timer_id,
            }
        );
    }
    connections.push(
        structs::Connection {
            state: structs::ConnectionState::ENTERED,
            message: structs::ConnectionMsg::SET_TO_ZERO,
            target_object_id: spawn_point_id as u32,
        }
    );

    layer.objects.as_mut_vec().push(
        structs::SclyObject {
            instance_id: ps.fresh_instance_id_range.next().unwrap(),
            connections: connections.into(),
            property_data: structs::SclyProperty::Trigger(
                Box::new(structs::Trigger {
                    name: b"dockteleporttrigger\0".as_cstr(),
                    position: source_position.into(),
                    scale: source_scale.into(),
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

    // Open the door when arriving into the room
    let mut connections: Vec::<structs::Connection> = vec![
        structs::Connection {
            state: structs::ConnectionState::ZERO,
            message: structs::ConnectionMsg::OPEN,
            target_object_id: door_id,
        },
    ];
    for id in disable_ids.iter() {
        connections.push(
            structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::DEACTIVATE,
                target_object_id: *id,
            }
        );
    }
    layer.objects.as_mut_vec().push(structs::SclyObject {
        instance_id: timer_id,
        property_data: structs::Timer {
            name: b"open-door-timer\0".as_cstr(),
            start_time: 0.1,
            max_random_add: 0.0,
            reset_to_zero: 0,
            start_immediately: 0,
            active: 1,
        }.into(),
        connections: connections.into(),
    });

    Ok(())
}

fn patch_modify_dock<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    scan_id: ResId<res_id::SCAN>,
    strg_id: ResId<res_id::STRG>,
    dock_num: u32,
    new_mrea_idx: u32,
)
-> Result<(), String>
{
    // Add dependencies for scan point
    let frme_id = ResId::<res_id::FRME>::new(0xDCEC3E77);
    let scan_dep: structs::Dependency = scan_id.into();
    area.add_dependencies(game_resources, 0, iter::once(scan_dep));
    let strg_dep: structs::Dependency = strg_id.into();
    area.add_dependencies(game_resources, 0, iter::once(strg_dep));
    let frme_dep: structs::Dependency = frme_id.into();
    area.add_dependencies(game_resources, 0, iter::once(frme_dep));

    let mrea_id = area.mlvl_area.mrea.to_u32();
    let attached_areas: &mut reader_writer::LazyArray<'r, u16> = &mut area.mlvl_area.attached_areas;
    let docks: &mut reader_writer::LazyArray<'r, structs::mlvl::Dock<'r>> = &mut area.mlvl_area.docks;

    if dock_num >= attached_areas.as_mut_vec().len() as u32
    {
        panic!("dock num #{} doesn't index attached areas in room 0x{:X}", dock_num, mrea_id);
    }

    if dock_num >= docks.as_mut_vec().len() as u32
    {
        panic!("dock num #{} doesn't index docks in room 0x{:X}", dock_num, mrea_id);
    }

    docks.as_mut_vec()[dock_num as usize].connecting_docks.as_mut_vec()[0].array_index = new_mrea_idx;
    attached_areas.as_mut_vec().push(new_mrea_idx as u16);
    area.mlvl_area.attached_area_count += 1;

    let layer = &mut area.mrea().scly_section_mut().layers.as_mut_vec()[0];

    // Find the dock script object(s)
    let mut docks: Vec<u32> = Vec::new();
    let mut other_docks: Vec<u32> = Vec::new();
    for obj in layer.objects.as_mut_vec() {
        if obj.property_data.is_dock() {
            let dock = obj.property_data.as_dock_mut().unwrap();
            if dock.dock_index == dock_num {
                docks.push(obj.instance_id&0x000FFFFF);
            } else {
                other_docks.push(obj.instance_id);
            }
        }
    }

    // Edit the door corresponding to this dock
    let mut door_id = 0;
    for obj in layer.objects.as_mut_vec() {
        if !obj.property_data.is_door() {
            continue;
        }

        for conn in obj.connections.as_mut_vec() {
            if docks.contains(&(conn.target_object_id&0x000FFFFF)) && conn.message == structs::ConnectionMsg::INCREMENT {
                door_id = obj.instance_id;

                let door = obj.property_data.as_door_mut().unwrap();
                let is_ceiling_door = door.ancs.file_id == 0xf57dd484 && door.rotation[0] > -90.0 && door.rotation[0] < 90.0;
                let is_floor_door = door.ancs.file_id == 0xf57dd484 && door.rotation[0] < -90.0 && door.rotation[0] > -270.0;
                let is_morphball_door = door.is_morphball_door != 0;

                if is_ceiling_door {
                    door.scan_offset[0] = 0.0;
                    door.scan_offset[1] = 0.0;
                    door.scan_offset[2] = -2.5;
                }
                else if is_floor_door {
                    door.scan_offset[0] = 0.0;
                    door.scan_offset[1] = 0.0;
                    door.scan_offset[2] = 2.5;
                } else if is_morphball_door {
                    door.scan_offset[0] = 0.0;
                    door.scan_offset[1] = 0.0;
                    door.scan_offset[2] = 1.0;
                }

                door.actor_params.scan_params.scan = scan_id;
                break;
            }
        }
    }

    if door_id == 0 {
        panic!("Failed to find door corresponding to patched dock in 0x{:X}", mrea_id);
    }

    // Remove autoloads from this room
    // let mut autoload_room = false;
    for obj in layer.objects.as_mut_vec() {
        if !obj.property_data.is_dock() {
            continue;
        }

        // remove all auto-loads in this room
        let dock = obj.property_data.as_dock_mut().unwrap();
        if dock.load_connected != 0 {
            dock.load_connected = 0;
        }
    }

    // In the event this room was an autoload, we need to rectify this room to properly ensure things are unloaded
    // when bouncing back and forth between doors like a maniac
    {
        let mut dock_has_loader = false;
        for obj in layer.objects.as_mut_vec() {
            for conn in obj.connections.as_mut_vec() {
                if docks.contains(&(conn.target_object_id&0x00FFFFFF)) && conn.message == structs::ConnectionMsg::SET_TO_MAX {
                    dock_has_loader = true;
                    break;
                }
            }
            if dock_has_loader {
                break;
            }
        }

        if !dock_has_loader {
            // find door unlock trigger
            let mut trigger_pos = [0.0, 0.0, 0.0];
            let mut trigger_scale = [0.0, 0.0, 0.0];
            for obj in layer.objects.as_mut_vec() {
                if !obj.property_data.is_trigger() {
                    continue;
                }

                let mut is_the_trigger = false;
                for conn in obj.connections.as_mut_vec() {
                    if conn.target_object_id&0x00FFFFFF == door_id&0x00FFFFFF {
                        is_the_trigger = true;
                        break;
                    }
                }

                if !is_the_trigger {
                    continue;
                }

                let trigger = obj.property_data.as_trigger_mut().unwrap();
                trigger_pos = trigger.position.into();
                trigger_scale = trigger.scale.into();

                break;
            }

            // If we couldn't find the door open trigger, then just give up and hope for the best (e.g. storage cave)
            if trigger_pos == trigger_scale {
                return Ok(());
            }

            // unload everything upon touching
            let mut connections: Vec::<structs::Connection> = Vec::new();
            for dock in other_docks {
                connections.push(
                    structs::Connection {
                        state: structs::ConnectionState::ENTERED,
                        message: structs::ConnectionMsg::SET_TO_ZERO,
                        target_object_id: dock,
                    },
                );
            }
            connections.push(
                structs::Connection {
                    state: structs::ConnectionState::INSIDE,
                    message: structs::ConnectionMsg::SET_TO_MAX,
                    target_object_id: docks[0],
                },
            );
            layer.objects.as_mut_vec().push(
                structs::SclyObject {
                    instance_id: _ps.fresh_instance_id_range.next().unwrap(),
                    property_data: structs::Trigger {
                        name: b"Trigger\0".as_cstr(),
                        position: trigger_pos.into(),
                        scale: [
                            trigger_scale[0] + 7.0,
                            trigger_scale[1] + 7.0,
                            trigger_scale[2] + 7.0,
                        ].into(),
                        damage_info: structs::scly_structs::DamageInfo {
                            weapon_type: 0,
                            damage: 0.0,
                            radius: 0.0,
                            knockback_power: 0.0
                        },
                        force: [0.0, 0.0, 0.0].into(),
                        flags: 0x1001, // detect morphed+player
                        active: 1,
                        deactivate_on_enter: 0,
                        deactivate_on_exit: 0
                    }.into(),
                    connections: connections.into(),
                }
            );
        }
    }

    Ok(())
}

fn patch_exo_scale<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    scale: f32,
)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec().iter_mut() {
        for obj in layer.objects.as_mut_vec().iter_mut() {
            if obj.property_data.is_metroidprimestage1() {
                let boss = obj.property_data.as_metroidprimestage1_mut().unwrap();
                boss.scale[0] *= scale;
                boss.scale[1] *= scale;
                boss.scale[2] *= scale;
            } else if obj.property_data.is_actor() && vec![
                0x00050090,
                0x00050002,
                0x00050076,
                0x0005008F,
            ].contains(&(obj.instance_id & 0x00FFFFFF)) {
                let boss = obj.property_data.as_actor_mut().unwrap();
                boss.scale[0] *= scale;
                boss.scale[1] *= scale;
                boss.scale[2] *= scale;
            }
        }
    }
    Ok(())
}

fn patch_ridley_scale<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    scale: f32,
)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec().iter_mut() {
        for obj in layer.objects.as_mut_vec().iter_mut() {
            if obj.property_data.is_ridley() {
                let boss = obj.property_data.as_ridley_mut().unwrap();
                boss.scale[0] *= scale;
                boss.scale[1] *= scale;
                boss.scale[2] *= scale;
            } else if obj.property_data.is_actor() && vec![
                0x00100218,
                0x00100222,
                0x001003D6,
                0x0010028C,
                0x00100472,
                0x00100377,
                0x001003C3,
                0x001003E1,
            ].contains(&(obj.instance_id & 0x00FFFFFF)) {
                let boss = obj.property_data.as_actor_mut().unwrap();
                boss.scale[0] *= scale;
                boss.scale[1] *= scale;
                boss.scale[2] *= scale;
            }
        }
    }

    Ok(())
}

fn patch_omega_pirate_scale<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    scale: f32,
)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec().iter_mut() {
        for obj in layer.objects.as_mut_vec().iter_mut() {
            if obj.property_data.is_omega_pirate() {
                let boss = obj.property_data.as_omega_pirate_mut().unwrap();
                boss.scale[0] *= scale;
                boss.scale[1] *= scale;
                boss.scale[2] *= scale;
                continue;
            }

            if obj.property_data.is_platform() {
                let boss = obj.property_data.as_platform_mut().unwrap();
                if !boss.name.to_str().ok().unwrap().to_string().to_lowercase().contains("armor") {
                    continue;
                }
                boss.scale[0] *= scale;
                boss.scale[1] *= scale;
                boss.scale[2] *= scale;
                continue;
            }

            if obj.property_data.is_actor() {
                let boss = obj.property_data.as_actor_mut().unwrap();
                if !boss.name.to_str().ok().unwrap().to_string().to_lowercase().contains("omega") {
                    continue;
                }
                boss.scale[0] *= scale;
                boss.scale[1] *= scale;
                boss.scale[2] *= scale;
                continue;
            }

            if obj.property_data.is_effect() {
                let boss = obj.property_data.as_effect_mut().unwrap();
                if !boss.name.to_str().ok().unwrap().to_string().to_lowercase().contains("armor") {
                    continue;
                }
                boss.scale[0] *= scale;
                boss.scale[1] *= scale;
                boss.scale[2] *= scale;
                continue;
            }
        }
    }

    Ok(())
}


fn patch_elite_pirate_scale<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    scale: f32,
)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec().iter_mut() {
        for obj in layer.objects.as_mut_vec().iter_mut() {
            if obj.property_data.is_elite_pirate() {
                let boss = obj.property_data.as_elite_pirate_mut().unwrap();
                boss.scale[0] *= scale;
                boss.scale[1] *= scale;
                boss.scale[2] *= scale;
            } else if obj.property_data.is_actor() && vec![
                0x00180126,
                0x001401C3,
                0x001401C4,
                0x00140385,
                0x00100337,
                0x000D03FA,
                0x000D01A7,
                0x0010036A,
            ].contains(&(obj.instance_id & 0x00FFFFFF)) {
                let boss = obj.property_data.as_actor_mut().unwrap();
                boss.scale[0] *= scale;
                boss.scale[1] *= scale;
                boss.scale[2] *= scale;
            }
        }
    }

    Ok(())
}


fn patch_sheegoth_scale<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    scale: f32,
)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec().iter_mut() {
        for obj in layer.objects.as_mut_vec().iter_mut() {
            if !obj.property_data.is_ice_sheegoth() { continue; }
            let boss = obj.property_data.as_ice_sheegoth_mut().unwrap();
            boss.scale[0] *= scale;
            boss.scale[1] *= scale;
            boss.scale[2] *= scale;
        }
    }

    Ok(())
}


fn patch_flaahgra_scale<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    scale: f32,
)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec().iter_mut() {
        for obj in layer.objects.as_mut_vec().iter_mut() {
            if !obj.property_data.is_flaahgra() { continue; }
            let boss = obj.property_data.as_flaahgra_mut().unwrap();
            boss.scale[0] *= scale;
            boss.scale[1] *= scale;
            boss.scale[2] *= scale;
            boss.dont_care[1] *= scale;
        }
    }

    Ok(())
}


fn patch_idrone_scale<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    scale: f32,
)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec().iter_mut() {
        for obj in layer.objects.as_mut_vec().iter_mut() {
            if !obj.property_data.is_actor_contraption() { continue; }
            let boss = obj.property_data.as_actor_contraption_mut().unwrap();
            boss.scale[0] *= scale;
            boss.scale[1] *= scale;
            boss.scale[2] *= scale;
        }
    }

    Ok(())
}

fn patch_pq_scale<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    scale: f32,
)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec().iter_mut() {
        for obj in layer.objects.as_mut_vec().iter_mut() {
            if obj.property_data.is_new_intro_boss() {
                let boss = obj.property_data.as_new_intro_boss_mut().unwrap();
                boss.scale[0] *= scale;
                boss.scale[1] *= scale;
                boss.scale[2] *= scale;
            }
            else if obj.property_data.is_actor() && vec![
                0x0019006C,
            ].contains(&(obj.instance_id & 0x00FFFFFF)) {
                let boss = obj.property_data.as_actor_mut().unwrap();
                boss.scale[0] *= scale;
                boss.scale[1] *= scale;
                boss.scale[2] *= scale;
            }
        }
    }

    Ok(())
}

fn patch_thardus_scale<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    scale: f32,
)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec().iter_mut() {
        for obj in layer.objects.as_mut_vec().iter_mut() {
            if obj.property_data.is_thardus() {
                let boss = obj.property_data.as_thardus_mut().unwrap();
                boss.scale[0] *= scale;
                boss.scale[1] *= scale;
                boss.scale[2] *= scale;
            }
            else if obj.property_data.is_platform() && vec![
                // 0x00180212, platform didn't scale right, so not doing this
            ].contains(&(obj.instance_id & 0x00FFFFFF)) {
                let boss = obj.property_data.as_platform_mut().unwrap();
                boss.scale[0] *= scale;
                boss.scale[1] *= scale;
                boss.scale[2] *= scale;
            }
        }
    }

    Ok(())
}

fn patch_essence_scale<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    scale: f32,
)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec().iter_mut() {
        for obj in layer.objects.as_mut_vec().iter_mut() {
            if obj.property_data.is_metroidprimestage2() {
                let boss = obj.property_data.as_metroidprimestage2_mut().unwrap();
                boss.scale[0] *= scale;
                boss.scale[1] *= scale;
                boss.scale[2] *= scale;
            }
            else if obj.property_data.is_actor() && vec![
                0x000B00F4,
                0x000B0101,
                0x000B012B,
                0x000B00EE,
                0x000B00D2,
                0x000B009F,
                0x000B0121,
                0x000B015D,
                0x000B0162,
                0x000B0163,
                0x000B0168,
                0x000B0195,
            ].contains(&(obj.instance_id & 0x00FFFFFF)) {
                let boss = obj.property_data.as_actor_mut().unwrap();
                boss.scale[0] *= scale;
                boss.scale[1] *= scale;
                boss.scale[2] *= scale;
            }
        }
    }

    Ok(())
}

fn patch_drone_scale<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    scale: f32,
)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec().iter_mut() {
        for obj in layer.objects.as_mut_vec().iter_mut() {
            if !obj.property_data.is_drone() { continue; }
            let boss = obj.property_data.as_drone_mut().unwrap();
            boss.scale[0] *= scale;
            boss.scale[1] *= scale;
            boss.scale[2] *= scale;
        }
    }

    Ok(())
}

fn patch_garbeetle_scale<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    scale: f32,
)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec().iter_mut() {
        for obj in layer.objects.as_mut_vec().iter_mut() {
            if !obj.property_data.is_beetle() { continue; }
            let boss = obj.property_data.as_beetle_mut().unwrap();
            if !boss.name.to_str().unwrap().to_lowercase().contains(&"garbeetle") {
                continue;
            }
            boss.scale[0] *= scale;
            boss.scale[1] *= scale;
            boss.scale[2] *= scale;
        }
    }

    Ok(())
}

fn patch_bnr(
    file: &mut structs::FstEntryFile,
    banner: &GameBanner,
)
    -> Result<(), String>
{
    let bnr = match file {
        structs::FstEntryFile::Bnr(bnr) => bnr,
        _ => panic!(),
    };

    bnr.pixels.clone_from_slice(include_bytes!("../extra_assets/banner_image.bin"));

    fn write_encoded_str(field: &str, s: &Option<String>, slice: &mut [u8]) -> Result<(), String>
    {
        if let Some(s) = s {
            let mut bytes = WINDOWS_1252.encode(&s, EncoderTrap::Strict)
                .map_err(|e| format!("Failed to encode banner field {}: {}", field, e))?;
            if bytes.len() >= (slice.len() - 1) {
                Err(format!("Invalid encoded length for banner field {}: expect {}, got {}",
                            field, slice.len() - 1, bytes.len()))?
            }
            bytes.resize(slice.len(), 0u8);
            slice.clone_from_slice(&bytes);
        }
        Ok(())
    }

    write_encoded_str("game_name", &banner.game_name, &mut bnr.english_fields.game_name)?;
    write_encoded_str("developer", &banner.developer, &mut bnr.english_fields.developer)?;
    write_encoded_str(
        "game_name_full",
        &banner.game_name_full,
        &mut bnr.english_fields.game_name_full
    )?;
    write_encoded_str(
        "developer_full",
        &banner.developer_full,
        &mut bnr.english_fields.developer_full)
    ?;
    write_encoded_str("description", &banner.description, &mut bnr.english_fields.description)?;

    Ok(())
}

#[derive(PartialEq, Copy, Clone)]
enum Version
{
    NtscU0_00,
    NtscU0_01,
    NtscU0_02,
    NtscK,
    NtscJ,
    Pal,
    NtscUTrilogy,
    NtscJTrilogy,
    PalTrilogy,
}

impl fmt::Display for Version
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error>
    {
        match self {
            Version::NtscU0_00    => write!(f, "1.00"),
            Version::NtscU0_01    => write!(f, "1.01"),
            Version::NtscU0_02    => write!(f, "1.02"),
            Version::NtscK        => write!(f, "kor"),
            Version::NtscJ        => write!(f, "jap"),
            Version::Pal          => write!(f, "pal"),
            Version::NtscUTrilogy => write!(f, "trilogy_ntsc_u"),
            Version::NtscJTrilogy => write!(f, "trilogy_ntsc_j"),
            Version::PalTrilogy   => write!(f, "trilogy_pal"),
        }
    }
}

fn patch_qol_game_breaking(
    patcher: &mut PrimePatcher,
    version: Version,
    force_vanilla_layout: bool,
    small_samus: bool,
)
{
    // Crashes
    patcher.add_scly_patch(
        resource_info!("00j_mines_connect.MREA").into(),
        patch_fix_central_dynamo_crash
    );
    patcher.add_scly_patch(
        resource_info!("00p_mines_connect.MREA").into(),
        patch_fix_pca_crash
    );

    // randomizer-induced bugfixes
    patcher.add_scly_patch(
        resource_info!("1a_morphballtunnel.MREA").into(),
        move |ps, area| patch_spawn_point_position(ps, area, [124.53, -79.78, 22.84], false, false)
    );
    patcher.add_scly_patch(
        resource_info!("05_bathhall.MREA").into(),
        move |ps, area| patch_spawn_point_position(ps, area, [210.512, -82.424, 19.2174], false, false)
    );
    patcher.add_scly_patch(
        resource_info!("00_mines_savestation_b.MREA").into(),
        move |ps, area| patch_spawn_point_position(ps, area, [216.7245, 4.4046, -139.8873], false, true)
    );
    patcher.add_scly_patch(
        resource_info!("00_mines_savestation_b.MREA").into(),
        move |ps, area| patch_spawn_point_position(ps, area, [216.7245, 4.4046, -139.8873], false, true)
    );
    // Turrets in Vent Shaft Section B always spawn
    patcher.add_scly_patch(
        resource_info!("08b_intro_ventshaft.MREA").into(),
        move |ps, area| patch_remove_ids(ps, area, vec![0x0013001A, 0x0013001C])
    );
    if small_samus {
        patcher.add_scly_patch(
            resource_info!("01_over_mainplaza.MREA").into(),
            move |_ps, area| patch_spawn_point_position(_ps, area, [0.0, 0.0, 0.5], true, false)
        );
        patcher.add_scly_patch(
            resource_info!("0_elev_lava_b.MREA").into(),
            move |_ps, area| patch_spawn_point_position(_ps, area, [0.0, 0.0, 0.7], true, false)
        );
    }
    // EQ Cutscene always Phazon Suit (avoids multiworld crash when player receives a suit during the fight)
    patcher.add_scly_patch(
        resource_info!("12_mines_eliteboss.MREA").into(),
        move |ps, area| patch_cutscene_force_phazon_suit(ps, area)
    );

    if force_vanilla_layout { return; }

    // undo retro "fixes"
    if version == Version::NtscU0_00 {
        patcher.add_scly_patch(
            resource_info!("00n_ice_connect.MREA").into(),
            patch_research_core_access_soft_lock
        );
    } else {
        patcher.add_scly_patch(
            resource_info!("08_courtyard.MREA").into(),
            patch_arboretum_invisible_wall
        );
        if version != Version::NtscU0_01 {
            patcher.add_scly_patch(
                resource_info!("05_ice_shorelines.MREA").into(),
                move |ps, area| patch_ruined_courtyard_thermal_conduits(ps, area, version)
            );
        }
    }
    if version == Version::NtscU0_02 {
        patcher.add_scly_patch(
            resource_info!("01_mines_mainplaza.MREA").into(),
            patch_main_quarry_door_lock_0_02
        );
        patcher.add_scly_patch(
            resource_info!("13_over_burningeffigy.MREA").into(),
            patch_geothermal_core_door_lock_0_02
        );
        patcher.add_scly_patch(
            resource_info!("19_hive_totem.MREA").into(),
            patch_hive_totem_boss_trigger_0_02
        );
        patcher.add_scly_patch(
            resource_info!("04_mines_pillar.MREA").into(),
            patch_ore_processing_door_lock_0_02
        );
    }
    if version == Version::Pal || version == Version::NtscJ || version == Version::NtscUTrilogy || version == Version::NtscJTrilogy || version == Version::PalTrilogy {
        patcher.add_scly_patch(
            resource_info!("04_mines_pillar.MREA").into(),
            patch_ore_processing_destructible_rock_pal
        );
        patcher.add_scly_patch(
            resource_info!("13_over_burningeffigy.MREA").into(),
            patch_geothermal_core_destructible_rock_pal
        );
        if version == Version::Pal {
            patcher.add_scly_patch(
                resource_info!("01_mines_mainplaza.MREA").into(),
                patch_main_quarry_door_lock_pal
            );
        }
    }

    // softlocks
    patcher.add_scly_patch(
        resource_info!("22_Flaahgra.MREA").into(),
        patch_sunchamber_prevent_wild_before_flaahgra
    );
    patcher.add_scly_patch(
        resource_info!("0v_connect_tunnel.MREA").into(),
        patch_sun_tower_prevent_wild_before_flaahgra
    );
    patcher.add_scly_patch(
        resource_info!("13_ice_vault.MREA").into(),
        patch_research_lab_aether_exploding_wall // Remove wall when dark labs is activated
    );
    patcher.add_scly_patch(
        resource_info!("12_ice_research_b.MREA").into(),
        patch_research_lab_aether_exploding_wall_2 // Remove AI jank factor from persuading Edward to jump through glass when doing backwards aether
    );
    patcher.add_scly_patch(
        resource_info!("11_ice_observatory.MREA").into(),
        patch_observatory_2nd_pass_solvablility
    );
    patcher.add_scly_patch(
        resource_info!("11_ice_observatory.MREA").into(),
        patch_observatory_1st_pass_softlock
    );
    patcher.add_scly_patch(
        resource_info!("02_mines_shotemup.MREA").into(),
        patch_mines_security_station_soft_lock
    );
    patcher.add_scly_patch(
        resource_info!("18_ice_gravity_chamber.MREA").into(),
        patch_gravity_chamber_stalactite_grapple_point
    );
    patcher.add_scly_patch(
        resource_info!("19_hive_totem.MREA").into(),
        patch_hive_totem_softlock
    );
}

fn make_elite_research_fight_prereq_patches(patcher: &mut PrimePatcher)
{
    patcher.add_scly_patch(resource_info!("03_mines.MREA").into(), |_ps, area| {
        let flags = &mut area.layer_flags.flags;
        *flags |= 1 << 1; // Turn on "3rd pass elite bustout"
        *flags &= !(1 << 5); // Turn off the "dummy elite"
        Ok(())
    });

    patcher.add_scly_patch(resource_info!("07_mines_electric.MREA").into(), |_ps, area| {
        let scly = area.mrea().scly_section_mut();
        scly.layers.as_mut_vec()[0].objects.as_mut_vec()
            .retain(|obj| obj.instance_id != 0x1B0525 && obj.instance_id != 0x1B0522);
        Ok(())
    });
}

fn patch_qol_logical(patcher: &mut PrimePatcher, config: &PatchConfig)
{
    if config.main_plaza_door {
        patcher.add_scly_patch(
            resource_info!("01_mainplaza.MREA").into(),
            make_main_plaza_locked_door_two_ways
        );
    }

    if config.phazon_elite_without_dynamo {
        make_elite_research_fight_prereq_patches(patcher);

    }

    if config.backwards_frigate {
        patcher.add_scly_patch(
            resource_info!("08b_under_intro_ventshaft.MREA").into(),
            patch_main_ventilation_shaft_section_b_door
        );
    }

    if config.backwards_labs {
        patcher.add_scly_patch(
            resource_info!("10_ice_research_a.MREA").into(),
            patch_research_lab_hydra_barrier
        );
    }

    if config.backwards_upper_mines {
        patcher.add_scly_patch(
            resource_info!("01_mines_mainplaza.MREA").into(),
            patch_main_quarry_barrier
        );
    }

    if config.backwards_lower_mines {
        patcher.add_scly_patch(
            resource_info!("00p_mines_connect.MREA").into(),
            patch_backwards_lower_mines_pca
        );
        patcher.add_scly_patch(
            resource_info!("00o_mines_connect.MREA").into(),
            patch_backwards_lower_mines_eqa
        );
        patcher.add_scly_patch(
            resource_info!("11_mines.MREA").into(),
            patch_backwards_lower_mines_mqb
        );
        patcher.add_scly_patch(
            resource_info!("08_mines.MREA").into(),
            patch_backwards_lower_mines_mqa
        );
        patcher.add_scly_patch(
            resource_info!("05_mines_forcefields.MREA").into(),
            patch_backwards_lower_mines_elite_control
        );
    }
}

fn patch_qol_cosmetic(
    patcher: &mut PrimePatcher,
    skip_ending_cinematic: bool,
    quick_patch: bool,
)
{
    if quick_patch {
        // Replace all non-critical files with empty ones to speed up patching
        const FILENAMES: &[&[u8]] = &[
            b"Video/00_first_start.thp",
            b"Video/01_startloop.thp",
            b"Video/02_start_fileselect_A.thp",
            b"Video/02_start_fileselect_B.thp",
            b"Video/02_start_fileselect_C.thp",
            b"Video/03_fileselectloop.thp",
            b"Video/04_fileselect_playgame_A.thp",
            b"Video/04_fileselect_playgame_B.thp",
            b"Video/04_fileselect_playgame_C.thp",
            b"Video/05_tallonText.thp",
            b"Video/06_fileselect_GBA.thp",
            b"Video/07_GBAloop.thp",
            b"Video/08_GBA_fileselect.thp",
            b"Video/AfterCredits.thp",
            b"Video/SpecialEnding.thp",
            b"Video/attract0.thp",
            b"Video/attract1.thp",
            b"Video/attract2.thp",
            b"Video/attract3.thp",
            b"Video/attract4.thp",
            b"Video/attract5.thp",
            b"Video/attract6.thp",
            b"Video/attract7.thp",
            b"Video/attract8.thp",
            b"Video/attract9.thp",
            b"Video/creditBG.thp",
            b"Video/from_gallery.thp",
            b"Video/losegame.thp",
            b"Video/to_gallery.thp",
            b"Video/win_bad_begin.thp",
            b"Video/win_bad_end.thp",
            b"Video/win_bad_loop.thp",
            b"Video/win_good_begin.thp",
            b"Video/win_good_end.thp",
            b"Video/win_good_loop.thp",
            b"Audio/CraterReveal2.dsp",
            b"Audio/END-escapeL.dsp",
            b"Audio/END-escapeR.dsp",
            b"Audio/Ruins-soto-AL.dsp",
            b"Audio/Ruins-soto-AR.dsp",
            b"Audio/Ruins-soto-BL.dsp",
            b"Audio/Ruins-soto-BR.dsp",
            b"Audio/amb_x_elevator_lp_02.dsp",
            b"Audio/cra_mainL.dsp",
            b"Audio/cra_mainR.dsp",
            b"Audio/cra_mprime1L.dsp",
            b"Audio/cra_mprime1R.dsp",
            b"Audio/cra_mprime2L.dsp",
            b"Audio/cra_mprime2R.dsp",
            b"Audio/crash-ship-3L.dsp",
            b"Audio/crash-ship-3R.dsp",
            b"Audio/crash-ship-maeL.dsp",
            b"Audio/crash-ship-maeR.dsp",
            b"Audio/ending3.rsf",
            b"Audio/evt_x_event_00.dsp",
            b"Audio/frontend_1.rsf",
            b"Audio/frontend_2.rsf",
            b"Audio/gen_SaveStationL.dsp",
            b"Audio/gen_SaveStationR.dsp",
            b"Audio/gen_ShortBattle2L.dsp",
            b"Audio/gen_ShortBattle2R.dsp",
            b"Audio/gen_ShortBattleL.dsp",
            b"Audio/gen_ShortBattleR.dsp",
            b"Audio/gen_elevatorL.dsp",
            b"Audio/gen_elevatorR.dsp",
            b"Audio/gen_puzzleL.dsp",
            b"Audio/gen_puzzleR.dsp",
            b"Audio/gen_rechargeL.dsp",
            b"Audio/gen_rechargeR.dsp",
            b"Audio/ice_chapelL.dsp",
            b"Audio/ice_chapelR.dsp",
            b"Audio/ice_connectL.dsp",
            b"Audio/ice_connectR.dsp",
            b"Audio/ice_kincyoL.dsp",
            b"Audio/ice_kincyoR.dsp",
            b"Audio/ice_shorelinesL.dsp",
            b"Audio/ice_shorelinesR.dsp",
            b"Audio/ice_thardusL.dsp",
            b"Audio/ice_thardusR.dsp",
            b"Audio/ice_worldmainL.dsp",
            b"Audio/ice_worldmainR.dsp",
            b"Audio/ice_x_wind_lp_00L.dsp",
            b"Audio/ice_x_wind_lp_00R.dsp",
            b"Audio/int_biohazardL.dsp",
            b"Audio/int_biohazardR.dsp",
            b"Audio/int_escapel.dsp",
            b"Audio/int_escaper.dsp",
            b"Audio/int_introcinemaL.dsp",
            b"Audio/int_introcinemaR.dsp",
            b"Audio/int_introstageL.dsp",
            b"Audio/int_introstageR.dsp",
            b"Audio/int_parasitequeenL.dsp",
            b"Audio/int_parasitequeenR.dsp",
            b"Audio/int_spaceL.dsp",
            b"Audio/int_spaceR.dsp",
            b"Audio/int_toujouL.dsp",
            b"Audio/int_toujouR.dsp",
            b"Audio/itm_x_short_02.dsp",
            b"Audio/jin_artifact.dsp",
            b"Audio/jin_itemattain.dsp",
            b"Audio/lav_lavamaeL.dsp",
            b"Audio/lav_lavamaeR.dsp",
            b"Audio/lav_lavamainL.dsp",
            b"Audio/lav_lavamainR.dsp",
            b"Audio/min_darkL.dsp",
            b"Audio/min_darkR.dsp",
            b"Audio/min_mainL.dsp",
            b"Audio/min_mainR.dsp",
            b"Audio/min_omegapirateL.dsp",
            b"Audio/min_omegapirateR.dsp",
            b"Audio/min_phazonL.dsp",
            b"Audio/min_phazonR.dsp",
            b"Audio/min_x_wind_lp_01L.dsp",
            b"Audio/min_x_wind_lp_01R.dsp",
            b"Audio/over-craterrevealL.dsp",
            b"Audio/over-craterrevealR.dsp",
            b"Audio/over-ridleyL.dsp",
            b"Audio/over-ridleyR.dsp",
            b"Audio/over-ridleydeathL.dsp",
            b"Audio/over-ridleydeathR.dsp",
            b"Audio/over-stonehengeL.dsp",
            b"Audio/over-stonehengeR.dsp",
            b"Audio/over-world-daichiL.dsp",
            b"Audio/over-world-daichiR.dsp",
            b"Audio/over-worldL.dsp",
            b"Audio/over-worldR.dsp",
            b"Audio/pir_battle3L.dsp",
            b"Audio/pir_battle3R.dsp",
            b"Audio/pir_isogiL.dsp",
            b"Audio/pir_isogiR.dsp",
            b"Audio/pir_yoinL.dsp",
            b"Audio/pir_yoinR.dsp",
            b"Audio/pir_zencyoL.dsp",
            b"Audio/pir_zencyoR.dsp",
            b"Audio/pvm01.dsp",
            b"Audio/rid_r_death_01.dsp",
            b"Audio/rui_chozobowlingL.dsp",
            b"Audio/rui_chozobowlingR.dsp",
            b"Audio/rui_flaaghraL.dsp",
            b"Audio/rui_flaaghraR.dsp",
            b"Audio/rui_hivetotemL.dsp",
            b"Audio/rui_hivetotemR.dsp",
            b"Audio/rui_monkeylowerL.dsp",
            b"Audio/rui_monkeylowerR.dsp",
            b"Audio/rui_samusL.dsp",
            b"Audio/rui_samusR.dsp",
            b"Audio/ruins-firstL.dsp",
            b"Audio/ruins-firstR.dsp",
            b"Audio/ruins-nakaL.dsp",
            b"Audio/ruins-nakaR.dsp",
            b"Audio/sam_samusappear.dsp",
            b"Audio/samusjak.rsf",
            b"Audio/tha_b_enraged_00.dsp",
            b"Audio/tha_r_death_00.dsp",
        ];
        const EMPTY: &[u8] = include_bytes!("../extra_assets/attract_mode.thp"); // empty file
        for name in FILENAMES {
            patcher.add_file_patch(name, |file| {
                *file = structs::FstEntryFile::ExternalFile(Box::new(EMPTY));
                Ok(())
            });
        }
    }
    else
    {
        // Replace the attract mode FMVs with empty files to reduce the amount of data we need to
        // copy and to make compressed ISOs smaller.
        const FMV_NAMES: &[&[u8]] = &[
            b"Video/attract0.thp",
            b"Video/attract1.thp",
            b"Video/attract2.thp",
            b"Video/attract3.thp",
            b"Video/attract4.thp",
            b"Video/attract5.thp",
            b"Video/attract6.thp",
            b"Video/attract7.thp",
            b"Video/attract8.thp",
            b"Video/attract9.thp",
        ];
        const FMV: &[u8] = include_bytes!("../extra_assets/attract_mode.thp");
        for name in FMV_NAMES {
            patcher.add_file_patch(name, |file| {
                *file = structs::FstEntryFile::ExternalFile(Box::new(FMV));
                Ok(())
            });
        }
    }

    patcher.add_resource_patch(
        resource_info!("FRME_BallHud.FRME").into(),
        patch_morphball_hud,
    );

    if skip_ending_cinematic {
        patcher.add_scly_patch(
            resource_info!("01_endcinema.MREA").into(),
            patch_ending_scene_straight_to_credits
        );
    }


    patcher.add_scly_patch(
        resource_info!("08_courtyard.MREA").into(),
        patch_arboretum_vines
    );

    // not shown here - hudmemos are nonmodal and item aquisition cutscenes are removed
}

fn patch_qol_competitive_cutscenes(patcher: &mut PrimePatcher, version: Version) {
    patcher.add_scly_patch(
        resource_info!("01_mines_mainplaza.MREA").into(), // main quarry (just pirate booty)
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![],
            vec![
                0x000203DE, 0x000203DC, 0x0002040D, 0x0002040C, // keep area entrance cutscene
                0x0002023E, 0x00020021, 0x00020253, // keep crane cutscenes
                0x0002043D, // keep barrier cutscene
            ],
            false,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("08_courtyard.MREA").into(), // Arboretum
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x0013012E, 0x00130131, 0x00130141], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("10_over_1alavaarea.MREA").into(), // magmoor workstation
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![0x00170153], false), // skip patching 1st cutscene (special floaty case)
    );
    patcher.add_scly_patch(
        resource_info!("05_over_xray.MREA").into(), // life grove (competitive only - watch raise post cutscenes)
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![0x002A01D0], true),
    );
    patcher.add_scly_patch(
        resource_info!("12_ice_research_b.MREA").into(),
        move |ps, area| patch_lab_aether_cutscene_trigger(ps, area, version)
    );
    patcher.add_scly_patch(
        resource_info!("00j_over_hall.MREA").into(), // temple security station
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], true),
    );
    patcher.add_scly_patch(
        resource_info!("15_ice_cave_a.MREA").into(), // frost cave
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x0029006C, 0x0029006B], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("15_energycores.MREA").into(), // energy core
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![
                0x002C00E8, 0x002C0101, 0x002C00F5, // activate core delay
                0x002C0068, 0x002C0055, 0x002C0079, // core energy flow activation delay
                0x002C0067, 0x002C00E7, 0x002C0102, // jingle finish delay
                0x002C0104, 0x002C00EB, // platform go up delay
                0x002C0069, // water go down delay
                0x002C01BC, // unlock door
            ],
            vec![],
            false,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("07_under_intro_reactor.MREA").into(), // reactor core
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("06_under_intro_freight.MREA").into(), // cargo freight lift
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x001B0100], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("05_under_intro_zoo.MREA").into(), // biohazard containment
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x001E028A], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("05_under_intro_specimen_chamber.MREA").into(), // biotech research area 1
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x002000DB], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("04_maproom_d.MREA").into(), // vault
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("0v_connect_tunnel.MREA").into(), // sun tower
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x001D00E5, 0x001D00E8], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("07_ruinedroof.MREA").into(), // training chamber
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x000C0153, 0x000C0154, 0x000C015B, 0x000C0151, 0x000C013E], vec![], true),
    );
    patcher.add_scly_patch(
        resource_info!("11_wateryhall.MREA").into(), // watery hall
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x0029280A, 0x002927FD], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("18_halfpipe.MREA").into(), // crossway
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("13_over_burningeffigy.MREA").into(), // geothermal core
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![0x001401DD, 0x001401E3], // immediately move parts
            vec![],
            false,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("06_ice_temple.MREA").into(), // chozo ice temple
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![0x00080201, 0x0008024E,0x00080170, 0x00080118], // speed up hands animation + grate open
            vec![],
            false),
    );
    patcher.add_scly_patch(
        resource_info!("04_ice_boost_canyon.MREA").into(), // Phendrana canyon
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("05_ice_shorelines.MREA").into(), // ruined courtyard
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("13_ice_vault.MREA").into(), // research core
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("03_mines.MREA").into(), // elite research (keep phazon elite cutscene)
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![0x000D04C8, 0x000D01CF], true),
    );
    patcher.add_scly_patch(
        resource_info!("02_mines_shotemup.MREA").into(), // mine security station
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x00070513], vec![], true),
    );
}

fn patch_qol_minor_cutscenes(patcher: &mut PrimePatcher, version: Version) {
    patcher.add_scly_patch(
        resource_info!("08_courtyard.MREA").into(), // Arboretum
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x0013012E, 0x00130131, 0x00130141], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("08_mines.MREA").into(), // MQA (just first cutscene)
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![],
            vec![0x002000CF], // 2nd cutscene
            false,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("12_ice_research_b.MREA").into(),
        move |ps, area| patch_lab_aether_cutscene_trigger(ps, area, version)
    );
    patcher.add_scly_patch(
        resource_info!("00j_over_hall.MREA").into(), // temple security station
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], true),
    );
    patcher.add_scly_patch(
        resource_info!("15_ice_cave_a.MREA").into(), // frost cave
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x0029006C, 0x0029006B], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("15_energycores.MREA").into(), // energy core
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![
                0x002C00E8, 0x002C0101, 0x002C00F5, // activate core delay
                0x002C0068, 0x002C0055, 0x002C0079, // core energy flow activation delay
                0x002C0067, 0x002C00E7, 0x002C0102, // jingle finish delay
                0x002C0104, 0x002C00EB, // platform go up delay
                0x002C0069, // water go down delay
                0x002C01BC, // unlock door
            ],
            vec![],
            false,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("10_over_1alavaarea.MREA").into(), // magmoor workstation
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![0x00170153], false), // skip patching 1st cutscene (special floaty case)
    );
    patcher.add_scly_patch(
        resource_info!("07_under_intro_reactor.MREA").into(), // reactor core
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("06_under_intro_freight.MREA").into(), // cargo freight lift
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x001B0100], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("05_under_intro_zoo.MREA").into(), // biohazard containment
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x001E028A], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("05_under_intro_specimen_chamber.MREA").into(), // biotech research area 1
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x002000DB], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("05_over_xray.MREA").into(), // life grove
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], true),
    );
    patcher.add_scly_patch(
        resource_info!("01_mainplaza.MREA").into(), // main plaza
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("01_mines_mainplaza.MREA").into(), // main quarry
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![0x00020443], // turn the forcefield off faster
            vec![],
            false,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("11_over_muddywaters_b.MREA").into(), // lava lake
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("04_maproom_d.MREA").into(), // vault
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("0v_connect_tunnel.MREA").into(), // sun tower
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x001D00E5, 0x001D00E8], vec![], false), // Open gate faster
    );
    patcher.add_scly_patch(
        resource_info!("07_ruinedroof.MREA").into(), // training chamber
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x000C0153, 0x000C0154, 0x000C015B, 0x000C0151, 0x000C013E], vec![], true),
    );
    patcher.add_scly_patch(
        resource_info!("11_wateryhall.MREA").into(), // watery hall
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x0029280A, 0x002927FD], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("18_halfpipe.MREA").into(), // crossway
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("17_chozo_bowling.MREA").into(), // hall of the elders
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![0x003400F4, 0x003400F8, 0x003400F9, 0x0034018C], // speed up release from bomb slots
            vec![
                0x003400F5, 0x00340046, 0x0034004A, 0x003400EA, 0x0034004F, // leave chozo bowling cutscenes to avoid getting stuck
                0x0034025C, 0x00340264, 0x00340268, 0x0034025B, // leave missile station cutsene
                0x00340142, 0x00340378, // leave ghost death cutscene (it's major b/c reposition)
            ],
            false,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("13_over_burningeffigy.MREA").into(), // geothermal core
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![0x001401DD, 0x001401E3], // immediately move parts
            vec![],
            false,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("00h_mines_connect.MREA").into(), // vent shaft
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![0x001200C3, 0x001200DE], // activate gas faster
            vec![],
            true,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("06_ice_temple.MREA").into(), // chozo ice temple
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![0x00080201, 0x0008024E,0x00080170, 0x00080118], // speed up hands animation + grate open
            vec![],
            false),
    );
    patcher.add_scly_patch(
        resource_info!("04_ice_boost_canyon.MREA").into(), // Phendrana canyon
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("05_ice_shorelines.MREA").into(), // ruined courtyard
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("11_ice_observatory.MREA").into(), // Observatory
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x001E0042, 0x001E000E], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("08_ice_ridley.MREA").into(), // control tower
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x002702DD, 0x002702D5, 0x00270544, 0x002703DF], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("13_ice_vault.MREA").into(), // research core
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("03_mines.MREA").into(), // elite research (keep phazon elite cutscene)
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![0x000D04C8, 0x000D01CF], true),
    );
    patcher.add_scly_patch(
        resource_info!("06_mines_elitebustout.MREA").into(), // omega reserach
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], true),
    );
    patcher.add_scly_patch(
        resource_info!("07_mines_electric.MREA").into(), // central dynamo
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![0x001B03F8], // activate maze faster
            vec![0x001B0349, 0x001B0356], // keep item aquisition cutscene (or players can get left down there)
            true,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("02_mines_shotemup.MREA").into(), // mine security station
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x00070513], vec![], true),
    );
    patcher.add_scly_patch(
        resource_info!("01_ice_plaza.MREA").into(), // phendrana shorelines
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x00020203], vec![0x000202A9, 0x000202A8, 0x000202B7], true), // keep the ridley cinematic
    );
}

pub fn patch_qol_major_cutscenes(patcher: &mut PrimePatcher) {
    patcher.add_scly_patch(
        resource_info!("08_courtyard.MREA").into(), // Arboretum
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x0013012E, 0x00130131, 0x00130141], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("01_endcinema.MREA").into(), // Impact Crater Escape Cinema (cause why not)
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], true),
    );
    // +Ghost death cutscene
    patcher.add_scly_patch(
        resource_info!("17_chozo_bowling.MREA").into(), // hall of the elders
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![0x003400F4, 0x003400F8, 0x003400F9, 0x0034018C], // speed up release from bomb slots
            vec![
                0x003400F5, 0x00340046, 0x0034004A, 0x003400EA, 0x0034004F, // leave chozo bowling cutscenes to avoid getting stuck
                0x0034025C, 0x00340264, 0x00340268, 0x0034025B, // leave missile station cutsene
            ],
            false,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("01_ice_plaza.MREA").into(), // phendrana shorelines
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x00020203], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("07_stonehenge.MREA").into(), // artifact temple
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![],
            vec![
                // progress cutscene
                0x00100463, 0x0010046F,
                // ridley intro cutscene
                0x0010036F, 0x0010026C, 0x00100202, 0x00100207, 0x00100373, 0x001003C4, 0x001003D9, 0x001003DC, 0x001003E6, 0x001003CE, 0x0010020C, 0x0010021A, 0x001003EF, 0x001003E9, 0x0010021A, 0x00100491, 0x001003EE, 0x001003F0, 0x001003FE, 0x0010021F,
                // crater entry/exit cutscene
                0x001002C8, 0x001002B8, 0x001002C2,
            ],
            true,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("03_mines.MREA").into(), // elite research
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x000D01A9], vec![], true),
    );
    patcher.add_scly_patch(
        resource_info!("19_hive_totem.MREA").into(), // hive totem
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("1a_morphball_shrine.MREA").into(), // ruined shrine
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], true),
    );
    patcher.add_scly_patch(
        resource_info!("03_monkey_lower.MREA").into(), // burn dome
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x0030017B], vec![], true),
    );
    patcher.add_scly_patch(
        resource_info!("22_Flaahgra.MREA").into(), // sunchamber
        move |ps, area| patch_remove_cutscenes(
            ps, area,
            vec![
                0x00250092, 0x00250093, 0x00250094, 0x002500A8, // release from bomb slot
                0x0025276A, // acid --> water (needed for floaty)
            ],
            vec![
                0x002500CA, 0x00252FE4, 0x00252727, 0x0025272C, 0x00252741,  // into cinematic works better if skipped normally
                0x0025000B, // you get put in vines timeout if you skip the first reposition:
                            // https://cdn.discordapp.com/attachments/761000402182864906/840707140364664842/no-spawnpoints.mp4
                0x00250123, // keep just the first camera angle of the death cutscene to prevent underwater when going for pre-floaty
                0x00252FC0, // the last reposition is important for floaty jump
            ],
            false,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("09_ice_lobby.MREA").into(), // research entrance
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![0x001402F7, 0x00140243, 0x001402D6, 0x001402D0, 0x001402B3], // start fight faster
            vec![],
            false
        ),
    );
    patcher.add_scly_patch(
        resource_info!("19_ice_thardus.MREA").into(), // Quarantine Cave
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], true),
    );
    patcher.add_scly_patch(
        resource_info!("05_mines_forcefields.MREA").into(), // elite control
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], true),
    );
    patcher.add_scly_patch(
        resource_info!("08_mines.MREA").into(), // MQA
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![
                0x002000D7, // Timer_pikeend
                0x002000DE, // Timer_coverstart
                0x002000E0, // Timer_steamshutoff
                0x00200708, // Timer - Shield Off, Play Battle Music
            ],
            vec![],
            false,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("12_mines_eliteboss.MREA").into(), // elite quarters
        move |ps, area| patch_remove_cutscenes(
            ps, area, vec![],
            vec![ // keep the first cutscene because the normal skip works out better
                0x001A0282, 0x001A0283, 0x001A02B3, 0x001A02BF, 0x001A0284, 0x001A031A, // cameras
                0x001A0294, 0x001A02B9, // player actor
            ],
            true,
        ),
    );
    patcher.add_scly_patch( // phazon infusion chamber
        resource_info!("03a_crater.MREA").into(),
        move |ps, area| patch_remove_cutscenes(
            ps, area, vec![],
            vec![ // keep first cutscene because vanilla skip is better
                0x0005002B, 0x0005002C, 0x0005007D, 0x0005002D, 0x00050032, 0x00050078, 0x00050033, 0x00050034, 0x00050035, 0x00050083, // cameras
                0x0005002E, 0x0005008B, 0x00050089, // player actors
            ],
            false,
        ),
    );

    // subchambers 1-4 (see special handling for exo aggro)
    patcher.add_scly_patch(
        resource_info!("03b_crater.MREA").into(),
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("03c_crater.MREA").into(),
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("03d_crater.MREA").into(),
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("03e_crater.MREA").into(),
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], true),
    );

    // play subchamber 5 cutscene normally (players can't natrually pass through the ceiling of prime's lair)

    patcher.add_scly_patch(
        resource_info!("03f_crater.MREA").into(), // metroid prime lair
        move |ps, area| patch_remove_cutscenes(
            ps, area, vec![],
            vec![
                // play the first cutscene so it can be skipped normally
                0x000B019D, 0x000B008B, 0x000B008D, 0x000B0093, 0x000B0094, 0x000B00A7,
                0x000B00AF, 0x000B00E1, 0x000B00DF, 0x000B00B0, 0x000B00D3, 0x000B00E3,
                0x000B00E6, 0x000B0095, 0x000B00E4,

                // play the first camera of the death cutcsene so races have a clean finish
                0x000B00ED,
            ],
            true,
        ),
    );
}

fn patch_power_conduits<'a>(patcher: &mut PrimePatcher<'_, 'a>)
{
    patcher.add_scly_patch(
        resource_info!("05_ice_shorelines.MREA").into(), // ruined courtyard
        patch_thermal_conduits_damage_vulnerabilities
    );

    patcher.add_scly_patch(
        resource_info!("13_ice_vault.MREA").into(), // research core
        patch_thermal_conduits_damage_vulnerabilities
    );

    patcher.add_scly_patch(
        resource_info!("08b_under_intro_ventshaft.MREA").into(), // Main Ventilation Shaft Section B
        patch_thermal_conduits_damage_vulnerabilities
    );

    patcher.add_scly_patch(
        resource_info!("07_under_intro_reactor.MREA").into(), // reactor core
        patch_thermal_conduits_damage_vulnerabilities
    );

    patcher.add_scly_patch(
        resource_info!("06_under_intro_to_reactor.MREA").into(), // reactor core access
        patch_thermal_conduits_damage_vulnerabilities
    );

    patcher.add_scly_patch(
        resource_info!("06_under_intro_freight.MREA").into(), // cargo freight lift to deck gamma
        patch_thermal_conduits_damage_vulnerabilities
    );

    patcher.add_scly_patch(
        resource_info!("05_under_intro_zoo.MREA").into(), // biohazard containment
        patch_thermal_conduits_damage_vulnerabilities
    );

    patcher.add_scly_patch(
        resource_info!("05_under_intro_specimen_chamber.MREA").into(), // biotech research area 1
        patch_thermal_conduits_damage_vulnerabilities
    );

    patcher.add_scly_patch(
        resource_info!("01_mines_mainplaza.MREA").into(), // main quarry
        patch_thermal_conduits_damage_vulnerabilities
    );

    patcher.add_scly_patch(
        resource_info!("10_over_1alavaarea.MREA").into(), // magmoor workstation
        patch_thermal_conduits_damage_vulnerabilities
    );
}

fn patch_hive_mecha<'a>(patcher: &mut PrimePatcher<'_, 'a>)
{
    patcher.add_scly_patch(resource_info!("19_hive_totem.MREA").into(), |_ps, area| {
        let flags = &mut area.layer_flags.flags;
        *flags &= !(1 << 1); // Turn off "1st pass" layer
        Ok(())
    });

    patcher.add_scly_patch(resource_info!("19_hive_totem.MREA").into(), |_ps, area| {
        let scly = area.mrea().scly_section_mut();

        let layer = &mut scly.layers.as_mut_vec()[0]; // Default

        let relay_id =
            layer.objects.iter()
            .find(|obj| obj.property_data.as_relay()
                .map(|relay| relay.name == b"Relay - Make Room Already Visited\0".as_cstr())
                .unwrap_or(false)
            )
            .map(|relay| relay.instance_id);

        if let Some(relay_id) = relay_id {
            layer.objects.as_mut_vec().push(structs::SclyObject {
                instance_id: _ps.fresh_instance_id_range.next().unwrap(),
                property_data: structs::Timer {
                    name: b"Auto start relay\0".as_cstr(),
                    start_time: 0.001,
                    max_random_add: 0f32,
                    reset_to_zero: 0,
                    start_immediately: 1,
                    active: 1,
                }.into(),
                connections: vec![
                    structs::Connection {
                        state: structs::ConnectionState::ZERO,
                        message: structs::ConnectionMsg::SET_TO_ZERO,
                        target_object_id: relay_id,
                    },
                ].into(),
            });
        }

        Ok(())
    });
}

fn patch_incinerator_drone_timer<'r>(area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>, timer_name: CString, minimum_time: Option<f32>, random_add: Option<f32>) -> Result<(), String> {
    let scly = area.mrea().scly_section_mut();

    let layer = &mut scly.layers.as_mut_vec()[0]; // Default

    for obj in layer.objects.iter_mut() {
        let timer_obj = obj.property_data.as_timer_mut();

        if timer_obj.is_some() {
            let timer_obj = timer_obj.unwrap();
            if timer_name.as_c_str() == timer_obj.name.as_ref() {
                if minimum_time.is_some() {
                    timer_obj.start_time = minimum_time.unwrap();
                }
                if random_add.is_some() {
                    timer_obj.max_random_add = random_add.unwrap();
                }
            }
        }
    }
    Ok(())
}

fn patch_arboretum_sandstone<'a>(patcher: &mut PrimePatcher<'_, 'a>)
{
    patcher.add_scly_patch(resource_info!("08_courtyard.MREA").into(), |_ps, area| {
        let scly = area.mrea().scly_section_mut();

        let layer = &mut scly.layers.as_mut_vec()[0]; // Default
        for obj in layer.objects.iter_mut() {
            if obj.property_data
                .as_damageable_trigger()
                .map(|dt| dt.name == b"DamageableTrigger-component\0".as_cstr())
                .unwrap_or(false) {
                obj.property_data.as_damageable_trigger_mut().unwrap().damage_vulnerability.power_bomb = 1;
            }
        }

        Ok(())
    });
}

pub fn patch_iso<T>(config: PatchConfig, mut pn: T) -> Result<(), String>
    where T: structs::ProgressNotifier
{
    let mut ct = Vec::new();
    let mut reader = Reader::new(&config.input_iso[..]);
    let mut gc_disc: structs::GcDisc = reader.read(());

    let version = match (&gc_disc.header.game_identifier(), gc_disc.header.disc_id, gc_disc.header.version) {
        (b"GM8E01", 0, 0)  => Version::NtscU0_00,
        (b"GM8E01", 0, 1)  => Version::NtscU0_01,
        (b"GM8E01", 0, 2)  => Version::NtscU0_02,
        (b"GM8E01", 0, 48) => Version::NtscK,
        (b"GM8J01", 0, 0)  => Version::NtscJ,
        (b"GM8P01", 0, 0)  => Version::Pal,
        (b"R3ME01", 0, 0)  => Version::NtscUTrilogy,
        (b"R3IJ01", 0, 0)  => Version::NtscJTrilogy,
        (b"R3MP01", 0, 0)  => Version::PalTrilogy,
        _ => Err(concat!(
                "The input ISO doesn't appear to be NTSC-US, NTSC-J, NTSC-K, PAL Metroid Prime, ",
                "or NTSC-US, NTSC-J, PAL Metroid Prime Trilogy."
            ))?
    };
    if gc_disc.find_file("randomprime.txt").is_some() {
        Err(concat!("The input ISO has already been randomized once before. ",
                    "You must start from an unmodified ISO every time."
        ))?
    }

    if config.run_mode == RunMode::ExportLogbook {
        export_logbook(&mut gc_disc, &config, version)?;
        return Ok(());
    } else if config.run_mode == RunMode::ExportAssets {
        export_assets(&mut gc_disc, &config)?;
        return Ok(());
    }

    build_and_run_patches(&mut gc_disc, &config, version)?;

    {
        let json_string = serde_json::to_string(&config)
            .map_err(|e| format!("Failed to serialize patch config: {}", e))?;
        writeln!(ct, "{}", json_string).unwrap();
        gc_disc.add_file("randomprime.json", structs::FstEntryFile::Unknown(Reader::new(&ct)))?;
    }

    let patches_rel_bytes = match version {
        Version::NtscU0_00    => Some(rel_files::PATCHES_100_REL),
        Version::NtscU0_01    => Some(rel_files::PATCHES_101_REL),
        Version::NtscU0_02    => Some(rel_files::PATCHES_102_REL),
        Version::Pal          => Some(rel_files::PATCHES_PAL_REL),
        Version::NtscK        => Some(rel_files::PATCHES_KOR_REL),
        Version::NtscJ        => Some(rel_files::PATCHES_JAP_REL),
        Version::NtscUTrilogy => None,
        Version::NtscJTrilogy => None,
        Version::PalTrilogy => None,
    };
    if let Some(patches_rel_bytes) = patches_rel_bytes {
        gc_disc.add_file(
            "patches.rel",
            structs::FstEntryFile::Unknown(Reader::new(patches_rel_bytes))
        )?;
    }

    match config.iso_format {
        IsoFormat::Iso => {
            let mut file = config.output_iso;
            file.set_len(structs::GC_DISC_LENGTH as u64)
                .map_err(|e| format!("Failed to resize output file: {}", e))?;
            gc_disc.write(&mut file, &mut pn)
                .map_err(|e| format!("Error writing output file: {}", e))?;
            pn.notify_flushing_to_disk();
        },
        IsoFormat::Gcz => {
            let mut gcz_writer = GczWriter::new(config.output_iso, structs::GC_DISC_LENGTH as u64)
                .map_err(|e| format!("Failed to prepare output file for writing: {}", e))?;
            gc_disc.write(&mut *gcz_writer, &mut pn)
                .map_err(|e| format!("Error writing output file: {}", e))?;
            pn.notify_flushing_to_disk();
        },
        IsoFormat::Ciso => {
            let mut ciso_writer = CisoWriter::new(config.output_iso)
                .map_err(|e| format!("Failed to prepare output file for writing: {}", e))?;
            gc_disc.write(&mut ciso_writer, &mut pn)
                .map_err(|e| format!("Error writing output file: {}", e))?;
            pn.notify_flushing_to_disk();
        }
    };
    Ok(())
}

fn export_logbook(gc_disc: &mut structs::GcDisc, config: &PatchConfig, _version: Version)
    -> Result<(), String>
{
    let filenames = [
        "Metroid1.pak",
        "Metroid2.pak",
        "Metroid3.pak",
        "Metroid4.pak",
        "metroid5.pak",
        "Metroid6.pak",
        "Metroid7.pak",
    ];

    let mut strgs = Vec::<Vec<String>>::new();

    for f in &filenames {
        let file_entry = gc_disc.find_file(f).unwrap();
        let pak = match *file_entry.file().unwrap() {
            structs::FstEntryFile::Pak(ref pak) => pak.clone(),
            structs::FstEntryFile::Unknown(ref reader) => reader.clone().read(()),
            _ => panic!(),
        };

        let resources = &pak.resources;

        for res in resources.iter() {
            if res.fourcc() != b"STRG".into() {
                continue;
            };

            let mut res = res.into_owned();
            let strg = res.kind.as_strg_mut().unwrap();
            let string_table = strg.string_tables.as_mut_vec()[0].strings.as_mut_vec();
            if string_table.len() != 3 {
                continue; // not a logbook entry
            }

            let entry_name = string_table[1].clone().into_string().replace("\u{0}", "");
            if entry_name.replace(" ", "") == "" {
                continue; // lore, but not logbook entry
            }

            if string_table[0].clone().into_string().contains(&"acquired!") {
                continue; // modal text box that coincidentally has 3 strings
            }

            let mut exists = false;
            for s in strgs.iter() {
                if s[1] == entry_name {
                    exists = true;
                    break;
                }
            }
            if exists {continue;}

            let mut strings = Vec::<String>::new();
            for string in string_table.iter_mut() {
                strings.push(string.clone().into_string().replace("\u{0}", ""));
            }
            strgs.push(strings);
        }
    }

    let logbook = format!("{:?}", strgs);
    let mut file = File::create(config.logbook_filename.as_ref().unwrap_or(&"logbook.json".to_string()))
        .map_err(|e| format!("Failed to create logbook file: {}", e))?;
    file.write_all(logbook.as_bytes())
        .map_err(|e| format!("Failed to write logbook file: {}", e))?;

    Ok(())
}

fn export_assets(_gc_disc: &mut structs::GcDisc, config: &PatchConfig)
    -> Result<(), String>
{
    let default_dir = &"assets".to_string();
    let asset_dir = config.export_asset_dir.as_ref().unwrap_or(default_dir);

    if !Path::new(&asset_dir).is_dir() {
        match fs::create_dir(&asset_dir) {
            Ok(()) => {},
            Err(error) => {
                panic!("Failed to create asset dir for exporting assets to: {}", error);
            },
        }
    }

    let bytes = include_bytes!("../extra_assets/phazon_suit_texure_1.txtr");
    let filename = "phazon_suit_texure_1.txtr";
    let mut file = File::create(format!("{}/{}", asset_dir, filename))
        .map_err(|e| format!("Failed to create asset file: {}", e))?;
    file.write_all(bytes)
        .map_err(|e| format!("Failed to write asset file: {}", e))?;

    let bytes = include_bytes!("../extra_assets/phazon_suit_texure_2.txtr");
    let filename = "phazon_suit_texure_2.txtr";
    let mut file = File::create(format!("{}/{}", asset_dir, filename))
        .map_err(|e| format!("Failed to create asset file: {}", e))?;
    file.write_all(bytes)
        .map_err(|e| format!("Failed to write asset file: {}", e))?;

    let bytes = include_bytes!("../extra_assets/nothing_texture.txtr"     );
    let filename = "nothing_texture.txtr";
    let mut file = File::create(format!("{}/{}", asset_dir, filename))
        .map_err(|e| format!("Failed to create asset file: {}", e))?;
    file.write_all(bytes)
        .map_err(|e| format!("Failed to write asset file: {}", e))?;

    let bytes = include_bytes!("../extra_assets/shiny-missile0.txtr"      );
    let filename = "shiny-missile0.txtr";
    let mut file = File::create(format!("{}/{}", asset_dir, filename))
        .map_err(|e| format!("Failed to create asset file: {}", e))?;
    file.write_all(bytes)
        .map_err(|e| format!("Failed to write asset file: {}", e))?;

    let bytes = include_bytes!("../extra_assets/shiny-missile1.txtr"      );
    let filename = "shiny-missile1.txtr";
    let mut file = File::create(format!("{}/{}", asset_dir, filename))
        .map_err(|e| format!("Failed to create asset file: {}", e))?;
    file.write_all(bytes)
        .map_err(|e| format!("Failed to write asset file: {}", e))?;

    let bytes = include_bytes!("../extra_assets/shiny-missile2.txtr"      );
    let filename = "shiny-missile2.txtr";
    let mut file = File::create(format!("{}/{}", asset_dir, filename))
        .map_err(|e| format!("Failed to create asset file: {}", e))?;
    file.write_all(bytes)
        .map_err(|e| format!("Failed to write asset file: {}", e))?;

    Ok(())
}


fn build_and_run_patches(gc_disc: &mut structs::GcDisc, config: &PatchConfig, version: Version)
    -> Result<(), String>
{
    let morph_ball_size = config.ctwk_config.morph_ball_size.clone().unwrap_or(1.0);
    let player_size = config.ctwk_config.player_size.clone().unwrap_or(1.0);

    let remove_ball_color = morph_ball_size < 0.999;
    let remove_control_disabler = player_size < 0.999 || morph_ball_size < 0.999;
    let move_item_loss_scan = player_size > 1.001;
    let mut rng = StdRng::seed_from_u64(config.seed);

    let mut level_data: HashMap<String, LevelConfig> = config.level_data.clone();
    let starting_room = SpawnRoomData::from_str(&config.starting_room);

    if config.shuffle_pickup_pos_all_rooms {
        for (pak_name, rooms) in pickup_meta::ROOM_INFO.iter() {
            let world = World::from_pak(pak_name).unwrap();

            if level_data.get(world.to_json_key()).is_none() {
                level_data.insert(world.to_json_key().to_string(), LevelConfig {
                        transports: HashMap::new(),
                        rooms: HashMap::new(),
                    }
                );
            }

            let level = level_data.get_mut(world.to_json_key()).unwrap();

            let mut items: Vec<PickupType> = Vec::new();
            for pt in PickupType::iter() {
                if !vec![
                    PickupType::IceBeam,
                    PickupType::WaveBeam,
                    PickupType::PlasmaBeam,
                    PickupType::Missile,
                    PickupType::ScanVisor,
                    PickupType::MorphBallBomb,
                    PickupType::PowerBomb,
                    PickupType::Flamethrower,
                    PickupType::ThermalVisor,
                    PickupType::ChargeBeam,
                    PickupType::SuperMissile,
                    PickupType::GrappleBeam,
                    PickupType::XRayVisor,
                    PickupType::IceSpreader,
                    PickupType::SpaceJumpBoots,
                    PickupType::MorphBall,
                    PickupType::BoostBall,
                    PickupType::SpiderBall,
                    PickupType::GravitySuit,
                    PickupType::VariaSuit,
                    PickupType::PhazonSuit,
                    PickupType::EnergyTank,
                    PickupType::HealthRefill,
                    PickupType::Wavebuster,
                    PickupType::ArtifactOfTruth,
                    PickupType::ArtifactOfStrength,
                    PickupType::ArtifactOfElder,
                    PickupType::ArtifactOfWild,
                    PickupType::ArtifactOfLifegiver,
                    PickupType::ArtifactOfWarrior,
                    PickupType::ArtifactOfChozo,
                    PickupType::ArtifactOfNature,
                    PickupType::ArtifactOfSun,
                    PickupType::ArtifactOfWorld,
                    PickupType::ArtifactOfSpirit,
                    PickupType::ArtifactOfNewborn,
                    PickupType::CombatVisor,
                    PickupType::PowerBeam,
                ].contains(&pt) {
                    continue;
                }
                items.push(pt.clone());
            }
            items.push(PickupType::Missile);
            items.push(PickupType::Missile);
            items.push(PickupType::Missile);
            items.push(PickupType::Missile);
            items.push(PickupType::Missile);
            items.push(PickupType::Missile);
            items.push(PickupType::Missile);
            items.push(PickupType::Missile);
            items.push(PickupType::Missile);
            items.push(PickupType::Missile);
            items.push(PickupType::Missile);
            items.push(PickupType::Missile);
            items.push(PickupType::PowerBomb);
            items.push(PickupType::PowerBomb);
            items.push(PickupType::EnergyTank);
            items.push(PickupType::EnergyTank);
            items.push(PickupType::EnergyTank);
            items.push(PickupType::EnergyTank);

            for room_info in rooms.iter() {
                let key = room_info.name.trim();
                if level.rooms.get(key).is_none() {
                    level.rooms.insert(key.to_string(), RoomConfig {
                            pickups: Some(vec![]),
                            extra_scans: None,
                            doors: None,
                            superheated: None,
                            remove_water: None,
                            submerge: None,
                            liquids: None,
                            spawn_position_override: None,
                            bounding_box_offset: None,
                            bounding_box_scale: None,
                            platforms: None,
                            blocks: None,
                            ambient_lighting_scale: None,
                            lock_on_points: None,
                        }
                    );
                }

                if level.rooms.get(key).unwrap().pickups.is_none() {
                    level.rooms.get_mut(key).unwrap().pickups = Some(vec![]);
                }

                if level.rooms.get_mut(key).unwrap().pickups.clone().unwrap().len() == 0 {
                    level.rooms.get_mut(key).unwrap().pickups = Some(
                        vec![
                            PickupConfig {
                                pickup_type: items.choose(&mut rng).unwrap().name().to_string(),
                                curr_increase: None,
                                max_increase: None,
                                model: None,
                                scan_text: None,
                                hudmemo_text: None,
                                respawn: None,
                                position: None,
                                modal_hudmemo: None,
                                jumbo_scan: None,
                            }
                        ]
                    );
                }
            }
        }
    }

    let frigate_done_room = {
        let mut destination_name = "Tallon:Landing Site";
        let frigate_level = level_data.get(World::FrigateOrpheon.to_json_key());
        if frigate_level.is_some() {
            let x = frigate_level.unwrap().transports.get(&"Frigate Escape Cutscene".to_string());
            if x.is_some() {
                destination_name = x.unwrap();
            }
        }

        SpawnRoomData::from_str(destination_name)
    };
    let essence_done_room = {
        let mut destination = None;
        let crater_level = level_data.get(World::ImpactCrater.to_json_key());
        if crater_level.is_some() {
            let x = crater_level.unwrap().transports.get(&"Essence Dead Cutscene".to_string());
            if x.is_some() {
                destination = Some(SpawnRoomData::from_str(x.unwrap()))
            }
        }

        destination
    };

    let artifact_totem_strings = build_artifact_temple_totem_scan_strings(&level_data, &mut rng, config.artifact_hints.clone());

    let show_starting_memo = config.starting_memo.is_some();

    let starting_memo = {
        if config.starting_memo.is_some() {
            Some(config.starting_memo.as_ref().unwrap().as_str())
        } else {
            None
        }
    };

    let (game_resources, pickup_hudmemos, pickup_scans, extra_scans, savw_scans_to_add, local_savw_scans_to_add, savw_scan_logbook_category, extern_models) =
        collect_game_resources(gc_disc, starting_memo, &config)?;

    let extern_models = &extern_models;
    let game_resources = &game_resources;
    let pickup_hudmemos = &pickup_hudmemos;
    let pickup_scans = &pickup_scans;
    let extra_scans = &extra_scans;

    let savw_scans_to_add = &savw_scans_to_add;
    let local_savw_scans_to_add = &local_savw_scans_to_add;
    let savw_scan_logbook_category = &savw_scan_logbook_category;

    
    // Remove unused artifacts from logbook
    let mut savw_to_remove_from_logbook: Vec<u32> = Vec::new();
    for i in 0..12 {
        let kind = i + 29;

        let exists = {
            let mut _exists = false;
            for (_, level) in level_data.iter() {
                if _exists {break;}
                for (_, room) in level.rooms.iter() {
                    if _exists {break;}
                    if room.pickups.is_none() { continue };
                    for pickup in room.pickups.as_ref().unwrap().iter() {
                        let pickup = PickupType::from_str(&pickup.pickup_type);
                        if pickup.kind() == kind {
                            _exists = true; // this artifact is placed somewhere in this world
                            break;
                        }
                    }
                }
            }

            let artifact_temple_layer_overrides = config.artifact_temple_layer_overrides.clone().unwrap_or(HashMap::new());
            for (key, value) in &artifact_temple_layer_overrides {
                let artifact_name = match kind {
                    33 => "lifegiver",
                    32 => "wild",
                    38 => "world",
                    37 => "sun",
                    31 => "elder",
                    39 => "spirit",
                    29 => "truth",
                    35 => "chozo",
                    34 => "warrior",
                    40 => "newborn",
                    36 => "nature",
                    30 => "strength",
                    _ => panic!("Unhandled artifact idx - '{}'", i),
                };

                if key.to_lowercase().contains(&artifact_name) {
                    _exists = _exists || *value; // if value is true, override
                    break;
                }
            }
            _exists
        };

        if exists {
            continue; // The artifact is in the game, or it's in another player's multiworld session
        }

        const ARTIFACT_TOTEM_SCAN_SCAN: &[ResourceInfo] = &[
            resource_info!("07_Over_Stonehenge Totem 1.SCAN"), // Truth
            resource_info!("07_Over_Stonehenge Totem 2.SCAN"), // Strength
            resource_info!("07_Over_Stonehenge Totem 3.SCAN"), // Elder
            resource_info!("07_Over_Stonehenge Totem 4.SCAN"), // Wild
            resource_info!("07_Over_Stonehenge Totem 5.SCAN"), // Lifegiver
            resource_info!("07_Over_Stonehenge Totem 6.SCAN"), // Warrior
            resource_info!("07_Over_Stonehenge Totem 7.SCAN"), // Chozo
            resource_info!("07_Over_Stonehenge Totem 8.SCAN"), // Nature
            resource_info!("07_Over_Stonehenge Totem 9.SCAN"), // Sun
            resource_info!("07_Over_Stonehenge Totem 10.SCAN"), // World
            resource_info!("07_Over_Stonehenge Totem 11.SCAN"), // Spirit
            resource_info!("07_Over_Stonehenge Totem 12.SCAN"), // Newborn
        ];

        savw_to_remove_from_logbook.push(
            ARTIFACT_TOTEM_SCAN_SCAN[i as usize].res_id
        );
    }
    let savw_to_remove_from_logbook = &savw_to_remove_from_logbook;

    let liquid_resources = collect_liquid_resources(gc_disc);
    let liquid_resources = &liquid_resources;

    // XXX These values need to out live the patcher
    let select_game_fmv_suffix = "A";
    let n = format!("Video/02_start_fileselect_{}.thp", select_game_fmv_suffix);
    let start_file_select_fmv = gc_disc.find_file(&n).unwrap().file().unwrap().clone();
    let n = format!("Video/04_fileselect_playgame_{}.thp", select_game_fmv_suffix);
    let file_select_play_game_fmv = gc_disc.find_file(&n).unwrap().file().unwrap().clone();

    let mut patcher = PrimePatcher::new();

    patcher.add_file_patch(b"opening.bnr", |file| patch_bnr(file, &config.game_banner));

    if let Some(flaahgra_music_files) = &config.flaahgra_music_files {
        const MUSIC_FILE_NAME: &[&[u8]] = &[
            b"Audio/rui_flaaghraR.dsp",
            b"Audio/rui_flaaghraL.dsp",
        ];
        for (file_name, music_file) in MUSIC_FILE_NAME.iter().zip(flaahgra_music_files.iter()) {
            patcher.add_file_patch(file_name, move |file| {
                *file = structs::FstEntryFile::ExternalFile(Box::new(music_file.clone()));
                Ok(())
            });
        }
    }

    // Patch Tweaks.pak
    if version == Version::NtscK {
        patcher.add_resource_patch(
            (&[ b"Tweaks.Pak" ], 0x37CE7FD6, FourCC::from_bytes(b"CTWK")), // Game.CTWK
            |res| patch_ctwk_game(res, &config.ctwk_config),
        );
        patcher.add_resource_patch(
            (&[ b"Tweaks.Pak" ], 0x26F1E0C1, FourCC::from_bytes(b"CTWK")), // Player.CTWK
            |res| patch_ctwk_player(res, &config.ctwk_config),
        );
        patcher.add_resource_patch(
            (&[ b"Tweaks.Pak" ], 0x8D698EC0, FourCC::from_bytes(b"CTWK")), // PlayerGun.CTWK
            |res| patch_ctwk_player_gun(res, &config.ctwk_config),
        );
        patcher.add_resource_patch(
            (&[ b"Tweaks.Pak" ], 0xFC2160E5, FourCC::from_bytes(b"CTWK")), // Ball.CTWK
            |res| patch_ctwk_ball(res, &config.ctwk_config),
        );
        patcher.add_resource_patch(
            (&[ b"Tweaks.Pak" ], 0x2DFB63BB, FourCC::from_bytes(b"CTWK")), // GuiColors.CTWK
            |res| patch_ctwk_gui_colors(res, &config.ctwk_config),
        );
    } else {
        patcher.add_resource_patch(
            resource_info!("Game.CTWK").into(),
            |res| patch_ctwk_game(res, &config.ctwk_config),
        );
        patcher.add_resource_patch(
            resource_info!("Player.CTWK").into(),
            |res| patch_ctwk_player(res, &config.ctwk_config),
        );
        patcher.add_resource_patch(
            resource_info!("PlayerGun.CTWK").into(),
            |res| patch_ctwk_player_gun(res, &config.ctwk_config),
        );
        patcher.add_resource_patch(
            resource_info!("Ball.CTWK").into(),
            |res| patch_ctwk_ball(res, &config.ctwk_config),
        );
        patcher.add_resource_patch(
            resource_info!("GuiColors.CTWK").into(),
            |res| patch_ctwk_gui_colors(res, &config.ctwk_config),
        );

        /* TODO: add more tweaks
        953a7c63.CTWK -> Game.CTWK
        264a4972.CTWK -> Player.CTWK
        f1ed8fd7.CTWK -> PlayerControls.CTWK
        3faec012.CTWK -> PlayerControls2.CTWK
        85ca11e9.CTWK -> PlayerRes.CTWK
        6907a32d.CTWK -> PlayerGun.CTWK
        33b3323a.CTWK -> GunRes.CTWK
        5ed56350.CTWK -> Ball.CTWK
        94c76ecd.CTWK -> Targeting.CTWK
        39ad28d3.CTWK -> CameraBob.CTWK
        5f24eff8.CTWK -> SlideShow.CTWK
        ed2e48a9.CTWK -> Gui.CTWK
        c9954e56.CTWK -> GuiColors.CTWK
        e66a4f86.CTWK -> AutoMapper.CTWK
        1d180d7c.CTWK -> Particle.CTWK
        */
    }

    patcher.add_resource_patch(
        resource_info!("FRME_CombatHud.FRME").into(),
        move |res| patch_combat_hud_color(res, &config.ctwk_config),
    );
    patcher.add_resource_patch(
        resource_info!("FRME_ScanHudFlat.FRME").into(),
        move |res| patch_combat_hud_color(res, &config.ctwk_config),
    );
    patcher.add_resource_patch(
        resource_info!("FRME_ScanHud.FRME").into(),
        move |res| patch_combat_hud_color(res, &config.ctwk_config),
    );
    patcher.add_resource_patch(
        resource_info!("FRME_MapScreen.FRME").into(),
        move |res| patch_combat_hud_color(res, &config.ctwk_config),
    );
    patcher.add_resource_patch(
        resource_info!("FRME_ThermalHud.FRME").into(),
        move |res| patch_combat_hud_color(res, &config.ctwk_config),
    );

    // Patch end sequence (player size)
    if config.ctwk_config.player_size.is_some() {
        patcher.add_scly_patch(
            resource_info!("01_endcinema.MREA").into(),
            move |ps, area| patch_samus_actor_size(ps, area, player_size),
        );
    }

    // Add hard-coded POI
    if config.qol_pickup_scans {
        patcher.add_scly_patch(
            resource_info!("01_over_mainplaza.MREA").into(), // Tallon Landing Site - Behind ship item
            move |ps, area| patch_remove_tangle_weed_scan_point(
                ps, area,
                vec![0x0000027E, 0x0000027F],
            ),
        );
        patcher.add_scly_patch(
            resource_info!("01_ice_plaza.MREA").into(), // Phen Shorelines - Scannable in tower
            move |ps, area| patch_add_poi(
                ps, area,
                game_resources,
                custom_asset_ids::SHORELINES_POI_SCAN,
                custom_asset_ids::SHORELINES_POI_STRG,
                [-98.0624, -162.3933, 28.5371],
            ),
        );
    }
    patcher.add_scly_patch(
        resource_info!("06_under_intro_freight.MREA").into(),
        move |ps, area| patch_add_poi(
            ps, area,
            game_resources,
            custom_asset_ids::CFLDG_POI_SCAN,
            custom_asset_ids::CFLDG_POI_STRG,
            [-44.0, 361.0, -120.0],
        ),
    );

    if config.qol_cutscenes == CutsceneMode::Competitive {
        patch_qol_competitive_cutscenes(&mut patcher, version);
    }

    if config.qol_cutscenes == CutsceneMode::Minor || config.qol_cutscenes == CutsceneMode::Major {
        patch_qol_minor_cutscenes(&mut patcher, version);
    }

    if config.qol_cutscenes == CutsceneMode::Major {
        patch_qol_major_cutscenes(&mut patcher);
        if !config.shuffle_pickup_position {
            patcher.add_scly_patch(
                resource_info!("07_ice_chapel.MREA").into(), // chapel of the elders
                move |ps, area| patch_remove_cutscenes(ps, area,
                    vec![0x000E0057], // Faster adult breakout
                    vec![0x000E019D, 0x000E019B], // keep fight start reposition for wavesun
                    true,
                ),
            );
        }
    }

    // Patch pickups
    let mut seed: u64 = 1;
    for (pak_name, rooms) in pickup_meta::ROOM_INFO.iter() {
        let world = World::from_pak(pak_name).unwrap();

        for room_info in rooms.iter() {
            if remove_control_disabler
            {
                patcher.add_scly_patch(
                    (pak_name.as_bytes(), room_info.room_id.to_u32()),
                    patch_remove_control_disabler,
                );
            }

            if config.remove_vanilla_blast_shields {
                patcher.add_scly_patch(
                    (pak_name.as_bytes(), room_info.room_id.to_u32()),
                    patch_remove_blast_shields,
                );
            }

            // Removed as this was letting the player unmorph in places they shouldn't
            // patcher.add_scly_patch(
            //     (pak_name.as_bytes(), room_info.room_id.to_u32()),
            //     patch_remove_visor_changer,
            // );

            if config.ctwk_config.player_size.is_some() {
                patcher.add_scly_patch(
                    (pak_name.as_bytes(), room_info.room_id.to_u32()),
                    move |ps, area| patch_samus_actor_size(ps, area, player_size),
                );
            }

            if config.force_vanilla_layout {continue;}

            // Remove objects patch
            if config.qol_cosmetic && !(config.shuffle_pickup_position && room_info.room_id.to_u32() == 0x40C548E9) {
                patcher.add_scly_patch((pak_name.as_bytes(), room_info.room_id.to_u32()), move |_, area| {
                    let layers = area.mrea().scly_section_mut().layers.as_mut_vec();
                    for otr in room_info.objects_to_remove {
                        layers[otr.layer as usize].objects.as_mut_vec()
                            .retain(|i| !otr.instance_ids.contains(&i.instance_id));
                    }
                    Ok(())
                });
            }

            if config.qol_cutscenes == CutsceneMode::Major && is_elevator(room_info.room_id.to_u32()) {
                patcher.add_scly_patch(
                    (pak_name.as_bytes(), room_info.room_id.to_u32()),
                    move |ps, area| patch_remove_cutscenes(
                        ps, area,
                        vec![],
                        vec![],
                        true,
                    ),
                );
            }

            // Get list of patches specified for this room
            let (pickups, scans, doors) = {
                let mut _pickups = Vec::new();
                let mut _scans = Vec::new();
                let mut _doors = HashMap::<u32, DoorConfig>::new();

                let level = level_data.get(world.to_json_key());
                if level.is_some() {
                    let room = level.unwrap().rooms.get(room_info.name.trim());
                    if room.is_some() {
                        let room = room.clone().unwrap();
                        if room.pickups.is_some() {
                            _pickups = room.pickups.clone().unwrap();
                        }

                        if room.extra_scans.is_some() {
                            _scans = room.extra_scans.clone().unwrap();
                        }

                        if room.doors.is_some() {
                            _doors = room.doors.clone().unwrap();
                        }

                        if room.superheated.is_some() {
                            patcher.add_scly_patch(
                                (pak_name.as_bytes(), room_info.room_id.to_u32()),
                                move |_ps, area| patch_deheat_room(_ps, area),
                            );

                            if room.superheated.clone().unwrap() {
                                patcher.add_scly_patch(
                                    (pak_name.as_bytes(), room_info.room_id.to_u32()),
                                    move |_ps, area| patch_superheated_room(_ps, area, config.heat_damage_per_sec),
                                );
                            }
                        }
                        
                        if room.spawn_position_override.is_some() {
                            patcher.add_scly_patch(
                                (pak_name.as_bytes(), room_info.room_id.to_u32()),
                                move |_ps, area| patch_spawn_point_position(_ps, area, room.spawn_position_override.unwrap(), false, false),
                            );
                        }

                        if room.bounding_box_offset.is_some() || room.bounding_box_scale.is_some() {
                            patcher.add_scly_patch(
                                (pak_name.as_bytes(), room_info.room_id.to_u32()),
                                move |_ps, area| patch_transform_bounding_box(_ps, area, room.bounding_box_offset.unwrap_or([0.0, 0.0, 0.0]), room.bounding_box_scale.unwrap_or([1.0, 1.0, 1.0])),
                            );
                        }

                        if room.platforms.is_some() {
                            for platform in room.platforms.as_ref().unwrap() {
                                patcher.add_scly_patch(
                                    (pak_name.as_bytes(), room_info.room_id.to_u32()),
                                    move |ps, area| patch_add_circle_platform(ps, area, game_resources, platform.position),
                                );
                            }
                        }

                        if room.blocks.is_some() {
                            for block in room.blocks.as_ref().unwrap() {
                                patcher.add_scly_patch(
                                    (pak_name.as_bytes(), room_info.room_id.to_u32()),
                                    move |ps, area| patch_add_block(ps, area, game_resources, block.position, block.scale),
                                );
                            }
                        }

                        if room.lock_on_points.is_some() {
                            for lock_on in room.lock_on_points.as_ref().unwrap() {
                                patcher.add_scly_patch(
                                    (pak_name.as_bytes(), room_info.room_id.to_u32()),
                                    move |ps, area| patch_lock_on_point(ps, area, game_resources, lock_on.position),
                                );
                            }
                        }

                        if room.ambient_lighting_scale.is_some() {
                            patcher.add_scly_patch(
                                (pak_name.as_bytes(), room_info.room_id.to_u32()),
                                move |_ps, area| patch_ambient_lighting(_ps, area, room.ambient_lighting_scale.unwrap()),
                            );
                        }

                        let submerge = room.submerge.clone().unwrap_or(false);
                        if room.remove_water.clone().unwrap_or(false) || submerge {
                            patcher.add_scly_patch(
                                (pak_name.as_bytes(), room_info.room_id.to_u32()),
                                move |_ps, area| patch_remove_water(_ps, area),
                            );
                        }

                        if submerge {
                            patcher.add_scly_patch(
                                (pak_name.as_bytes(), room_info.room_id.to_u32()),
                                move |_ps, area| patch_submerge_room(_ps, area, liquid_resources),
                            );
                        }

                        if room.liquids.is_some() {
                            for liquid in room.liquids.as_ref().unwrap().iter() {
                                patcher.add_scly_patch(
                                    (pak_name.as_bytes(), room_info.room_id.to_u32()),
                                    move |ps, area| patch_add_liquid(ps, area, liquid, liquid_resources),
                                );
                            }
                        }
                    }
                }
                (_pickups, _scans, _doors)
            };

            // Patch existing item locations
            let mut idx = 0;
            let pickups_config_len = pickups.len();
            for pickup_location in room_info.pickup_locations.iter() {
                let pickup = {
                    if idx >= pickups_config_len {
                        PickupConfig {
                            pickup_type: "Nothing".to_string(), // TODO: Could figure out the vanilla item instead
                            curr_increase: Some(0),
                            max_increase: Some(0),
                            position: None,
                            hudmemo_text: None,
                            scan_text: None,
                            model: None,
                            respawn: None,
                            modal_hudmemo: None,
                            jumbo_scan: None,
                        }
                    } else {
                        pickups[idx].clone() // TODO: cloning is suboptimal
                    }
                };

                let key = PickupHashKey {
                    level_id: world.mlvl(),
                    room_id: room_info.room_id.to_u32(),
                    pickup_idx: idx as u32,
                };

                let skip_hudmemos = {
                    if config.qol_cosmetic {
                        !(pickup.modal_hudmemo.clone().unwrap_or(false)) // make them modal if the client specified
                    } else {
                        false // leave them as they are in vanilla, modal
                    }
                };

                let hudmemo_delay = {
                    if pickup.modal_hudmemo.clone().unwrap_or(false) {
                        3.0 // manually specified modal hudmemos are 3s
                    } else {
                        0.0 // otherwise, leave unchanged from vanilla
                    }
                };

                // modify pickup, connections, hudmemo etc.
                patcher.add_scly_patch(
                    (pak_name.as_bytes(), room_info.room_id.to_u32()),
                    move |ps, area| modify_pickups_in_mrea(
                            ps,
                            area,
                            &pickup,
                            *pickup_location,
                            game_resources,
                            pickup_hudmemos,
                            pickup_scans,
                            key,
                            skip_hudmemos,
                            hudmemo_delay,
                            config.qol_pickup_scans,
                            extern_models,
                            config.shuffle_pickup_position,
                            config.seed + seed,
                            !config.starting_items.combat_visor && !config.starting_items.scan_visor && !config.starting_items.thermal_visor && !config.starting_items.xray,
                    )
                );

                idx = idx + 1;
                seed = seed + 1;
            }

            // Patch extra item locations
            while idx < pickups_config_len {
                let pickup = pickups[idx].clone(); // TODO: cloning is suboptimal

                let key = PickupHashKey {
                    level_id: world.mlvl(),
                    room_id: room_info.room_id.to_u32(),
                    pickup_idx: idx as u32,
                };

                let skip_hudmemos = {
                    if config.qol_cosmetic {
                        !(pickup.modal_hudmemo.clone().unwrap_or(false))
                    } else {
                        true
                    }
                };

                patcher.add_scly_patch(
                    (pak_name.as_bytes(), room_info.room_id.to_u32()),
                    move |_ps, area| patch_add_item(
                        _ps,
                        area,
                        &pickup,
                        game_resources,
                        pickup_hudmemos,
                        pickup_scans,
                        key,
                        skip_hudmemos,
                        extern_models,
                        config.shuffle_pickup_pos_all_rooms,
                        config.seed,
                        !config.starting_items.combat_visor && !config.starting_items.scan_visor && !config.starting_items.thermal_visor && !config.starting_items.xray,
                    ),
                );

                idx = idx + 1;
            }

            // Add extra scans (poi)
            idx = 0;
            for scan in scans.iter() {
                let scan = scan.clone();
                let key = PickupHashKey {
                    level_id: world.mlvl(),
                    room_id: room_info.room_id.to_u32(),
                    pickup_idx: idx as u32,
                };

                let (scan_id, strg_id) = extra_scans.get(&key).unwrap();

                patcher.add_scly_patch(
                    (pak_name.as_bytes(), room_info.room_id.to_u32()),
                    move |ps, area| patch_add_poi(ps, area, game_resources, scan_id.clone(), strg_id.clone(), scan.position),
                );

                if scan.combat_visible.unwrap_or(false) {
                    patcher.add_scly_patch(
                        (pak_name.as_bytes(), room_info.room_id.to_u32()),
                        move |ps, area| patch_add_scan_actor(ps, area, game_resources, scan.position, scan.rotation.unwrap_or(0.0)),
                    );
                }

                idx = idx + 1;
            }

            // Edit doors
            for (dock_num, door_config) in doors {
                let is_vertical_dock = vec![
                    (0x11BD63B7, 0), // Tower Chamber
                    (0x0D72F1F7, 1), // Tower of Light
                    (0xFB54A0CB, 4), // Hall of the Elders
                    (0xE1981EFC, 0), // Elder Chamber
                    (0x43E4CC25, 1), // Research Lab Hydra
                    (0x37BBB33C, 1), // Observatory Access
                    (0xD8E905DD, 1), // Research Core Access
                    (0x21B4BFF6, 1), // Research Lab Aether
                    (0x3F375ECC, 2), // Omega Research
                    (0xF517A1EA, 1), // Dynamo Access (Careful of Chozo room w/ same name)
                    (0x8A97BB54, 1), // Elite Research
                    (0xA20201D4, 0), // Security Access B (both doors)
                    (0xA20201D4, 1), // Security Access B (both doors)
                    (0x956F1552, 1), // Mine Security Station
                    (0xC50AF17A, 2), // Elite Control
                    (0x90709AAC, 1), // Ventilation Shaft
                ].contains(&(room_info.room_id.to_u32(), dock_num));

                // Find the corresponding traced info for this dock
                let mut maybe_door_location: Option<DoorLocation> = None;
                for dl in room_info.door_locations {
                    if dl.dock_number != dock_num {
                        continue;
                    }

                    let door_location = dl.clone();
                    maybe_door_location = Some(door_location.clone());

                    if door_config.shield_type.is_none() && door_config.blast_shield_type.is_none()
                    {
                        break;
                    }

                    if door_location.door_location.is_none() {
                        panic!("Tried to modify shield of door in {} on a dock which does not have a door", room_info.name);
                    }

                    // Patch door color and blast shield //
                    let mut door_type: Option<DoorType> = None;
                    if door_config.shield_type.is_some() {
                        let shield_name = door_config.shield_type.as_ref().unwrap();
                        door_type = DoorType::from_string(shield_name.to_string());
                        if door_type.is_none() {
                            panic!("Unexpected Shield Type - {}", shield_name);
                        }

                        if is_vertical_dock
                        {
                            door_type = Some(door_type.as_ref().unwrap().to_vertical());
                        }
                    }

                    let mut blast_shield_type: Option<BlastShieldType> = None;
                    if door_config.blast_shield_type.is_some() {
                        let blast_shield_name = door_config.blast_shield_type.as_ref().unwrap();
                        blast_shield_type = BlastShieldType::from_str(blast_shield_name);
                        if blast_shield_type.is_none() {
                            panic!("Unexpected Blast Shield Type - {}", blast_shield_name);
                        }
                    }

                    patcher.add_scly_patch(
                        (pak_name.as_bytes(), room_info.room_id.to_u32()),
                        move |ps, area| patch_door(
                            ps, area,
                            door_location,
                            door_type,
                            blast_shield_type,
                            game_resources,
                        )
                    );

                    if room_info.mapa_id != 0
                    {
                        let map_object_type: u32 = if door_type.is_some()
                        {
                            door_type.as_ref().unwrap().map_object_type()
                        }
                        else
                        {
                            let counterpart = blast_shield_type.as_ref().unwrap().door_type_counterpart();
                            if is_vertical_dock
                            {
                                counterpart.to_vertical().map_object_type()
                            }
                            else
                            {
                                counterpart.map_object_type()
                            }
                        };

                        patcher.add_resource_patch(
                            (&[pak_name.as_bytes()], room_info.mapa_id.to_u32(), b"MAPA".into()),
                            move |res| patch_map_door_icon(res, door_location, map_object_type)
                        );
                    }

                    break;
                }

                if maybe_door_location.is_none() {
                    panic!("Could not find dock #{} in '{}'", dock_num, room_info.name);
                }
                let door_location = maybe_door_location.unwrap();

                // If specified, patch this door's connection
                if door_config.destination.is_some() {
                    if door_location.door_location.is_none() {
                        panic!("Tried to shuffle door destination in {} on a dock which does not have a door", room_info.name);
                    }

                    // Get the resource info for premade scan point with destination info
                    let key = PickupHashKey {
                        level_id: world.mlvl(),
                        room_id: room_info.room_id.to_u32(),
                        pickup_idx: idx as u32,
                    };
                    idx = idx + 1;
                    let (dest_scan_id, dest_strg_id) = extra_scans.get(&key).unwrap();

                    // Get info about the destination room
                    let destination = door_config.destination.clone().unwrap();
                    let destination_room = SpawnRoomData::from_str(format!("{}:{}", world.to_str(), destination.room_name).as_str());
                    let source_room = SpawnRoomData::from_str(format!("{}:{}", world.to_str(), room_info.name).as_str());

                    if destination_room.mrea == source_room.mrea {
                        panic!("Dock destination cannot be in same room");
                    }

                    // Get size index (used for slowing door open)
                    // let destination_size_index = {
                    //     let mut size_index = -1.0;

                    //     for _room_info in rooms.iter() {
                    //         if _room_info.room_id == destination_room.mrea {
                    //             size_index = _room_info.size_index;
                    //         }
                    //     }

                    //     if size_index < 0.0 {
                    //         panic!("Failed size_index lookup");
                    //     }
                    //     size_index
                    // };

                    // Patch the current room to lead to the new destination room
                    patcher.add_scly_patch(
                        (pak_name.as_bytes(), room_info.room_id.to_u32()),
                        move |ps, area| patch_modify_dock(ps, area, game_resources, dest_scan_id.clone(), dest_strg_id.clone(), dock_num, destination_room.mrea_idx),
                    );

                    // Patch the destination room to "catch" the player with a teleporter at the same location as this room's dock

                    // Scale the height down a little so you can transition the dock without teleporting from OoB
                    let mut position: [f32;3] = door_location.dock_position.into();
                    let mut scale: [f32;3] = door_location.dock_scale.into();

                    if is_vertical_dock {
                        scale = [scale[0], scale[1], 0.01];
                    }
                    else {
                        let mut rotation = door_location.door_rotation.clone().unwrap();
                        position[2] -= 0.9;
                        let mut trigger_offset: f32 = 0.5;

                        if scale[2] > 4.0 && scale[2] < 8.0 { // if normal door
                            scale = [scale[0]*0.75, scale[1]*0.75, scale[2] - 1.8];
                        } else if scale[2] > 9.0 { // square frigate door
                            rotation[2] += 90.0;
                            trigger_offset = 0.58;
                        } else if scale[2] < 3.0 { // morph ball door
                            scale[0] = 0.5;
                            scale[1] = 0.5;
                            scale[2] = 0.1;
                        }

                        // Move teleport triggers slightly more into their respective rooms so that adjacent teleport triggers leading to the same room do not overlap
                        if rotation[2] >= 45.0 && rotation[2] < 135.0 {
                            // North
                            position[1] -= trigger_offset;
                        } else if (rotation[2] >= 135.0 && rotation[2] < 225.0) || (rotation[2] < -135.0 && rotation[2] > -225.0) {
                            // East
                            position[0] += trigger_offset;
                        } else if rotation[2] >= -135.0 && rotation[2] < -45.0 {
                            // South
                            position[1] += trigger_offset;
                        } else if rotation[2] >= -45.0 && rotation[2] < 45.0 {
                            // West
                            position[0] -= trigger_offset;
                        }
                    }

                    patcher.add_scly_patch(
                        (pak_name.as_bytes(), destination_room.mrea),
                        move |ps, area| patch_add_dock_teleport(
                            ps, area,
                            position,
                            scale,
                            destination.dock_num,
                            None, // If Some, override destination spawn point
                            Some(source_room.mrea_idx),
                        ),
                    );
                }
            }
        }
    }

    let (skip_frigate, skip_ending_cinematic) = make_elevators_patch(
        &mut patcher,
        &level_data,
        config.auto_enabled_elevators,
        player_size,
        config.force_vanilla_layout,
    );
    let skip_frigate = skip_frigate && starting_room.mlvl != World::FrigateOrpheon.mlvl();

    let mut smoother_teleports = false;
    for (_, level) in level_data.iter() {
        if smoother_teleports { break; }
        for (_, room) in level.rooms.iter() {
            if smoother_teleports { break; }
            if room.doors.is_none() { continue };
            for (_, door) in room.doors.as_ref().unwrap().iter() {
                if door.destination.is_some() {
                    smoother_teleports = true;
                    break;
                }
            }
        }
    }

    if smoother_teleports {
        patcher.add_file_patch(
            b"default.dol",
            |file| patch_dol(
                file,
                starting_room,
                version,
                config,
                remove_ball_color,
                true,
                config.skip_splash_screens,
            )
        );

        // Quarantine Monitor doesn't have a load trigger
        patcher.add_scly_patch(
            resource_info!("pickup04.MREA").into(),
            move |ps, area| patch_add_load_trigger(
                ps,
                area,
                [304.0, -606.0, 69.0],
                [5.0, 5.0, 5.0],
                0,
            ),
        );
    } else {
        patcher.add_file_patch(
            b"default.dol",
            |file| patch_dol(
                file,
                starting_room,
                version,
                config,
                remove_ball_color,
                false,
                config.skip_splash_screens,
            )
        );
    }

    let rel_config = create_rel_config_file(starting_room, config.quickplay);

    if skip_frigate {
        // remove frigate data to save time/space
        patcher.add_file_patch(b"Metroid1.pak", empty_frigate_pak);
    } else {
        // redirect end of frigate cutscene to room specified in layout
        patcher.add_scly_patch(
            resource_info!("01_intro_hanger.MREA").into(),
            move |_ps, area| patch_teleporter_destination(area, frigate_done_room),
        );

        if move_item_loss_scan {
            patcher.add_scly_patch(
                resource_info!("02_intro_elevator.MREA").into(),
                patch_move_item_loss_scan,
            );
        }
    }

    if essence_done_room.is_some() {
        // redirect end of crater cutscene to room specified in layout
        patcher.add_scly_patch(
            resource_info!("03f_crater.MREA").into(),
            move |_ps, area| patch_teleporter_destination(area, essence_done_room.unwrap()),
        );
    }

    gc_disc.add_file(
        "rel_config.bin",
        structs::FstEntryFile::ExternalFile(Box::new(rel_config)),
    )?;

    if !config.force_vanilla_layout {
        const ARTIFACT_TOTEM_SCAN_STRGS: &[ResourceInfo] = &[
            resource_info!("07_Over_Stonehenge Totem 5.STRG"), // Lifegiver
            resource_info!("07_Over_Stonehenge Totem 4.STRG"), // Wild
            resource_info!("07_Over_Stonehenge Totem 10.STRG"), // World
            resource_info!("07_Over_Stonehenge Totem 9.STRG"), // Sun
            resource_info!("07_Over_Stonehenge Totem 3.STRG"), // Elder
            resource_info!("07_Over_Stonehenge Totem 11.STRG"), // Spirit
            resource_info!("07_Over_Stonehenge Totem 1.STRG"), // Truth
            resource_info!("07_Over_Stonehenge Totem 7.STRG"), // Chozo
            resource_info!("07_Over_Stonehenge Totem 6.STRG"), // Warrior
            resource_info!("07_Over_Stonehenge Totem 12.STRG"), // Newborn
            resource_info!("07_Over_Stonehenge Totem 8.STRG"), // Nature
            resource_info!("07_Over_Stonehenge Totem 2.STRG"), // Strength
        ];
        for (res_info, strg_text) in ARTIFACT_TOTEM_SCAN_STRGS.iter().zip(artifact_totem_strings.iter()) {
            patcher.add_resource_patch(
                (*res_info).into(),
                move |res| patch_artifact_totem_scan_strg(res, &strg_text),
            );
        }
        patcher.add_scly_patch(
            resource_info!("07_stonehenge.MREA").into(),
            |_ps, area| patch_tournament_winners(_ps, area, game_resources)
        );
    }
    patcher.add_resource_patch(
        resource_info!("STRG_Main.STRG").into(),// 0x0552a456
        |res| patch_main_strg(res, version, &config.main_menu_message)
    );
    patcher.add_resource_patch(
        resource_info!("FRME_NewFileSelect.FRME").into(),
        patch_main_menu
    );
    patcher.add_resource_patch(
        resource_info!("STRG_Credits.STRG").into(),
        |res| patch_credits(res, version, config, &level_data)
    );
    patcher.add_scly_patch(
        resource_info!("07_stonehenge.MREA").into(),
        |ps, area| fix_artifact_of_truth_requirements(ps, area, config)
    );
    patcher.add_scly_patch(
        resource_info!("07_stonehenge.MREA").into(),
        |ps, area| patch_artifact_hint_availability(ps, area, config.artifact_hint_behavior)
    );

    patcher.add_resource_patch(
        resource_info!("TXTR_SaveBanner.TXTR").into(),
        patch_save_banner_txtr
    );

    if config.patch_power_conduits {
        patch_power_conduits(&mut patcher);
    }

    if config.remove_mine_security_station_locks {
        patcher.add_scly_patch(
            resource_info!("02_mines_shotemup.MREA").into(), // Mines Security Station
            remove_door_locks,
        );
    }

    if config.remove_hive_mecha {
        patch_hive_mecha(&mut patcher);
    }

    if config.power_bomb_arboretum_sandstone {
        patch_arboretum_sandstone(&mut patcher);
    }

    if config.incinerator_drone_config.is_some() {
        let incinerator_drone_config = config.incinerator_drone_config.clone().unwrap();

        let reset_contraption_minimum_time = incinerator_drone_config.contraption_start_delay_minimum_time.clone();
        let reset_contraption_random_time = incinerator_drone_config.contraption_start_delay_random_time.clone();
        let eye_stay_up_minimum_time = incinerator_drone_config.eye_stay_up_minimum_time.clone();
        let eye_stay_up_random_time = incinerator_drone_config.eye_stay_up_random_time.clone();
        let eye_wait_initial_minimum_time = incinerator_drone_config.eye_wait_initial_minimum_time.clone();
        let eye_wait_initial_random_time = incinerator_drone_config.eye_wait_initial_random_time.clone();
        let eye_wait_minimum_time = incinerator_drone_config.eye_wait_minimum_time.clone();
        let eye_wait_random_time = incinerator_drone_config.eye_wait_random_time.clone();

        patcher.add_scly_patch(
            resource_info!("03_monkey_lower.MREA").into(),
            move |_ps, area|
            patch_incinerator_drone_timer(area,
                CString::new("Time Contraption Start Delay").unwrap(),
                incinerator_drone_config.contraption_start_delay_minimum_time,
                incinerator_drone_config.contraption_start_delay_random_time,
            )
        );

        patcher.add_scly_patch(
            resource_info!("03_monkey_lower.MREA").into(),
            move |_ps, area|
            patch_incinerator_drone_timer(area,
                CString::new("Timer Reset Contraption").unwrap(),
                reset_contraption_minimum_time,
                reset_contraption_random_time,
            )
        );

        patcher.add_scly_patch(
            resource_info!("03_monkey_lower.MREA").into(),
            move |_ps, area|
            patch_incinerator_drone_timer(area,
                CString::new("Timer Eye Stay Up Time").unwrap(),
                eye_stay_up_minimum_time,
                eye_stay_up_random_time,
            )
        );

        patcher.add_scly_patch(
            resource_info!("03_monkey_lower.MREA").into(),
            move |_ps, area|
            patch_incinerator_drone_timer(area,
                CString::new("Timer Eye Wait (Initial)").unwrap(),
                eye_wait_initial_minimum_time,
                eye_wait_initial_random_time,
            )
        );

        patcher.add_scly_patch(
            resource_info!("03_monkey_lower.MREA").into(),
            move |_ps, area|
            patch_incinerator_drone_timer(area,
                CString::new("Timer Eye Wait").unwrap(),
                eye_wait_minimum_time,
                eye_wait_random_time,
            )
        );
    }

    if config.maze_seeds.is_some() {
        let mut maze_seeds = config.maze_seeds.clone().unwrap();
        maze_seeds.shuffle(&mut rng);
        patcher.add_resource_patch(
            resource_info!("DUMB_MazeSeeds.DUMB").into(),//0x5d88cac0
            move |res| patch_maze_seeds(res, maze_seeds.clone())
        );
    }

    patcher.add_resource_patch(
        resource_info!("!TalonOverworld_Master.SAVW").into(),
        move |res| patch_add_scans_to_savw(
            res,
            &savw_scans_to_add,
            &savw_scan_logbook_category,
            &savw_to_remove_from_logbook,
        ),
    );
    patcher.add_resource_patch(
        resource_info!("!TalonOverworld_Master.SAVW").into(),
        move |res| patch_add_scans_to_savw(
            res,
            &local_savw_scans_to_add[World::TallonOverworld as usize],
            &savw_scan_logbook_category,
            &savw_to_remove_from_logbook,
        ),
    );

    patcher.add_resource_patch(
        resource_info!("!RuinsWorld_Master.SAVW").into(),
        move |res| patch_add_scans_to_savw(
            res,
            &savw_scans_to_add,
            &savw_scan_logbook_category,
            &savw_to_remove_from_logbook,
        ),
    );
    patcher.add_resource_patch(
        resource_info!("!RuinsWorld_Master.SAVW").into(),
        move |res| patch_add_scans_to_savw(
            res,
            &local_savw_scans_to_add[World::ChozoRuins as usize],
            &savw_scan_logbook_category,
            &savw_to_remove_from_logbook,
        ),
    );

    patcher.add_resource_patch(
        resource_info!("!LavaWorld_Master.SAVW").into(),
        move |res| patch_add_scans_to_savw(
            res,
            &savw_scans_to_add,
            &savw_scan_logbook_category,
            &savw_to_remove_from_logbook,
        ),
    );
    patcher.add_resource_patch(
        resource_info!("!LavaWorld_Master.SAVW").into(),
        move |res| patch_add_scans_to_savw(
            res,
            &local_savw_scans_to_add[World::MagmoorCaverns as usize],
            &savw_scan_logbook_category,
            &savw_to_remove_from_logbook,
        ),
    );

    patcher.add_resource_patch(
        resource_info!("!IceWorld_Master.SAVW").into(),
        move |res| patch_add_scans_to_savw(
            res,
            &savw_scans_to_add,
            &savw_scan_logbook_category,
            &savw_to_remove_from_logbook,
        ),
    );
    patcher.add_resource_patch(
        resource_info!("!IceWorld_Master.SAVW").into(),
        move |res| patch_add_scans_to_savw(
            res,
            &local_savw_scans_to_add[World::PhendranaDrifts as usize],
            &savw_scan_logbook_category,
            &savw_to_remove_from_logbook,
        ),
    );

    patcher.add_resource_patch(
        resource_info!("!MinesWorld_Master.SAVW").into(),
        move |res| patch_add_scans_to_savw(
            res,
            &savw_scans_to_add,
            &savw_scan_logbook_category,
            &savw_to_remove_from_logbook,
        ),
    );
    patcher.add_resource_patch(
        resource_info!("!MinesWorld_Master.SAVW").into(),
        move |res| patch_add_scans_to_savw(
            res,
            &local_savw_scans_to_add[World::PhazonMines as usize],
            &savw_scan_logbook_category,
            &savw_to_remove_from_logbook,
        ),
    );

    patcher.add_resource_patch(
        resource_info!("!CraterWorld_Master.SAVW").into(),
        move |res| patch_add_scans_to_savw(
            res,
            &savw_scans_to_add,
            &savw_scan_logbook_category,
            &savw_to_remove_from_logbook,
        ),
    );
    patcher.add_resource_patch(
        resource_info!("!CraterWorld_Master.SAVW").into(),
        move |res| patch_add_scans_to_savw(
            res,
            &local_savw_scans_to_add[World::ImpactCrater as usize],
            &savw_scan_logbook_category,
            &savw_to_remove_from_logbook,
        ),
    );

    patcher.add_scly_patch(
        (starting_room.pak_name.as_bytes(), starting_room.mrea),
        move |ps, area| patch_starting_pickups(
            ps,
            area,
            &config.starting_items,
            show_starting_memo,
            &game_resources,
        )
    );

    if !skip_frigate {
        patcher.add_scly_patch(
            resource_info!("02_intro_elevator.MREA").into(),
            move |ps, area| patch_starting_pickups(
                ps,
                area,
                &config.item_loss_items,
                false,
                &game_resources,
            )
        );

        patcher.add_resource_patch(
            resource_info!("!Intro_Master.SAVW").into(),
            move |res| patch_add_scans_to_savw(
                res,
                &savw_scans_to_add,
                &savw_scan_logbook_category,
                &savw_to_remove_from_logbook,
            ),
        );
        patcher.add_resource_patch(
            resource_info!("!Intro_Master.SAVW").into(),
            move |res| patch_add_scans_to_savw(
                res,
                &local_savw_scans_to_add[World::FrigateOrpheon as usize],
                &savw_scan_logbook_category,
                &savw_to_remove_from_logbook,
            ),
        );

        if config.disable_item_loss {
            patcher.add_scly_patch(
                resource_info!("02_intro_elevator.MREA").into(),
                patch_disable_item_loss
            );
        }

        if !config.force_vanilla_layout {
            // Patch frigate so that it can be explored any direction without crashing or soft-locking
            patcher.add_scly_patch(
                resource_info!("01_intro_hanger_connect.MREA").into(),
                patch_post_pq_frigate
            );
            patcher.add_scly_patch(
                resource_info!("00h_intro_mechshaft.MREA").into(),
                patch_post_pq_frigate
            );
            patcher.add_scly_patch(
                resource_info!("04_intro_specimen_chamber.MREA").into(),
                patch_post_pq_frigate
            );
            patcher.add_scly_patch(
                resource_info!("06_intro_freight_lifts.MREA").into(),
                patch_post_pq_frigate
            );
            patcher.add_scly_patch(
                resource_info!("06_intro_to_reactor.MREA").into(),
                patch_post_pq_frigate
            );
            patcher.add_scly_patch(
                resource_info!("02_intro_elevator.MREA").into(),
                patch_post_pq_frigate
            );
            patcher.add_scly_patch(
                resource_info!("04_intro_specimen_chamber.MREA").into(),
                move |ps, res| patch_add_circle_platform(
                    ps,
                    res,
                    game_resources,
                    [43.0, -194.0, -44.0],
                ),
            );
            patcher.add_scly_patch(
                resource_info!("04_intro_specimen_chamber.MREA").into(),
                move |ps, res| patch_add_circle_platform(
                    ps,
                    res,
                    game_resources,
                    [39.0, -186.0, -41.0],
                ),
            );
            patcher.add_scly_patch(
                resource_info!("04_intro_specimen_chamber.MREA").into(),
                move |ps, res| patch_add_circle_platform(
                    ps,
                    res,
                    game_resources,
                    [36.0, -181.0, -39.0],
                ),
            );
            patcher.add_scly_patch(
                resource_info!("04_intro_specimen_chamber.MREA").into(),
                move |ps, res| patch_add_circle_platform(
                    ps,
                    res,
                    game_resources,
                    [36.0, -192.0, -39.0],
                ),
            );
        }
    }

    if !config.force_vanilla_layout {
        if starting_room.mrea != SpawnRoom::LandingSite.spawn_room_data().mrea || config.qol_cutscenes == CutsceneMode::Major {
            // If we have a non-default start point, patch the landing site to avoid
            // weirdness with cutscene triggers and the ship spawning.
            patcher.add_scly_patch(
                resource_info!("01_over_mainplaza.MREA").into(),
                patch_landing_site_cutscene_triggers
            );
        }
    }

    patch_heat_damage_per_sec(&mut patcher, config.heat_damage_per_sec);

    // Always patch out the white flash for photosensitive epileptics
    if version == Version::NtscU0_00 {
        patcher.add_scly_patch(
            resource_info!("03f_crater.MREA").into(),
            patch_essence_cinematic_skip_whitescreen
        );
    }
    if [Version::NtscU0_00, Version::NtscU0_02, Version::Pal].contains(&version) {
        patcher.add_scly_patch(
            resource_info!("03f_crater.MREA").into(),
            patch_essence_cinematic_skip_nomusic
        );
    }

    let mut boss_permadeath = false;
    if level_data.contains_key(World::ImpactCrater.to_json_key())
    {
        let transports = &level_data.get(World::ImpactCrater.to_json_key()).unwrap().transports;
        if transports.contains_key("Essence Dead Cutscene")
        {
            let destination = &transports.get("Essence Dead Cutscene").unwrap();
            if destination.trim().to_lowercase() != "credits"
            {
                boss_permadeath = true;
            }
        }
    }

    if config.qol_game_breaking {
        patch_qol_game_breaking(&mut patcher, version, config.force_vanilla_layout, player_size < 0.9);
        if boss_permadeath {
            patcher.add_scly_patch(
                resource_info!("03a_crater.MREA").into(),
                move |ps, area| patch_final_boss_permadeath(ps, area, game_resources)
            );
            patcher.add_scly_patch(
                resource_info!("03b_crater.MREA").into(),
                move |ps, area| patch_final_boss_permadeath(ps, area, game_resources)
            );
            patcher.add_scly_patch(
                resource_info!("03c_crater.MREA").into(),
                move |ps, area| patch_final_boss_permadeath(ps, area, game_resources)
            );
            patcher.add_scly_patch(
                resource_info!("03d_crater.MREA").into(),
                move |ps, area| patch_final_boss_permadeath(ps, area, game_resources)
            );
            patcher.add_scly_patch(
                resource_info!("03e_crater.MREA").into(),
                move |ps, area| patch_final_boss_permadeath(ps, area, game_resources)
            );
            // patcher.add_scly_patch(
            //     resource_info!("03e_f_crater.MREA").into(), // five
            //     move |ps, area| patch_final_boss_permadeath(ps, area, game_resources)
            // );
            patcher.add_scly_patch(
                resource_info!("03f_crater.MREA").into(), // lair
                move |ps, area| patch_final_boss_permadeath(ps, area, game_resources)
            );
            patcher.add_scly_patch(
                resource_info!("03e_f_crater.MREA").into(), // subchamber five
                move |ps, area| patch_subchamber_five_nintendont_fix(ps, area)
            );
            patcher.add_scly_patch(
                resource_info!("03e_f_crater.MREA").into(), // Subchamber five
                move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], true),
            );
            // patcher.add_scly_patch(
            //     resource_info!("03f_crater.MREA").into(), // lair
            //     move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
            // );
            patcher.add_scly_patch(
                resource_info!("03f_crater.MREA").into(), // lair
                move |ps, area| patch_add_dock_teleport(ps, area,
                    [42.955109, -287.172638, -278.084354], // source position
                    [75.0, 75.0, 50.0], // source scale
                    0, // destination dock #
                    Some([41.5365,-287.8581,-284.6025]),
                    None,
                )
            );
        }
    }

    // not only is this game-breaking, but it's nonsensical and counterintuitive, always fix //
    patcher.add_scly_patch(
        resource_info!("00i_mines_connect.MREA").into(), // Dynamo Access (Mines)
        move |ps, area| patch_spawn_point_position(ps, area, [0.0, 0.0, 0.0], true, false)
    );

    if config.qol_cosmetic {
        patch_qol_cosmetic(&mut patcher, skip_ending_cinematic, config.quickpatch);

        // Replace the FMVs that play when you select a file so each ISO always plays the only one.
        const SELECT_GAMES_FMVS: &[&[u8]] = &[
            b"Video/02_start_fileselect_A.thp",
            b"Video/02_start_fileselect_B.thp",
            b"Video/02_start_fileselect_C.thp",
            b"Video/04_fileselect_playgame_A.thp",
            b"Video/04_fileselect_playgame_B.thp",
            b"Video/04_fileselect_playgame_C.thp",
        ];
        for fmv_name in SELECT_GAMES_FMVS {
            let fmv_ref = if fmv_name[7] == b'2' {
                &start_file_select_fmv
            } else {
                &file_select_play_game_fmv
            };
            patcher.add_file_patch(fmv_name, move |file| {
                *file = fmv_ref.clone();
                Ok(())
            });
        }
    }

    if !config.force_vanilla_layout {
        patch_qol_logical(&mut patcher, config);
    }

    for (_boss_name, scale) in config.boss_sizes.iter() {
        let boss_name = _boss_name.to_lowercase().replace(" ", "").replace("_", "");
        let scale = *scale;
        if boss_name == "parasitequeen"
        {
            if !skip_frigate {
                patcher.add_scly_patch(
                    resource_info!("07_intro_reactor.MREA").into(),
                    move |_ps, area| patch_pq_scale(_ps, area, scale)
                );
            }
        }
        else if boss_name == "idrone" || boss_name == "incineratordrone" || boss_name == "zoid"
        {
            patcher.add_scly_patch(
                resource_info!("03_monkey_lower.MREA").into(),
                move |_ps, area| patch_idrone_scale(_ps, area, scale)
            );
        }
        else if boss_name == "flaahgra"
        {
            patcher.add_scly_patch(
                resource_info!("22_Flaahgra.MREA").into(),
                move |_ps, area| patch_flaahgra_scale(_ps, area, scale)
            );
        }
        else if boss_name == "adultsheegoth"
        {
            patcher.add_scly_patch(
                resource_info!("07_ice_chapel.MREA").into(),
                move |_ps, area| patch_sheegoth_scale(_ps, area, scale)
            );
        }
        else if boss_name == "thardus"
        {
            patcher.add_scly_patch(
                resource_info!("19_ice_thardus.MREA").into(),
                move |_ps, area| patch_thardus_scale(_ps, area, scale)
            );
        }
        else if boss_name == "elitepirate1"
        {
            patcher.add_scly_patch(
                resource_info!("05_mines_forcefields.MREA").into(),
                move |_ps, area| patch_elite_pirate_scale(_ps, area, scale)
            );
        }
        else if boss_name == "elitepirate2"
        {
            patcher.add_scly_patch(
                resource_info!("00i_mines_connect.MREA").into(),
                move |_ps, area| patch_elite_pirate_scale(_ps, area, scale)
            );
        }
        else if boss_name == "elitepirate3"
        {
            patcher.add_scly_patch(
                resource_info!("06_mines_elitebustout.MREA").into(),
                move |_ps, area| patch_elite_pirate_scale(_ps, area, scale)
            );
        }
        else if boss_name == "phazonelite"
        {
            patcher.add_scly_patch(
                resource_info!("03_mines.MREA").into(),
                move |_ps, area| patch_elite_pirate_scale(_ps, area, scale)
            );
        }
        else if boss_name == "omegapirate"
        {
            patcher.add_scly_patch(
                resource_info!("12_mines_eliteboss.MREA").into(),
                move |_ps, area| patch_omega_pirate_scale(_ps, area, scale)
            );
        }
        else if boss_name == "ridley" || boss_name == "metaridley"
        {
            patcher.add_scly_patch(
                resource_info!("07_stonehenge.MREA").into(),
                move |_ps, area| patch_ridley_scale(_ps, area, scale)
            );
        }
        else if boss_name == "exo" || boss_name == "metroidprime" || boss_name == "metroidprimeexoskeleton"
        {
            patcher.add_scly_patch(
                resource_info!("03a_crater.MREA").into(),
                move |_ps, area| patch_exo_scale(_ps, area, scale)
            );
            if scale > 1.7 {
                patcher.add_scly_patch(
                    resource_info!("03b_crater.MREA").into(),
                    move |_ps, area| patch_exo_scale(_ps, area, 1.7)
                );
            } else {
                patcher.add_scly_patch(
                    resource_info!("03b_crater.MREA").into(),
                    move |_ps, area| patch_exo_scale(_ps, area, scale)
                );
            }
            patcher.add_scly_patch(
                resource_info!("03c_crater.MREA").into(),
                move |_ps, area| patch_exo_scale(_ps, area, scale)
            );
            patcher.add_scly_patch(
                resource_info!("03d_crater.MREA").into(),
                move |_ps, area| patch_exo_scale(_ps, area, scale)
            );
            patcher.add_scly_patch(
                resource_info!("03e_crater.MREA").into(),
                move |_ps, area| patch_exo_scale(_ps, area, scale)
            );
        }
        else if boss_name == "essence" || boss_name == "metroidprimeessence"
        {
            patcher.add_scly_patch(
                resource_info!("03f_crater.MREA").into(),
                move |_ps, area| patch_essence_scale(_ps, area, scale)
            );
        }
        else if boss_name == "platedbeetle"
        {
            patcher.add_scly_patch(
                resource_info!("1a_morphball_shrine.MREA").into(),
                move |_ps, area| patch_garbeetle_scale(_ps, area, scale)
            );
        }
        else if boss_name == "cloakeddrone"
        {
            patcher.add_scly_patch(
                resource_info!("07_mines_electric.MREA").into(),
                move |_ps, area| patch_drone_scale(_ps, area, scale)
            );
        }
        else
        {
            panic!("Unexpected boss name {}", _boss_name);
        }
    }

    // remove doors
    if config.no_doors {
        for (pak_name, rooms) in pickup_meta::ROOM_INFO.iter() {
            for room_info in rooms.iter() {
                patcher.add_scly_patch(
                    (pak_name.as_bytes(), room_info.room_id.to_u32()),
                    move |ps, area| patch_remove_doors(ps, area)
                );
            
            }
        }
    }

    if config.suit_colors.is_some() {
        let suit_colors = config.suit_colors.as_ref().unwrap();
        let mut suit_textures = Vec::new();
        let mut angles = Vec::new();

        if suit_colors.power_deg.is_some() {
            suit_textures.push(POWER_SUIT_TEXTURES);
            angles.push(suit_colors.power_deg.clone().unwrap());
        }
        if suit_colors.varia_deg.is_some() {
            suit_textures.push(VARIA_SUIT_TEXTURES);
            angles.push(suit_colors.varia_deg.clone().unwrap());
        }
        if suit_colors.gravity_deg.is_some() {
            suit_textures.push(GRAVITY_SUIT_TEXTURES);
            angles.push(suit_colors.gravity_deg.clone().unwrap());
        }
        if suit_colors.phazon_deg.is_some() {
            suit_textures.push(PHAZON_SUIT_TEXTURES);
            angles.push(suit_colors.phazon_deg.clone().unwrap());
        }

        let mut complained: bool = false;
        if !Path::new(&config.cache_dir).is_dir() {
            match fs::create_dir(&config.cache_dir) {
                Ok(()) => {},
                Err(error) => {
                    println!("Failed to create cache dir for optimal suit rotation: {}", error);
                    complained = true;
                },
            }
        }
        for i in 0..suit_textures.len() {
            let angle = angles[i] % 360;
            if angle == 0 {
                continue;
            }
            let angle = angle as f32;

            let cache_subdir = format!("{}/{}", config.cache_dir, angle);
            if !Path::new(&cache_subdir).is_dir() {
                match fs::create_dir(cache_subdir) {
                    Ok(()) => {},
                    Err(error) => {
                        if !complained {
                            println!("Failed to create cache subdir for optimal suit rotation: {}", error);
                            complained = true;
                        }
                    },
                }
            }

            let matrix = huerotate_matrix(angle);
            for texture in suit_textures[i] {
                patcher.add_resource_patch((*texture).into(), move |res| {
                    let res_data;
                    let data;
                    let mut txtr: structs::Txtr = match &res.kind {
                        structs::ResourceKind::Unknown(_, _) => {
                            res_data = crate::ResourceData::new(res);
                            data = res_data.decompress().into_owned();
                            let mut reader = Reader::new(&data[..]);
                            reader.read(())
                        },
                        structs::ResourceKind::External(_, _) => {
                            res_data = crate::ResourceData::new_external(res);
                            data = res_data.decompress().into_owned();
                            let mut reader = Reader::new(&data[..]);
                            reader.read(())
                        },
                        _ => panic!("Unsupported resource kind for recoloring."),
                    };
                    let mut w = txtr.width as usize;
                    let mut h = txtr.height as usize;
                    for mipmap in txtr.pixel_data.as_mut_vec() {
                        let hash: u64 = calculate_hash(&mipmap.as_mut_vec().to_vec());
                        // Read file contents to RAM
                        let filename = format!("{}/{}/{}", config.cache_dir, angle, hash);
                        let file_ok = File::open(&filename).is_ok();
                        let file = File::open(&filename).ok();
                        if file_ok && file.is_some() {
                            let metadata = fs::metadata(&filename).expect("unable to read metadata");
                            let mut bytes = vec![0; metadata.len() as usize];
                            file.unwrap().read(&mut bytes)
                                .map_err(|e| format!("Failed to read cache file: {}", e))?;
                            *mipmap.as_mut_vec() = bytes;
                        }
                        else
                        {
                            let mut decompressed_bytes = vec![0u8; w * h * 4];
                            cmpr_decompress(&mipmap.as_mut_vec()[..], h, w, &mut decompressed_bytes[..]);
                            huerotate_in_place(&mut decompressed_bytes[..], w, h, matrix);
                            cmpr_compress(&(decompressed_bytes[..]), w, h, &mut mipmap.as_mut_vec()[..]);
                            // cache.insert(hash, mipmap.as_mut_vec().to_vec());
                            match File::create(filename) {
                                Ok(mut file) => {
                                    match file.write_all(&mipmap.as_mut_vec().to_vec()) {
                                        Ok(()) => {},
                                        Err(error) => {
                                            if !complained {
                                                println!("Failed to write cache file for optimal suit rotation: {}", error);
                                                complained = true;
                                            }
                                        },
                                    }
                                },
                                Err(error) => {
                                    if !complained {
                                        println!("Failed to create cache file for optimal suit rotation: {}", error);
                                        complained = true;
                                    }
                                },
                            }
                        }
                        w = w / 2;
                        h = h / 2;
                    }
                    let mut bytes = vec![];
                    txtr.write_to(&mut bytes).unwrap();
                    res.kind = structs::ResourceKind::External(bytes, b"TXTR".into());
                    res.compressed = false;
                    Ok(())
                })
            }
        }
    }

    if config.warp_to_start
    {
        const SAVE_STATIONS_ROOMS: &[ResourceInfo] = &[
            // Space Pirate Frigate
            resource_info!("06_intro_to_reactor.MREA"),
            // Chozo Ruins
            resource_info!("1_savestation.MREA"),
            resource_info!("2_savestation.MREA"),
            resource_info!("3_savestation.MREA"),
            // Phendrana Drifts
            resource_info!("mapstation_ice.MREA"),
            resource_info!("savestation_ice_b.MREA"),
            resource_info!("savestation_ice_c.MREA"),
            resource_info!("pickup01.MREA"),
            // Tallon Overworld
            resource_info!("01_over_mainplaza.MREA"),
            resource_info!("06_under_intro_save.MREA"),
            // Phazon Mines
            resource_info!("savestation_mines_a.MREA"),
            resource_info!("00_mines_savestation_c.MREA"),
            resource_info!("00_mines_savestation_d.MREA"),
            // Magmoor Caverns
            resource_info!("lava_savestation_a.MREA"),
            resource_info!("lava_savestation_b.MREA"),
            // Impact Crater
            resource_info!("00_crater_over_elev_j.MREA"),
        ];

        for save_station_room in SAVE_STATIONS_ROOMS.iter() {
            patcher.add_scly_patch(
                (*save_station_room).into(),
                move |ps, area| patch_save_station_for_warp_to_start(
                    ps,
                    area,
                    &game_resources,
                    starting_room,
                    version,
                    config.warp_to_start_delay_s,
                )
            );
        }

        patcher.add_resource_patch(
            resource_info!("STRG_MemoryCard.STRG").into(),// 0x19C3F7F7
            |res| patch_memorycard_strg(res, version)
        );
    }

    let time = Instant::now();
    patcher.run(gc_disc)?;
    println!("Created patches in {:?}", time.elapsed());

    Ok(())
}

fn patch_maze_seeds(res: &mut structs::Resource, seeds: Vec<u32>) -> Result<(), String> {
    let res = res.kind.as_dumb_mut();

    if let Some(res) = res {
        let mut seeds = seeds.into_iter().cycle();
        for i in 0..300 {
            res.data[i] = seeds.next().unwrap();
        }
    }

    Ok(())
}

/* For mipmapcache */
fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}
