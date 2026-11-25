//! Wi-Fi dongle, typically 192.168.0.10:35000. Also handy for testing
//! against the bundled simulator on 127.0.0.1.

use std::io::{ErrorKind, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use caruspex_i18n::l;

use crate::transport::Transport;

/// How long connecting may take. A Wi-Fi adapter answers within
/// milliseconds; without a limit the system waits 21 s (Windows) to two
/// minutes (Linux) when the computer is not on the adapter's network, and
/// the worker with it.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// A command is a few bytes; a send that does not get rid of them in this
/// time goes nowhere.
const WRITE_TIMEOUT: Duration = Duration::from_secs(2);
/// A connection that carries nothing for this long is asked by the system
/// whether the other side is still there (TCP keepalive): a Wi-Fi that went
/// out of range closes nothing, so otherwise an idle connection would look
/// alive for ever. The question is an empty TCP segment, the adapter's
/// program never sees it.
///
/// Generous on purpose: a Wi-Fi that stumbles for some seconds must not cost
/// the connection, and an idle one has time.
const KEEPALIVE_AFTER: Duration = Duration::from_secs(10);
const KEEPALIVE_EVERY: Duration = Duration::from_secs(3);
const KEEPALIVE_TRIES: u32 = 5;

#[derive(Default)]
pub struct TcpTransport {
    socket: Option<TcpStream>,
    name: String,
    /// The adapter closed the connection or it broke.
    gone: bool,
}

impl TcpTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, host: &str, port: u16) -> Result<(), String> {
        self.open_within(host, port, CONNECT_TIMEOUT)
    }

    fn open_within(&mut self, host: &str, port: u16, limit: Duration) -> Result<(), String> {
        self.socket = None;
        self.gone = false;
        let service = port.to_string();
        let addresses: Vec<_> = match (host, port).to_socket_addrs() {
            Ok(list) => list.collect(),
            Err(_) => Vec::new(),
        };
        if addresses.is_empty() {
            return Err(format!(
                "{}{}{}",
                l("Adresse ", "Cannot resolve address "),
                host,
                l(" nicht auflösbar", "")
            ));
        }

        let Some(stream) = addresses
            .iter()
            .find_map(|a| TcpStream::connect_timeout(a, limit).ok())
        else {
            return Err(format!(
                "{}{}:{}{}",
                l("Keine Verbindung zu ", "No connection to "),
                host,
                service,
                l(
                    " (ist das WLAN des Adapters verbunden?)",
                    " (are you connected to the adapter's Wi-Fi?)"
                )
            ));
        };
        let _ = stream.set_nodelay(true);
        let _ = stream.set_write_timeout(Some(WRITE_TIMEOUT));
        let keepalive = socket2::TcpKeepalive::new()
            .with_time(KEEPALIVE_AFTER)
            .with_interval(KEEPALIVE_EVERY)
            .with_retries(KEEPALIVE_TRIES);
        let _ = socket2::SockRef::from(&stream).set_tcp_keepalive(&keepalive);
        self.socket = Some(stream);
        self.name = format!("{}:{}{}", host, service, l(" (WLAN)", " (Wi-Fi)"));
        Ok(())
    }
}

impl Transport for TcpTransport {
    fn write(&mut self, data: &[u8]) -> bool {
        match self.socket.as_mut() {
            Some(s) if !self.gone => match s.write_all(data) {
                Ok(()) => true,
                Err(e) => {
                    self.gone = !temporary(&e);
                    false
                }
            },
            _ => false,
        }
    }

    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        let started = Instant::now();
        let gone = self.gone;
        let Some(s) = self.socket.as_mut().filter(|_| !gone) else {
            if gone {
                crate::timing::wait_out(started, timeout_ms);
            }
            return Vec::new();
        };
        // A zero timeout would mean "wait forever" to the socket.
        let _ = s.set_read_timeout(Some(Duration::from_millis(timeout_ms.max(1))));
        let mut buffer = [0u8; 512];
        match s.read(&mut buffer) {
            Ok(n) if n > 0 => buffer[..n].to_vec(),
            Err(e) if temporary(&e) => Vec::new(),
            // Closed by the adapter (0 bytes) or broken: every later read
            // would come back at once.
            _ => {
                self.gone = true;
                crate::timing::wait_out(started, timeout_ms);
                Vec::new()
            }
        }
    }

    fn flush_input(&mut self) {
        let Some(s) = self.socket.as_mut() else {
            return;
        };
        let _ = s.set_nonblocking(true);
        let mut buffer = [0u8; 512];
        while matches!(s.read(&mut buffer), Ok(n) if n > 0) {}
        let _ = s.set_nonblocking(false);
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn lost(&mut self) -> bool {
        let Some(s) = self.socket.as_ref().filter(|_| !self.gone) else {
            return true;
        };
        // Looks at the next byte without taking it: 0 bytes means closed.
        if s.set_nonblocking(true).is_err() {
            return false;
        }
        let seen = s.peek(&mut [0u8; 1]);
        let _ = s.set_nonblocking(false);
        self.gone = match seen {
            Ok(n) => n == 0,
            Err(e) => !temporary(&e),
        };
        self.gone
    }
}

