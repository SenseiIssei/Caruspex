//! What Caruspex and the bridge say to each other over the pipe: one call,
//! one reply, each as a length (u32, little endian) and that many bytes.
//! The numbers are little endian, texts and byte strings carry their length
//! in front.

use std::io::{self, Read, Write};

use crate::api::{Ioctl, IoctlOut, Message};

/// A frame larger than this is a broken pipe, not a message.
const MAX_FRAME: usize = 1 << 20;

/// One PassThru function call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Call {
    /// Load the driver DLL at this path.
    Load(String),
    Open,
    Close(u32),
    Connect {
        device: u32,
        protocol: u32,
        flags: u32,
        baud: u32,
    },
    Disconnect(u32),
    Read {
        channel: u32,
        timeout_ms: u32,
    },
    Write {
        channel: u32,
        message: Message,
        timeout_ms: u32,
    },
    Filter {
        channel: u32,
        kind: u32,
        mask: Message,
        pattern: Message,
        flow: Option<Message>,
    },
    Ioctl {
        target: u32,
        request: Ioctl,
    },
    LastError,
}

/// The answer to a [`Call`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Reply {
    /// Load: empty if the driver loaded, else why not.
    Loaded(String),
    Status(i32),
    Number(Result<u32, i32>),
    Read(Result<Option<Message>, i32>),
    Ioctl(Result<IoctlOut, i32>),
    Text(String),
}

fn broken(what: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, what.to_string())
}

#[derive(Default)]
struct Out(Vec<u8>);

impl Out {
    fn u8(&mut self, v: u8) -> &mut Self {
        self.0.push(v);
        self
    }
    fn u32(&mut self, v: u32) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn i32(&mut self, v: i32) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn bytes(&mut self, v: &[u8]) -> &mut Self {
        self.u32(v.len() as u32);
        self.0.extend_from_slice(v);
        self
    }
    fn message(&mut self, m: &Message) -> &mut Self {
        self.u32(m.protocol_id)
            .u32(m.rx_status)
            .u32(m.tx_flags)
            .u32(m.timestamp)
            .u32(m.extra_data_index)
            .bytes(&m.data)
    }
    fn status<T>(&mut self, v: &Result<T, i32>, ok: impl FnOnce(&mut Self, &T)) -> &mut Self {
        match v {
            Ok(value) => {
                self.u8(0);
                ok(self, value);
            }
            Err(status) => {
                self.u8(1).i32(*status);
            }
        }
        self
    }
}

struct In<'a>(&'a [u8]);

impl In<'_> {
    fn take(&mut self, n: usize) -> io::Result<&[u8]> {
        if self.0.len() < n {
            return Err(broken("short frame"));
        }
        let (head, rest) = self.0.split_at(n);
        self.0 = rest;
        Ok(head)
    }
    fn u8(&mut self) -> io::Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> io::Result<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn i32(&mut self) -> io::Result<i32> {
        Ok(self.u32()? as i32)
    }
    fn bytes(&mut self) -> io::Result<Vec<u8>> {
        let n = self.u32()? as usize;
        Ok(self.take(n)?.to_vec())
    }
    fn text(&mut self) -> io::Result<String> {
        String::from_utf8(self.bytes()?).map_err(|_| broken("text"))
    }
    fn message(&mut self) -> io::Result<Message> {
        Ok(Message {
            protocol_id: self.u32()?,
            rx_status: self.u32()?,
            tx_flags: self.u32()?,
            timestamp: self.u32()?,
            extra_data_index: self.u32()?,
            data: self.bytes()?,
        })
    }
    fn status(&mut self) -> io::Result<Result<(), i32>> {
        match self.u8()? {
            0 => Ok(Ok(())),
            1 => Ok(Err(self.i32()?)),
            _ => Err(broken("status")),
        }
    }
}

