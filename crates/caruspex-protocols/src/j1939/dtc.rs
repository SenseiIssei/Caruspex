//! Faults in J1939: the diagnostic messages DM1 (active) and DM2
//! (previously active). Both start with two bytes of lamp status and
//! flashing, followed by four bytes per fault:
//!
//! - the suspect parameter number (SPN, 19 bits): which value or part;
//! - the failure mode identifier (FMI, 5 bits): what is wrong with it;
//! - the conversion method bit (CM) and the occurrence count (7 bits).
//!
//! Older control units packed the SPN in other ways. With CM = 0 the
//! layout is the one used today (version 4): the lowest eight bits first,
//! then the next eight, the top three bits in the high bits of the third
//! byte. CM = 1 says "one of the older layouts" without saying which:
//!
//! - version 1: the 19 bits most significant first over the three bytes;
//! - version 2: the upper 16 bits as a little-endian word in the first two
//!   bytes, the lowest three bits in the third byte;
//! - version 3: the layout of version 4, only with CM set.
//!
//! [`SpnConversion`] names them; for CM = 1 the caller may fix the version
//! (some makers document it), otherwise the version that gives a known SPN
//! wins, version 3 when none or several do.

use super::fmi::fmi_text;
use super::params::find_param;

/// One of the four lamps a control unit reports with its faults.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LampState {
    #[default]
    Off,
    On,
    /// The two bits say "error".
    Error,
    /// The unit has no such lamp.
    NotAvailable,
}

impl LampState {
    fn from_bits(bits: u8) -> LampState {
        match bits & 0x3 {
            0 => LampState::Off,
            1 => LampState::On,
            2 => LampState::Error,
            _ => LampState::NotAvailable,
        }
    }
    fn bits(self) -> u8 {
        match self {
            LampState::Off => 0,
            LampState::On => 1,
            LampState::Error => 2,
            LampState::NotAvailable => 3,
        }
    }
}

/// How a lamp that is on flashes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LampFlash {
    /// Steady (the two bits say "not available" or "reserved").
    #[default]
    Steady,
    /// About once a second.
    Slow,
    /// About twice a second.
    Fast,
}

impl LampFlash {
    fn from_bits(bits: u8) -> LampFlash {
        match bits & 0x3 {
            0 => LampFlash::Slow,
            1 => LampFlash::Fast,
            _ => LampFlash::Steady,
        }
    }
    fn bits(self) -> u8 {
        match self {
            LampFlash::Slow => 0,
            LampFlash::Fast => 1,
            LampFlash::Steady => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Lamp {
    pub state: LampState,
    pub flash: LampFlash,
}

impl Lamp {
    pub fn on(self) -> bool {
        self.state == LampState::On
    }
}

/// The four lamps of a DM1 or DM2.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Lamps {
    /// The emission warning lamp (MIL), the one of OBD.
    pub mil: Lamp,
    /// Red stop lamp: stop the vehicle as soon as it is safe.
    pub red_stop: Lamp,
    /// Amber warning lamp: something to look at soon, driving on is possible.
    pub amber_warning: Lamp,
    /// Protect lamp: a problem that is not an engine one (often a
    /// system protecting itself).
    pub protect: Lamp,
}

impl Lamps {
    /// From the lamp status byte and the flash byte: two bits each, MIL in
    /// the top bits, then red stop, amber warning and protect.
    pub(crate) fn from_bytes(status: u8, flash: u8) -> Lamps {
        let lamp = |shift: u8| Lamp {
            state: LampState::from_bits(status >> shift),
            flash: LampFlash::from_bits(flash >> shift),
        };
        Lamps {
            mil: lamp(6),
            red_stop: lamp(4),
            amber_warning: lamp(2),
            protect: lamp(0),
        }
    }

    pub fn to_bytes(self) -> [u8; 2] {
        let status = (self.mil.state.bits() << 6)
            | (self.red_stop.state.bits() << 4)
            | (self.amber_warning.state.bits() << 2)
            | self.protect.state.bits();
        let flash = (self.mil.flash.bits() << 6)
            | (self.red_stop.flash.bits() << 4)
            | (self.amber_warning.flash.bits() << 2)
            | self.protect.flash.bits();
        [status, flash]
    }

    pub fn any_on(&self) -> bool {
        self.mil.on() || self.red_stop.on() || self.amber_warning.on() || self.protect.on()
    }
}

/// How the SPN of a fault is packed, see the module text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SpnConversion {
    V1,
    V2,
    V3,
    /// Today's layout, always meant when the CM bit is 0.
    #[default]
    V4,
}

impl SpnConversion {
    pub const ALL: [SpnConversion; 4] = [
        SpnConversion::V1,
        SpnConversion::V2,
        SpnConversion::V3,
        SpnConversion::V4,
    ];