/// Nothing there right now, as opposed to a connection that is gone.
fn temporary(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn talks_to_a_local_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut peer, _) = listener.accept().unwrap();
            let mut buf = [0u8; 16];
            let n = peer.read(&mut buf).unwrap();
            assert_eq!(&buf[..n], b"ATZ\r");
            peer.write_all(b"ELM327 v1.5\r\r>").unwrap();
            std::thread::sleep(Duration::from_millis(200));
        });
        let mut t = TcpTransport::new();
        t.open("127.0.0.1", port).unwrap();
        assert!(t.name().starts_with(&format!("127.0.0.1:{port} (")));
        assert!(t.write(b"ATZ\r"));
        let mut got = Vec::new();
        for _ in 0..20 {
            got.extend(t.read(100));
            if got.contains(&b'>') {
                break;
            }
        }
        assert_eq!(got, b"ELM327 v1.5\r\r>");
        t.flush_input();
        assert!(t.read(10).is_empty());
        server.join().unwrap();
    }

    #[test]
    fn a_connection_the_adapter_closed_waits_instead_of_spinning() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (peer, _) = listener.accept().unwrap();
            drop(peer);
        });
        let mut t = TcpTransport::new();
        t.open("127.0.0.1", port).unwrap();
        server.join().unwrap();
        // The ELM327 driver reads in a loop until its deadline: count how
        // often it would read during 300 ms. Before, every read came back
        // at once and this was in the hundreds of thousands.
        let started = Instant::now();
        let mut reads = 0;
        while started.elapsed() < Duration::from_millis(300) {
            assert!(t.read(50).is_empty());
            reads += 1;
        }
        assert!(reads <= 8, "{reads} reads");
        assert!(!t.write(b"ATZ\r"));
    }

    #[test]
    fn a_connection_the_adapter_closed_is_lost_without_a_request() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let mut t = TcpTransport::new();
        t.open("127.0.0.1", port).unwrap();
        let (mut peer, _) = listener.accept().unwrap();
        assert!(!t.lost(), "idle is not lost");
        peer.write_all(b"OK\r\r>").unwrap();
        std::thread::sleep(Duration::from_millis(50));
        assert!(!t.lost());
        // Looking took nothing away.
        assert_eq!(t.read(500), b"OK\r\r>");
        drop(peer);
        let until = Instant::now() + Duration::from_secs(2);
        while !t.lost() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(t.lost());
        assert!(!t.write(b"ATI\r"));
    }

    #[test]
    fn an_idle_connection_is_watched_by_the_system() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut t = TcpTransport::new();
        t.open("127.0.0.1", listener.local_addr().unwrap().port())
            .unwrap();
        let socket = t.socket.as_ref().unwrap();
        assert!(socket2::SockRef::from(socket).keepalive().unwrap());
    }

    #[test]
    fn connecting_to_an_address_nobody_answers_gives_up_in_time() {
        // TEST-NET-1 (RFC 5737): routed nowhere. Without a limit Windows
        // waits 21 s here and Linux up to two minutes.
        let started = Instant::now();
        let mut t = TcpTransport::new();
        assert!(
            t.open_within("192.0.2.1", 35000, Duration::from_millis(300))
                .is_err()
        );
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn a_closed_transport_does_nothing() {
        let mut t = TcpTransport::new();
        assert!(!t.write(b"x"));
        assert!(t.read(1).is_empty());
    }
}
