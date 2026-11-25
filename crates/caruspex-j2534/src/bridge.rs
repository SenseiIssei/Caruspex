//! The 32-bit bridge. A 64-bit program cannot load a 32-bit DLL, and many
//! PassThru drivers of older interfaces only exist as one. So Caruspex
//! starts `caruspex-j2534-bridge.exe`, a 32-bit build of this crate, which
//! creates a named pipe, loads the driver and carries out every call that
//! comes over the pipe ([`serve`]). On the Caruspex side [`Bridge`] is a
//! [`PassThru`] like the DLL itself, so the J2534 code above does not see
//! the difference.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::{Duration, Instant};

use crate::api::constants::{ERR_BRIDGE, ERR_BRIDGE_TIMEOUT};
use crate::api::{Ioctl, IoctlOut, Message, PassThru};
use crate::wire::{self, Call, Reply};

/// The file name of the helper program, next to caruspex.exe.
pub(crate) const BRIDGE_EXE: &str = "caruspex-j2534-bridge.exe";

/// How long the helper may take to start and open its pipe.
const START_TIMEOUT: Duration = Duration::from_secs(10);
/// How long the helper waits for Caruspex, which ended or was ended when it
/// does not come.
#[cfg(windows)]
const CLIENT_TIMEOUT: Duration = Duration::from_secs(30);

/// How long the driver behind the pipe may take. A driver that hangs (a
/// device that was unplugged at the wrong moment, a message box nobody
/// sees) would otherwise hang Caruspex with it: the limits are far above
/// what a working driver needs.
#[derive(Clone, Copy, Debug)]
struct Limits {
    /// Loading the DLL and opening the device: drivers look for it on USB,
    /// Bluetooth or the network first.
    start: Duration,
    /// Every other call without a timeout of its own (the 5-baud init takes
    /// about 2.5 s).
    call: Duration,
    /// Reading and writing: this much more than the timeout they carry.
    grace: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            start: Duration::from_secs(30),
            call: Duration::from_secs(10),
            grace: Duration::from_secs(5),
        }
    }
}

impl Limits {
    fn of(&self, call: &Call) -> Duration {
        match call {
            Call::Load(_) | Call::Open => self.start,
            Call::Read { timeout_ms, .. } | Call::Write { timeout_ms, .. } => {
                Duration::from_millis(u64::from(*timeout_ms)) + self.grace
            }
            _ => self.call,
        }
    }
}

/// The helper program next to the running program, if it is there.
pub fn bridge_next_to_program() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let path = exe.parent()?.join(BRIDGE_EXE);
    path.is_file().then_some(path)
}

/// Carries out the calls that come over `stream` until the other side
/// closes it. The first call must be `Load`; `load` turns the DLL path into
/// the driver or the reason it is none.
pub fn serve(
    stream: &mut (impl Read + Write),
    mut load: impl FnMut(&str) -> Result<Box<dyn PassThru>, String>,
) -> std::io::Result<()> {
    let mut driver: Option<Box<dyn PassThru>> = None;
    while let Some(frame) = wire::receive(stream)? {
        let call = Call::decode(&frame)?;
        let reply = match (call, driver.as_mut()) {
            (Call::Load(path), None) => match load(&path) {
                Ok(d) => {
                    driver = Some(d);
                    Reply::Loaded(String::new())
                }
                Err(reason) => Reply::Loaded(reason),
            },
            (Call::Load(_), Some(_)) => Reply::Loaded("loaded already".into()),
            (_, None) => Reply::Status(ERR_BRIDGE),
            (call, Some(d)) => carry_out(d.as_mut(), call),
        };
        wire::send(stream, &reply.encode())?;
    }
    Ok(())
}

