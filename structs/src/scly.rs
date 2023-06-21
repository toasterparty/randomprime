use auto_struct_macros::auto_struct;

use reader_writer::{FourCC, LCow, RoArray, LazyArray, Readable, Reader, Writable};

use std::io;
use std::borrow::Cow;
use std::fmt;

use crate::scly_props;


#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Scly<'r>
{
    #[auto_struct(expect = FourCC::from_bytes(b"SCLY"))]
    magic: FourCC,

    pub unknown: u32,

    #[auto_struct(derive = layers.len() as u32)]
    layer_count: u32,

    #[auto_struct(derive_from_iter = layers.iter()
            .map(&|i: LCow<SclyLayer>| i.size() as u32))]
    #[auto_struct(init = (layer_count as usize, ()))]
    _layer_sizes: RoArray<'r, u32>,

    #[auto_struct(init = (layer_count as usize, ()))]
    pub layers: LazyArray<'r, SclyLayer<'r>>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct SclyLayer<'r>
{
    pub unknown: u8,

    #[auto_struct(derive = objects.len() as u32)]
    object_count: u32,
    // TODO: Consider using DiffList here. Maybe requires profiling to decide...

    #[auto_struct(init = (object_count as usize, ()))]
    pub objects: LazyArray<'r, SclyObject<'r>>,

    #[auto_struct(pad_align = 32)]
    _pad: (),
}

