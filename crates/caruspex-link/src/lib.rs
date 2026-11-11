//! Ways to the adapter (serial, TCP, Bluetooth LE, classic Bluetooth) and
//! the ELM327 driver.
//!
//! The two traits here are shared by every crate that talks to a car:
//! [`Transport`] carries the text dialogue with an ELM327 (or anything that
//! emulates one, like the ENET and CAN front ends and the demo car), and
//! [`ByteStream`] carries single bytes on a K-line for KWP1281 and KWP2000.
//!
//! No `unsafe` except in [`bt_classic`], which talks to the system's
//! Bluetooth sockets.
#![deny(unsafe_code)]

pub mod adapter_probe;
#[cfg(feature = "ble")]
pub mod ble;
pub mod ble_profile;
pub mod bluetooth;
#[cfg(feature = "bt-classic")]
pub mod bt_classic;
pub mod elm327;
pub mod kline_stream;
pub mod serial;
pub mod tcp;
pub mod timing;
pub mod transport;

pub use adapter_probe::{AdapterReport, BilingualText, CommandCheck, probe_adapter};
pub use elm327::{
    AdaptiveTiming, ConnectTrace, Elm327, ElmMessage, ElmResult, ElmStatus, HeaderFormat,
    decode_messages, header_format_for, hex_to_bytes, protocol_name, split_messages,
};
pub use kline_stream::KLineStream;
pub use serial::{PortInfo, Serial, SerialTransport, enumerate_ports};
pub use tcp::TcpTransport;
pub use transport::{ByteStream, LogFn, Transport};
