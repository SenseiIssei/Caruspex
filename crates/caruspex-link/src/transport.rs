//! Common interfaces for everything between the program and the car.

/// Log callback used throughout the core: one finished line per call.
pub type LogFn = Box<dyn Fn(&str) + Send + Sync>;

/// A byte channel to an ELM327 or to something that answers like one.
///
/// The ELM327 dialogue is text, but the bytes are passed through unchanged,
/// so a transport never has to care about encodings.
pub trait Transport: Send {
    /// Sends everything or returns false.
    fn write(&mut self, data: &[u8]) -> bool;
    /// Whatever arrives within `timeout_ms`. Empty = nothing there. A
    /// transport whose link is gone waits the time as well, so a caller
    /// that reads until its deadline does not spin.
    fn read(&mut self, timeout_ms: u64) -> Vec<u8>;
    /// Drops everything already received and not read yet.
    fn flush_input(&mut self);
    /// Shown in the log and the status line ("COM5 @ 38400", "192.168.0.10:35000").
    fn name(&self) -> String;
    /// Several ISO-TP sessions at once, if the way to the car can do that:
    /// the raw CAN interfaces, which see every frame on the bus. An ELM327
    /// can only wait for one control unit at a time.
    fn iso_tp_batch(&mut self) -> Option<&mut dyn IsoTpBatch> {
        None
    }
    /// Whether the driver may put the answer count digit after a request
    /// (`Elm327::answer_count_hint`). A replay says no: it can only answer
    /// the requests the way they were recorded.
    fn count_hint_allowed(&self) -> bool {
        true
    }
    /// The way to the adapter is known to be gone for good: the port or the
    /// connection was closed from the other side, the cable pulled, the
    /// Bluetooth device out of range. Asks the operating system only and
    /// sends nothing, so it may be called between two requests as often as
    /// wanted. `false` is no promise that somebody still answers.
    fn lost(&mut self) -> bool {
        false
    }
}

/// One physical ISO-TP request of a batch: to one control unit, as a
/// single frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IsoTpRequest {
    /// CAN id of the request (29 bit with the priority on top).
    pub request: u32,
    /// CAN id of the answer.
    pub response: u32,
    /// 29-bit ids.
    pub extended: bool,
    /// BMW style extended addressing: the target byte in front of every
    /// request frame; the answer frames begin with the tester, F1.
    pub target: Option<u8>,
    /// Service and parameters, at most 7 bytes (6 with a target byte).
    pub data: Vec<u8>,
}

/// ISO-TP to many control units at the same time.
pub trait IsoTpBatch {
    /// Sends every request at once and collects the answers as they come,
    /// with flow control per unit. Per request the complete messages in
    /// order: "still busy" (7F xx 78) ones first, then the answer; empty if
    /// the unit said nothing within `timeout_ms` (a busy unit gets more).
    fn exchange_all(&mut self, requests: &[IsoTpRequest], timeout_ms: u64) -> Vec<Vec<Vec<u8>>>;
}

/// One frame with a 29-bit id, as SAE J1939 (trucks, buses, machines) puts
/// it on the bus.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BusFrame {
    /// The 29-bit id: priority, PGN, destination or group, source.
    pub id: u32,
    /// 0 to 8 bytes.
    pub data: Vec<u8>,
}

/// A CAN bus heard frame by frame, for SAE J1939. Most of the traffic
/// there is broadcast, so the way in is listening: a raw CAN interface
/// hands over every frame, an ELM327 in protocol A prints them while it
/// monitors.
pub trait FrameLink: Send {
    /// Sends `send` (if any) and then hands every 29-bit frame heard to
    /// `seen`, until `listen_ms` have passed or `seen` returns true. An
    /// error means the way to the bus is gone.
    fn exchange(
        &mut self,
        send: Option<&BusFrame>,
        listen_ms: u64,
        seen: &mut dyn FnMut(&BusFrame) -> bool,
    ) -> Result<(), String>;
    /// Shown in the log ("ELM327 v1.5, protocol A", "PCAN-USB (250 kbit/s)").
    fn name(&self) -> String;
}

impl<T: FrameLink + ?Sized> FrameLink for Box<T> {
    fn exchange(
        &mut self,
        send: Option<&BusFrame>,
        listen_ms: u64,
        seen: &mut dyn FnMut(&BusFrame) -> bool,
    ) -> Result<(), String> {
        (**self).exchange(send, listen_ms, seen)
    }
    fn name(&self) -> String {
        (**self).name()
    }
}

/// Parity of a serial K-line. BMW's DS2 runs with 8E1, everything else
/// on the K-line with 8N1.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LineParity {
    #[default]
    None,
    Even,
}

