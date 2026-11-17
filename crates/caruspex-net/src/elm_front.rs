//! The part of an ELM327 that every front end of this crate shares: the
//! display settings (echo, line feeds, spaces, headers), CAN auto
//! formatting, the header (ATSH) and the receive filter (ATCRA), how a
//! command is told apart (AT, ST, hex bytes) and how the answer goes out
//! with the prompt. [`crate::can_bus::CanElm`] and [`crate::enet::EnetElm`]
//! put their own way to the car behind it through [`ElmBackend`].

use crate::text::{hex, is_hex, normalize, parse_hex};

/// The settings and the answer buffer of an emulated ELM327.
pub(crate) struct ElmFront {
    /// Answer bytes not read yet.
    pub pending: Vec<u8>,
    pub echo: bool,
    pub linefeeds: bool,
    pub spaces: bool,
    pub headers: bool,
    pub auto_format: bool,
    /// ATSH (the lower three bytes for 29 bit).
    pub header: u32,
    /// ATCRA.
    pub filter: Option<u32>,
}

impl ElmFront {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
            echo: true,
            linefeeds: false,
            spaces: true,
            headers: false,
            auto_format: true,
            header: 0x7DF,
            filter: None,
        }
    }

    /// What ATZ and ATD put back; the answer buffer stays.
    pub fn reset(&mut self) {
        self.echo = true;
        self.linefeeds = false;
        self.spaces = true;
        self.headers = false;
        self.auto_format = true;
        self.header = 0x7DF;
        self.filter = None;
    }

    pub fn end(&self) -> &'static str {
        if self.linefeeds { "\r\n" } else { "\r" }
    }

    pub fn hex(&self, bytes: &[u8]) -> String {
        hex(bytes, self.spaces)
    }

    /// The display and formatting switches every front end knows (E, L, S,
    /// H, CAF); `None` for any other AT command.
    pub fn setting(&mut self, c: &str) -> Option<Vec<String>> {
        let flag = match c {
            "E0" | "E1" => &mut self.echo,
            "L0" | "L1" => &mut self.linefeeds,
            "S0" | "S1" => &mut self.spaces,
            "H0" | "H1" => &mut self.headers,
            "CAF0" | "CAF1" => &mut self.auto_format,
            _ => return None,
        };
        *flag = c.ends_with('1');
        Some(vec!["OK".into()])
    }

    /// Echo, the answer lines and (if `prompt`) the empty line with the
    /// prompt, into the buffer. Echo and line end are the ones that held
    /// when the command came, as an ELM327 answers ATE0 and ATL1.
    fn answer(&mut self, data: &[u8], echo: bool, end: &str, lines: &[String], prompt: bool) {
        let mut out = Vec::new();
        if echo {
            let cut = data.iter().position(|&c| c == b'\r').unwrap_or(data.len());
            out.extend_from_slice(&data[..cut]);
            out.push(b'\r');
        }
        for line in lines {
            out.extend_from_slice(line.as_bytes());
            out.extend_from_slice(end.as_bytes());
        }
        if prompt {
            out.extend_from_slice(end.as_bytes());
            out.push(b'>');
        }
        self.pending.extend_from_slice(&out);
    }
}

/// What a front end does with the commands [`ElmFront`] does not answer
/// itself.
pub(crate) trait ElmBackend {
    fn front(&mut self) -> &mut ElmFront;
    /// An AT command without the "AT" that is not a display setting.
    fn at(&mut self, command: &str) -> Vec<String>;
    /// Request bytes with CAN auto formatting on (the front end adds the
    /// PCI byte and joins the answer).
    fn formatted(&mut self, bytes: &[u8]) -> Vec<String>;
    /// Raw frame bytes with auto formatting off (BMW 6F1 addressing).
    fn raw(&mut self, bytes: &[u8]) -> Vec<String>;
    /// Whether the prompt follows the answer; not while the bus monitor runs.
    fn prompt(&self) -> bool {
        true
    }
}

/// One command line written to the front end: handled, and the answer with
/// echo and prompt put into the buffer for the next read.
pub(crate) fn respond(backend: &mut impl ElmBackend, data: &[u8]) {
    let echo = backend.front().echo;
    let end = backend.front().end();
    let lines = handle(backend, data);
    let prompt = backend.prompt();
    backend.front().answer(data, echo, end, &lines, prompt);
}

/// The answer lines to one command line as an ELM327 gives them.
pub(crate) fn handle(backend: &mut impl ElmBackend, data: &[u8]) -> Vec<String> {
    let command = normalize(data);
    if command.is_empty() {
        return Vec::new();
    }
    if let Some(rest) = command.strip_prefix("AT") {
        if let Some(lines) = backend.front().setting(rest) {
            return lines;
        }
        return backend.at(rest);
    }
    if command.starts_with("ST") {
        return vec!["?".into()]; // no STN commands here
    }
    let mut digits = command;
    // A trailing single digit is the ELM327's "expected answers" hint.
    if digits.len() % 2 == 1 {
        digits.pop();
    }
    if !is_hex(&digits) {
        return vec!["?".into()];
    }
    if backend.front().auto_format {
        backend.formatted(&parse_hex(&digits))
    } else {
        backend.raw(&parse_hex(&digits))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Echo {
        front: ElmFront,
    }

    impl ElmBackend for Echo {
        fn front(&mut self) -> &mut ElmFront {
            &mut self.front
        }
        fn at(&mut self, command: &str) -> Vec<String> {
            vec![format!("AT {command}")]
        }
        fn formatted(&mut self, bytes: &[u8]) -> Vec<String> {
            vec![format!("F {}", self.front.hex(bytes))]
        }
        fn raw(&mut self, bytes: &[u8]) -> Vec<String> {
            vec![format!("R {}", self.front.hex(bytes))]
        }
    }

    #[test]
    fn commands_are_told_apart_and_answered_with_the_prompt() {
        let mut e = Echo {
            front: ElmFront::new(),
        };
        assert_eq!(handle(&mut e, b"at e0\r"), ["OK"]);
        assert!(!e.front.echo);
        assert_eq!(handle(&mut e, b"ATDP\r"), ["AT DP"]);
        assert_eq!(handle(&mut e, b"STDI\r"), ["?"]);
        assert_eq!(handle(&mut e, b"01001\r"), ["F 01 00"]);
        assert_eq!(handle(&mut e, b"0G\r"), ["?"]);
        assert!(handle(&mut e, b"\r").is_empty());
        handle(&mut e, b"ATCAF0\r");
        handle(&mut e, b"ATS0\r");
        assert_eq!(handle(&mut e, b"12021001\r"), ["R 12021001"]);

        handle(&mut e, b"ATCAF1\r");
        respond(&mut e, b"0100\r");
        assert_eq!(e.front.pending, b"F 0100\r\r>");
        e.front.pending.clear();
        // Echo and line end as they were when the command came.
        e.front.echo = true;
        respond(&mut e, b"ATE0\r");
        assert_eq!(e.front.pending, b"ATE0\rOK\r\r>");
        e.front.pending.clear();
        respond(&mut e, b"ATL1\r");
        assert_eq!(e.front.pending, b"OK\r\r>");
        e.front.pending.clear();
        respond(&mut e, b"ATI\r");
        assert_eq!(e.front.pending, b"AT I\r\n\r\n>");
    }
}
