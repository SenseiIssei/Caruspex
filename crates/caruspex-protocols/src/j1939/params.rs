//! The common J1939 broadcast values: where each sits in its parameter
//! group, how many bits it takes and how the raw number becomes a reading.
//!
//! Only positions, scaling and offsets that are widely published are used
//! here; the names and explanations are our own. Values are little endian.
//! A raw value in the top range of its size means "error" (FE.. for one
//! byte, FExx for two) or "not available" (FF..); such readings are kept
//! apart instead of shown as numbers.

use caruspex_i18n::Text;

use super::{PGN_DM21, PGN_ENGINE_HOURS, PGN_FUEL_CONSUMPTION, PGN_IDLE_OPERATION};

/// Live keys of J1939 values start here (key = base + SPN), far above the
/// OBD PIDs and their sub values.
pub const J1939_KEY_BASE: i32 = 0x0100_0000;

const EEC1: u32 = 61444;
const EEC2: u32 = 61443;
const ETC2: u32 = 61445;
const EBC1: u32 = 61441;
const ET1: u32 = 65262;
const EFL_P1: u32 = 65263;
const DASH: u32 = 65276;
const CCVS1: u32 = 65265;
const VEP1: u32 = 65271;
const VEHICLE_DISTANCE: u32 = 65248;
const VEHICLE_DISTANCE_HR: u32 = 65217;
const FUEL_ECONOMY: u32 = 65266;
const AMBIENT: u32 = 65269;
const INLET_EXHAUST: u32 = 65270;
const TRANSMISSION_FLUIDS: u32 = 65272;
const AIR_SUPPLY: u32 = 65198;
const DEF_TANK: u32 = 65110;
const TACHOGRAPH: u32 = 65132;
const WHEEL_SPEEDS: u32 = 65215;
/// AT1S, aftertreatment 1 service: soot and ash load of the particulate filter.
const DPF_SERVICE: u32 = 64891;
/// DPFC1, particulate filter control: lamps and regeneration state.
const DPF_CONTROL: u32 = 64892;

/// One value of the table.
#[derive(Clone, Copy, Debug)]
pub struct J1939Param {
    pub spn: u32,
    /// The parameter group it is broadcast in.
    pub pgn: u32,
    /// First byte, counted from 0.
    pub byte: u8,
    /// First bit inside that byte, 0 is the lowest.
    pub bit: u8,
    /// Length in bits.
    pub bits: u8,
    /// Reading = raw * scale + offset.
    pub scale: f64,
    pub offset: f64,
    pub unit: &'static str,
    /// Sensible range for gauges and charts.
    pub min: f64,
    pub max: f64,
    /// A state with numbered values (0, 1, 2, ...) instead of a measurement:
    /// only "all bits set" means "not available", the next to last step is
    /// a real state (SPN 3700 uses 2 for "regeneration needed").
    pub states: bool,
    name: Text,
    about: Text,
}

