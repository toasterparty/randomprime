use std::{
    ffi::CStr,
    collections::HashMap,
    fs::{File, OpenOptions},
    fs,
};

use clap::{
    Arg,
    App,
    crate_version,
};

use serde::{Serialize, Deserialize};

use crate::{
    starting_items::StartingItems,
    pickup_meta::PickupType,
    custom_assets::custom_asset_ids,
};

use reader_writer::{FourCC};

use structs::{res_id, ResId};

/*** Parsed Config (fn patch_iso) ***/

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum RunMode
{
    CreateIso,
    ExportLogbook,
    ExportAssets,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum IsoFormat
{
    Iso,
    Gcz,
    Ciso,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum ArtifactHintBehavior
{
    Default,
    None,
    All,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum CutsceneMode
{
    Original,
    Competitive,
    Minor,
    Major,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum Visor
{
    Combat,
    XRay,
    Scan,
    Thermal,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum Beam
{
    Power,
    Ice,
    Wave,
    Plasma,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GameBanner
{
    pub game_name: Option<String>,
    pub game_name_full: Option<String>,
    pub developer: Option<String>,
    pub developer_full: Option<String>,
    pub description: Option<String>,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PickupConfig
{
    #[serde(alias  = "type")]
    pub pickup_type: String,
    pub curr_increase: Option<i32>,
    pub max_increase: Option<i32>,
    pub model: Option<String>,
    pub scan_text: Option<String>,
    pub hudmemo_text: Option<String>,
    pub respawn: Option<bool>,
    pub position: Option<[f32;3]>,
    pub modal_hudmemo: Option<bool>,
    pub jumbo_scan: Option<bool>,
    pub destination: Option<String>,
    pub show_icon: Option<bool>,
    pub invisible_and_silent: Option<bool>,
    pub thermal_only: Option<bool>,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScanConfig
{
    pub position: [f32;3],
    pub combat_visible: Option<bool>,
    pub rotation: Option<f32>,
    pub is_red: Option<bool>,
    pub logbook_category: Option<u32>,
    pub logbook_title: Option<String>,
    pub text: String,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DoorDestination
{
    pub room_name: String,
    pub dock_num: u32,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DoorConfig
{
    #[serde(alias  = "type")]
    pub shield_type: Option<String>,
    pub blast_shield_type: Option<String>,
    pub destination: Option<DoorDestination>, // Must be in same area. Ex: "destination":"Main Plaza"
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SuitColors
{
    pub power_deg: Option<i16>,
    pub varia_deg: Option<i16>,
    pub gravity_deg: Option<i16>,
    pub phazon_deg: Option<i16>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DefaultGameOptions
{
    pub screen_brightness: Option<u32>,
    pub screen_offset_x: Option<i32>,
    pub screen_offset_y: Option<i32>,
    pub screen_stretch: Option<i32>,
    pub sound_mode: Option<u32>,
    pub sfx_volume: Option<u32>,
    pub music_volume: Option<u32>,
    pub visor_opacity: Option<u32>,
    pub helmet_opacity: Option<u32>,
    pub hud_lag: Option<bool>,
    pub reverse_y_axis: Option<bool>,
    pub rumble: Option<bool>,
    pub swap_beam_controls: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WaterConfig
{
    #[serde(alias = "type")]
    pub liquid_type: String,
    pub position: [f32;3],
    pub scale: [f32;3],
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlatformConfig
{
    pub position: [f32;3],
    pub rotation: Option<[f32;3]>,
    pub alt_platform: Option<bool>,
    pub xray_only: Option<bool>,
    pub thermal_only: Option<bool>,
    // pub scale: [f32;3],
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum GenericTexture
{
    Grass,     // TXTR_BE288047
    Crater,    // TXTR_8E899523
    Mine,      // TXTR_7D77CEE0
    Snow,      // TXTR_2E6E5FC1
    Sandstone, // TXTR_AA452C33
}

impl GenericTexture
{
    pub fn txtr(self) -> ResId::<res_id::TXTR> {
        let id = match self {
            GenericTexture::Grass     => 0xBE288047,
            GenericTexture::Crater    => 0x8E899523,
            GenericTexture::Mine      => 0x7D77CEE0,
            GenericTexture::Snow      => 0x2E6E5FC1,
            GenericTexture::Sandstone => 0xAA452C33,
        };

        ResId::new(id)
    }

    pub fn cmdl(self) -> ResId::<res_id::CMDL> {
        let id = match self {
            GenericTexture::Grass     => custom_asset_ids::BLOCK_COLOR_0,
            GenericTexture::Crater    => custom_asset_ids::BLOCK_COLOR_1,
            GenericTexture::Mine      => custom_asset_ids::BLOCK_COLOR_2,
            GenericTexture::Snow      => custom_asset_ids::BLOCK_COLOR_3,
            GenericTexture::Sandstone => custom_asset_ids::BLOCK_COLOR_4,
        };

        id
    }

    pub fn dependencies(self) -> Vec<(u32, FourCC)> {
        vec![
            (self.cmdl().to_u32(), FourCC::from_bytes(b"CMDL")),
            (self.txtr().to_u32(), FourCC::from_bytes(b"TXTR")),
        ]
    }

    pub fn iter() -> impl Iterator<Item = GenericTexture>
    {
        [
            GenericTexture::Grass,
            GenericTexture::Crater,
            GenericTexture::Mine,
            GenericTexture::Snow,
            GenericTexture::Sandstone,
        ].iter().map(|i| *i)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlockConfig
{
    pub position: [f32;3],
    pub scale: Option<[f32;3]>,
    pub texture: Option<GenericTexture>,
    // pub rotation: [f32;3],
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EscapeSequenceConfig
{
    // pub time_s: f32,
    pub start_trigger_pos: [f32;3],
    pub start_trigger_scale: [f32;3],
    pub stop_trigger_pos: [f32;3],
    pub stop_trigger_scale: [f32;3],
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraHintConfig
{
    pub trigger_pos: [f32;3],
    pub trigger_scale: [f32;3],
    pub camera_pos: [f32;3],
    pub camera_rot: [f32;3],

    /**
        enum class EBallCameraBehaviour {
            Default,
            FreezeLookPosition, // Unused
            HintBallToCam,
            HintInitializePosition,
            HintFixedPosition,
            HintFixedTransform,
            PathCameraDesiredPos, // Unused
            PathCamera,
            SpindleCamera
        };
     */
    pub behavior: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LockOnPoint
{
    pub position: [f32;3],
    pub is_grapple: Option<bool>,
    pub no_lock: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TriggerConfig
{
    pub position: [f32;3],
    pub scale: [f32;3],
    pub force: Option<[f32;3]>,
    pub damage_type: Option<String>,
    pub damage_amount: Option<f32>,
}

// None = 0,
// PerspLin = 2,
// PerspExp = 4,
// PerspExp2 = 5,
// PerspRevExp = 6,
// PerspRevExp2 = 7,
// OrthoLin = 10,
// OrthoExp = 12,
// OrthoExp2 = 13,
// OrthoRevExp = 14,
// OrthoRevExp2 = 15,

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FogConfig
{
    pub mode: u32,
    pub explicit: bool,
    pub color: [f32;4], // RGBA
    pub range: [f32;2], // X, Y
    pub color_delta: Option<f32>,
    pub range_delta: Option<[f32;2]>,
    pub keep_original: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RepositionConfig
{
    pub trigger_position: [f32;3],
    pub trigger_scale: [f32;3],
    pub destination_position: [f32;3],
    pub destination_rotation: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HudmemoConfig
{
    pub trigger_position: [f32;3],
    pub trigger_scale: [f32;3],
    pub text: String,
    pub disable_on_enter: Option<bool>, // default - true
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum EnviornmentalEffect {
    None,
    Snow,
    Rain,
    Bubbles,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoomConfig
{
    pub superheated: Option<bool>,
    pub remove_water: Option<bool>,
    pub submerge: Option<bool>,
	pub map_default_state: Option<structs::MapaObjectVisibilityMode>,
    pub liquids: Option<Vec<WaterConfig>>,
    pub pickups: Option<Vec<PickupConfig>>,
    pub extra_scans: Option<Vec<ScanConfig>>,
    pub doors: Option<HashMap<u32, DoorConfig>>,
    pub spawn_position_override: Option<[f32;3]>,
    pub bounding_box_offset: Option<[f32;3]>,
    pub bounding_box_scale: Option<[f32;3]>,
    pub platforms: Option<Vec<PlatformConfig>>,
    pub camera_hints: Option<Vec<CameraHintConfig>>,
    pub blocks: Option<Vec<BlockConfig>>,
    pub lock_on_points: Option<Vec<LockOnPoint>>,
    pub triggers: Option<Vec<TriggerConfig>>,
    pub fog: Option<FogConfig>,
    pub ambient_lighting_scale: Option<f32>, // 1.0 is default lighting
    pub enviornmental_effect: Option<EnviornmentalEffect>,
    pub initial_enviornmental_effect: Option<f32>,
    pub initial_thermal_heat_level: Option<f32>,
    pub xray_fog_distance: Option<f32>,
    pub escape_sequences: Option<Vec<EscapeSequenceConfig>>,
    pub repositions: Option<Vec<RepositionConfig>>,
    pub hudmemos: Option<Vec<HudmemoConfig>>,
    pub enabled_layers: Option<Vec<u32>>,
    pub disabled_layers: Option<Vec<u32>>,
    pub delete_ids: Option<Vec<u32>>,
    pub audio_override: Option<HashMap<String, String>>, // key=instance_id, value=/audio/min_phazonL.dsp|/audio/min_phazonR.dsp
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LevelConfig
{
    pub transports: HashMap<String, String>,
    pub rooms: HashMap<String, RoomConfig>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CtwkConfig
{
    pub fov: Option<f32>,
    pub player_size: Option<f32>,
    pub morph_ball_size: Option<f32>,
    pub easy_lava_escape: Option<bool>,
    pub move_while_scan: Option<bool>,
    pub scan_range: Option<f32>,
    pub bomb_jump_height: Option<f32>,
    pub bomb_jump_radius: Option<f32>,
    pub grapple_beam_speed: Option<f32>,
    pub aim_assist_angle: Option<f32>,
    pub gravity: Option<f32>,
    pub ice_break_timeout: Option<f32>,
    pub ice_break_jump_count: Option<u32>,
    pub ground_friction: Option<f32>,
    pub coyote_frames: Option<u32>,
    pub move_during_free_look: Option<bool>,
    pub recenter_after_freelook: Option<bool>,
    pub max_speed: Option<f32>,
    pub max_acceleration: Option<f32>,
    pub space_jump_impulse: Option<f32>,
    pub vertical_space_jump_accel: Option<f32>,
    pub horizontal_space_jump_accel: Option<f32>,
    pub eye_offset: Option<f32>,
    pub toggle_free_look: Option<bool>,
    pub two_buttons_for_free_look: Option<bool>,
    pub disable_dash: Option<bool>,
    pub varia_damage_reduction: Option<f32>,
    pub gravity_damage_reduction: Option<f32>,
    pub phazon_damage_reduction: Option<f32>,
    pub hardmode_damage_mult: Option<f32>,
    pub hardmode_weapon_mult: Option<f32>,
    pub turn_speed: Option<f32>,
    pub underwater_fog_distance: Option<f32>,
    pub step_up_height: Option<f32>,
    pub allowed_jump_time: Option<f32>,
    pub allowed_space_jump_time: Option<f32>,
    pub min_space_jump_window: Option<f32>,
    pub max_space_jump_window: Option<f32>,
    pub min_jump_time: Option<f32>,
    pub min_space_jump_time: Option<f32>,
    pub falling_space_jump: Option<bool>,
    pub impulse_space_jump: Option<bool>,

    // PlayerGun.CTWK
    pub gun_position: Option<[f32;3]>, // offset
    pub gun_damage: Option<f32>,
    pub gun_cooldown: Option<f32>,

    // Ball.CTWK
    pub max_translation_accel: Option<f32>,
    pub translation_friction: Option<f32>,
    pub translation_max_speed: Option<f32>,
    pub ball_forward_braking_accel: Option<f32>,
    pub ball_gravity: Option<f32>,
    pub ball_water_gravity: Option<f32>,
    pub boost_drain_time: Option<f32>,
    pub boost_min_charge_time: Option<f32>,
    pub boost_min_rel_speed_for_damage: Option<f32>,
    pub boost_charge_time0: Option<f32>,
    pub boost_charge_time1: Option<f32>,
    pub boost_charge_time2: Option<f32>,
    pub boost_incremental_speed0: Option<f32>,
    pub boost_incremental_speed1: Option<f32>,
    pub boost_incremental_speed2: Option<f32>,

    // GuiColors.CTWK
    pub hud_color: Option<[f32;3]>, // RGB, 0 - 1.0
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IncineratorDroneConfig {
    pub contraption_start_delay_minimum_time: Option<f32>,
    pub contraption_start_delay_random_time: Option<f32>,
    pub eye_stay_up_minimum_time: Option<f32>,
    pub eye_stay_up_random_time: Option<f32>,
    pub eye_wait_initial_minimum_time: Option<f32>,
    pub eye_wait_initial_random_time: Option<f32>,
    pub eye_wait_minimum_time: Option<f32>,
    pub eye_wait_random_time: Option<f32>,
    pub reset_contraption_minimum_time: Option<f32>,
    pub reset_contraption_random_time: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HallOfTheEldersBombSlotCoversConfig {
    pub wave: Option<BombSlotCover>,
    pub ice: Option<BombSlotCover>,
    pub plasma: Option<BombSlotCover>,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum BombSlotCover {
    Wave,
    Ice,
    Plasma,
}

#[derive(Debug, Serialize)]
pub struct PatchConfig
{
    pub run_mode: RunMode,
    pub logbook_filename: Option<String>,
    pub export_asset_dir: Option<String>,
    pub extern_assets_dir: Option<String>,
    pub seed: u64,

    pub force_vanilla_layout: bool,

    #[serde(skip_serializing)]
    pub input_iso: memmap::Mmap,
    pub iso_format: IsoFormat,
    #[serde(skip_serializing)]
    pub output_iso: File,

    pub qol_cutscenes: CutsceneMode,
    pub qol_game_breaking: bool,
    pub qol_cosmetic: bool,
    pub qol_pickup_scans: bool,

    pub phazon_elite_without_dynamo: bool,
    pub main_plaza_door: bool,
    pub backwards_labs: bool,
    pub backwards_frigate: bool,
    pub backwards_upper_mines: bool,
    pub backwards_lower_mines: bool,
    pub patch_power_conduits: bool,
    pub remove_mine_security_station_locks: bool,
    pub remove_hive_mecha: bool,
    pub power_bomb_arboretum_sandstone: bool,
    pub visible_bounding_box: bool,

    pub incinerator_drone_config: Option<IncineratorDroneConfig>,
    pub hall_of_the_elders_bomb_slot_covers: Option<HallOfTheEldersBombSlotCoversConfig>,
    pub maze_seeds: Option<Vec<u32>>,

    #[serde(skip_serializing)] // stop racers from peeking at locations
    pub level_data: HashMap<String, LevelConfig>,

    pub strg: HashMap<String, Vec<String>>, // "<decimal asset ID>": <non-null terminated table of strings>

    pub starting_room: String,
    pub starting_memo: Option<String>,
    pub spring_ball: bool,
    pub warp_to_start: bool,
    pub warp_to_start_delay_s: f32,

    pub automatic_crash_screen: bool,
    pub etank_capacity: u32,
    pub shuffle_pickup_position: bool,
    pub shuffle_pickup_pos_all_rooms: bool,
    pub remove_vanilla_blast_shields: bool,
    pub nonvaria_heat_damage: bool,
    pub heat_damage_per_sec: f32,
    pub staggered_suit_damage: bool,
    pub item_max_capacity: HashMap<PickupType, u32>,
    // Use RoomConfig::map_default_state instead of global map_default_state
    pub map_default_state: structs::MapState,
    pub auto_enabled_elevators: bool,
    pub multiworld_dol_patches: bool,
    pub update_hint_state_replacement: Option<Vec<u8>>,
    pub quiet: bool,

    pub starting_items: StartingItems,
    pub item_loss_items: StartingItems,
    pub disable_item_loss: bool,
    pub starting_visor: Visor,
    pub starting_beam: Beam,

    pub artifact_hint_behavior: ArtifactHintBehavior,

    #[serde(skip_serializing)]
    pub flaahgra_music_files: Option<[nod_wrapper::FileWrapper; 2]>,

    pub skip_splash_screens: bool,
    pub default_game_options: Option<DefaultGameOptions>,
    pub suit_colors: Option<SuitColors>,
    pub force_fusion: bool,
    pub cache_dir: String,

    pub quickplay: bool,
    pub quickpatch: bool,

    pub game_banner: GameBanner,
    pub comment: String,
    pub main_menu_message: String,

    pub credits_string: Option<String>,
    pub results_string: Option<String>,
    pub artifact_hints: Option<HashMap<String,String>>, // e.g. "Strength":"This item can be found in Ruined Fountain"
    pub required_artifact_count: Option<u32>,
    pub artifact_temple_layer_overrides: Option<HashMap<String,bool>>,
    pub no_doors: bool,
    pub boss_sizes: HashMap<String,f32>,
    pub ctwk_config: CtwkConfig,
}

/*** Un-Parsed Config (doubles as JSON input specification) ***/

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Preferences
{
    skip_splash_screens: Option<bool>,
    default_game_options: Option<DefaultGameOptions>,
    suit_colors: Option<SuitColors>,
    force_fusion: Option<bool>,
    cache_dir: Option<String>,

    qol_game_breaking: Option<bool>,
    qol_cosmetic: Option<bool>,
    qol_cutscenes: Option<String>,
    qol_pickup_scans: Option<bool>,

    map_default_state: Option<String>,
    artifact_hint_behavior: Option<String>,
    automatic_crash_screen: Option<bool>,
    visible_bounding_box: Option<bool>,

    trilogy_disc_path: Option<String>,
    quickplay: Option<bool>,
    quickpatch: Option<bool>,
    quiet: Option<bool>,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GameConfig
{
    starting_room: Option<String>,
    starting_memo: Option<String>,
    spring_ball: Option<bool>,
    warp_to_start: Option<bool>,
    warp_to_start_delay_s: Option<f32>,

    shuffle_pickup_position: Option<bool>,
    shuffle_pickup_pos_all_rooms: Option<bool>,
    remove_vanilla_blast_shields: Option<bool>,
    nonvaria_heat_damage: Option<bool>,
    staggered_suit_damage: Option<bool>,
    heat_damage_per_sec: Option<f32>,
    auto_enabled_elevators: Option<bool>,
    multiworld_dol_patches: Option<bool>,
    update_hint_state_replacement: Option<Vec<u8>>,

    starting_items: Option<StartingItems>,
    item_loss_items: Option<StartingItems>,
    disable_item_loss: Option<bool>,
    starting_visor: Option<String>,
    starting_beam: Option<String>,

    etank_capacity: Option<u32>,
    item_max_capacity: Option<HashMap<String,u32>>,

    phazon_elite_without_dynamo: Option<bool>,
    main_plaza_door: Option<bool>,
    backwards_labs: Option<bool>,
    backwards_frigate: Option<bool>,
    backwards_upper_mines: Option<bool>,
    backwards_lower_mines: Option<bool>,
    patch_power_conduits: Option<bool>,
    remove_mine_security_station_locks: Option<bool>,
    remove_hive_mecha: Option<bool>,
    power_bomb_arboretum_sandstone: Option<bool>,

    incinerator_drone_config: Option<IncineratorDroneConfig>,
    maze_seeds: Option<Vec<u32>>,
    hall_of_the_elders_bomb_slot_covers: Option<HallOfTheEldersBombSlotCoversConfig>,

    game_banner: Option<GameBanner>,
    comment: Option<String>,
    main_menu_message: Option<String>,

    credits_string: Option<String>,
    results_string: Option<String>,
    artifact_hints: Option<HashMap<String,String>>, // e.g. "Strength":"This item can be found in Ruined Fountain"
    artifact_temple_layer_overrides: Option<HashMap<String,bool>>,
    required_artifact_count: Option<u32>,
    no_doors: Option<bool>, // Remove every door from the game
    boss_sizes: Option<HashMap<String,f32>>,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PatchConfigPrivate
{
    run_mode: Option<String>,
    logbook_filename: Option<String>,
    export_asset_dir: Option<String>,
    input_iso: Option<String>,
    output_iso: Option<String>,
    force_vanilla_layout: Option<bool>,
    extern_assets_dir: Option<String>,
    seed: Option<u64>,

    #[serde(default)]
    preferences: Preferences,

    #[serde(default)]
    game_config: GameConfig,

    #[serde(default)]
    tweaks: CtwkConfig,

    #[serde(default)]
    level_data: HashMap<String, LevelConfig>,
    
    #[serde(default)]
    strg: HashMap<String, Vec<String>>, // "<decimal asset ID>": <non-null terminated table of strings>
}

/*** Parse Patcher Input ***/

/// Takes a string of jsonc content and returns a comment free version
/// which should parse fine as regular json.
/// Nested block comments are supported.
/// preserve_locations will replace most comments with spaces, so that JSON parsing
/// errors should point to the right location.
pub fn strip_jsonc_comments(jsonc_input: &str, preserve_locations: bool) -> String {
    let mut json_output = String::new();

    let mut block_comment_depth: u8 = 0;
    let mut is_in_string: bool = false; // Comments cannot be in strings

    for line in jsonc_input.split('\n') {
        let mut last_char: Option<char> = None;
        for cur_char in line.chars() {
            // Check whether we're in a string
            if block_comment_depth == 0 && last_char != Some('\\') && cur_char == '"' {
                is_in_string = !is_in_string;
            }

            // Check for line comment start
            if !is_in_string && last_char == Some('/') && cur_char == '/' {
                last_char = None;
                if preserve_locations {
                    json_output.push_str("  ");
                }
                break; // Stop outputting or parsing this line
            }
            // Check for block comment start
            if !is_in_string && last_char == Some('/') && cur_char == '*' {
                block_comment_depth += 1;
                last_char = None;
                if preserve_locations {
                    json_output.push_str("  ");
                }
            // Check for block comment end
            } else if !is_in_string && last_char == Some('*') && cur_char == '/' {
                if block_comment_depth > 0 {
                    block_comment_depth -= 1;
                }
                last_char = None;
                if preserve_locations {
                    json_output.push_str("  ");
                }
            // Output last char if not in any block comment
            } else {
                if block_comment_depth == 0 {
                    if let Some(last_char) = last_char {
                        json_output.push(last_char);
                    }
                } else {
                    if preserve_locations {
                        json_output.push_str(" ");
                    }
                }
                last_char = Some(cur_char);
            }
        }

        // Add last char and newline if not in any block comment
        if let Some(last_char) = last_char {
            if block_comment_depth == 0 {
                json_output.push(last_char);
            } else if preserve_locations {
                json_output.push(' ');
            }
        }

        // Remove trailing whitespace from line
        while json_output.ends_with(' ') {
            json_output.pop();
        }
        json_output.push('\n');
    }

    json_output
}

impl PatchConfig
{
    pub fn from_json(json: &str) -> Result<Self, String>
    {
        let json_config: PatchConfigPrivate = serde_json::from_str(strip_jsonc_comments(json, true).as_str())
            .map_err(|e| format!("JSON parse failed: {}", e))?;
        json_config.parse()
    }

    pub fn from_cli_options() -> Result<Self, String>
    {
        let matches = App::new("randomprime ISO patcher")
            .version(crate_version!())
            .arg(Arg::with_name("input iso path")
                .long("input-iso")
                .takes_value(true))
            .arg(Arg::with_name("output iso path")
                .long("output-iso")
                .takes_value(true))
            .arg(Arg::with_name("extern assets dir")
                .long("extern-assets-dir")
                .takes_value(true))
            .arg(Arg::with_name("profile json path")
                .long("profile")
                .help("Path to JSON file with patch configuration (cli config takes priority). See documentation for details.")
                .takes_value(true))
            .arg(Arg::with_name("force vanilla layout")
                .long("force-vanilla-layout")
                .help("use this to play the vanilla game, but with a custom size factor"))
            .arg(Arg::with_name("qol game breaking")
                .long("qol-game-breaking")
                .help("Fix soft locks and crashes that retro didn't bother addressing"))
            .arg(Arg::with_name("qol cosmetic")
                .long("qol-cosmetic")
                .help("Patch cutscenes to fix continuity errors and UI to improve QoL without affecting IGT or the story"))
            .arg(Arg::with_name("qol cutscenes")
                .long("qol-cutscenes")
                .help("Original, Competitive, Minor, Major")
                .takes_value(true))
            .arg(Arg::with_name("starting room")
                .long("starting-room")
                .help("Room which the player starts their adventure from. Format - <world>:<room name>, where <world> is [Frigate|Tallon|Chozo|Magmoor|Phendrana|Mines|Crater]")
                .takes_value(true))
            .arg(Arg::with_name("starting memo")
                .long("starting-memo")
                .help("String which is shown to the player after they start a new save file")
                .takes_value(true))
            .arg(Arg::with_name("spring ball")
                .long("spring-ball")
                .help("Allows player to use spring ball when bombs are acquired"))
            .arg(Arg::with_name("warp to start")
                .long("warp-to-start")
                .help("Allows player to warp to start from any save station"))
            .arg(Arg::with_name("automatic crash screen")
                .long("automatic-crash-screen")
                .help("Makes the crash screen appear without any button combination required"))
            .arg(Arg::with_name("etank capacity")
                .long("etank-capacity")
                .help("Set the etank capacity and base health")
                .takes_value(true))
            .arg(Arg::with_name("nonvaria heat damage")
                .long("nonvaria-heat-damage")
                .help("If the Varia Suit has not been collect, heat damage applies"))
            .arg(Arg::with_name("heat damage per sec")
                .long("heat-damage-per-sec")
                .help("Set the heat damage per seconds spent in a superheated room")
                .takes_value(true))
            .arg(Arg::with_name("staggered suit damage")
                .long("staggered-suit-damage")
                .help(concat!("The suit damage reduction is determinted by the number of suits ",
                                "collected rather than the most powerful one collected.")))
            .arg(Arg::with_name("map default state")
                .long("map-default-state")
                .help("Change the default state of map for each world (Either default, visible or visited)")
                .takes_value(true))
            .arg(Arg::with_name("auto enabled elevators")
                .long("auto-enabled-elevators")
                .help("Every elevator will be automatically enabled without scaning its terminal"))
            .arg(Arg::with_name("artifact hint behavior")
                .long("artifact-hint-behavior")
                .help("Set the behavior of artifact temple hints. Can be 'all', 'none', or 'default' (vanilla)")
                .takes_value(true))
            .arg(Arg::with_name("trilogy disc path")
                .long("flaahgra-music-disc-path")
                .help(concat!("Location of a ISO of Metroid Prime Trilogy. If provided the ",
                                "Flaahgra fight music will be used to replace the original"))
                .takes_value(true))
            .arg(Arg::with_name("quiet")
                .long("quiet")
                .help("Don't print the progress messages"))
            .arg(Arg::with_name("main menu message")
                .long("main-menu-message")
                .hidden(true)
                .takes_value(true))
            .arg(Arg::with_name("starting items")
                .long("starting-items")
                .takes_value(true)
                .validator(|s| s.parse::<u64>().map(|_| ())
                                            .map_err(|_| "Expected an integer".to_string())))
            .arg(Arg::with_name("item loss items")
                .long("item-loss-items")
                .takes_value(true)
                .validator(|s| s.parse::<u64>().map(|_| ())
                                            .map_err(|_| "Expected an integer".to_string())))
            .arg(Arg::with_name("quickplay")
                .long("quickplay")
                .hidden(true))
            .arg(Arg::with_name("quickpatch")
                .long("quickpatch")
                .hidden(true))
            .arg(Arg::with_name("text file comment")
                .long("text-file-comment")
                .hidden(true)
                .takes_value(true))
            .get_matches();

        let mut patch_config = if matches.is_present("profile json path") {
            let json_path = matches.value_of("profile json path").unwrap();
            let cli_json_config_raw: &str = &fs::read_to_string(json_path)
                .map_err(|e| format!("Could not read JSON file: {}", e)).unwrap();

            serde_json::from_str( strip_jsonc_comments(cli_json_config_raw, true).as_str())
                .map_err(|e| format!("JSON parse failed: {}", e))?
        } else {
            PatchConfigPrivate::default()
        };

        macro_rules! populate_config_bool {
            ($matches:expr; $($name:expr => $cfg:expr,)*) => {
                $(if $matches.is_present($name) {
                    $cfg = Some(true);
                })*
            };
        }

        // bool
        populate_config_bool!(matches;
            "force vanilla layout" => patch_config.force_vanilla_layout,
            "qol game breaking" => patch_config.preferences.qol_game_breaking,
            "qol cosmetic" => patch_config.preferences.qol_cosmetic,
            "qol scans" => patch_config.preferences.qol_pickup_scans,
            "automatic crash screen" => patch_config.preferences.automatic_crash_screen,
            "quickplay" => patch_config.preferences.quickplay,
            "quickpatch" => patch_config.preferences.quickpatch,
            "quiet" => patch_config.preferences.quiet,
            "nonvaria heat damage" => patch_config.game_config.nonvaria_heat_damage,
            "staggered suit damage" => patch_config.game_config.staggered_suit_damage,
            "auto enabled elevators" => patch_config.game_config.auto_enabled_elevators,
            "spring ball" => patch_config.game_config.spring_ball,
            "warp to start" => patch_config.game_config.warp_to_start,
        );

        // string
        if let Some(input_iso_path) = matches.value_of("input iso path") {
            patch_config.input_iso  = Some(input_iso_path.to_string());
        }
        if let Some(output_iso_path) = matches.value_of("output iso path") {
            patch_config.output_iso = Some(output_iso_path.to_string());
        }
        if let Some(extern_assets_dir) = matches.value_of("extern assets dir") {
            patch_config.extern_assets_dir = Some(extern_assets_dir.to_string());
        }
        if let Some(map_default_state) = matches.value_of("map default state") {
            patch_config.preferences.map_default_state = Some(map_default_state.to_string());
        }
        if let Some(artifact_behavior) = matches.value_of("artifact hint behavior") {
            patch_config.preferences.artifact_hint_behavior = Some(artifact_behavior.to_string());
        }
        if let Some(trilogy_disc_path) = matches.value_of("trilogy disc path") {
            patch_config.preferences.trilogy_disc_path = Some(trilogy_disc_path.to_string());
        }
        if let Some(starting_room) = matches.value_of("starting room") {
            patch_config.game_config.starting_room = Some(starting_room.to_string());
        }
        if let Some(qol_cutscenes) = matches.value_of("qol cutscenes") {
            patch_config.preferences.qol_cutscenes = Some(qol_cutscenes.to_string());
        }

        // integer/float
        if let Some(s) = matches.value_of("seed") {
            patch_config.seed = Some(s.parse::<u64>().unwrap());
        }
        if let Some(damage) = matches.value_of("heat damage per sec") {
            patch_config.game_config.heat_damage_per_sec = Some(damage.parse::<f32>().unwrap());
        }
        if let Some(etank_capacity) = matches.value_of("etank capacity") {
            patch_config.game_config.etank_capacity = Some(etank_capacity.parse::<u32>().unwrap());
        }
        if let Some(warp_to_start_delay_s) = matches.value_of("warp to start delay") {
            patch_config.game_config.warp_to_start_delay_s = Some(warp_to_start_delay_s.parse::<f32>().unwrap());
        }

        // custom
        if let Some(starting_items_str) = matches.value_of("starting items") {
            patch_config.game_config.starting_items = Some(
                StartingItems::from_u64(starting_items_str.parse::<u64>().unwrap())
            );
        }
        if let Some(item_loss_items_str) = matches.value_of("item loss items") {
            patch_config.game_config.item_loss_items = Some(
                StartingItems::from_u64(item_loss_items_str.parse::<u64>().unwrap())
            );
        }

        patch_config.parse()
    }
}

impl PatchConfigPrivate
{
    fn parse(&self) -> Result<PatchConfig, String>
    {
        let run_mode = {
            if self.run_mode.is_some() {
                match self.run_mode.as_ref().unwrap().to_lowercase().trim() {
                    "create_iso" => RunMode::CreateIso,
                    "export_logbook" => RunMode::ExportLogbook,
                    "export_assets" => RunMode::ExportAssets,
                    _ => panic!("Unsupported run mode: {}", self.run_mode.as_ref().unwrap())
                }
            } else {
                RunMode::CreateIso
            }
        };

        let input_iso_path = self.input_iso.as_deref().unwrap_or("prime.iso");
        let input_iso_file = File::open(input_iso_path.trim())
            .map_err(|e| format!("Failed to open {}: {}", input_iso_path, e))?;

        let input_iso = unsafe { memmap::Mmap::map(&input_iso_file) }
            .map_err(|e| format!("Failed to open {}: {}", input_iso_path,  e))?;

        let output_iso_path = self.output_iso.as_deref().unwrap_or("prime_out.iso");

        let output_iso = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&output_iso_path)
            .map_err(|e| format!("Failed to open {}: {}", output_iso_path, e))?;

        let iso_format = if output_iso_path.ends_with(".gcz") {
            IsoFormat::Gcz
        } else if output_iso_path.ends_with(".ciso") {
            IsoFormat::Ciso
        } else {
            IsoFormat::Iso
        };

        let force_vanilla_layout = self.force_vanilla_layout.unwrap_or(false);

        let artifact_hint_behavior = {
            let artifact_hint_behavior_string = self.preferences.artifact_hint_behavior
                .as_deref()
                .unwrap_or("all")
                .trim()
                .to_lowercase();
            if artifact_hint_behavior_string == "all" {
                ArtifactHintBehavior::All
            } else if artifact_hint_behavior_string == "none" {
                ArtifactHintBehavior::None
            } else if artifact_hint_behavior_string == "default" {
                ArtifactHintBehavior::Default
            } else {
                Err(format!(
                    "Unhandled artifact hint behavior - '{}'",
                    artifact_hint_behavior_string
                ))?
            }
        };

        let map_default_state = {
            let map_default_state_string = self.preferences.map_default_state
                                            .as_deref()
                                            .unwrap_or("default")
                                            .trim()
                                            .to_lowercase();
            match &map_default_state_string[..] {
                "default" => structs::MapState::Default,
                "visited" => structs::MapState::Visited,
                "visible" => structs::MapState::Visible,
                _ => Err(format!(
                    "Unhandled map default state - '{}'",
                    map_default_state_string
                ))?,
            }
       };

        let flaahgra_music_files = self.preferences.trilogy_disc_path.as_ref()
            .map(|path| extract_flaahgra_music_files(path))
            .transpose()?;

        let mut item_max_capacity = match &self.game_config.item_max_capacity {
            Some(max_capacity) => {
                max_capacity.iter()
                    .map(|(name, capacity) | (PickupType::from_str(name), *capacity))
                    .collect()
            },
            None => HashMap::new(),
        };
        if !item_max_capacity.contains_key(&PickupType::EnergyTank) && !force_vanilla_layout {
            item_max_capacity.insert(PickupType::EnergyTank, 200);
        }

        let qol_game_breaking = self.preferences.qol_game_breaking.unwrap_or(!force_vanilla_layout);
        let qol_cosmetic = self.preferences.qol_cosmetic.unwrap_or(!force_vanilla_layout);
        let qol_pickup_scans = self.preferences.qol_pickup_scans.unwrap_or(!force_vanilla_layout);
        let qol_cutscenes = match self.preferences.qol_cutscenes.as_ref().unwrap_or(&"original".to_string()).to_lowercase().trim() {
            "original" => CutsceneMode::Original,
            "competitive" => CutsceneMode::Competitive,
            "minor" => CutsceneMode::Minor,
            "major" => CutsceneMode::Major,
            _ => panic!("Unknown cutscene mode {}", self.preferences.qol_cutscenes.as_ref().unwrap()),
        };

        let starting_room = {
            if force_vanilla_layout {
                "Frigate:Exterior Docking Hangar".to_string()
            } else {
                self.game_config.starting_room.clone().unwrap_or("Tallon:Landing Site".to_string())
            }
        };

        let starting_items: StartingItems = {
            if force_vanilla_layout {
                StartingItems::from_u64(2188378143)
            } else {
                self.game_config.starting_items.clone().unwrap_or_else(|| StartingItems::from_u64(1))
            }
        };

        let default_starting_visor = if starting_items.combat_visor {
            "combat"
        } else if starting_items.thermal_visor {
            "thermal"
        } else if starting_items.xray {
            "xray"
        } else if starting_items.scan_visor {
            "scan"
        } else {
            "scan"
        };

        let starting_visor =match self.game_config.starting_visor.as_ref().unwrap_or(&default_starting_visor.to_string()).to_lowercase().trim() {
            "combat" => Visor::Combat,
            "scan" => Visor::Scan,
            "thermal" => Visor::Thermal,
            "xray" => Visor::XRay,
            _ => panic!("Unknown starting visor {}", self.game_config.starting_visor.as_ref().unwrap()),
        };

        let default_starting_beam = if starting_items.power_beam {
            "power"
        } else if starting_items.plasma {
            "plasma"
        } else if starting_items.ice {
            "ice"
        } else if starting_items.wave {
            "wave"
        } else {
            "power"
        };

        let starting_beam =match self.game_config.starting_beam.as_ref().unwrap_or(&default_starting_beam.to_string()).to_lowercase().trim() {
            "power" => Beam::Power,
            "ice" => Beam::Ice,
            "wave" => Beam::Wave,
            "plasma" => Beam::Plasma,
            _ => panic!("Unknown starting beam {}", self.game_config.starting_beam.as_ref().unwrap()),
        };

        let spring_ball = self.game_config.spring_ball.unwrap_or(false);
        let warp_to_start = self.game_config.warp_to_start.unwrap_or(false);
        let main_menu_message = {
            if force_vanilla_layout {
                "".to_string()
            } else {
                self.game_config.main_menu_message.clone().unwrap_or_else(|| "randomprime".to_string())
            }
        };

        let credits_string = {
            if force_vanilla_layout {
                Some("".to_string())
            } else {
                self.game_config.credits_string.clone()
            }
        };

        let results_string = {
            if force_vanilla_layout {
                Some("".to_string())
            } else {
                self.game_config.results_string.clone()
            }
        };

        Ok(PatchConfig {
            run_mode,
            logbook_filename: self.logbook_filename.clone(),
            export_asset_dir: self.export_asset_dir.clone(),
            input_iso,
            iso_format,
            output_iso,
            force_vanilla_layout,

            seed: self.seed.unwrap_or(123),
            extern_assets_dir: self.extern_assets_dir.clone(),

            level_data: self.level_data.clone(),
            strg: self.strg.clone(),

            qol_game_breaking,
            qol_cosmetic,
            qol_cutscenes,
            qol_pickup_scans,

            phazon_elite_without_dynamo: self.game_config.phazon_elite_without_dynamo.unwrap_or(true),
            main_plaza_door: self.game_config.main_plaza_door.unwrap_or(true),
            backwards_labs: self.game_config.backwards_labs.unwrap_or(true),
            backwards_frigate: self.game_config.backwards_frigate.unwrap_or(true),
            backwards_upper_mines: self.game_config.backwards_upper_mines.unwrap_or(true),
            backwards_lower_mines: self.game_config.backwards_lower_mines.unwrap_or(false),
            patch_power_conduits: self.game_config.patch_power_conduits.unwrap_or(false),
            remove_mine_security_station_locks: self.game_config.remove_mine_security_station_locks.unwrap_or(false),
            remove_hive_mecha: self.game_config.remove_hive_mecha.unwrap_or(false),
            power_bomb_arboretum_sandstone: self.game_config.power_bomb_arboretum_sandstone.unwrap_or(false),


            incinerator_drone_config: self.game_config.incinerator_drone_config.clone(),
            maze_seeds: self.game_config.maze_seeds.clone(),
            hall_of_the_elders_bomb_slot_covers: self
                .game_config
                .hall_of_the_elders_bomb_slot_covers
                .clone(),

            automatic_crash_screen: self.preferences.automatic_crash_screen.unwrap_or(true),
            visible_bounding_box: self.preferences.visible_bounding_box.unwrap_or(false),
            artifact_hint_behavior,
            flaahgra_music_files,
            suit_colors: self.preferences.suit_colors.clone(),
            force_fusion: self.preferences.force_fusion.clone().unwrap_or(false),
            cache_dir: self.preferences.cache_dir.clone().unwrap_or("cache".to_string()),
            skip_splash_screens: self.preferences.skip_splash_screens.unwrap_or(false),
            default_game_options: self.preferences.default_game_options.clone(),
            quiet: self.preferences.quiet.unwrap_or(false),
            quickplay: self.preferences.quickplay.unwrap_or(false),
            quickpatch: self.preferences.quickpatch.unwrap_or(false),

            starting_room,
            starting_memo: self.game_config.starting_memo.clone(),
            spring_ball,
            warp_to_start,
            warp_to_start_delay_s: self.game_config.warp_to_start_delay_s.unwrap_or(0.0),

            shuffle_pickup_position: self.game_config.shuffle_pickup_position.unwrap_or(false),
            shuffle_pickup_pos_all_rooms: self.game_config.shuffle_pickup_pos_all_rooms.unwrap_or(false),
            remove_vanilla_blast_shields: self.game_config.remove_vanilla_blast_shields.unwrap_or(false),
            nonvaria_heat_damage: self.game_config.nonvaria_heat_damage.unwrap_or(false),
            staggered_suit_damage: self.game_config.staggered_suit_damage.unwrap_or(false),
            heat_damage_per_sec: self.game_config.heat_damage_per_sec.unwrap_or(10.0),
            auto_enabled_elevators: self.game_config.auto_enabled_elevators.unwrap_or(false),
            multiworld_dol_patches: self.game_config.multiworld_dol_patches.unwrap_or(false),
            update_hint_state_replacement: self.game_config.update_hint_state_replacement.clone(),
            artifact_temple_layer_overrides: self.game_config.artifact_temple_layer_overrides.clone(),
            no_doors: self.game_config.no_doors.unwrap_or(false),
            boss_sizes: self.game_config.boss_sizes.clone().unwrap_or(HashMap::new()),
            map_default_state,

            starting_items,
            item_loss_items: self.game_config.item_loss_items.clone()
            .unwrap_or_else(|| StartingItems::from_u64(1)),
            disable_item_loss: self.game_config.disable_item_loss.unwrap_or(true),
            starting_visor,
            starting_beam,

            etank_capacity: self.game_config.etank_capacity.unwrap_or(100),
            item_max_capacity: item_max_capacity,

            game_banner: self.game_config.game_banner.clone().unwrap_or_default(),
            comment: self.game_config.comment.clone().unwrap_or(String::new()),
            main_menu_message,

            credits_string,
            results_string,
            artifact_hints: self.game_config.artifact_hints.clone(),
            required_artifact_count: self.game_config.required_artifact_count.clone(),

            ctwk_config: self.tweaks.clone(),
        })
    }
}

/*** Helper Methods ***/

pub fn extract_flaahgra_music_files(iso_path: &str) -> Result<[nod_wrapper::FileWrapper; 2], String>
{
    let res = (|| {
        let dw = nod_wrapper::DiscWrapper::new(iso_path)?;
        Ok([
            dw.open_file(CStr::from_bytes_with_nul(b"rui_flaaghraR.dsp\0").unwrap())?,
            dw.open_file(CStr::from_bytes_with_nul(b"rui_flaaghraL.dsp\0").unwrap())?,
        ])
    })();
    res.map_err(|s: String| format!("Failed to extract Flaahgra music files: {}", s))
}