fn carry_out(d: &mut dyn PassThru, call: Call) -> Reply {
    match call {
        Call::Load(_) => Reply::Status(ERR_BRIDGE),
        Call::Open => Reply::Number(d.open()),
        Call::Close(device) => Reply::Status(d.close(device)),
        Call::Connect {
            device,
            protocol,
            flags,
            baud,
        } => Reply::Number(d.connect(device, protocol, flags, baud)),
        Call::Disconnect(channel) => Reply::Status(d.disconnect(channel)),
        Call::Read {
            channel,
            timeout_ms,
        } => Reply::Read(d.read(channel, timeout_ms)),
        Call::Write {
            channel,
            message,
            timeout_ms,
        } => Reply::Number(d.write(channel, &message, timeout_ms)),
        Call::Filter {
            channel,
            kind,
            mask,
            pattern,
            flow,
        } => Reply::Number(d.start_filter(channel, kind, &mask, &pattern, flow.as_ref())),
        Call::Ioctl { target, request } => Reply::Ioctl(d.ioctl(target, &request)),
        Call::LastError => Reply::Text(d.last_error()),
    }
}

/// The pipe server of the helper program: creates the pipe `name`, waits
/// for Caruspex and serves the driver DLLs it names.
#[cfg(windows)]
pub fn serve_pipe(name: &str) -> std::io::Result<()> {
    // Waiting for the client cannot be cut short from outside: a helper
    // whose Caruspex has ended in the meantime ends itself.
    let (connected, waited_for) = channel::<()>();
    std::thread::spawn(move || {
        if waited_for.recv_timeout(CLIENT_TIMEOUT) == Err(RecvTimeoutError::Timeout) {
            std::process::exit(3);
        }
    });
    let mut pipe = pipe::create_and_wait(name)?;
    let _ = connected.send(());
    serve(&mut pipe, |path| {
        crate::dll::Dll::load(path)
            .map(|d| Box::new(d) as Box<dyn PassThru>)
            .map_err(|e| format!("{e:?}"))
    })
}

#[cfg(windows)]
mod pipe {
    //! CreateNamedPipeW and ConnectNamedPipe, which std has no wrapper for.
    #![allow(unsafe_code)]

    use std::ffi::c_void;
    use std::fs::File;
    use std::io;
    use std::os::windows::io::FromRawHandle;

    const PIPE_ACCESS_DUPLEX: u32 = 0x3;
    const FILE_FLAG_FIRST_PIPE_INSTANCE: u32 = 0x0008_0000;
    const PIPE_REJECT_REMOTE_CLIENTS: u32 = 0x8;
    const ERROR_PIPE_CONNECTED: i32 = 535;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateNamedPipeW(
            name: *const u16,
            open_mode: u32,
            pipe_mode: u32,
            max_instances: u32,
            out_buffer: u32,
            in_buffer: u32,
            default_timeout: u32,
            security: *mut c_void,
        ) -> *mut c_void;
        fn ConnectNamedPipe(pipe: *mut c_void, overlapped: *mut c_void) -> i32;
    }

    /// One pipe instance, local clients only, byte mode, blocking; returns
    /// once the client is there.
    pub(crate) fn create_and_wait(name: &str) -> io::Result<File> {
        let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        // SAFETY: a terminated name, no security attributes (the default:
        // the same user and the administrators).
        let handle = unsafe {
            CreateNamedPipeW(
                wide.as_ptr(),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
                PIPE_REJECT_REMOTE_CLIENTS,
                1,
                65536,
                65536,
                0,
                std::ptr::null_mut(),
            )
        };
        if handle.is_null() || handle as isize == -1 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the handle is ours and valid; File closes it.
        let file = unsafe { File::from_raw_handle(handle) };
        // SAFETY: a valid pipe handle, no overlapped structure (blocking).
        if unsafe { ConnectNamedPipe(handle, std::ptr::null_mut()) } == 0 {
            let error = io::Error::last_os_error();
            // The client came between creating and waiting: connected too.
            if error.raw_os_error() != Some(ERROR_PIPE_CONNECTED) {
                return Err(error);
            }
        }
        Ok(file)
    }
}

/// A PassThru driver in the helper program, reached over its pipe.
pub struct Bridge {
    /// The thread that owns the pipe: a blocking read of a pipe cannot be
    /// given a time limit, waiting for the thread can. `None` once the pipe
    /// is broken or a call took too long.
    pipe: Option<PipeThread>,
    /// The last call took longer than its limit.
    timed_out: bool,
    limits: Limits,
    child: Option<Child>,
    library: String,
}

