//! The AT commands of the ELM327 and the ST commands of the OBDLink
//! (STN chips), with batched requests.

use super::*;

impl Impl {
    pub(super) fn unknown(&self) -> Response {
        Response::one(if self.profile.ok_to_all { "OK" } else { "?" }, 2.0)
    }

    fn ok(&self, level: i32) -> Response {
        if !self.supports(level) {
            return self.unknown();
        }
        Response::new(&["OK"], 2.0)
    }

    fn flag(&mut self, level: i32, set: impl FnOnce(&mut Self)) -> Response {
        if !self.supports(level) {
            return self.unknown();
        }
        set(self);
        Response::new(&["OK"], 2.0)
    }

    fn describe_protocol(&self) -> String {
        let shown = if self.active { 6 } else { self.protocol };
        if !self.auto_search {
            return protocol_description(shown).to_string();
        }
        if shown == 0 {
            return "AUTO".to_string();
        }
        format!("AUTO, {}", protocol_description(shown))
    }

    // ATSP and ATTP: the same here, since nothing is stored anyway.
    fn set_protocol(&mut self, arg: &str) -> Response {
        let mut rest = arg;
        let mut automatic = false;
        if rest.len() == 2 && rest.starts_with('A') {
            automatic = true;
            rest = after(rest, 1);
        }
        if rest == "00" && self.supports(V14) {
            rest = "0";
        }
        let mut chars = rest.chars();
        let (Some(digit), None) = (chars.next(), chars.next()) else {
            return self.unknown();
        };
        if !is_hex(digit) || hex_value(digit) > 12 {
            return self.unknown();
        }
        self.protocol = hex_value(digit);
        self.auto_search = automatic || self.protocol == 0;
        self.active = false;
        self.bus = Bus::High; // the ELM327 protocols all use pins 6 and 14
        self.stp = 0;
        Response::new(&["OK"], 2.0)
    }

    fn programmable_parameters(&self) -> Response {
        // Factory values of the parameters the ELM327 documents; everything
        // else is shown as FF (off), as on most adapters.
        fn value(index: i32) -> i32 {
            match index {
                0x03 => 0x32,
                0x04 => 0x01,
                0x06 => 0xF1,
                0x07 => 0x09,
                0x0A => 0x0A,
                0x0C => 0x68,
                0x0D => 0x0D,
                0x0E => 0x9A,
                0x10 => 0x0D,
                0x11 => 0x00,
                0x13 => 0x32,
                0x16 => 0x92,
                0x17 => 0x92,
                0x18 => 0x00,
                0x19 => 0x28,
                0x1A => 0x0A,
                0x24 => 0x00,
                0x25 => 0x00,
                0x26 => 0x00,
                0x2A => 0x38,
                0x2B => 0x02,
                0x2C => 0xE0,
                0x2D => 0x04,
                0x2E => 0x80,
                0x2F => 0x0A,
                _ => 0xFF,
            }
        }
        let mut r = Response::empty();
        let mut row = 0;
        while row < 0x30 {
            let mut line = String::new();
            for i in row..row + 4 {
                if i > row {
                    line += "  ";
                }
                line += &(hex2(i) + ":" + &hex2(value(i)) + " F");
            }
            r.lines.push(line);
            row += 4;
        }
        r.delay_ms = 8.0;
        r
    }

