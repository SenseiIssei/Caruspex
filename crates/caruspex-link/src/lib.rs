//! Ways to the adapter (serial, TCP, Bluetooth LE) and the ELM327 driver.
//!
//! The two traits here are shared by every crate that talks to a car:
//! [`Transport`] carries the text dialogue with an ELM327 (or anything that
//! emulates one, like the ENET and CAN front ends and the demo car), and
//! [`ByteStream`] carries single bytes on a K-line for KWP1281 and KWP2000.

pub mod timing;
pub mod transport;

pub use transport::{ByteStream, LogFn, Transport};
