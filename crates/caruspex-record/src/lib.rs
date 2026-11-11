//! Recordings and replay of adapter sessions, custom values (Torque CSV,
//! equations, manufacturer PID packs, the packs imported from OBDb) and the
//! small JSON writer of the console tool.
#![forbid(unsafe_code)]

pub mod custom_pids;
#[rustfmt::skip]
mod generated;
pub mod json_writer;
pub mod obdb;
pub mod pid_packs;
pub mod recording;

pub use custom_pids::{
    CustomPid, CustomPidReader, CustomReading, PACK_KEY_BASE, TorqueEquation, USER_KEY_BASE,
    is_custom_key, parse_request_hex, parse_torque_csv, read_custom, write_torque_csv,
};
pub use json_writer::{JsonWriter, ToJson};
pub use obdb::{
    OBDB_KEY_BASE, OBDB_PREFIX, ObdbPack, ObdbStats, obdb_pack, obdb_pack_pids, obdb_packs,
    obdb_packs_for, obdb_pid, obdb_search, obdb_stats,
};
pub use pid_packs::{built_in_packs, pack_category, pack_names, pack_pids, pack_title};
pub use recording::{
    RECORDING_FORMAT, RECORDING_VERSION, RecordedDirection, RecordedEvent, Recording,
    RecordingHandle, RecordingInfo, RecordingMeta, RecordingTransport, ReplayPacing,
    ReplayTransport, active_recording, describe_recording, load_recording, normalize_command,
    parse_recording, recording_summary, start_recording, stop_recording,
};