fn encode_ioctl(out: &mut Out, request: &Ioctl) {
    match request {
        Ioctl::SetConfig(pairs) => {
            out.u8(0).u32(pairs.len() as u32);
            for &(p, v) in pairs {
                out.u32(p).u32(v);
            }
        }
        Ioctl::GetConfig(parameters) => {
            out.u8(1).u32(parameters.len() as u32);
            for &p in parameters {
                out.u32(p);
            }
        }
        Ioctl::ReadVbatt => {
            out.u8(2);
        }
        Ioctl::FiveBaudInit(address, room) => {
            out.u8(3).bytes(address).u32(*room);
        }
        Ioctl::FastInit(m) => {
            out.u8(4).message(m);
        }
        Ioctl::ClearTxBuffer => {
            out.u8(5);
        }
        Ioctl::ClearRxBuffer => {
            out.u8(6);
        }
    }
}

fn decode_ioctl(input: &mut In<'_>) -> io::Result<Ioctl> {
    Ok(match input.u8()? {
        0 => {
            let n = input.u32()? as usize;
            let mut pairs = Vec::with_capacity(n.min(64));
            for _ in 0..n {
                pairs.push((input.u32()?, input.u32()?));
            }
            Ioctl::SetConfig(pairs)
        }
        1 => {
            let n = input.u32()? as usize;
            let mut parameters = Vec::with_capacity(n.min(64));
            for _ in 0..n {
                parameters.push(input.u32()?);
            }
            Ioctl::GetConfig(parameters)
        }
        2 => Ioctl::ReadVbatt,
        3 => Ioctl::FiveBaudInit(input.bytes()?, input.u32()?),
        4 => Ioctl::FastInit(input.message()?),
        5 => Ioctl::ClearTxBuffer,
        6 => Ioctl::ClearRxBuffer,
        _ => return Err(broken("ioctl")),
    })
}

impl Call {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Out::default();
        match self {
            Call::Load(path) => {
                out.u8(0).bytes(path.as_bytes());
            }
            Call::Open => {
                out.u8(1);
            }
            Call::Close(device) => {
                out.u8(2).u32(*device);
            }
            Call::Connect {
                device,
                protocol,
                flags,
                baud,
            } => {
                out.u8(3).u32(*device).u32(*protocol).u32(*flags).u32(*baud);
            }
            Call::Disconnect(channel) => {
                out.u8(4).u32(*channel);
            }
            Call::Read {
                channel,
                timeout_ms,
            } => {
                out.u8(5).u32(*channel).u32(*timeout_ms);
            }
            Call::Write {
                channel,
                message,
                timeout_ms,
            } => {
                out.u8(6).u32(*channel).message(message).u32(*timeout_ms);
            }
            Call::Filter {
                channel,
                kind,
                mask,
                pattern,
                flow,
            } => {
                out.u8(7)
                    .u32(*channel)
                    .u32(*kind)
                    .message(mask)
                    .message(pattern);
                match flow {
                    Some(f) => out.u8(1).message(f),
                    None => out.u8(0),
                };
            }
            Call::Ioctl { target, request } => {
                out.u8(8).u32(*target);
                encode_ioctl(&mut out, request);
            }
            Call::LastError => {
                out.u8(9);
            }
        }
        out.0
    }

    pub fn decode(bytes: &[u8]) -> io::Result<Call> {
        let mut input = In(bytes);
        Ok(match input.u8()? {
            0 => Call::Load(input.text()?),
            1 => Call::Open,
            2 => Call::Close(input.u32()?),
            3 => Call::Connect {
                device: input.u32()?,
                protocol: input.u32()?,
                flags: input.u32()?,
                baud: input.u32()?,
            },
            4 => Call::Disconnect(input.u32()?),
            5 => Call::Read {
                channel: input.u32()?,
                timeout_ms: input.u32()?,
            },
            6 => Call::Write {
                channel: input.u32()?,
                message: input.message()?,
                timeout_ms: input.u32()?,
            },
            7 => Call::Filter {
                channel: input.u32()?,
                kind: input.u32()?,
                mask: input.message()?,
                pattern: input.message()?,
                flow: match input.u8()? {
                    0 => None,
                    _ => Some(input.message()?),
                },
            },
            8 => Call::Ioctl {
                target: input.u32()?,
                request: decode_ioctl(&mut input)?,
            },
            9 => Call::LastError,
            _ => return Err(broken("call")),
        })
    }
}

