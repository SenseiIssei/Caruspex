//! Diagnosis protocols over the ELM327 and the K-line: OBD-II (EOBD), UDS
//! (ISO 14229), KWP2000 (ISO 14230) and VAG's KWP1281.
//!
//! [`obd2::Obd2`] and [`uds::Uds`] borrow a connected
//! [`caruspex_link::Elm327`]; the K-line protocols own a
//! [`caruspex_link::ByteStream`] (a KKL cable or a simulated control unit)
//! and hand it back with `into_stream`.
#![forbid(unsafe_code)]

pub mod ecu_address;
pub mod kwp1281;
pub mod kwp2000;
pub mod obd2;
pub mod uds;
pub mod vag_kline;

pub use ecu_address::{EcuRole, ecu_display_name, ecu_role, ecu_role_name};
pub use kwp1281::{Ecu, KLineLog, Kwp1281, Kwp1281Fault, MeasuringValue, decode_measurement};
pub use kwp2000::{
    KLineProtocol, Kwp2000, decode_kwp2000_fault, negative_response_text, protocol_from_key_bytes,
    protocol_label, wake_up_kline,
};
pub use obd2::{
    EcuInfo, Fault, FreezeFrame, InUseCounter, InUsePerformance, LiveValue, MisfireCount,
    MonitorResult, Obd2, OxygenSensorTest, ReadinessItem, VehicleIdentity, VehicleStatus,
    batchable_pid, misfire_counts, split_multi_pid_answer,
};
pub use uds::{
    ExtendedReply, Uds, UdsDtc, UdsEcu, UdsEcuInfo, decode_bmw_dtc, decode_uds_dtc,
    extended_request, scan_ecus,
};
pub use vag_kline::VagKLine;