impl<'r> SclyLayer<'r>
{
    pub fn new() -> SclyLayer<'r>
    {
        SclyLayer {
            unknown: 0,
            objects: vec![].into(),
        }
    }
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct SclyObject<'r>
{
    #[auto_struct(derive = property_data.object_type())]
    object_type: u8,

    #[auto_struct(derive = (8 + connections.size() + property_data.size()) as u32)]
    instance_size: u32,

    pub instance_id: u32,

    #[auto_struct(derive = connections.len() as u32)]
    connection_count: u32,
    #[auto_struct(init = (connection_count as usize, ()))]
    pub connections: LazyArray<'r, Connection>,

    #[auto_struct(init = (object_type, (instance_size - 8) as usize - connections.size()))]
    pub property_data: SclyProperty<'r>,
}

macro_rules! build_scly_property {
    ($($name:ident, $is_check:ident, $accessor:ident, $accessor_mut:ident,)*) => {

        #[derive(Clone, Debug)]
        pub enum SclyProperty<'r>
        {
            Unknown {
                object_type: u8,
                data: Reader<'r>
            },

            $($name(Box<scly_props::$name<'r >> ),)*
        }

        impl<'r> SclyProperty<'r>
        {
            pub fn object_type(&self) -> u8
            {
                match *self {
                    SclyProperty::Unknown { object_type, .. } => object_type,
                    $(SclyProperty::$name(_) =>
                      <scly_props::$name as SclyPropertyData>::OBJECT_TYPE,)*
                }
            }

            pub fn guess_kind(&mut self)
            {
                let (mut reader, object_type) = match *self {
                    SclyProperty::Unknown { ref data, object_type }
                        => (data.clone(), object_type),
                    _ => return,
                };
                *self = if false {
                    return
                } $(else if object_type == <scly_props::$name as SclyPropertyData>::OBJECT_TYPE {
                    SclyProperty::$name(reader.read(()))
                })* else {
                    return
                };
            }

            $(
                pub fn $is_check(&self) -> bool
                {
                    match *self {
                        SclyProperty::$name(_) => true,
                        SclyProperty::Unknown { object_type, .. } =>
                            object_type == <scly_props::$name as SclyPropertyData>::OBJECT_TYPE,
                        _ => false,
                    }
                }

                pub fn $accessor(&self) -> Option<Cow<scly_props::$name<'r>>>
                {
                    match *self {
                        SclyProperty::$name(ref inst) => Some(Cow::Borrowed(inst)),
                        SclyProperty::Unknown { ref data, object_type, .. } => {
                            if object_type == <scly_props::$name as SclyPropertyData>::OBJECT_TYPE {
                                Some(Cow::Owned(data.clone().read(())))
                            } else {
                                None
                            }
                        },
                        _ => None,
                    }
                }

                pub fn $accessor_mut(&mut self) -> Option<&mut scly_props::$name<'r>>
                {
                    let (mut data, object_type) = match *self {
                        SclyProperty::Unknown { ref data, object_type, .. } =>
                            (data.clone(), object_type),
                        SclyProperty::$name(ref mut inst) => return Some(inst),
                        _ => return None,
                    };
                    if object_type != <scly_props::$name as SclyPropertyData>::OBJECT_TYPE {
                        return None
                    }
                    *self = SclyProperty::$name(data.read(()));

                    // println!("type=0x{:X} is {} bytes big", object_type, self.size());

                    match *self {
                        SclyProperty::$name(ref mut inst) => return Some(inst),
                        _ => panic!(),
                    }
                }
            )*
        }

        impl<'r> Readable<'r> for SclyProperty<'r>
        {
            type Args = (u8, usize);
            fn read_from(reader: &mut Reader<'r>, (otype, size): Self::Args) -> Self
            {
                let prop = SclyProperty::Unknown {
                    object_type: otype,
                    data: reader.truncated(size),
                };
                reader.advance(size);
                prop
            }

            fn size(&self) -> usize
            {
                match *self {
                    SclyProperty::Unknown { ref data, .. } => data.len(),
                    $(SclyProperty::$name(ref i) => i.size(),)*
                }
            }
        }

        impl<'r> Writable for SclyProperty<'r>
        {
            fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
            {
                match *self {
                    SclyProperty::Unknown { ref data, .. } => {
                        writer.write_all(&data)?;
                        Ok(data.len() as u64)
                    },
                    $(SclyProperty::$name(ref i) => i.write_to(writer),)*
                }
            }
        }

        $(
        impl<'r> From<scly_props::$name<'r>> for SclyProperty<'r>
        {
            fn from(prop: scly_props::$name<'r>) -> SclyProperty<'r>
            {
                SclyProperty::$name(Box::new(prop))
            }
        }
        )*

    };
}

build_scly_property!(
    Actor,                is_actor,                  as_actor,                  as_actor_mut,
    ActorKeyFrame,        is_actor_key_frame,        as_actor_key_frame,        as_actor_key_frame_mut,
    BallTrigger,          is_ball_trigger,           as_ball_trigger,           as_ball_trigger_mut,
    Camera,               is_camera,                 as_camera,                 as_camera_mut,
    CameraBlurKeyframe,   is_camera_blur_keyframe,   as_camera_blur_keyframe,   as_camera_blur_keyframe_mut,
    CameraFilterKeyframe, is_camera_filter_keyframe, as_camera_filter_keyframe, as_camera_filter_keyframe_mut,
    CameraHint,           is_camera_hint,            as_camera_hint,            as_camera_hint_mut,
    CameraHintTrigger,    is_camera_hint_trigger,    as_camera_hint_trigger,    as_camera_hint_trigger_mut,
    Counter,              is_counter,                as_counter,                as_counter_mut,
    DamageableTrigger,    is_damageable_trigger,     as_damageable_trigger,     as_damageable_trigger_mut,
    DistanceFog,          is_distance_fog,           as_distance_fog,           as_distance_fog_mut,
    Dock,                 is_dock,                   as_dock,                   as_dock_mut,
    Door,                 is_door,                   as_door,                   as_door_mut,
    Effect,               is_effect,                 as_effect,                 as_effect_mut,
    GrapplePoint,         is_grapple_point,          as_grapple_point,          as_grapple_point_mut,
    HudMemo,              is_hud_memo,               as_hud_memo,               as_hud_memo_mut,
    MemoryRelay,          is_memory_relay,           as_memory_relay,           as_memory_relay_mut,
    NewCameraShaker,      is_new_camera_shaker,      as_new_camera_shaker,      as_new_camera_shaker_mut,
    Pickup,               is_pickup,                 as_pickup,                 as_pickup_mut,
    PickupGenerator,      is_pickup_generator,       as_pickup_generator,       as_pickup_generator_mut,
    Platform,             is_platform,               as_platform,               as_platform_mut,
    PlayerActor,          is_player_actor,           as_player_actor,           as_player_actor_mut,
    PlayerHint,           is_player_hint,            as_player_hint,            as_player_hint_mut,
    PointOfInterest,      is_point_of_interest,      as_point_of_interest,      as_point_of_interest_mut,
    Relay,                is_relay,                  as_relay,                  as_relay_mut,
    SnakeWeedSwarm,       is_snake_weed_swarm,       as_snake_weed_swarm,       as_snake_weed_swarm_mut,
    Sound,                is_sound,                  as_sound,                  as_sound_mut,
    SpawnPoint,           is_spawn_point,            as_spawn_point,            as_spawn_point_mut,
    SpecialFunction,      is_special_function,       as_special_function,       as_special_function_mut,
    StreamedAudio,        is_streamed_audio,         as_streamed_audio,         as_streamed_audio_mut,
    Timer,                is_timer,                  as_timer,                  as_timer_mut,
    Trigger,              is_trigger,                as_trigger,                as_trigger_mut,
    Waypoint,             is_waypoint,               as_waypoint,               as_waypoint_mut,
    Water,                is_water,                  as_water,                  as_water_mut,
    WorldTransporter,     is_world_transporter,      as_world_transporter,      as_world_transporter_mut,

    // bosses
    Beetle,               is_beetle,                 as_beetle,                 as_beetle_mut,
    Drone,                is_drone,                  as_drone,                  as_drone_mut,
    NewIntroBoss,         is_new_intro_boss,         as_new_intro_boss,         as_new_intro_boss_mut,
    ActorContraption,     is_actor_contraption,      as_actor_contraption,      as_actor_contraption_mut,
    Flaahgra,             is_flaahgra,               as_flaahgra,               as_flaahgra_mut,
    IceSheegoth,          is_ice_sheegoth,           as_ice_sheegoth,           as_ice_sheegoth_mut,
    Thardus,              is_thardus,                as_thardus,                as_thardus_mut,
    ElitePirate,          is_elite_pirate,           as_elite_pirate,           as_elite_pirate_mut,
    OmegaPirate,          is_omega_pirate,           as_omega_pirate,           as_omega_pirate_mut,
    RidleyV1,             is_ridley_v1,              as_ridley_v1,              as_ridley_v1_mut,
    RidleyV2,             is_ridley_v2,              as_ridley_v2,              as_ridley_v2_mut,
    MetroidPrimeStage1,   is_metroidprimestage1,     as_metroidprimestage1,     as_metroidprimestage1_mut,
    MetroidPrimeStage2,   is_metroidprimestage2,     as_metroidprimestage2,     as_metroidprimestage2_mut,
);

pub trait SclyPropertyData
{
    const OBJECT_TYPE: u8;
}


#[auto_struct(Readable, FixedSize, Writable)]
#[derive(Debug, Clone)]
pub struct Connection
{
    pub state: ConnectionState,
    pub message: ConnectionMsg,
    pub target_object_id: u32,
}

macro_rules! build_scly_conn_field {
    ($struct_name:ident { $($field:ident = $value:expr,)+ }) => {
        impl $struct_name
        {
            $(pub const $field: $struct_name = $struct_name($value);)+
        }

        impl fmt::Debug for $struct_name
        {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
            {
                match self.0 {
                    $(
                    $value => f.write_fmt(format_args!("{}::{}", stringify!($struct_name), stringify!($field))),
                    )+
                    n => f.write_fmt(format_args!("{}(0x{:x})", stringify!($struct_name), n)),
                }
            }
        }

        impl<'r> Readable<'r> for $struct_name
        {
            type Args = ();

            fn read_from(reader: &mut Reader<'r>, (): Self::Args) -> Self
            {
                let i = reader.read(());
                $struct_name(i)
            }

            fn fixed_size() -> Option<usize>
            {
                u32::fixed_size()
            }
        }

        impl Writable for $struct_name
        {
            fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
            {
                self.0.write_to(writer)
            }
        }
    };
}
  

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct ConnectionState(pub u32);
build_scly_conn_field!(ConnectionState {
    ANY = 0xFFFFFFFF,
    ACTIVE = 0x0,
    ARRIVED = 0x1,
    CLOSED = 0x2,
    ENTERED = 0x3,
    EXITED = 0x4,
    INACTIVE = 0x5,
    INSIDE = 0x6,
    MAX_REACHED = 0x7,
    OPEN = 0x8,
    ZERO = 0x9,
    ATTACK = 0xA,
    RETREAT = 0xC,
    PATROL = 0xD,
    DEAD = 0xE,
    CAMERA_PATH = 0xF,
    CAMERA_TARGET = 0x10,
    DEACTIVATE_STATE = 0x11,
    PLAY = 0x12,
    MASSIVE_DEATH = 0x13,
    DEATH_RATTLE = 0x14,
    ABOUT_TO_MASSIVELY_DIE = 0x15,
    DAMAGE = 0x16,
    INVULN_DAMAGE = 0x17,
    MASSIVE_FROZEN_DEATH = 0x18,
    MODIFY = 0x19,
    SCAN_START = 0x1A,
    SCAN_PROCESSING = 0x1B,
    SCAN_DONE = 0x1C,
    UNFROZEN = 0x1D,
    DEFAULT = 0x1E,
    REFLECTED_DAMAGE = 0x1F,
    INHERIT_BOUNDS = 0x20,
});

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct ConnectionMsg(pub u32);
build_scly_conn_field!(ConnectionMsg {
    NONE = 0xFFFFFFFF,
    UNKM0 = 0x0,
    ACTIVATE = 0x1,
    ARRIVED = 0x2,
    CLOSE = 0x3,
    DEACTIVATE = 0x4,
    DECREMENT = 0x5,
    FOLLOW = 0x6,
    INCREMENT = 0x7,
    NEXT = 0x8,
    OPEN = 0x9,
    RESET = 0xA,
    RESET_AND_START = 0xB,
    SET_TO_MAX = 0xC,
    SET_TO_ZERO = 0xD,
    START = 0xE,
    STOP = 0xF,
    STOP_AND_RESET = 0x10,
    TOGGLE_ACTIVE = 0x11,
    UNKM18 = 0x12,
    ACTION = 0x13,
    PLAY = 0x14,
    ALERT = 0x15,
    INTERNAL_MESSAGE00 = 0x16,
    ON_FLOOR = 0x17,
    INTERNAL_MESSAGE02 = 0x18,
    INTERNAL_MESSAGE03 = 0x19,
    FALLING = 0x1A,
    ON_ICE_SURFACE = 0x1B,
    ON_MUD_SLOW_SURFACE = 0x1C,
    ON_NORMAL_SURFACE = 0x1D,
    TOUCHED = 0x1E,
    ADD_PLATFORM_RIDER = 0x1F,
    LAND_ON_NOT_FLOOR = 0x20,
    REGISTERED = 0x21,
    DELETED = 0x22,
    INITIALIZED_IN_AREA = 0x23,
    WORLD_INITIALIZED = 0x24,
    ADD_SPLASH_INHABITANT = 0x25,
    UPDATE_SPLASH_INHABITANT = 0x26,
    REMOVE_SPLASH_INHABITANT = 0x27,
    JUMPED = 0x28,
    DAMAGE = 0x29,
    INVULN_DAMAGE = 0x2A,
    PROJECTILE_COLLIDE = 0x2B,
    IN_SNAKE_WEED = 0x2C,
    ADD_PHAZON_POOOL_INHABITANT = 0x2D,
    UPDATE_PHAZON_POOL_INHABITANT = 0x2E,
    REMOVE_PHAZON_POOL_INHABITANT = 0x2F,
    SUSPENDED_MOVE = 0x30,
});