struct PipeThread {
    calls: Sender<Vec<u8>>,
    /// The reply frame, or `None` when the pipe broke.
    replies: Receiver<Option<Vec<u8>>>,
}

impl PipeThread {
    fn start(mut stream: impl Read + Write + Send + 'static) -> Result<Self, String> {
        let (calls, next_call) = channel::<Vec<u8>>();
        let (reply, replies) = channel();
        std::thread::Builder::new()
            .name("caruspex-j2534-pipe".into())
            .spawn(move || {
                for frame in next_call {
                    let answer = wire::send(&mut stream, &frame)
                        .and_then(|()| wire::receive(&mut stream))
                        .ok()
                        .flatten();
                    let broken = answer.is_none();
                    if reply.send(answer).is_err() || broken {
                        break;
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self { calls, replies })
    }
}

impl Bridge {
    /// Starts the helper program `exe`, connects to its pipe and has it load
    /// the driver `library`.
    pub fn start(exe: &Path, library: &str) -> Result<Bridge, String> {
        let name = pipe_name();
        let mut command = Command::new(exe);
        command
            .arg("--pipe")
            .arg(&name)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut child = command
            .spawn()
            .map_err(|e| format!("{}: {e}", exe.display()))?;
        let started = Instant::now();
        let stream = loop {
            match std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&name)
            {
                Ok(file) => break file,
                Err(_) if started.elapsed() < START_TIMEOUT => {
                    if let Ok(Some(status)) = child.try_wait() {
                        return Err(format!("{} ended: {status}", exe.display()));
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => {
                    child.kill().ok();
                    child.wait().ok();
                    return Err(format!("{}: {e}", exe.display()));
                }
            }
        };
        Self::with(stream, Some(child), library, Limits::default())
    }

    /// A bridge over any stream to a [`serve`] loop, for the tests.
    pub fn over(stream: impl Read + Write + Send + 'static, library: &str) -> Result<Self, String> {
        Self::with(stream, None, library, Limits::default())
    }

    fn with(
        stream: impl Read + Write + Send + 'static,
        child: Option<Child>,
        library: &str,
        limits: Limits,
    ) -> Result<Self, String> {
        let mut bridge = Bridge {
            pipe: None,
            timed_out: false,
            limits,
            child,
            library: library.to_string(),
        };
        bridge.pipe = Some(PipeThread::start(stream)?);
        bridge.load()?;
        Ok(bridge)
    }

    fn load(&mut self) -> Result<(), String> {
        match self.call(&Call::Load(self.library.clone())) {
            Some(Reply::Loaded(error)) if error.is_empty() => Ok(()),
            Some(Reply::Loaded(error)) => Err(error),
            _ if self.timed_out => Err("the driver did not answer in time".into()),
            _ => Err("no answer from the bridge".into()),
        }
    }

    /// One call and its reply; `None` once the pipe is broken or when the
    /// driver took longer than its limit.
    fn call(&mut self, call: &Call) -> Option<Reply> {
        let pipe = self.pipe.as_ref()?;
        let answer = match pipe.calls.send(call.encode()) {
            Ok(()) => pipe.replies.recv_timeout(self.limits.of(call)),
            Err(_) => Err(RecvTimeoutError::Disconnected),
        };
        self.timed_out = answer == Err(RecvTimeoutError::Timeout);
        let reply = answer
            .ok()
            .flatten()
            .and_then(|frame| Reply::decode(&frame).ok());
        if reply.is_none() {
            // The helper is gone or hangs: every later call fails at once.
            self.pipe = None;
            // Ending a hung helper also breaks the pipe, which frees the
            // thread waiting for it.
            if self.timed_out
                && let Some(child) = self.child.as_mut()
            {
                child.kill().ok();
            }
        }
        reply
    }

    /// The status code of a call that got no reply.
    fn failure(&self) -> i32 {
        if self.timed_out {
            ERR_BRIDGE_TIMEOUT
        } else {
            ERR_BRIDGE
        }
    }

    fn status(&mut self, call: Call) -> i32 {
        match self.call(&call) {
            Some(Reply::Status(s)) => s,
            _ => self.failure(),
        }
    }

    fn number(&mut self, call: Call) -> Result<u32, i32> {
        match self.call(&call) {
            Some(Reply::Number(n)) => n,
            Some(Reply::Status(s)) => Err(s),
            _ => Err(self.failure()),
        }
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        // Closing the pipe ends the helper; it gets a moment, then it is ended.
        self.pipe = None;
        if let Some(mut child) = self.child.take() {
            let until = Instant::now() + Duration::from_secs(2);
            while Instant::now() < until {
                if let Ok(Some(_)) = child.try_wait() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            child.kill().ok();
            child.wait().ok();
        }
    }
}

/// A pipe name nobody else uses: process id, time and a random number.
fn pipe_name() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let mut h = RandomState::new().build_hasher();
    h.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    );
    format!(
        r"\\.\pipe\caruspex-j2534-{}-{:016x}",
        std::process::id(),
        h.finish()
    )
}

impl PassThru for Bridge {
    fn open(&mut self) -> Result<u32, i32> {
        self.number(Call::Open)
    }

    fn close(&mut self, device: u32) -> i32 {
        self.status(Call::Close(device))
    }

    fn connect(&mut self, device: u32, protocol: u32, flags: u32, baud: u32) -> Result<u32, i32> {
        self.number(Call::Connect {
            device,
            protocol,
            flags,
            baud,
        })
    }

    fn disconnect(&mut self, channel: u32) -> i32 {
        self.status(Call::Disconnect(channel))
    }

    fn read(&mut self, channel: u32, timeout_ms: u32) -> Result<Option<Message>, i32> {
        match self.call(&Call::Read {
            channel,
            timeout_ms,
        }) {
            Some(Reply::Read(r)) => r,
            Some(Reply::Status(s)) => Err(s),
            _ => Err(self.failure()),
        }
    }

    fn write(&mut self, channel: u32, message: &Message, timeout_ms: u32) -> Result<u32, i32> {
        self.number(Call::Write {
            channel,
            message: message.clone(),
            timeout_ms,
        })
    }

    fn start_filter(
        &mut self,
        channel: u32,
        kind: u32,
        mask: &Message,
        pattern: &Message,
        flow: Option<&Message>,
    ) -> Result<u32, i32> {
        self.number(Call::Filter {
            channel,
            kind,
            mask: mask.clone(),
            pattern: pattern.clone(),
            flow: flow.cloned(),
        })
    }

    fn ioctl(&mut self, target: u32, request: &Ioctl) -> Result<IoctlOut, i32> {
        match self.call(&Call::Ioctl {
            target,
            request: request.clone(),
        }) {
            Some(Reply::Ioctl(r)) => r,
            Some(Reply::Status(s)) => Err(s),
            _ => Err(self.failure()),
        }
    }

    fn last_error(&mut self) -> String {
        match self.call(&Call::LastError) {
            Some(Reply::Text(t)) => t,
            _ => String::new(),
        }
    }

    fn library(&self) -> &str {
        &self.library
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::constants::{ERR_NOT_SUPPORTED, PROTOCOL_CAN};
    use std::net::{TcpListener, TcpStream};

    /// A driver that answers every frame with the same frame plus one.
    #[derive(Default)]
    struct Loop {
        inbox: Vec<Message>,
    }

    impl PassThru for Loop {
        fn open(&mut self) -> Result<u32, i32> {
            Ok(1)
        }
        fn close(&mut self, _device: u32) -> i32 {
            0
        }
        fn connect(&mut self, _d: u32, protocol: u32, _f: u32, baud: u32) -> Result<u32, i32> {
            if protocol == PROTOCOL_CAN && baud == 500000 {
                Ok(2)
            } else {
                Err(7)
            }
        }
        fn disconnect(&mut self, _channel: u32) -> i32 {
            0
        }
        fn read(&mut self, _channel: u32, _timeout_ms: u32) -> Result<Option<Message>, i32> {
            if self.inbox.is_empty() {
                Err(0x10)
            } else {
                Ok(Some(self.inbox.remove(0)))
            }
        }
        fn write(&mut self, _channel: u32, m: &Message, _timeout_ms: u32) -> Result<u32, i32> {
            let mut answer = m.clone();
            answer.data.iter_mut().for_each(|b| *b = b.wrapping_add(1));
            self.inbox.push(answer);
            Ok(1)
        }
        fn start_filter(
            &mut self,
            _channel: u32,
            _kind: u32,
            _mask: &Message,
            _pattern: &Message,
            _flow: Option<&Message>,
        ) -> Result<u32, i32> {
            Ok(9)
        }
        fn ioctl(&mut self, _target: u32, request: &Ioctl) -> Result<IoctlOut, i32> {
            match request {
                Ioctl::ReadVbatt => Ok(IoctlOut::Value(12480)),
                _ => Err(ERR_NOT_SUPPORTED),
            }
        }
        fn last_error(&mut self) -> String {
            "loop driver".into()
        }
        fn library(&self) -> &str {
            "loop.dll"
        }
    }

    /// A driver that hangs in the calls named, like one whose device was
    /// unplugged in the middle of a transfer.
    struct Hanging {
        open: bool,
        inner: Loop,
    }

    const HANG: Duration = Duration::from_secs(20);

    impl PassThru for Hanging {
        fn open(&mut self) -> Result<u32, i32> {
            if self.open {
                std::thread::sleep(HANG);
            }
            Ok(1)
        }
        fn close(&mut self, device: u32) -> i32 {
            self.inner.close(device)
        }
        fn connect(&mut self, d: u32, protocol: u32, f: u32, baud: u32) -> Result<u32, i32> {
            self.inner.connect(d, protocol, f, baud)
        }
        fn disconnect(&mut self, channel: u32) -> i32 {
            self.inner.disconnect(channel)
        }
        fn read(&mut self, _channel: u32, _timeout_ms: u32) -> Result<Option<Message>, i32> {
            std::thread::sleep(HANG);
            Ok(None)
        }
        fn write(&mut self, channel: u32, m: &Message, timeout_ms: u32) -> Result<u32, i32> {
            self.inner.write(channel, m, timeout_ms)
        }
        fn start_filter(
            &mut self,
            channel: u32,
            kind: u32,
            mask: &Message,
            pattern: &Message,
            flow: Option<&Message>,
        ) -> Result<u32, i32> {
            self.inner.start_filter(channel, kind, mask, pattern, flow)
        }
        fn ioctl(&mut self, target: u32, request: &Ioctl) -> Result<IoctlOut, i32> {
            self.inner.ioctl(target, request)
        }
        fn last_error(&mut self) -> String {
            self.inner.last_error()
        }
        fn library(&self) -> &str {
            "hanging.dll"
        }
    }

    const SHORT: Limits = Limits {
        start: Duration::from_millis(300),
        call: Duration::from_millis(300),
        grace: Duration::from_millis(200),
    };

    /// A bridge to a hanging driver with limits fit for a test. The server
    /// thread is left to itself, like a helper that has to be killed.
    fn hanging_bridge(open: bool) -> Bridge {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = serve(&mut stream, |_| {
                Ok(Box::new(Hanging {
                    open,
                    inner: Loop::default(),
                }) as Box<dyn PassThru>)
            });
        });
        Bridge::with(
            TcpStream::connect(address).unwrap(),
            None,
            "hanging.dll",
            SHORT,
        )
        .unwrap()
    }

    #[test]
    fn a_driver_that_hangs_in_a_read_is_given_up_in_time() {
        let mut b = hanging_bridge(false);
        let device = b.open().unwrap();
        let channel = b.connect(device, PROTOCOL_CAN, 0, 500000).unwrap();
        let started = Instant::now();
        // The read's own 100 ms and the grace, instead of for ever.
        assert_eq!(b.read(channel, 100), Err(ERR_BRIDGE_TIMEOUT));
        let took = started.elapsed();
        assert!(took >= Duration::from_millis(290), "took {took:?}");
        assert!(took < Duration::from_secs(3), "took {took:?}");
        // Every later call fails at once and keeps the reason, closing too.
        let started = Instant::now();
        let frame = Message::new(PROTOCOL_CAN, &[0, 0, 7, 0xDF, 2, 1, 0]);
        assert_eq!(b.write(channel, &frame, 100), Err(ERR_BRIDGE_TIMEOUT));
        assert_eq!(b.last_error(), "");
        assert_eq!(b.disconnect(channel), ERR_BRIDGE_TIMEOUT);
        drop(b);
        assert!(started.elapsed() < Duration::from_millis(500));
    }

    #[test]
    fn a_driver_that_hangs_while_opening_is_given_up_in_time() {
        let mut b = hanging_bridge(true);
        let started = Instant::now();
        assert_eq!(b.open(), Err(ERR_BRIDGE_TIMEOUT));
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn the_limits_leave_room_for_the_time_a_call_carries() {
        let limits = Limits::default();
        let read = Call::Read {
            channel: 1,
            timeout_ms: 60_000,
        };
        assert_eq!(limits.of(&read), Duration::from_secs(65));
        assert!(limits.of(&Call::Open) >= Duration::from_secs(30));
        // The 5-baud init takes about 2.5 s on the wire.
        let init = Call::Ioctl {
            target: 1,
            request: Ioctl::FiveBaudInit(vec![0x33], 2),
        };
        assert!(limits.of(&init) >= Duration::from_secs(8));
    }

    /// What the thread between Caruspex and the pipe costs per call:
    /// cargo test -p caruspex-j2534 --release -- --ignored --nocapture cost
    #[test]
    #[ignore = "prints a measurement"]
    fn the_cost_of_a_call_over_the_bridge() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_nodelay(true).unwrap();
            serve(&mut stream, |_| {
                Ok(Box::new(Loop::default()) as Box<dyn PassThru>)
            })
            .unwrap();
        });
        let stream = TcpStream::connect(address).unwrap();
        stream.set_nodelay(true).unwrap();
        let mut b = Bridge::over(stream, "loop.dll").unwrap();
        let rounds = 20_000;
        let started = Instant::now();
        for _ in 0..rounds {
            assert_eq!(b.read(2, 10), Err(0x10));
        }
        let each = started.elapsed() / rounds;
        println!("one call over the bridge: {each:?}");
        drop(b);
        server.join().unwrap();
    }

