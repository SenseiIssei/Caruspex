//! SAE J1939: the CAN protocol of trucks, buses, tractors and machines.
//!
//! Everything runs on 29-bit ids at 250 kbit/s (500 kbit/s on newer
//! vehicles with the green type 2 socket). Most of the traffic is
//! broadcast: every control unit sends its values over and over, and its
//! active faults once a second (DM1). Caruspex therefore listens first and
//! asks only for what is not broadcast (previously active faults, VIN,
//! component and software identification, engine hours).
//!
//! The pieces:
//! - [`J1939Id`]: priority, parameter group number (PGN), destination and
//!   source in one id;
//! - [`dtc`]: the diagnostic messages DM1, DM2 (faults with their lamps)
//!   and the four ways a fault number (SPN) can be packed;
//! - [`params`]: the common broadcast values with their scaling;
//! - [`fmi`]: what each failure mode (FMI) means;
//! - [`tp`]: the transport protocol for messages longer than eight bytes
//!   (BAM to everybody, RTS/CTS to one receiver);
//! - [`J1939Session`]: listening, requests, the address claim of the
//!   tester, reading and clearing faults over any
//!   [`caruspex_link::FrameLink`] (an ELM327 in protocol A, a raw CAN
//!   interface, the demo truck).

pub mod dtc;
pub mod fmi;
pub mod params;
mod session;
pub mod tp;

use caruspex_i18n::l;

pub use dtc::{DiagnosticMessage, J1939Dtc, LampState, Lamps, SpnConversion, describe, parse_code};
pub use fmi::fmi_weight;
pub use params::{J1939Param, J1939Value, find_param};
pub use session::{AckKind, ClearResult, ComponentId, J1939Session, J1939Unit, UnitFaults};

/// PGN 59904: request for another PGN.
pub(crate) const PGN_REQUEST: u32 = 0xEA00;
/// PGN 59392: acknowledgement (positive, negative, access denied, busy).
pub(crate) const PGN_ACK: u32 = 0xE800;
/// PGN 60416: transport protocol, connection management.
pub(crate) const PGN_TP_CM: u32 = 0xEC00;
/// PGN 60160: transport protocol, data transfer.
pub(crate) const PGN_TP_DT: u32 = 0xEB00;
/// PGN 60928: address claimed.
pub(crate) const PGN_ADDRESS_CLAIM: u32 = 0xEE00;
/// PGN 65226: DM1, active faults.
pub(crate) const PGN_DM1: u32 = 0xFECA;
/// PGN 65227: DM2, previously active faults.
pub(crate) const PGN_DM2: u32 = 0xFECB;
/// PGN 65228: DM3, clear previously active faults.
pub(crate) const PGN_DM3: u32 = 0xFECC;
/// PGN 65235: DM11, clear active faults.
pub(crate) const PGN_DM11: u32 = 0xFED3;
/// PGN 65260: vehicle identification number.
pub(crate) const PGN_VIN: u32 = 0xFEEC;
/// PGN 65259: component identification (make, model, serial, unit).
pub(crate) const PGN_COMPONENT_ID: u32 = 0xFEEB;
/// PGN 65242: software identification.
pub(crate) const PGN_SOFTWARE_ID: u32 = 0xFEDA;
/// PGN 65253: engine hours and revolutions, usually only on request.
pub(crate) const PGN_ENGINE_HOURS: u32 = 0xFEE5;
/// PGN 65257: fuel consumption (trip and total), usually only on request.
pub(crate) const PGN_FUEL_CONSUMPTION: u32 = 0xFEE9;
/// PGN 65244: idle operation (total idle hours and idle fuel), on request.
pub(crate) const PGN_IDLE_OPERATION: u32 = 0xFEDC;
/// PGN 65248: vehicle distance (trip and total, 125 m steps).
pub(crate) const PGN_VEHICLE_DISTANCE: u32 = 0xFEE0;
/// PGN 65217: high resolution vehicle distance (5 m steps).
pub(crate) const PGN_VEHICLE_DISTANCE_HR: u32 = 0xFEC1;
/// PGN 49408: DM21, distance and time with the emission lamp on and since
/// the faults were last cleared. PDU1, so it is asked of every unit.
pub(crate) const PGN_DM21: u32 = 0xC100;

/// The global address: to everybody.
pub(crate) const GLOBAL: u8 = 0xFF;
/// The null address: a node without a claimed address.
pub(crate) const NULL_ADDRESS: u8 = 0xFE;
/// The preferred addresses of an off-board diagnostic tool.
pub(crate) const TESTER_ADDRESSES: [u8; 2] = [0xF9, 0xFA];

