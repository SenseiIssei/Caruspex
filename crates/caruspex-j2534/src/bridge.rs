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
use std::time::{Duration, Instant};

use crate::api::constants::ERR_BRIDGE;
use crate::api::{Ioctl, IoctlOut, Message, PassThru};
use crate::wire::{self, Call, Reply};

/// The file name of the helper program, next to caruspex.exe.
pub const BRIDGE_EXE: &str = "caruspex-j2534-bridge.exe";

/// How long the helper may take to start and open its pipe.
const START_TIMEOUT: Duration = Duration::from_secs(10);

/// Both directions of the pipe.
trait Duplex: Read + Write + Send {}
impl<T: Read + Write + Send> Duplex for T {}

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
    let mut pipe = pipe::create_and_wait(name)?;
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
    pub fn create_and_wait(name: &str) -> io::Result<File> {
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
    stream: Option<Box<dyn Duplex>>,
    child: Option<Child>,
    library: String,
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
        let mut bridge = Bridge {
            stream: Some(Box::new(stream)),
            child: Some(child),
            library: library.to_string(),
        };
        bridge.load()?;
        Ok(bridge)
    }

    /// A bridge over any stream to a [`serve`] loop, for the tests.
    pub fn over(stream: impl Read + Write + Send + 'static, library: &str) -> Result<Self, String> {
        let mut bridge = Bridge {
            stream: Some(Box::new(stream)),
            child: None,
            library: library.to_string(),
        };
        bridge.load()?;
        Ok(bridge)
    }

    fn load(&mut self) -> Result<(), String> {
        match self.call(&Call::Load(self.library.clone())) {
            Some(Reply::Loaded(error)) if error.is_empty() => Ok(()),
            Some(Reply::Loaded(error)) => Err(error),
            _ => Err("no answer from the bridge".into()),
        }
    }

    /// One call and its reply; `None` once the pipe is broken.
    fn call(&mut self, call: &Call) -> Option<Reply> {
        let stream = self.stream.as_mut()?;
        let answer = wire::send(stream, &call.encode())
            .and_then(|()| wire::receive(stream))
            .ok()
            .flatten()
            .and_then(|frame| Reply::decode(&frame).ok());
        if answer.is_none() {
            self.stream = None; // the helper is gone, every later call fails at once
        }
        answer
    }

    fn status(&mut self, call: Call) -> i32 {
        match self.call(&call) {
            Some(Reply::Status(s)) => s,
            _ => ERR_BRIDGE,
        }
    }

    fn number(&mut self, call: Call) -> Result<u32, i32> {
        match self.call(&call) {
            Some(Reply::Number(n)) => n,
            Some(Reply::Status(s)) => Err(s),
            _ => Err(ERR_BRIDGE),
        }
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        // Closing the pipe ends the helper; it gets a moment, then it is ended.
        self.stream = None;
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
            _ => Err(ERR_BRIDGE),
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
            _ => Err(ERR_BRIDGE),
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