/// One byte out, one byte in, for KWP1281 and KWP2000 on a K-line.
/// Kept separate from the cable so the state machines can be tested against
/// a simulated control unit.
pub trait ByteStream: Send {
    fn write_byte(&mut self, value: u8) -> bool;
    /// `None` on timeout.
    fn read_byte(&mut self, timeout_ms: u64) -> Option<u8>;
    /// The 5-baud wake-up address. Afterwards the bus must run at the normal baud rate.
    fn slow_init(&mut self, address: u8) -> bool;
    /// The K-line is single-wire: everything sent comes back as an echo.
    fn echoes_own_bytes(&self) -> bool {
        true
    }
    /// ISO 14230 fast init (25 ms low, 25 ms high) followed by the
    /// StartCommunication request `start` (a whole message with checksum).
    /// The control unit's answer, or `None` if the line cannot do it.
    fn fast_init(&mut self, start: &[u8]) -> Option<Vec<u8>> {
        let _ = start;
        None
    }
    /// Switches baud rate and parity without closing the line. BMW's
    /// E series mixes DS2 (9600 8E1), KWP2000 (10400 8N1) and BMW-FAST
    /// (115200 8N1) on one K-line. False if the line cannot do it.
    fn set_line(&mut self, baud: u32, parity: LineParity) -> bool {
        let _ = (baud, parity);
        false
    }
}

/// Whole diagnostic messages to one control unit at a time, for ways to the
/// car that frame the messages themselves (VAG TP 2.0 on CAN). The KWP2000
/// services on top are the same as on the K-line.
pub trait MessageLink: Send {
    /// Opens the logical channel to the control unit with this address
    /// (the same numbers as on the K-line: 0x01 engine, 0x17 instruments).
    fn open_channel(&mut self, address: u8) -> Result<(), String>;
    /// Closes the channel; harmless when none is open.
    fn close_channel(&mut self);
    fn channel_open(&self) -> bool;
    /// Sends one whole message, service byte first.
    fn send_message(&mut self, payload: &[u8]) -> Result<(), String>;
    /// The next whole message from the control unit within `timeout_ms`.
    fn receive_message(&mut self, timeout_ms: u64) -> Result<Vec<u8>, String>;
    /// Tells the control unit that the tester is still there.
    fn keep_alive(&mut self) -> Result<(), String>;
    fn name(&self) -> String;
}

impl<T: MessageLink + ?Sized> MessageLink for Box<T> {
    fn open_channel(&mut self, address: u8) -> Result<(), String> {
        (**self).open_channel(address)
    }
    fn close_channel(&mut self) {
        (**self).close_channel()
    }
    fn channel_open(&self) -> bool {
        (**self).channel_open()
    }
    fn send_message(&mut self, payload: &[u8]) -> Result<(), String> {
        (**self).send_message(payload)
    }
    fn receive_message(&mut self, timeout_ms: u64) -> Result<Vec<u8>, String> {
        (**self).receive_message(timeout_ms)
    }
    fn keep_alive(&mut self) -> Result<(), String> {
        (**self).keep_alive()
    }
    fn name(&self) -> String {
        (**self).name()
    }
}

impl<T: ByteStream + ?Sized> ByteStream for Box<T> {
    fn write_byte(&mut self, value: u8) -> bool {
        (**self).write_byte(value)
    }
    fn read_byte(&mut self, timeout_ms: u64) -> Option<u8> {
        (**self).read_byte(timeout_ms)
    }
    fn slow_init(&mut self, address: u8) -> bool {
        (**self).slow_init(address)
    }
    fn echoes_own_bytes(&self) -> bool {
        (**self).echoes_own_bytes()
    }
    fn fast_init(&mut self, start: &[u8]) -> Option<Vec<u8>> {
        (**self).fast_init(start)
    }
    fn set_line(&mut self, baud: u32, parity: LineParity) -> bool {
        (**self).set_line(baud, parity)
    }
}

impl<T: Transport + ?Sized> Transport for Box<T> {
    fn write(&mut self, data: &[u8]) -> bool {
        (**self).write(data)
    }
    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        (**self).read(timeout_ms)
    }
    fn flush_input(&mut self) {
        (**self).flush_input()
    }
    fn name(&self) -> String {
        (**self).name()
    }
    fn iso_tp_batch(&mut self) -> Option<&mut dyn IsoTpBatch> {
        (**self).iso_tp_batch()
    }
    fn count_hint_allowed(&self) -> bool {
        (**self).count_hint_allowed()
    }
    fn lost(&mut self) -> bool {
        (**self).lost()
    }
}
