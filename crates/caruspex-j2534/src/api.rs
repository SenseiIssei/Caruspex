//! The PassThru API with plain Rust types, the same for the DLL in the
//! program and for the bridge.

/// The numbers of SAE J2534-1 04.04 that Caruspex uses, and the CAN FD
/// protocols of J2534-2.
pub mod constants {
    pub const PROTOCOL_ISO9141: u32 = 3;
    pub const PROTOCOL_ISO14230: u32 = 4;
    pub const PROTOCOL_CAN: u32 = 5;
    /// CAN FD frames (J2534-2/11, "FD_CAN").
    pub const PROTOCOL_FD_CAN: u32 = 0x8011;
    pub const CAN_ID_BOTH: u32 = 0x800;
    pub const CAN_29BIT_ID: u32 = 0x100;
    /// Connect flag: the device neither adds nor checks the checksum.
    pub const ISO9141_NO_CHECKSUM: u32 = 0x200;
    /// Connect flag: K-line only, no L-line.
    pub const ISO9141_K_LINE_ONLY: u32 = 0x1000;
    /// RxStatus: echo of our own frame.
    pub const TX_MSG_TYPE: u32 = 0x01;
    /// RxStatus: only the indication that a message begins, no data.
    pub const START_OF_MESSAGE: u32 = 0x02;
    pub const PASS_FILTER: u32 = 1;
    // Ioctl ids.
    pub const GET_CONFIG: u32 = 0x01;
    pub const SET_CONFIG: u32 = 0x02;
    pub const READ_VBATT: u32 = 0x03;
    pub const FIVE_BAUD_INIT: u32 = 0x04;
    pub const FAST_INIT: u32 = 0x05;
    pub const CLEAR_TX_BUFFER: u32 = 0x07;
    pub const CLEAR_RX_BUFFER: u32 = 0x08;
    // Configuration parameters (units of 0.5 ms for the P times).
    pub const LOOPBACK: u32 = 0x03;
    pub const P3_MIN: u32 = 0x0A;
    pub const FIVE_BAUD_MOD: u32 = 0x20;
    /// The device sends the inverted key byte 2 itself and does not wait for
    /// an inverted address.
    pub const FIVE_BAUD_INVERTED_KEY_BYTE: u32 = 1;
    pub const NO_ERROR: i32 = 0;
    pub const ERR_NOT_SUPPORTED: i32 = 0x01;
    pub const ERR_FAILED: i32 = 0x07;
    /// The bridge lost its helper program; not a J2534 code.
    pub const ERR_BRIDGE: i32 = -1;
    /// Room for data in one PASSTHRU_MSG.
    pub const MESSAGE_ROOM: usize = 4128;
}

/// One PASSTHRU_MSG.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Message {
    pub protocol_id: u32,
    pub rx_status: u32,
    pub tx_flags: u32,
    pub timestamp: u32,
    pub extra_data_index: u32,
    /// At most [`constants::MESSAGE_ROOM`] bytes.
    pub data: Vec<u8>,
}

impl Message {
    pub fn new(protocol_id: u32, data: &[u8]) -> Self {
        Self {
            protocol_id,
            data: data[..data.len().min(constants::MESSAGE_ROOM)].to_vec(),
            ..Self::default()
        }
    }
}

/// The ioctls Caruspex calls, with their input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ioctl {
    SetConfig(Vec<(u32, u32)>),
    GetConfig(Vec<u32>),
    ReadVbatt,
    /// The address bytes, and room for this many key bytes.
    FiveBaudInit(Vec<u8>, u32),
    FastInit(Message),
    ClearTxBuffer,
    ClearRxBuffer,
}

impl Ioctl {
    pub fn id(&self) -> u32 {
        use constants::*;
        match self {
            Ioctl::SetConfig(_) => SET_CONFIG,
            Ioctl::GetConfig(_) => GET_CONFIG,
            Ioctl::ReadVbatt => READ_VBATT,
            Ioctl::FiveBaudInit(..) => FIVE_BAUD_INIT,
            Ioctl::FastInit(_) => FAST_INIT,
            Ioctl::ClearTxBuffer => CLEAR_TX_BUFFER,
            Ioctl::ClearRxBuffer => CLEAR_RX_BUFFER,
        }
    }
}