impl Reply {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Out::default();
        match self {
            Reply::Loaded(error) => {
                out.u8(0).bytes(error.as_bytes());
            }
            Reply::Status(status) => {
                out.u8(1).i32(*status);
            }
            Reply::Number(v) => {
                out.u8(2).status(v, |o, n| {
                    o.u32(*n);
                });
            }
            Reply::Read(v) => {
                out.u8(3).status(v, |o, m| {
                    match m {
                        Some(m) => o.u8(1).message(m),
                        None => o.u8(0),
                    };
                });
            }
            Reply::Ioctl(v) => {
                out.u8(4).status(v, |o, result| {
                    match result {
                        IoctlOut::Nothing => o.u8(0),
                        IoctlOut::Config(pairs) => {
                            o.u8(1).u32(pairs.len() as u32);
                            for &(p, v) in pairs {
                                o.u32(p).u32(v);
                            }
                            o
                        }
                        IoctlOut::Value(v) => o.u8(2).u32(*v),
                        IoctlOut::Bytes(b) => o.u8(3).bytes(b),
                        IoctlOut::Message(m) => o.u8(4).message(m),
                    };
                });
            }
            Reply::Text(text) => {
                out.u8(5).bytes(text.as_bytes());
            }
        }
        out.0
    }

    pub fn decode(bytes: &[u8]) -> io::Result<Reply> {
        let mut input = In(bytes);
        Ok(match input.u8()? {
            0 => Reply::Loaded(input.text()?),
            1 => Reply::Status(input.i32()?),
            2 => Reply::Number(match input.status()? {
                Ok(()) => Ok(input.u32()?),
                Err(s) => Err(s),
            }),
            3 => Reply::Read(match input.status()? {
                Ok(()) => Ok(match input.u8()? {
                    0 => None,
                    _ => Some(input.message()?),
                }),
                Err(s) => Err(s),
            }),
            4 => Reply::Ioctl(match input.status()? {
                Ok(()) => Ok(match input.u8()? {
                    0 => IoctlOut::Nothing,
                    1 => {
                        let n = input.u32()? as usize;
                        let mut pairs = Vec::with_capacity(n.min(64));
                        for _ in 0..n {
                            pairs.push((input.u32()?, input.u32()?));
                        }
                        IoctlOut::Config(pairs)
                    }
                    2 => IoctlOut::Value(input.u32()?),
                    3 => IoctlOut::Bytes(input.bytes()?),
                    4 => IoctlOut::Message(input.message()?),
                    _ => return Err(broken("ioctl reply")),
                }),
                Err(s) => Err(s),
            }),
            5 => Reply::Text(input.text()?),
            _ => return Err(broken("reply")),
        })
    }
}

/// One frame out: its length, then the bytes.
pub(crate) fn send(to: &mut impl Write, body: &[u8]) -> io::Result<()> {
    let mut frame = Vec::with_capacity(4 + body.len());
    frame.extend_from_slice(&(body.len() as u32).to_le_bytes());
    frame.extend_from_slice(body);
    to.write_all(&frame)?;
    to.flush()
}