    pub(super) fn at_command(&mut self, c: &str) -> Response {
        if let Some(r) = self.j1939_at(c) {
            return r;
        }
        if c == "Z" || c == "WS" {
            self.reset_settings();
            let version = self.profile.version;
            return Response::new(&["", "", version], if c == "Z" { 40.0 } else { 20.0 });
        }
        if c == "D" {
            let keep_echo = self.echo;
            self.reset_settings();
            self.echo = keep_echo;
            return Response::new(&["OK"], 2.0);
        }
        if c == "I" {
            return Response::new(&[self.profile.version], 2.0);
        }
        if c == "@1" {
            return Response::new(&[self.profile.description], 2.0);
        }
        if c == "@2" {
            return if self.supports(V13) {
                Response::new(&["?"], 2.0)
            } else {
                self.unknown()
            };
        }
        if c.starts_with("@3") && c.len() == 14 {
            return self.ok(V13); // stored nowhere here
        }
        match c {
            "E0" => return self.flag(V10, |s| s.echo = false),
            "E1" => return self.flag(V10, |s| s.echo = true),
            "L0" => return self.flag(V10, |s| s.linefeeds = false),
            "L1" => return self.flag(V10, |s| s.linefeeds = true),
            "S0" => return self.flag(V13, |s| s.spaces = false),
            "S1" => return self.flag(V13, |s| s.spaces = true),
            "H0" => return self.flag(V10, |s| s.headers = false),
            "H1" => return self.flag(V10, |s| s.headers = true),
            "D0" => return self.flag(V13, |s| s.show_dlc = false),
            "D1" => return self.flag(V13, |s| s.show_dlc = true),
            "CAF0" => return self.flag(V10, |s| s.auto_format = false),
            "CAF1" => return self.flag(V10, |s| s.auto_format = true),
            _ => {}
        }
        if matches!(
            c,
            "M0" | "M1" | "R0" | "R1" | "CFC0" | "CFC1" | "AL" | "NL" | "BI" | "IB10" | "IB96"
        ) {
            return self.ok(V10);
        }
        if matches!(c, "V0" | "V1" | "JE" | "JS") {
            return self.ok(V13);
        }
        if matches!(c, "AT0" | "AT1" | "AT2" | "KW0" | "KW1") {
            return self.ok(V12);
        }
        if c == "FE" {
            return self.ok(V13A);
        }
        if matches!(
            c,
            "CSM0" | "CSM1" | "SS" | "LP" | "CEA" | "IB48" | "JTM1" | "JTM5" | "CV0000"
        ) {
            return self.ok(V14);
        }
        if c == "JHF0" || c == "JHF1" {
            return self.ok(V14B);
        }
        if c == "FCSH" || c == "FCSD" {
            return self.unknown(); // need arguments
        }
        if matches!(c, "FCSM0" | "FCSM1" | "FCSM2") {
            return self.ok(V11);
        }
        if c.starts_with("FCSH") && all_hex(after(c, 4)) && (c.len() == 7 || c.len() == 12) {
            return self.ok(V11);
        }
        if c.starts_with("FCSD")
            && all_hex(after(c, 4))
            && c.len() >= 6
            && c.len() <= 14
            && c.len().is_multiple_of(2)
        {
            return self.ok(V11);
        }
        if c == "AR" {
            if !self.supports(V12) {
                return self.unknown();
            }
            self.receive_filter = -1;
            self.can_filter = -1;
            return Response::new(&["OK"], 2.0);
        }
        if c == "CRA" {
            if !self.supports(V14B) {
                return self.unknown();
            }
            self.receive_filter = -1;
            self.can_filter = -1;
            return Response::new(&["OK"], 2.0);
        }
        if let Some(value) = hex_arg(c, "CRA", 3) {
            if !self.supports(V13) {
                return self.unknown();
            }
            self.receive_filter = value;
            return Response::new(&["OK"], 2.0);
        }
        if hex_arg(c, "CRA", 8).is_some() {
            if !self.supports(V13) {
                return self.unknown();
            }
            self.receive_filter = 0x7FFF_FFFF; // a 29 bit address: nothing on this bus matches
            return Response::new(&["OK"], 2.0);
        }
        if let Some(value) = hex_arg(c, "CF", 3) {
            self.can_filter = value;
            return Response::new(&["OK"], 2.0);
        }
        if let Some(value) = hex_arg(c, "CM", 3) {
            self.can_mask = value;
            if self.can_filter < 0 {
                self.can_filter = 0;
            }
            return Response::new(&["OK"], 2.0);
        }
        if hex_arg(c, "CF", 8).is_some()
            || hex_arg(c, "CM", 8).is_some()
            || hex_arg(c, "CP", 2).is_some()
            || hex_arg(c, "SW", 2).is_some()
            || hex_arg(c, "SR", 2).is_some()
            || hex_arg(c, "ST", 2).is_some()
        {
            return self.ok(V10);
        }
        if hex_arg(c, "IIA", 2).is_some() || hex_arg(c, "BRT", 2).is_some() {
            return self.ok(V12);
        }
        if hex_arg(c, "RA", 2).is_some() {
            return self.ok(V13);
        }
        if hex_arg(c, "TA", 2).is_some()
            || hex_arg(c, "CEA", 2).is_some()
            || hex_arg(c, "PB", 4).is_some()
        {
            return self.ok(V14);
        }
        if hex_arg(c, "CER", 2).is_some() || hex_arg(c, "AMT", 2).is_some() {
            return self.ok(V20);
        }
        if let Some(value) = hex_arg(c, "SD", 2) {
            if !self.supports(V14) {
                return self.unknown();
            }
            self.saved_byte = value;
            return Response::new(&["OK"], 2.0);
        }
        if c == "RD" {
            return if self.supports(V14) {
                Response::one(hex2(self.saved_byte), 2.0)
            } else {
                self.unknown()
            };
        }
        if c.starts_with("WM")
            && all_hex(after(c, 2))
            && c.len() >= 8
            && c.len() <= 14
            && c.len().is_multiple_of(2)
        {
            return self.ok(V10);
        }
        if c.starts_with("CV") && c.len() == 6 && after(c, 2).chars().all(|ch| ch.is_ascii_digit())
        {
            return self.ok(V10);
        }
        if let Some(value) = hex_arg(c, "SH", 3).or_else(|| hex_arg(c, "SH", 6)) {
            // With xxyyzz only the low 11 bits matter on an 11 bit bus.
            self.header = value & 0x7FF;
            return Response::new(&["OK"], 2.0);
        }
        if hex_arg(c, "SH", 8).is_some() {
            if !self.supports(V20) {
                return self.unknown();
            }
            self.header = -1; // 29 bit: nobody answers on this car
            return Response::new(&["OK"], 2.0);
        }
        if c.starts_with("SP") || c.starts_with("TP") {
            return self.set_protocol(after(c, 2));
        }
        if c == "DP" {
            return Response::one(self.describe_protocol(), 2.0);
        }
        if c == "DPN" {
            let shown = if self.active { 6 } else { self.protocol };
            let text = format!(
                "{}{}",
                if self.auto_search { "A" } else { "" },
                hex_digit(shown as usize)
            );
            return Response::one(text, 2.0);
        }
        if c == "PC" {
            self.active = false;
            return Response::new(&["OK"], 2.0);
        }
        if c == "RV" {
            // Measured at pin 16, a little below the battery itself.
            let s = simulate(self.now(), self.options.engine_start_seconds);
            return Response::one(format!("{:.1}V", s.battery - 0.05), 3.0);
        }
        if c == "IGN" {
            return if self.supports(V14) {
                Response::new(&["ON"], 2.0)
            } else {
                self.unknown()
            };
        }
        if c == "AMC" {
            return if self.supports(V20) {
                Response::new(&["00"], 2.0)
            } else {
                self.unknown()
            };
        }
        if c == "PPS" {
            return if self.supports(V11) {
                self.programmable_parameters()
            } else {
                self.unknown()
            };
        }
        if c.starts_with("PP") && c.len() >= 6 {
            let b = c.as_bytes();
            if is_hex(b[2] as char) && is_hex(b[3] as char) {
                // Accepted, but the demo has no memory that outlives it anyway.
                let rest = after(c, 4);
                let rb = rest.as_bytes();
                if rest == "ON"
                    || rest == "OFF"
                    || (rest.len() == 4
                        && rest.starts_with("SV")
                        && is_hex(rb[2] as char)
                        && is_hex(rb[3] as char))
                {
                    return self.ok(V11);
                }
                return self.unknown();
            }
        }
        if c == "CS" {
            return Response::new(&["T:00 R:00 "], 2.0);
        }
        if c == "BD" {
            let mut line = hex2(self.last_frame.len() as i32);
            for i in 0..12 {
                if self.spaces {
                    line.push(' ');
                }
                let byte = if i < self.last_frame.len() {
                    self.last_frame[i]
                } else {
                    0
                };
                line += &hex2(byte as i32);
            }
            return Response::one(line, 2.0);
        }
        if c == "RTR" {
            return if self.supports(V13) {
                Response::new(&["NO DATA"], 40.0)
            } else {
                self.unknown()
            };
        }
        if c == "MA" || hex_arg(c, "MR", 2).is_some() || hex_arg(c, "MT", 2).is_some() {
            // The bus behind the gateway stays silent: monitoring shows
            // nothing until any character stops it.
            let mut r = Response::empty();
            r.prompt = false;
            self.monitoring = true;
            return r;
        }
        self.unknown()
    }