/// What an ioctl gave back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IoctlOut {
    Nothing,
    /// GET_CONFIG: parameter and value.
    Config(Vec<(u32, u32)>),
    /// READ_VBATT in millivolts.
    Value(u32),
    /// FIVE_BAUD_INIT: the key bytes.
    Bytes(Vec<u8>),
    /// FAST_INIT: the answer.
    Message(Message),
}

/// A PassThru driver: the functions of SAE J2534-1 04.04. A failing call
/// gives the J2534 status code.
pub trait PassThru: Send {
    /// PassThruOpen of the first device; its id.
    fn open(&mut self) -> Result<u32, i32>;
    fn close(&mut self, device: u32) -> i32;
    /// PassThruConnect; the channel id.
    fn connect(&mut self, device: u32, protocol: u32, flags: u32, baud: u32) -> Result<u32, i32>;
    fn disconnect(&mut self, channel: u32) -> i32;
    /// One message within `timeout_ms`; `Ok(None)` if none came.
    fn read(&mut self, channel: u32, timeout_ms: u32) -> Result<Option<Message>, i32>;
    /// One message; how many the driver took.
    fn write(&mut self, channel: u32, message: &Message, timeout_ms: u32) -> Result<u32, i32>;
    /// PassThruStartMsgFilter; the filter id.
    fn start_filter(
        &mut self,
        channel: u32,
        kind: u32,
        mask: &Message,
        pattern: &Message,
        flow: Option<&Message>,
    ) -> Result<u32, i32>;
    /// On a device (READ_VBATT) or a channel. `ERR_NOT_SUPPORTED` if the
    /// driver has no PassThruIoctl.
    fn ioctl(&mut self, target: u32, request: &Ioctl) -> Result<IoctlOut, i32>;
    /// PassThruGetLastError; empty if the driver has none.
    fn last_error(&mut self) -> String;
    /// The DLL path.
    fn library(&self) -> &str;
}

/// The machine a DLL is built for, from its PE header: 0x14C for 32-bit
/// x86, 0x8664 for x64, 0xAA64 for ARM64. `None` if the file cannot be
/// read or is no PE file.
pub fn dll_machine(path: &str) -> Option<u16> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).ok()?;
    let mut dos = [0u8; 64];
    file.read_exact(&mut dos).ok()?;
    if &dos[..2] != b"MZ" {
        return None;
    }
    let pe = u32::from_le_bytes([dos[60], dos[61], dos[62], dos[63]]);
    file.seek(SeekFrom::Start(u64::from(pe))).ok()?;
    let mut head = [0u8; 6];
    file.read_exact(&mut head).ok()?;
    if &head[..4] != b"PE\0\0" {
        return None;
    }
    Some(u16::from_le_bytes([head[4], head[5]]))
}

/// True for a 32-bit x86 DLL, which a 64-bit program cannot load.
pub fn is_32bit_dll(path: &str) -> bool {
    dll_machine(path) == Some(0x14C)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_machine_of_a_pe_file() {
        let dir = std::env::temp_dir().join(format!("caruspex-j2534-pe-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let write = |name: &str, machine: u16| {
            let mut bytes = vec![0u8; 0x90];
            bytes[..2].copy_from_slice(b"MZ");
            bytes[60..64].copy_from_slice(&0x80u32.to_le_bytes());
            bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
            bytes[0x84..0x86].copy_from_slice(&machine.to_le_bytes());
            let path = dir.join(name);
            std::fs::write(&path, bytes).unwrap();
            path.to_string_lossy().into_owned()
        };
        let old = write("old.dll", 0x14C);
        let new = write("new.dll", 0x8664);
        assert!(is_32bit_dll(&old));
        assert!(!is_32bit_dll(&new));
        assert_eq!(dll_machine(&new), Some(0x8664));
        std::fs::write(dir.join("text.dll"), b"no PE file at all").unwrap();
        assert_eq!(dll_machine(&dir.join("text.dll").to_string_lossy()), None);
        assert_eq!(dll_machine("C:/no/such/file.dll"), None);
        std::fs::remove_dir_all(&dir).ok();
    }
}
