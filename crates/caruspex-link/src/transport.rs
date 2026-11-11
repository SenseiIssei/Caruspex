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
    /// Whatever arrives within `timeout_ms`. Empty = nothing there.
    fn read(&mut self, timeout_ms: u64) -> Vec<u8>;
    /// Drops everything already received and not read yet.
    fn flush_input(&mut self);
    /// Shown in the log and the status line ("COM5 @ 38400", "192.168.0.10:35000").
    fn name(&self) -> String;
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
}