impl J1939Param {
    pub fn name(&self) -> &'static str {
        self.name.get()
    }
    /// One sentence on what the value says.
    pub fn about(&self) -> &'static str {
        self.about.get()
    }
    /// The live key of the value (see [`J1939_KEY_BASE`]).
    pub fn key(&self) -> i32 {
        J1939_KEY_BASE + self.spn as i32
    }

    /// The raw number out of a frame of its group; `None` if the frame is
    /// too short.
    pub fn raw(&self, data: &[u8]) -> Option<u64> {
        let first = usize::from(self.byte);
        let end_bit = usize::from(self.bit) + usize::from(self.bits);
        let count = end_bit.div_ceil(8);
        if data.len() < first + count {
            return None;
        }
        let mut word: u64 = 0;
        for (i, &b) in data[first..first + count].iter().enumerate() {
            word |= u64::from(b) << (8 * i);
        }
        let mask = if self.bits >= 64 {
            u64::MAX
        } else {
            (1u64 << self.bits) - 1
        };
        Some((word >> self.bit) & mask)
    }

    /// The reading out of a frame of its group.
    pub fn decode(&self, data: &[u8]) -> Reading {
        let Some(raw) = self.raw(data) else {
            return Reading::NotAvailable;
        };
        if self.states {
            let top = if self.bits >= 64 {
                u64::MAX
            } else {
                (1u64 << self.bits) - 1
            };
            return if raw == top {
                Reading::NotAvailable
            } else {
                Reading::Value(raw as f64)
            };
        }
        match raw_kind(raw, self.bits) {
            RawKind::Value => Reading::Value(raw as f64 * self.scale + self.offset),
            RawKind::Error => Reading::Error,
            RawKind::NotAvailable => Reading::NotAvailable,
        }
    }

    /// The raw number for a reading, for building frames (demo truck, tests).
    pub fn encode(&self, value: f64, data: &mut [u8]) {
        let raw = ((value - self.offset) / self.scale).round().max(0.0) as u64;
        self.put_raw(raw, data);
    }

    /// Puts a raw number into its place (also the error and n/a codes).
    pub(crate) fn put_raw(&self, raw: u64, data: &mut [u8]) {
        let first = usize::from(self.byte);
        let count = (usize::from(self.bit) + usize::from(self.bits)).div_ceil(8);
        if data.len() < first + count {
            return;
        }
        let mask = ((1u64 << self.bits) - 1) << self.bit;
        let mut word: u64 = 0;
        for (i, &b) in data[first..first + count].iter().enumerate() {
            word |= u64::from(b) << (8 * i);
        }
        word = (word & !mask) | ((raw << self.bit) & mask);
        for (i, b) in data[first..first + count].iter_mut().enumerate() {
            *b = (word >> (8 * i)) as u8;
        }
    }
}

/// What a raw number stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawKind {
    Value,
    Error,
    NotAvailable,
}

/// The ranges of J1939: of every size the top part is reserved, the next
/// to last step means "error", the last "not available". For one byte: up
/// to FA a value, FE error, FF not available (FB to FD are kept for
/// later and counted as not available).
pub(crate) fn raw_kind(raw: u64, bits: u8) -> RawKind {
    if bits < 8 {
        // Two bit states: 2 error, 3 not available; longer ones alike.
        let top = (1u64 << bits) - 1;
        return if raw == top {
            RawKind::NotAvailable
        } else if raw == top - 1 && bits >= 2 {
            RawKind::Error
        } else {
            RawKind::Value
        };
    }
    let shift = u32::from(bits) - 8;
    let high = raw >> shift;
    match high {
        0..=0xFA => RawKind::Value,
        0xFE => RawKind::Error,
        _ => RawKind::NotAvailable,
    }
}

/// A decoded value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Reading {
    Value(f64),
    /// The unit says its own measurement is faulty.
    Error,
    /// The unit has no such value.
    NotAvailable,
}

#[allow(clippy::too_many_arguments)]
const fn p(
    spn: u32,
    pgn: u32,
    byte: u8,
    bits: u8,
    scale: f64,
    offset: f64,
    unit: &'static str,
    min: f64,
    max: f64,
    name: Text,
    about: Text,
) -> J1939Param {
    J1939Param {
        spn,
        pgn,
        byte,
        bit: 0,
        bits,
        scale,
        offset,
        unit,
        min,
        max,
        states: false,
        name,
        about,
    }
}

/// A state of a few bits at `bit` inside `byte`, values 0 to `max`.
#[allow(clippy::too_many_arguments)]
const fn state(
    spn: u32,
    pgn: u32,
    byte: u8,
    bit: u8,
    bits: u8,
    max: f64,
    name: Text,
    about: Text,
) -> J1939Param {
    J1939Param {
        spn,
        pgn,
        byte,
        bit,
        bits,
        scale: 1.0,
        offset: 0.0,
        unit: "",
        min: 0.0,
        max,
        states: true,
        name,
        about,
    }
}