/// One frame in; `Ok(None)` when the other side closed the pipe.
pub(crate) fn receive(from: &mut impl Read) -> io::Result<Option<Vec<u8>>> {
    let mut length = [0u8; 4];
    match from.read_exact(&mut length) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => return Ok(None),
        Err(e) => return Err(e),
    }
    let length = u32::from_le_bytes(length) as usize;
    if length > MAX_FRAME {
        return Err(broken("frame too large"));
    }
    let mut body = vec![0u8; length];
    from.read_exact(&mut body)?;
    Ok(Some(body))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message() -> Message {
        Message {
            protocol_id: 5,
            rx_status: 1,
            tx_flags: 0x100,
            timestamp: 7,
            extra_data_index: 0,
            data: vec![0, 0, 7, 0xE0, 2, 1, 0],
        }
    }

    #[test]
    fn every_call_and_reply_comes_back_as_it_went() {
        let calls = [
            Call::Load("C:\\Program Files (x86)\\Old\\op20pt32.dll".into()),
            Call::Open,
            Call::Close(1),
            Call::Connect {
                device: 1,
                protocol: 5,
                flags: 0x800,
                baud: 500000,
            },
            Call::Disconnect(2),
            Call::Read {
                channel: 2,
                timeout_ms: 50,
            },
            Call::Write {
                channel: 2,
                message: message(),
                timeout_ms: 100,
            },
            Call::Filter {
                channel: 2,
                kind: 1,
                mask: message(),
                pattern: Message::new(5, &[0; 4]),
                flow: None,
            },
            Call::Filter {
                channel: 2,
                kind: 3,
                mask: message(),
                pattern: message(),
                flow: Some(message()),
            },
            Call::Ioctl {
                target: 2,
                request: Ioctl::SetConfig(vec![(3, 0), (0x20, 1)]),
            },
            Call::Ioctl {
                target: 2,
                request: Ioctl::GetConfig(vec![1]),
            },
            Call::Ioctl {
                target: 1,
                request: Ioctl::ReadVbatt,
            },
            Call::Ioctl {
                target: 4,
                request: Ioctl::FiveBaudInit(vec![0x01], 8),
            },
            Call::Ioctl {
                target: 4,
                request: Ioctl::FastInit(message()),
            },
            Call::Ioctl {
                target: 4,
                request: Ioctl::ClearRxBuffer,
            },
            Call::Ioctl {
                target: 4,
                request: Ioctl::ClearTxBuffer,
            },
            Call::LastError,
        ];
        for call in calls {
            assert_eq!(Call::decode(&call.encode()).unwrap(), call);
        }
        let replies = [
            Reply::Loaded(String::new()),
            Reply::Loaded("not loadable".into()),
            Reply::Status(0x10),
            Reply::Number(Ok(2)),
            Reply::Number(Err(7)),
            Reply::Read(Ok(None)),
            Reply::Read(Ok(Some(message()))),
            Reply::Read(Err(0x10)),
            Reply::Ioctl(Ok(IoctlOut::Nothing)),
            Reply::Ioctl(Ok(IoctlOut::Config(vec![(1, 500000)]))),
            Reply::Ioctl(Ok(IoctlOut::Value(12480))),
            Reply::Ioctl(Ok(IoctlOut::Bytes(vec![0xEF, 0x8F]))),
            Reply::Ioctl(Ok(IoctlOut::Message(message()))),
            Reply::Ioctl(Err(0x1A)),
            Reply::Text("fake driver".into()),
        ];
        for reply in replies {
            assert_eq!(Reply::decode(&reply.encode()).unwrap(), reply);
        }
        assert!(Call::decode(&[42]).is_err());
        assert!(Reply::decode(&[2, 0]).is_err());
    }

    #[test]
    fn frames_carry_their_length_and_end_with_the_pipe() {
        let mut pipe = Vec::new();
        send(&mut pipe, b"abc").unwrap();
        send(&mut pipe, b"").unwrap();
        let mut from = &pipe[..];
        assert_eq!(receive(&mut from).unwrap().unwrap(), b"abc");
        assert_eq!(receive(&mut from).unwrap().unwrap(), b"");
        assert_eq!(receive(&mut from).unwrap(), None);
        let huge = (u32::MAX).to_le_bytes();
        assert!(receive(&mut &huge[..]).is_err());
    }
}