    /// The SPN out of the first three bytes of a fault in this layout.
    pub fn spn(self, bytes: [u8; 3]) -> u32 {
        let [a, b, c] = bytes.map(u32::from);
        match self {
            SpnConversion::V1 => (a << 11) | (b << 3) | (c >> 5),
            SpnConversion::V2 => (b << 11) | (a << 3) | (c >> 5),
            SpnConversion::V3 | SpnConversion::V4 => a | (b << 8) | ((c >> 5) << 16),
        }
    }

    /// The three bytes an SPN takes in this layout (the FMI goes into the
    /// low five bits of the third byte afterwards).
    pub fn pack(self, spn: u32) -> [u8; 3] {
        let spn = spn & 0x7FFFF;
        let low = ((spn & 0x7) << 5) as u8;
        match self {
            SpnConversion::V1 => [(spn >> 11) as u8, (spn >> 3) as u8, low],
            SpnConversion::V2 => [(spn >> 3) as u8, (spn >> 11) as u8, low],
            SpnConversion::V3 | SpnConversion::V4 => {
                [spn as u8, (spn >> 8) as u8, ((spn >> 16) << 5) as u8]
            }
        }
    }

    /// The CM bit that goes with the layout.
    pub(crate) fn cm_bit(self) -> bool {
        self != SpnConversion::V4
    }
}

/// One fault of a DM1 or DM2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct J1939Dtc {
    pub spn: u32,
    /// 0 to 31, see [`super::fmi_text`].
    pub fmi: u8,
    /// How often the fault went from inactive to active; `None` when the
    /// unit does not count (127).
    pub occurrences: Option<u8>,
    /// The layout the SPN was read in.
    pub conversion: SpnConversion,
}

impl J1939Dtc {
    /// Reads one fault. `older` is the layout for CM = 1; `None` guesses.
    pub fn decode(bytes: [u8; 4], older: Option<SpnConversion>) -> J1939Dtc {
        let spn_bytes = [bytes[0], bytes[1], bytes[2]];
        let conversion = if bytes[3] & 0x80 == 0 {
            SpnConversion::V4
        } else {
            older.unwrap_or_else(|| guess_older(spn_bytes))
        };
        let count = bytes[3] & 0x7F;
        J1939Dtc {
            spn: conversion.spn(spn_bytes),
            fmi: bytes[2] & 0x1F,
            occurrences: (count != 0x7F).then_some(count),
            conversion,
        }
    }

    /// The four bytes of this fault in its layout.
    pub fn encode(&self) -> [u8; 4] {
        let [a, b, c] = self.conversion.pack(self.spn);
        let cm = if self.conversion.cm_bit() { 0x80 } else { 0 };
        [
            a,
            b,
            c | (self.fmi & 0x1F),
            cm | self.occurrences.unwrap_or(0x7F).min(0x7F),
        ]
    }

    /// "SPN 110 FMI 0", the code as the fault list shows it.
    pub fn code(&self) -> String {
        format!("SPN {} FMI {}", self.spn, self.fmi)
    }
}