#[rustfmt::skip]
static PARAMS: &[J1939Param] = &[
    // Engine controller (EEC1, EEC2)
    p(190, EEC1, 3, 16, 0.125, 0.0, "1/min", 0.0, 3000.0,
      Text::new("Motordrehzahl", "Engine speed"),
      Text::new("Umdrehungen der Kurbelwelle pro Minute.", "Turns of the crankshaft per minute.")),
    p(513, EEC1, 2, 8, 1.0, -125.0, "%", -125.0, 125.0,
      Text::new("Motordrehmoment (Ist)", "Engine torque (actual)"),
      Text::new("Was der Motor gerade abgibt, als Anteil seines Bezugsmoments.", "What the engine delivers now, as a share of its reference torque.")),
    p(512, EEC1, 1, 8, 1.0, -125.0, "%", -125.0, 125.0,
      Text::new("Drehmomentwunsch des Fahrers", "Driver's torque demand"),
      Text::new("Das Drehmoment, das Fahrpedal und Tempomat gerade verlangen.", "The torque the accelerator pedal and cruise control ask for right now.")),
    p(91, EEC2, 1, 8, 0.4, 0.0, "%", 0.0, 100.0,
      Text::new("Fahrpedalstellung", "Accelerator pedal position"),
      Text::new("Wie weit das Gaspedal getreten ist.", "How far the accelerator pedal is pressed.")),
    p(92, EEC2, 2, 8, 1.0, 0.0, "%", 0.0, 125.0,
      Text::new("Motorlast", "Engine load"),
      Text::new("Last bei der aktuellen Drehzahl, gemessen am Höchstmöglichen.", "Load at the current speed, measured against what is possible.")),
    // Temperatures (ET1)
    p(110, ET1, 0, 8, 1.0, -40.0, "°C", -40.0, 130.0,
      Text::new("Kühlmitteltemperatur", "Coolant temperature"),
      Text::new("Warm ist der Motor meist zwischen 80 und 95 °C.", "A warm engine usually sits between 80 and 95 °C.")),
    p(174, ET1, 1, 8, 1.0, -40.0, "°C", -40.0, 120.0,
      Text::new("Kraftstofftemperatur", "Fuel temperature"),
      Text::new("Temperatur des Diesels im Kraftstoffsystem.", "Temperature of the diesel in the fuel system.")),
    p(175, ET1, 2, 16, 0.03125, -273.0, "°C", -40.0, 150.0,
      Text::new("Öltemperatur", "Oil temperature"),
      Text::new("Temperatur des Motoröls.", "Temperature of the engine oil.")),
    // Fluid levels and pressures (EFL/P1)
    p(100, EFL_P1, 3, 8, 4.0, 0.0, "kPa", 0.0, 800.0,
      Text::new("Öldruck", "Oil pressure"),
      Text::new("Fällt er bei laufendem Motor stark ab, sofort anhalten.", "If it drops sharply with the engine running, stop at once.")),
    p(98, EFL_P1, 2, 8, 0.4, 0.0, "%", 0.0, 100.0,
      Text::new("Ölstand", "Oil level"),
      Text::new("Füllstand der Ölwanne, wenn der Motor ihn misst.", "Fill level of the oil sump, where the engine measures it.")),
    p(94, EFL_P1, 0, 8, 4.0, 0.0, "kPa", 0.0, 1000.0,
      Text::new("Kraftstoffvordruck", "Fuel delivery pressure"),
      Text::new("Druck, mit dem die Förderpumpe den Kraftstoff liefert.", "Pressure at which the lift pump delivers the fuel.")),
    p(101, EFL_P1, 4, 16, 0.0078125, -250.0, "kPa", -10.0, 10.0,
      Text::new("Kurbelgehäusedruck", "Crankcase pressure"),
      Text::new("Steigt er, kann Abgas an den Kolbenringen vorbeiströmen.", "If it rises, exhaust may be blowing past the piston rings.")),
    p(109, EFL_P1, 6, 8, 2.0, 0.0, "kPa", 0.0, 300.0,
      Text::new("Kühlmitteldruck", "Coolant pressure"),
      Text::new("Druck im Kühlkreislauf.", "Pressure in the cooling circuit.")),
    p(111, EFL_P1, 7, 8, 0.4, 0.0, "%", 0.0, 100.0,
      Text::new("Kühlmittelstand", "Coolant level"),
      Text::new("Füllstand im Ausgleichsbehälter.", "Fill level in the expansion tank.")),
    // Dash display
    p(96, DASH, 1, 8, 0.4, 0.0, "%", 0.0, 100.0,
      Text::new("Tankinhalt", "Fuel level"),
      Text::new("Füllstand des Kraftstofftanks.", "Fill level of the fuel tank.")),
    // Speed (CCVS1, tachograph, wheel speeds)
    p(84, CCVS1, 1, 16, 1.0 / 256.0, 0.0, "km/h", 0.0, 130.0,
      Text::new("Geschwindigkeit", "Vehicle speed"),
      Text::new("Aus den Raddrehzahlen berechnet.", "Worked out from the wheel speeds.")),
    p(1624, TACHOGRAPH, 6, 16, 1.0 / 256.0, 0.0, "km/h", 0.0, 130.0,
      Text::new("Geschwindigkeit laut Tachograph", "Speed according to the tachograph"),
      Text::new("Die Geschwindigkeit, die der Fahrtenschreiber aufzeichnet.", "The speed the tachograph records.")),
    p(904, WHEEL_SPEEDS, 0, 16, 1.0 / 256.0, 0.0, "km/h", 0.0, 130.0,
      Text::new("Geschwindigkeit Vorderachse", "Front axle speed"),
      Text::new("Mittlere Geschwindigkeit der Vorderräder, vom Bremssteuergerät.", "Mean speed of the front wheels, from the brake control unit.")),
    // Electrics (VEP1)
    p(168, VEP1, 4, 16, 0.05, 0.0, "V", 0.0, 32.0,
      Text::new("Batteriespannung", "Battery voltage"),
      Text::new("Lkw haben meist 24 V: bei laufendem Motor um 28 V, im Stand um 25 V.", "Trucks mostly have 24 V: about 28 V with the engine running, about 25 V standing.")),
    p(167, VEP1, 2, 16, 0.05, 0.0, "V", 0.0, 32.0,
      Text::new("Ladespannung", "Charging voltage"),
      Text::new("Spannung, die die Lichtmaschine liefert.", "Voltage the alternator delivers.")),
    p(158, VEP1, 6, 16, 0.05, 0.0, "V", 0.0, 32.0,
      Text::new("Spannung am Zündschloss", "Voltage at the ignition switch"),
      Text::new("Spannung hinter dem Zündschalter.", "Voltage behind the ignition switch.")),
    p(114, VEP1, 0, 8, 1.0, -125.0, "A", -125.0, 125.0,
      Text::new("Batteriestrom", "Battery current"),
      Text::new("Positiv lädt die Batterie, negativ wird sie entladen.", "Positive charges the battery, negative drains it.")),
    p(115, VEP1, 1, 8, 1.0, 0.0, "A", 0.0, 250.0,
      Text::new("Lichtmaschinenstrom", "Alternator current"),
      Text::new("Strom, den die Lichtmaschine abgibt.", "Current the alternator puts out.")),
    // Counters (on request on most trucks)
    p(247, PGN_ENGINE_HOURS, 0, 32, 0.05, 0.0, "h", 0.0, 100_000.0,
      Text::new("Motorbetriebsstunden", "Engine hours"),
      Text::new("Gesamte Laufzeit des Motors, wichtig für Wartung und Gebrauchtkauf.", "Total running time of the engine, useful for service and when buying used.")),
    p(249, PGN_ENGINE_HOURS, 4, 32, 1000.0, 0.0, "", 0.0, 4.0e9,
      Text::new("Motorumdrehungen gesamt", "Total engine revolutions"),
      Text::new("Alle Kurbelwellenumdrehungen seit dem Bau.", "Every crankshaft revolution since the engine was built.")),
    p(245, VEHICLE_DISTANCE, 4, 32, 0.125, 0.0, "km", 0.0, 3.0e6,
      Text::new("Gesamtstrecke", "Total distance"),
      Text::new("Kilometerstand.", "Odometer reading.")),
    p(244, VEHICLE_DISTANCE, 0, 32, 0.125, 0.0, "km", 0.0, 100_000.0,
      Text::new("Tagesstrecke", "Trip distance"),
      Text::new("Strecke seit dem letzten Zurücksetzen.", "Distance since the last reset.")),
    p(917, VEHICLE_DISTANCE_HR, 0, 32, 0.005, 0.0, "km", 0.0, 3.0e6,
      Text::new("Gesamtstrecke (fein)", "Total distance (fine)"),
      Text::new("Kilometerstand in Schritten von 5 m.", "Odometer reading in steps of 5 m.")),
    p(918, VEHICLE_DISTANCE_HR, 4, 32, 0.005, 0.0, "km", 0.0, 100_000.0,
      Text::new("Tagesstrecke (fein)", "Trip distance (fine)"),
      Text::new("Tagesstrecke in Schritten von 5 m.", "Trip distance in steps of 5 m.")),
    p(250, PGN_FUEL_CONSUMPTION, 4, 32, 0.5, 0.0, "l", 0.0, 2.0e6,
      Text::new("Kraftstoff gesamt", "Total fuel used"),
      Text::new("Aller Kraftstoff, den der Motor seit dem Bau verbraucht hat.", "All the fuel the engine has burned since it was built.")),
    p(182, PGN_FUEL_CONSUMPTION, 0, 32, 0.5, 0.0, "l", 0.0, 100_000.0,
      Text::new("Kraftstoff Fahrt", "Trip fuel"),
      Text::new("Verbrauch seit dem letzten Zurücksetzen.", "Fuel used since the last reset.")),
    // Fuel economy (LFE1)
    p(183, FUEL_ECONOMY, 0, 16, 0.05, 0.0, "l/h", 0.0, 100.0,
      Text::new("Kraftstoffverbrauch", "Fuel rate"),
      Text::new("Was der Motor in diesem Moment verbraucht.", "What the engine burns at this moment.")),
    p(184, FUEL_ECONOMY, 2, 16, 1.0 / 512.0, 0.0, "km/l", 0.0, 10.0,
      Text::new("Momentane Reichweite je Liter", "Instant distance per litre"),
      Text::new("Kilometer je Liter beim aktuellen Verbrauch.", "Kilometres per litre at the current fuel rate.")),
    p(51, FUEL_ECONOMY, 6, 8, 0.4, 0.0, "%", 0.0, 100.0,
      Text::new("Drosselklappe", "Throttle valve"),
      Text::new("Öffnung der Drosselklappe, bei Dieseln oft fast immer offen.", "Opening of the throttle valve, nearly always open on many diesels.")),
    // Ambient (AMB)
    p(171, AMBIENT, 3, 16, 0.03125, -273.0, "°C", -40.0, 60.0,
      Text::new("Außentemperatur", "Ambient temperature"),
      Text::new("Lufttemperatur draußen.", "Air temperature outside.")),
    p(108, AMBIENT, 0, 8, 0.5, 0.0, "kPa", 50.0, 110.0,
      Text::new("Luftdruck", "Barometric pressure"),
      Text::new("Umgebungsluftdruck, sinkt in den Bergen.", "Air pressure around the vehicle, lower in the mountains.")),
    p(172, AMBIENT, 5, 8, 1.0, -40.0, "°C", -40.0, 80.0,
      Text::new("Ansauglufttemperatur", "Intake air temperature"),
      Text::new("Temperatur der Luft am Ansaugstutzen.", "Temperature of the air at the air inlet.")),
    // Intake and exhaust (IC1)
    p(102, INLET_EXHAUST, 1, 8, 2.0, 0.0, "kPa", 0.0, 400.0,
      Text::new("Ladedruck", "Boost pressure"),
      Text::new("Überdruck im Ansaugkrümmer durch den Turbolader.", "Pressure above ambient in the intake manifold from the turbocharger.")),
    p(105, INLET_EXHAUST, 2, 8, 1.0, -40.0, "°C", -40.0, 120.0,
      Text::new("Ladelufttemperatur", "Intake manifold temperature"),
      Text::new("Temperatur der Luft im Ansaugkrümmer, hinter dem Ladeluftkühler.", "Temperature of the air in the intake manifold, after the charge air cooler.")),
    p(106, INLET_EXHAUST, 3, 8, 2.0, 0.0, "kPa", 0.0, 500.0,
      Text::new("Luftdruck am Einlass", "Air inlet pressure"),
      Text::new("Druck am Lufteinlass des Motors.", "Pressure at the air inlet of the engine.")),
    p(173, INLET_EXHAUST, 5, 16, 0.03125, -273.0, "°C", 0.0, 800.0,
      Text::new("Abgastemperatur", "Exhaust gas temperature"),
      Text::new("Temperatur des Abgases.", "Temperature of the exhaust gas.")),
    // Transmission
    p(523, ETC2, 3, 8, 1.0, -125.0, "", -1.0, 18.0,
      Text::new("Eingelegter Gang", "Current gear"),
      Text::new("Negativ ist rückwärts, 0 ist Leerlauf.", "Negative is reverse, 0 is neutral.")),
    p(524, ETC2, 0, 8, 1.0, -125.0, "", -1.0, 18.0,
      Text::new("Gewählter Gang", "Selected gear"),
      Text::new("Der Gang, den Fahrer oder Getriebe als Nächstes wollen.", "The gear the driver or the gearbox wants next.")),
    p(177, TRANSMISSION_FLUIDS, 4, 16, 0.03125, -273.0, "°C", -40.0, 150.0,
      Text::new("Getriebeöltemperatur", "Transmission oil temperature"),
      Text::new("Temperatur des Getriebeöls.", "Temperature of the gearbox oil.")),
    // Brakes and air
    p(521, EBC1, 1, 8, 0.4, 0.0, "%", 0.0, 100.0,
      Text::new("Bremspedalstellung", "Brake pedal position"),
      Text::new("Wie weit das Bremspedal getreten ist.", "How far the brake pedal is pressed.")),
    p(46, AIR_SUPPLY, 0, 8, 8.0, 0.0, "kPa", 0.0, 1200.0,
      Text::new("Vorratsdruck Druckluft", "Air supply pressure"),
      Text::new("Druck im Vorratsbehälter der Druckluftanlage.", "Pressure in the reservoir of the air system.")),
    // Exhaust aftertreatment
    p(1761, DEF_TANK, 0, 8, 0.4, 0.0, "%", 0.0, 100.0,
      Text::new("AdBlue-Füllstand", "DEF (AdBlue) level"),
      Text::new("Füllstand des Harnstofftanks für die Abgasreinigung.", "Fill level of the urea tank for the exhaust cleaning.")),
    // AT1T1I byte 2: 1 °C per bit from -40 °C (J1939-71).
    p(3031, DEF_TANK, 1, 8, 1.0, -40.0, "°C", -40.0, 90.0,
      Text::new("AdBlue-Temperatur", "DEF (AdBlue) temperature"),
      Text::new("Unter etwa -11 °C friert AdBlue; der Tank wird dann beheizt.", "Below about -11 °C DEF freezes; the tank is heated then.")),
    // AT1S bytes 1 and 2: 1 % per bit; bytes 3 to 6: 1 s per bit, shown in hours.
    p(3719, DPF_SERVICE, 0, 8, 1.0, 0.0, "%", 0.0, 250.0,
      Text::new("Rußbeladung Partikelfilter", "Particulate filter soot load"),
      Text::new("Wie voll der Dieselpartikelfilter mit Ruß ist; 100 % ist die Schwelle zum Freibrennen.", "How full of soot the diesel particulate filter is; 100 % is the threshold for burning it off.")),
    p(3720, DPF_SERVICE, 1, 8, 1.0, 0.0, "%", 0.0, 250.0,
      Text::new("Aschebeladung Partikelfilter", "Particulate filter ash load"),
      Text::new("Asche brennt nicht ab; ist der Filter voll davon, muss er gereinigt oder getauscht werden.", "Ash does not burn off; once the filter is full of it, it has to be cleaned or replaced.")),
    p(3721, DPF_SERVICE, 2, 32, 1.0 / 3600.0, 0.0, "h", 0.0, 1000.0,
      Text::new("Zeit seit dem letzten Freibrennen", "Time since the last regeneration"),
      Text::new("Wie lange die letzte aktive Regeneration des Partikelfilters her ist.", "How long ago the last active regeneration of the particulate filter was.")),
    // DPFC1 byte 2: bits 1-2 are the passive regeneration (SPN 3699), bits 3-4
    // the active regeneration, bits 5-7 the filter state.
    state(3700, DPF_CONTROL, 1, 2, 2, 2.0,
      Text::new("Freibrennen des Partikelfilters", "Particulate filter regeneration"),
      Text::new("0: läuft nicht, 1: läuft gerade, 2: nötig, beginnt gleich von selbst.", "0: not running, 1: running now, 2: needed, about to start by itself.")),
    state(3701, DPF_CONTROL, 1, 4, 3, 3.0,
      Text::new("Zustand des Partikelfilters", "Particulate filter state"),
      Text::new("0: Freibrennen nicht nötig, 1 bis 3: nötig, von niedriger bis höchster Stufe.", "0: no regeneration needed, 1 to 3: needed, from the lowest to the highest level.")),
    // Idle operation (IO), on request: bytes 1-4 0.5 l per bit, bytes 5-8 0.05 h per bit.
    p(236, PGN_IDLE_OPERATION, 0, 32, 0.5, 0.0, "l", 0.0, 2.0e6,
      Text::new("Kraftstoff im Leerlauf gesamt", "Total idle fuel used"),
      Text::new("Aller Kraftstoff, den der Motor im Leerlauf verbraucht hat.", "All the fuel the engine has burned while idling.")),
    p(235, PGN_IDLE_OPERATION, 4, 32, 0.05, 0.0, "h", 0.0, 100_000.0,
      Text::new("Leerlaufstunden gesamt", "Total idle hours"),
      Text::new("Wie lange der Motor insgesamt im Leerlauf lief.", "How long the engine ran at idle in total.")),
    // DM21, on request: 1 km and 1 min per bit, two bytes each.
    p(3069, PGN_DM21, 0, 16, 1.0, 0.0, "km", 0.0, 64_255.0,
      Text::new("Strecke mit Abgaslampe", "Distance with the emission lamp on"),
      Text::new("Gefahrene Kilometer, während die Abgaslampe (MIL) leuchtete.", "Kilometres driven while the emission lamp (MIL) was lit.")),
    p(3294, PGN_DM21, 2, 16, 1.0, 0.0, "km", 0.0, 64_255.0,
      Text::new("Strecke seit dem Löschen", "Distance since faults were cleared"),
      Text::new("Gefahrene Kilometer seit dem letzten Löschen des Fehlerspeichers.", "Kilometres driven since the fault memory was last cleared.")),
    p(3295, PGN_DM21, 4, 16, 1.0, 0.0, "min", 0.0, 64_255.0,
      Text::new("Laufzeit mit Abgaslampe", "Engine time with the emission lamp on"),
      Text::new("Minuten Motorlauf, während die Abgaslampe leuchtete.", "Minutes of engine running while the emission lamp was lit.")),
    p(3296, PGN_DM21, 6, 16, 1.0, 0.0, "min", 0.0, 64_255.0,
      Text::new("Zeit seit dem Löschen", "Time since faults were cleared"),
      Text::new("Minuten Motorlauf seit dem letzten Löschen des Fehlerspeichers.", "Minutes of engine running since the fault memory was last cleared.")),
];