    // STN (OBDLink) commands, as the OBDLink Family Reference and Programming
    // Manual describes them: identification, protocol presets, monitoring,
    // filters and the batched commands. Unsupported ones say "?".
    pub(super) fn stn_command(&mut self, c: &str) -> Response {
        if let Some(r) = self.stn_rate_command(c) {
            return r;
        }
        let unknown = || Response::new(&["?"], 2.0);
        let ok = || Response::new(&["OK"], 2.0);
        match c {
            "STI" => return Response::one(self.profile.stn_id.unwrap_or(""), 2.0),
            "STDI" => return Response::one(self.profile.stn_device.unwrap_or(""), 2.0),
            "STPR" => return Response::one(self.stn_protocol_number().to_string(), 2.0),
            "STPRS" => return Response::one(self.stn_protocol_text(), 2.0),
            "STPO" => return ok(),
            "STPC" => {
                self.active = false;
                return ok();
            }
            "STBC0" | "STBC1" if self.profile.batch => {
                self.batch = c == "STBC1";
                return ok();
            }
            "STBCOF0" | "STBCOF1" | "STBCOF2" if self.profile.batch => return ok(),
            "STCMM0" | "STCMM1" | "STCMM2" => return ok(),
            "STFA" | "STFAC" | "STFCA" | "STFPC" | "STFCP" | "STFBC" | "STFCB" | "STFFCC"
            | "STFCFC" => return ok(),
            _ => {}
        }
        if let Some(p) = c.strip_prefix("STP")
            && !p.is_empty()
            && p.chars().all(|ch| ch.is_ascii_digit())
        {
            return self.stn_set_protocol(p.parse().unwrap_or(0));
        }
        if let Some(mode) = c.strip_prefix("STCSWM") {
            let valid = matches!(mode, "0" | "1" | "2" | "3" | "5");
            return if valid && self.profile.single_wire {
                ok()
            } else {
                unknown()
            };
        }
        for filter in ["STFPA", "STFAP", "STFBA", "STFAB", "STFFCA", "STFAFC"] {
            if let Some(args) = c.strip_prefix(filter) {
                let parts: Vec<&str> = args.split(',').collect();
                let valid = parts.len() == 2
                    && parts[0].len() == parts[1].len()
                    && parts[0].len() <= 10
                    && all_hex(parts[0])
                    && all_hex(parts[1]);
                return if valid { ok() } else { unknown() };
            }
        }
        // STMA [n] and STM [n]: the bus stays silent, so the count is never
        // reached and monitoring runs until a character stops it.
        for monitor in ["STMA", "STM"] {
            if let Some(count) = c.strip_prefix(monitor)
                && count.chars().all(|ch| ch.is_ascii_digit())
            {
                let n: u32 = count.parse().unwrap_or(1);
                if !count.is_empty() && !(1..=32767).contains(&n) {
                    return unknown();
                }
                let mut r = Response::empty();
                r.prompt = false;
                self.monitoring = true;
                return r;
            }
        }
        unknown()
    }