    #[test]
    fn calls_go_over_the_pipe_to_the_driver_and_back() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            serve(&mut stream, |path| {
                if path == "loop.dll" {
                    Ok(Box::new(Loop::default()) as Box<dyn PassThru>)
                } else {
                    Err("NotLoadable".into())
                }
            })
            .unwrap();
        });
        let wrong = {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let refusing = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                serve(&mut stream, |_| Err("NotLoadable".into())).unwrap();
            });
            let result = Bridge::over(TcpStream::connect(address).unwrap(), "gone.dll");
            refusing.join().unwrap();
            result.err()
        };
        assert_eq!(wrong.as_deref(), Some("NotLoadable"));

        let mut b = Bridge::over(TcpStream::connect(address).unwrap(), "loop.dll").unwrap();
        assert_eq!(b.library(), "loop.dll");
        let device = b.open().unwrap();
        assert_eq!(b.connect(device, PROTOCOL_CAN, 0, 250000), Err(7));
        let channel = b.connect(device, PROTOCOL_CAN, 0, 500000).unwrap();
        let pass = Message::new(PROTOCOL_CAN, &[0; 4]);
        assert_eq!(b.start_filter(channel, 1, &pass, &pass, None), Ok(9));
        assert_eq!(b.read(channel, 10), Err(0x10));
        let frame = Message::new(PROTOCOL_CAN, &[0, 0, 7, 0xDF, 2, 1, 0]);
        assert_eq!(b.write(channel, &frame, 100), Ok(1));
        let answer = b.read(channel, 10).unwrap().unwrap();
        assert_eq!(answer.data, [1, 1, 8, 0xE0, 3, 2, 1]);
        assert_eq!(
            b.ioctl(device, &Ioctl::ReadVbatt),
            Ok(IoctlOut::Value(12480))
        );
        assert_eq!(
            b.ioctl(channel, &Ioctl::ClearRxBuffer),
            Err(ERR_NOT_SUPPORTED)
        );
        assert_eq!(b.last_error(), "loop driver");
        assert_eq!(b.disconnect(channel), 0);
        assert_eq!(b.close(device), 0);
        drop(b);
        server.join().unwrap();
    }
}