/// Every value of the table.
#[cfg(test)]
pub(crate) fn j1939_params() -> &'static [J1939Param] {
    PARAMS
}

/// The table entry of an SPN.
pub fn find_param(spn: u32) -> Option<&'static J1939Param> {
    PARAMS.iter().find(|p| p.spn == spn)
}

/// One decoded value with its sender.
#[derive(Clone, Debug, PartialEq)]
pub struct J1939Value {
    pub spn: u32,
    pub source: u8,
    pub name: String,
    pub unit: String,
    pub reading: Reading,
    pub min: f64,
    pub max: f64,
}

/// Every value of the table in one frame of the parameter group `pgn`.
pub(crate) fn decode_params(pgn: u32, source: u8, data: &[u8]) -> Vec<J1939Value> {
    PARAMS
        .iter()
        .filter(|p| p.pgn == pgn)
        .map(|p| J1939Value {
            spn: p.spn,
            source,
            name: p.name().to_string(),
            unit: p.unit.to_string(),
            reading: p.decode(data),
            min: p.min,
            max: p.max,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(spn: u32, data: &[u8]) -> Reading {
        find_param(spn).unwrap().decode(data)
    }

    #[test]
    fn engine_speed_and_torque_from_eec1() {
        // 0x1A40 = 6720 * 0.125 = 840 rpm; torque 0x96 - 125 = 25 %.
        let eec1 = [0xF0, 0x7D, 0x96, 0x40, 0x1A, 0x00, 0xFF, 0xFF];
        assert_eq!(value(190, &eec1), Reading::Value(840.0));
        assert_eq!(value(513, &eec1), Reading::Value(25.0));
    }

    #[test]
    fn coolant_oil_fuel_speed_battery() {
        let et1 = [0x7D, 0x46, 0x60, 0x2C, 0xFF, 0xFF, 0xFF, 0xFF];
        assert_eq!(value(110, &et1), Reading::Value(85.0));
        // 0x2C60 = 11360 * 0.03125 - 273 = 82 °C.
        assert_eq!(value(175, &et1), Reading::Value(82.0));
        let eflp = [0xFF, 0xFF, 0xFF, 0x4B, 0xFF, 0xFF, 0xFF, 0xFA];
        assert_eq!(value(100, &eflp), Reading::Value(300.0));
        assert_eq!(value(111, &eflp), Reading::Value(100.0));
        let dash = [0xFF, 0x9B, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        assert!(matches!(value(96, &dash), Reading::Value(v) if (v - 62.0).abs() < 1e-9));
        let ccvs = [0xFF, 0x00, 0x50, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        assert_eq!(value(84, &ccvs), Reading::Value(80.0));
        let vep = [0xFF, 0xFF, 0xFF, 0xFF, 0x2C, 0x02, 0xFF, 0xFF];
        assert!(matches!(value(168, &vep), Reading::Value(v) if (v - 27.8).abs() < 1e-9));
    }

    #[test]
    fn counters_need_all_four_bytes() {
        let hours = 12345.6f64;
        let mut data = [0xFF; 8];
        find_param(247).unwrap().encode(hours, &mut data);
        assert!(matches!(value(247, &data), Reading::Value(v) if (v - hours).abs() < 0.03));
        assert_eq!(value(247, &data[..3]), Reading::NotAvailable);
    }

    #[test]
    fn error_and_not_available() {
        let mut ic1 = [0xFF; 8];
        ic1[2] = 0xFE;
        assert_eq!(value(105, &ic1), Reading::Error);
        assert_eq!(value(102, &ic1), Reading::NotAvailable);
        assert_eq!(raw_kind(0xFAFF, 16), RawKind::Value);
        assert_eq!(raw_kind(0xFE12, 16), RawKind::Error);
        assert_eq!(raw_kind(0xFFFF, 16), RawKind::NotAvailable);
        assert_eq!(raw_kind(2, 2), RawKind::Error);
        assert_eq!(raw_kind(1, 2), RawKind::Value);
    }

    #[test]
    fn the_table_is_consistent() {
        let mut seen = std::collections::BTreeSet::new();
        for p in j1939_params() {
            assert!(seen.insert(p.spn), "SPN {} twice", p.spn);
            assert!(
                usize::from(p.byte) * 8 + usize::from(p.bits) <= 64,
                "SPN {}",
                p.spn
            );
            assert!(p.min < p.max, "SPN {}", p.spn);
            assert!(!p.name().is_empty() && !p.about().is_empty());
            // A value written into a frame comes back.
            let mut data = [0xFF; 8];
            let mid = (p.min.max(p.offset) + p.max) / 2.0;
            p.encode(mid, &mut data);
            match p.decode(&data) {
                Reading::Value(v) => assert!((v - mid).abs() <= p.scale, "SPN {}", p.spn),
                other => panic!("SPN {}: {other:?}", p.spn),
            }
        }
        assert!(decode_params(61444, 0, &[0xFF; 8]).len() >= 3);
    }
}