    // STP presets of the manual: 11 and 12 J1850, 21 to 25 ISO 9141 and
    // 14230, 31 to 36 high speed CAN, 51 to 54 medium speed CAN, 61 to 64
    // single wire CAN. Only ISO 15765 is modelled on CAN; the car answers 11
    // bit at 500 kbit on the high speed bus.
    fn stn_set_protocol(&mut self, p: i32) -> Response {
        let (protocol, bus) = match p {
            11 => (1, Bus::High),
            12 => (2, Bus::High),
            22 => (3, Bus::High),
            24 => (4, Bus::High),
            25 => (5, Bus::High),
            33 => (6, Bus::High),
            34 => (7, Bus::High),
            35 => (8, Bus::High),
            36 => (9, Bus::High),
            53 if self.profile.medium_speed => (6, Bus::Medium),
            54 if self.profile.medium_speed => (7, Bus::Medium),
            63 if self.profile.single_wire => (6, Bus::SingleWire),
            64 if self.profile.single_wire => (7, Bus::SingleWire),
            _ => return Response::new(&["?"], 2.0),
        };
        self.protocol = protocol;
        self.auto_search = false;
        self.active = false;
        self.bus = bus;
        self.stp = p;
        Response::new(&["OK"], 2.0)
    }

    fn stn_protocol_number(&self) -> i32 {
        if self.stp > 0 {
            return self.stp;
        }
        let shown = if self.active { 6 } else { self.protocol };
        match shown {
            1 => 11,
            2 => 12,
            3 => 22,
            4 => 24,
            5 => 25,
            6 => 33,
            7 => 34,
            8 => 35,
            9 => 36,
            _ => 0,
        }
    }

