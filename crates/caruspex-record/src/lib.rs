//! Recordings and replay of adapter sessions, plus the small JSON writer of
//! the console tool.
//!
//! CustomPids and PidPacks follow in a later round (they need the UDS code
//! of `caruspex-protocols`).
#![forbid(unsafe_code)]

pub mod json_writer;
pub mod recording;

pub use json_writer::{JsonWriter, ToJson};
pub use recording::{
    RECORDING_FORMAT, RECORDING_VERSION, RecordedDirection, RecordedEvent, Recording,
    RecordingHandle, RecordingInfo, RecordingMeta, RecordingTransport, ReplayPacing,
    ReplayTransport, active_recording, describe_recording, load_recording, normalize_command,
    parse_recording, recording_summary, start_recording, stop_recording,
};