/// CM = 1 without a fixed layout: the one that gives an SPN of the value
/// table, version 3 when none or more than one does.
fn guess_older(bytes: [u8; 3]) -> SpnConversion {
    let known: Vec<SpnConversion> = [SpnConversion::V1, SpnConversion::V2, SpnConversion::V3]
        .into_iter()
        .filter(|c| find_param(c.spn(bytes)).is_some())
        .collect();
    match known.as_slice() {
        [only] => *only,
        _ => SpnConversion::V3,
    }
}

/// What a fault means: the name of the SPN (or "SPN n" when the table
/// does not know it) and what the FMI says.
pub fn describe(spn: u32, fmi: u8) -> String {
    let name = find_param(spn).map_or_else(|| format!("SPN {spn}"), |p| p.name().to_string());
    format!("{name}: {}", fmi_text(fmi))
}

/// "SPN 110 FMI 0" (as [`J1939Dtc::code`] writes it) back into SPN and FMI.
pub fn parse_code(code: &str) -> Option<(u32, u8)> {
    let mut parts = code.split_whitespace();
    if !parts.next()?.eq_ignore_ascii_case("SPN") {
        return None;
    }
    let spn: u32 = parts.next()?.parse().ok()?;
    if !parts.next()?.eq_ignore_ascii_case("FMI") {
        return None;
    }
    let fmi: u8 = parts.next()?.parse().ok()?;
    (parts.next().is_none() && spn <= 0x7FFFF && fmi <= 31).then_some((spn, fmi))
}

/// A DM1 or DM2: the lamps and the faults.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiagnosticMessage {
    pub lamps: Lamps,
    pub dtcs: Vec<J1939Dtc>,
}

/// Reads a DM1 or DM2 (also DM6, DM12, DM23, which share the layout).
/// The entry "SPN 0, FMI 0" means "no fault" and is left out, as are the
/// padding bytes (FF) at the end.
pub(crate) fn decode_dm(data: &[u8], older: Option<SpnConversion>) -> DiagnosticMessage {
    if data.len() < 2 {
        return DiagnosticMessage::default();
    }
    let lamps = Lamps::from_bytes(data[0], data[1]);
    let mut dtcs = Vec::new();
    for &bytes in data[2..].as_chunks::<4>().0 {
        if bytes == [0; 4] || bytes[..3] == [0xFF; 3] {
            continue;
        }
        let dtc = J1939Dtc::decode(bytes, older);
        if dtc.spn == 0 || dtc.spn == 0x7FFFF {
            continue;
        }
        dtcs.push(dtc);
    }
    DiagnosticMessage { lamps, dtcs }
}

