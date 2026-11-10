//! Wi-Fi dongle, typically 192.168.0.10:35000. Also handy for testing
//! against the bundled simulator on 127.0.0.1.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use caruspex_i18n::l;

use crate::transport::Transport;

#[derive(Default)]
pub struct TcpTransport {
    socket: Option<TcpStream>,
    name: String,
}

impl TcpTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, host: &str, port: u16) -> Result<(), String> {
        self.socket = None;
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

        let Some(stream) = addresses.iter().find_map(|a| TcpStream::connect(a).ok()) else {
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
        self.socket = Some(stream);
        self.name = format!("{}:{}{}", host, service, l(" (WLAN)", " (Wi-Fi)"));
        Ok(())
    }
}

impl Transport for TcpTransport {
    fn write(&mut self, data: &[u8]) -> bool {
        match self.socket.as_mut() {
            Some(s) => s.write_all(data).is_ok(),
            None => false,
        }
    }

    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        let Some(s) = self.socket.as_mut() else {
            return Vec::new();
        };
        // A zero timeout would mean "wait forever" to the socket.
        let _ = s.set_read_timeout(Some(Duration::from_millis(timeout_ms.max(1))));
        let mut buffer = [0u8; 512];
        match s.read(&mut buffer) {
            Ok(n) if n > 0 => buffer[..n].to_vec(),
            _ => Vec::new(),
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
    fn a_closed_transport_does_nothing() {
        let mut t = TcpTransport::new();
        assert!(!t.write(b"x"));
        assert!(t.read(1).is_empty());
    }
}
