//! SAE J2534-1 (04.04) PassThru drivers as Caruspex uses them.
//!
//! [`PassThru`] is the driver API with plain Rust types. It comes in two
//! kinds: [`Dll`] loads the driver DLL into the program itself, [`Bridge`]
//! starts the small helper program `caruspex-j2534-bridge` and talks to it
//! over a named pipe. The helper is built for 32-bit Windows, so the many
//! older PassThru drivers that only exist as 32-bit DLLs work with the
//! 64-bit Caruspex too. Both kinds behave the same; caruspex-net picks one
//! from the DLL file ([`dll_machine`]).
//!
//! The texts here are not translated: they are driver details that
//! caruspex-net puts behind its own messages.

// Only the DLL module (loading the driver, calling it, the message buffers)
// and the pipe server of the bridge may use unsafe.
#![deny(unsafe_code)]

mod api;
mod bridge;
#[cfg(windows)]
mod dll;
mod wire;

pub use api::{Ioctl, IoctlOut, Message, PassThru, constants, is_32bit_dll};
#[cfg(windows)]
pub use bridge::serve_pipe;
pub use bridge::{Bridge, bridge_next_to_program, serve};
#[cfg(windows)]
pub use dll::{Dll, LoadError};