/// A DM1 or DM2 as it goes on the bus: at least eight bytes, a message
/// without faults carries the "no fault" entry.
#[cfg(test)]
pub(crate) fn encode_dm(lamps: Lamps, dtcs: &[J1939Dtc]) -> Vec<u8> {
    let mut out = lamps.to_bytes().to_vec();
    if dtcs.is_empty() {
        out.extend_from_slice(&[0, 0, 0, 0]);
    }
    for dtc in dtcs {
        out.extend_from_slice(&dtc.encode());
    }
    out.resize(out.len().max(8), 0xFF);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lamps_from_the_two_bytes() {
        // MIL on, amber on, the others off; no flashing.
        let lamps = Lamps::from_bytes(0x44, 0xFF);
        assert!(lamps.mil.on());
        assert!(!lamps.red_stop.on());
        assert!(lamps.amber_warning.on());
        assert!(!lamps.protect.on());
        assert_eq!(lamps.mil.flash, LampFlash::Steady);
        assert_eq!(lamps.to_bytes(), [0x44, 0xFF]);
        let red = Lamps::from_bytes(0x10, 0xDF);
        assert!(red.red_stop.on());
        assert_eq!(red.red_stop.flash, LampFlash::Fast);
        assert_eq!(
            Lamps::from_bytes(0xFF, 0xFF).mil.state,
            LampState::NotAvailable
        );
    }

    #[test]
    fn spn_in_all_four_layouts() {
        for spn in [84u32, 110, 190, 3251, 524_286, 0x5_5555] {
            for conversion in SpnConversion::ALL {
                let dtc = J1939Dtc {
                    spn,
                    fmi: 3,
                    occurrences: Some(7),
                    conversion,
                };
                let bytes = dtc.encode();
                let back = J1939Dtc::decode(bytes, Some(conversion));
                assert_eq!(back.spn, spn, "{conversion:?}");
                assert_eq!(back.fmi, 3);
                assert_eq!(back.occurrences, Some(7));
                // CM = 0 is always version 4, whatever the caller prefers.
                if conversion == SpnConversion::V4 {
                    assert_eq!(J1939Dtc::decode(bytes, Some(SpnConversion::V1)).spn, spn);
                }
            }
        }
    }

    #[test]
    fn known_example_bytes() {
        // SPN 110 (coolant temperature), FMI 0, seen 3 times, version 4.
        let dtc = J1939Dtc::decode([0x6E, 0x00, 0x00, 0x03], None);
        assert_eq!((dtc.spn, dtc.fmi, dtc.occurrences), (110, 0, Some(3)));
        assert_eq!(dtc.code(), "SPN 110 FMI 0");
        // SPN 3251 with the top bits in the third byte: 3251 = 0x0CB3.
        let dtc = J1939Dtc::decode([0xB3, 0x0C, 0x02, 0x7F], None);
        assert_eq!((dtc.spn, dtc.fmi, dtc.occurrences), (3251, 2, None));
        // Version 1 of SPN 100: the 19 bits most significant first.
        let v1 = J1939Dtc::decode([0x00, 0x0C, 0x81, 0x81], Some(SpnConversion::V1));
        assert_eq!((v1.spn, v1.fmi, v1.occurrences), (100, 1, Some(1)));
    }

    #[test]
    fn older_layout_is_guessed_by_the_value_table() {
        // SPN 190 in version 1 reads as nonsense in version 3, so version 1
        // wins without being asked.
        let packed = SpnConversion::V1.pack(190);
        let bytes = [packed[0], packed[1], packed[2] | 2, 0x80 | 5];
        let dtc = J1939Dtc::decode(bytes, None);
        assert_eq!(dtc.conversion, SpnConversion::V1);
        assert_eq!(dtc.spn, 190);
    }

    #[test]
    fn codes_and_their_meaning() {
        assert_eq!(parse_code("SPN 110 FMI 0"), Some((110, 0)));
        assert_eq!(parse_code("spn 3251 fmi 31"), Some((3251, 31)));
        assert_eq!(parse_code("P0171"), None);
        assert_eq!(parse_code("SPN 110 FMI 32"), None);
        assert!(describe(110, 0).contains(':'));
        assert!(describe(999_999 & 0x7FFFF, 2).starts_with("SPN "));
    }

    #[test]
    fn dm_with_and_without_faults() {
        let none = decode_dm(&[0x00, 0xFF, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF], None);
        assert!(none.dtcs.is_empty());
        assert!(!none.lamps.any_on());
        let lamps = Lamps::from_bytes(0x44, 0xFF);
        let faults = [
            J1939Dtc {
                spn: 105,
                fmi: 3,
                occurrences: Some(7),
                conversion: SpnConversion::V4,
            },
            J1939Dtc {
                spn: 111,
                fmi: 17,
                occurrences: Some(2),
                conversion: SpnConversion::V4,
            },
        ];
        let bytes = encode_dm(lamps, &faults);
        assert_eq!(bytes.len(), 10);
        let back = decode_dm(&bytes, None);
        assert_eq!(back.lamps, lamps);
        assert_eq!(back.dtcs, faults.to_vec());
        assert_eq!(encode_dm(Lamps::default(), &[]).len(), 8);
    }
}
