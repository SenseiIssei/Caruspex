//! Network and raw CAN: BMW ENET (HSFZ), DoIP, SLCAN, SocketCAN, J2534.
//!
//! Every way to the car in this crate ends in something that answers like an
//! ELM327 ([`enet::EnetElm`], [`can_bus::CanElm`]), so the rest of Caruspex
//! works through it unchanged.

// The FFI modules (J2534, SocketCAN) allow unsafe for themselves; nothing
// else may use it.
#![deny(unsafe_code)]

pub mod can_bus;
pub mod can_signals;
pub mod enet;
#[cfg(windows)]
mod j2534;
#[cfg(target_os = "linux")]
mod socketcan;
mod text;

pub use can_bus::{
    CanElm, CanFrame, CanLink, J2534Device, SerialOpener, SlcanLink, j2534_devices,
    open_can_interface, open_j2534, open_socket_can, probe_slcan, socket_can_interfaces,
};
pub use can_signals::{
    CanSignal, can_bits, decode_can_signal, parse_can_signal, parse_monitor_request,
};
pub use enet::{
    DiagAnswer, DiagLink, DoipSession, EnetElm, Flavor, HsfzSession, NetStream, NetworkCar,
    discover_doip, discover_enet, open_doip, open_enet, open_network_car,
};