    fn stn_protocol_text(&self) -> String {
        match (self.bus, self.stp) {
            (Bus::Medium, 53) => "ISO 15765, 11-bit Tx, 125kbps, DLC=8".to_string(),
            (Bus::Medium, _) => "ISO 15765, 29-bit Tx, 125kbps, DLC=8".to_string(),
            (Bus::SingleWire, 63) => "ISO 15765, 11-bit Tx, 33.3kbps, DLC=8".to_string(),
            (Bus::SingleWire, _) => "ISO 15765, 29-bit Tx, 33.3kbps, DLC=8".to_string(),
            (Bus::High, _) => self.describe_protocol(),
        }
    }

    // Batched commands (STBC 1): every part runs in turn, each answer after
    // the first starts with '|', and an error other than NO DATA ends the
    // batch.
    pub(super) fn batched(&mut self, line: &str) -> Response {
        let mut out = Response::empty();
        out.delay_ms = 0.0;
        for (i, part) in line.split('|').enumerate() {
            let r = self.execute_one(part);
            out.delay_ms += r.delay_ms;
            if !r.prompt {
                // A monitor inside the batch runs until stopped.
                out.prompt = false;
                return out;
            }
            let failed = r.lines.iter().any(|l| {
                matches!(
                    l.as_str(),
                    "?" | "CAN ERROR" | "BUS ERROR" | "BUS INIT: ERROR" | "BUS INIT: ...ERROR"
                )
            });
            let mut lines = r.lines;
            if lines.is_empty() {
                lines.push(String::new());
            }
            if i > 0 {
                lines[0] = format!("|{}", lines[0]);
            }
            out.lines.extend(lines);
            if failed {
                break;
            }
        }
        out
    }
}