/// One 29-bit id, taken apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct J1939Id {
    /// 0 (highest) to 7.
    pub priority: u8,
    /// The parameter group number, with the destination taken out for the
    /// PDU1 groups (PF below 240).
    pub pgn: u32,
    /// The receiver of a PDU1 group; [`GLOBAL`] for the broadcast PDU2 groups.
    pub destination: u8,
    pub source: u8,
}

impl J1939Id {
    pub fn parse(id: u32) -> J1939Id {
        let pf = (id >> 16) & 0xFF;
        let ps = ((id >> 8) & 0xFF) as u8;
        let page = (id >> 24) & 0x3;
        let (pgn, destination) = if pf < 240 {
            ((page << 16) | (pf << 8), ps)
        } else {
            ((page << 16) | (pf << 8) | u32::from(ps), GLOBAL)
        };
        J1939Id {
            priority: ((id >> 26) & 0x7) as u8,
            pgn,
            destination,
            source: (id & 0xFF) as u8,
        }
    }

    /// The 29-bit id. For a PDU2 group the destination is ignored.
    pub(crate) fn to_id(self) -> u32 {
        let pf = (self.pgn >> 8) & 0xFF;
        let mut id = (u32::from(self.priority & 0x7) << 26) | ((self.pgn & 0x3FF00) << 8);
        if pf < 240 {
            id |= u32::from(self.destination) << 8;
        } else {
            id |= (self.pgn & 0xFF) << 8;
        }
        id | u32::from(self.source)
    }

    /// Builds an id from its parts.
    pub fn raw(priority: u8, pgn: u32, destination: u8, source: u8) -> u32 {
        J1939Id {
            priority,
            pgn,
            destination,
            source,
        }
        .to_id()
    }
}

/// True for a PGN that carries its destination (PDU1, PF below 240).
#[cfg(test)]
pub(crate) fn is_pdu1(pgn: u32) -> bool {
    ((pgn >> 8) & 0xFF) < 240
}

/// What a source address usually stands for: the preferred addresses of
/// the common control units. A maker may put a unit elsewhere, so this is
/// a hint, not a promise.
pub(crate) fn address_name(address: u8) -> &'static str {
    match address {
        0x00 => l("Motor", "Engine"),
        0x01 => l("Motor 2", "Engine 2"),
        0x03 => l("Getriebe", "Transmission"),
        0x04 => l("Getriebe 2", "Transmission 2"),
        0x0B => l("Bremsen (ABS/EBS)", "Brakes (ABS/EBS)"),
        0x0F => l("Motorbremse", "Engine retarder"),
        0x10 => l("Retarder", "Driveline retarder"),
        0x11 => l("Tempomat", "Cruise control"),
        0x17 => l("Kombiinstrument", "Instrument cluster"),
        0x19 => l("Klimaanlage", "Climate control"),
        0x21 => l("Karosserie-Steuergerät", "Body controller"),
        0x31 => l("Fahrerhaus-Steuergerät", "Cab controller"),
        0x3D => l("Abgasnachbehandlung", "Exhaust aftertreatment"),
        0xEE => l("Tachograph", "Tachograph"),
        0xF9 | 0xFA => l("Diagnosegerät", "Diagnostic tool"),
        _ => l("Steuergerät", "Control unit"),
    }
}

/// "Motor (00)": the name with the address in hex.
pub fn address_label(address: u8) -> String {
    format!("{} ({address:02X})", address_name(address))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_come_apart_and_back() {
        // EEC1 from the engine: PDU2, priority 3.
        let eec1 = J1939Id::parse(0x0CF0_0400);
        assert_eq!(eec1.priority, 3);
        assert_eq!(eec1.pgn, 61444);
        assert_eq!(eec1.destination, GLOBAL);
        assert_eq!(eec1.source, 0x00);
        assert_eq!(eec1.to_id(), 0x0CF0_0400);
        // A request from the tester F9 to the engine: PDU1, the PS byte is
        // the destination and not part of the PGN.
        let request = J1939Id::parse(0x18EA_00F9);
        assert_eq!(request.pgn, PGN_REQUEST);
        assert_eq!(request.destination, 0x00);
        assert_eq!(request.source, 0xF9);
        assert_eq!(J1939Id::raw(6, PGN_REQUEST, 0x00, 0xF9), 0x18EA_00F9);
        // The data page bit belongs to the PGN.
        let paged = J1939Id::parse(0x19FE_CA00);
        assert_eq!(paged.pgn, 0x1FECA);
        assert_eq!(paged.to_id(), 0x19FE_CA00);
        assert!(is_pdu1(PGN_TP_CM));
        assert!(!is_pdu1(PGN_DM1));
    }
}
