//! The table of live values: one description per value, built once per
//! language. Only names and units are picked here; status texts are functions
//! and follow the language when they are called.

use std::sync::Arc;

use caruspex_i18n::Text;

use super::text::*;
use super::{PidAvailable, PidCalc, PidCategory, PidSpec, PidText, PidValueSpec, live_key};

// ---------------------------------------------------------------------------
// Reading raw bytes. d[0] is byte A, the first byte after the PID number.
// ---------------------------------------------------------------------------

fn raw_unsigned(d: &[i32], offset: usize, length: usize) -> u32 {
    let mut v: u32 = 0;
    for i in 0..length {
        v = (v << 8) | (d.get(offset + i).copied().unwrap_or(0) & 0xFF) as u32;
    }
    v
}

/// Two's complement for one or two bytes, as SAE J1979 uses for signed values.
fn raw_signed(d: &[i32], offset: usize, length: usize) -> i32 {
    let v = raw_unsigned(d, offset, length);
    let sign = 1u32 << (length * 8 - 1);
    if v & sign != 0 {
        (i64::from(v) - i64::from(sign << 1)) as i32
    } else {
        v as i32
    }
}

/// A value inside the response: where it sits and how it is converted.
struct Field {
    calc: PidCalc,
    /// Bytes the value needs, counted from A.
    need: usize,
}

/// value = raw * mul / div + add. Multiplying first and dividing afterwards
/// keeps round numbers exact (14000 mV / 1000 is exactly 14 V).
fn scaled_with(offset: usize, length: usize, mul: f64, div: f64, add: f64, signed: bool) -> Field {
    Field {
        need: offset + length,
        calc: Arc::new(move |d| {
            let raw = if signed {
                f64::from(raw_signed(d, offset, length))
            } else {
                f64::from(raw_unsigned(d, offset, length))
            };
            raw * mul / div + add
        }),
    }
}

fn scaled(offset: usize, length: usize, mul: f64, div: f64) -> Field {
    scaled_with(offset, length, mul, div, 0.0, false)
}

fn raw8(offset: usize) -> Field {
    scaled(offset, 1, 1.0, 1.0)
}

fn raw16(offset: usize) -> Field {
    scaled(offset, 2, 1.0, 1.0)
}

fn raw32(offset: usize) -> Field {
    scaled(offset, 4, 1.0, 1.0)
}

/// 0 to 100 %, 100/255 per bit.
fn percent(offset: usize) -> Field {
    scaled(offset, 1, 100.0, 255.0)
}

/// -100 to 99.2 %, 100/128 per bit with offset: fuel trims and EGR error.
fn trim(offset: usize) -> Field {
    scaled_with(offset, 1, 100.0, 128.0, -100.0, false)
}

/// -40 to 215 C, 1 C per bit.
fn temp8(offset: usize) -> Field {
    scaled_with(offset, 1, 1.0, 1.0, -40.0, false)
}

/// -40 to 6513.5 C, 0.1 C per bit.
fn temp16(offset: usize) -> Field {
    scaled_with(offset, 2, 1.0, 10.0, -40.0, false)
}

/// -125 to 130 %, torque values.
fn torque(offset: usize) -> Field {
    scaled_with(offset, 1, 1.0, 1.0, -125.0, false)
}

/// Equivalence ratio lambda, 0 to < 2, 2/65536 per bit.
fn lambda(offset: usize) -> Field {
    scaled(offset, 2, 2.0, 65536.0)
}

/// A single bit as 0 or 1.
fn bit_of(offset: usize, bit: u32) -> Field {
    Field {
        need: offset + 1,
        calc: Arc::new(move |d| f64::from((d.get(offset).copied().unwrap_or(0) >> bit) & 1)),
    }
}

/// Several bits as a small number.
fn bits_of(offset: usize, shift: u32, mask: i32) -> Field {
    Field {
        need: offset + 1,
        calc: Arc::new(move |d| f64::from((d.get(offset).copied().unwrap_or(0) >> shift) & mask)),
    }
}

/// Temperature byte with an optional wide range: if the given bit in byte A
/// is set, the control unit reports 4 C per bit instead of 1 C (J1979-DA,
/// used by EGR and charge air cooler temperatures). Reserved bits are zero,
/// so control units without wide range data get the normal scaling.
fn temp8_wide(offset: usize, wide_bit: u32) -> Field {
    Field {
        need: offset + 1,
        calc: Arc::new(move |d| {
            let raw = f64::from(d.get(offset).copied().unwrap_or(0) & 0xFF);
            let wide = ((d.first().copied().unwrap_or(0) >> wide_bit) & 1) != 0;
            (if wide { raw * 4.0 } else { raw }) - 40.0
        }),
    }
}

fn support_bit(bit: u32) -> PidAvailable {
    Arc::new(move |d| ((d.first().copied().unwrap_or(0) >> bit) & 1) != 0)
}

// ---------------------------------------------------------------------------
// Building the table
// ---------------------------------------------------------------------------

/// Names that are put together in loops need storage for the whole program
/// run, because the specs only hold plain references. The table is built once
/// per language, so the leak is bounded.
fn keep(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

fn keep_text(de: String, en: String) -> Text {
    Text::new(keep(de), keep(en))
}

fn tx(de: &'static str, en: &'static str) -> Text {
    Text::new(de, en)
}

/// Description of one value.
struct ValueDef {
    name: Text,
    unit: &'static str,
    field: Field,
    /// Support bit in byte A, -1 = always present.
    bit: i32,
    /// Gauge range.
    min: f64,
    max: f64,
    warn_low: f64,
    warn_high: f64,
    text: Option<PidText>,
    /// Custom presence check (instead of a bit).
    available: Option<PidAvailable>,
    /// Only if the unit differs in English.
    unit_en: Option<&'static str>,
}

fn v(name: Text, unit: &'static str, field: Field) -> ValueDef {
    ValueDef {
        name,
        unit,
        field,
        bit: -1,
        min: 0.0,
        max: 0.0,
        warn_low: 0.0,
        warn_high: 0.0,
        text: None,
        available: None,
        unit_en: None,
    }
}

impl ValueDef {
    fn range(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    fn warn(mut self, low: f64, high: f64) -> Self {
        self.warn_low = low;
        self.warn_high = high;
        self
    }

    fn bit(mut self, bit: i32) -> Self {
        self.bit = bit;
        self
    }

    fn text(mut self, text: PidText) -> Self {
        self.text = Some(text);
        self
    }

    fn available(mut self, f: impl Fn(&[i32]) -> bool + Send + Sync + 'static) -> Self {
        self.available = Some(Arc::new(f));
        self
    }

    fn unit_en(mut self, unit: &'static str) -> Self {
        self.unit_en = Some(unit);
        self
    }
}

struct TableBuilder {
    english: bool,
    table: Vec<PidSpec>,
}

impl TableBuilder {
    fn new(english: bool) -> Self {
        Self {
            english,
            table: Vec::new(),
        }
    }

    fn pick(&self, t: Text) -> &'static str {
        if self.english { t.en } else { t.de }
    }

    fn add(
        &mut self,
        pid: i32,
        bytes: i32,
        category: PidCategory,
        title: Option<Text>,
        defs: Vec<ValueDef>,
    ) {
        let mut values = Vec::with_capacity(defs.len());
        for (sub, def) in defs.into_iter().enumerate() {
            let sub = sub as i32;
            let available = match (def.available, def.bit) {
                (Some(f), _) => Some(f),
                (None, bit) if bit >= 0 => Some(support_bit(bit as u32)),
                _ => None,
            };
            let unit = match def.unit_en {
                Some(unit) if self.english => unit,
                _ => def.unit,
            };
            values.push(PidValueSpec {
                sub,
                key: live_key(pid, sub),
                name: self.pick(def.name),
                unit,
                need_bytes: def.field.need,
                calc: def.field.calc,
                text: def.text,
                available,
                display_min: def.min,
                display_max: def.max,
                warn_low: def.warn_low,
                warn_high: def.warn_high,
            });
        }
        let primary = &values[0];
        let name = primary.name;
        let spec = PidSpec {
            pid,
            name,
            unit: primary.unit,
            bytes,
            calc: primary.calc.clone(),
            text: primary.text,
            warn_low: primary.warn_low,
            warn_high: primary.warn_high,
            title: title.map_or(name, |t| self.pick(t)),
            category,
            display_min: primary.display_min,
            display_max: primary.display_max,
            values,
        };
        self.table.push(spec);
    }

    /// Shorthand for the common single-value PID.
    fn single(&mut self, pid: i32, bytes: i32, category: PidCategory, def: ValueDef) {
        self.add(pid, bytes, category, None, vec![def]);
    }
}

/// Position names of the oxygen sensors when PID 0x13 describes the layout
/// (two banks with up to four sensors each), which almost every car uses.
/// Sensor 1 sits before the catalytic converter, sensor 2 behind it.
fn o2_position(index: i32, english: bool) -> String {
    let bank = index / 4 + 1;
    let sensor = index % 4 + 1;
    let mut out = format!(
        "{}{bank}{}{sensor}",
        if english { "bank " } else { "Bank " },
        if english { " sensor " } else { " Sonde " }
    );
    if sensor == 1 {
        out.push_str(if english {
            " (before catalyst)"
        } else {
            " (vor Kat)"
        });
    }
    if sensor == 2 {
        out.push_str(if english {
            " (after catalyst)"
        } else {
            " (hinter Kat)"
        });
    }
    out
}

/// Builds the table for one language.
pub(super) fn build_table(english: bool) -> Vec<PidSpec> {
    use PidCategory as C;
    let mut t = TableBuilder::new(english);

    // --- 0x01 to 0x1F --------------------------------------------------------
    t.add(
        0x01,
        4,
        C::Diagnostics,
        Some(tx("Fehlerlampe und Fehlerzahl", "MIL and number of codes")),
        vec![
            v(
                tx("Motorkontrollleuchte", "Check engine light (MIL)"),
                "",
                bit_of(0, 7),
            )
            .range(0.0, 1.0)
            .text(on_off_text),
            // The count lives in the low seven bits, bit 7 is the lamp.
            v(
                tx(
                    "Anzahl gespeicherter Fehlercodes",
                    "Number of stored trouble codes",
                ),
                "",
                bits_of(0, 0, 0x7F),
            )
            .range(0.0, 20.0),
            v(
                tx(
                    "Motorart laut Steuergerät",
                    "Engine type reported by the ECU",
                ),
                "",
                bit_of(1, 3),
            )
            .text(engine_type_text),
        ],
    );
    t.add(
        0x03,
        2,
        C::Fuel,
        Some(tx("Gemischregelung", "Fuel system status")),
        vec![
            v(tx("Gemischregelung", "Fuel system status"), "", raw8(0)).text(fuel_status_text),
            v(
                tx("Gemischregelung, zweites System", "Fuel system 2 status"),
                "",
                raw8(1),
            )
            .text(fuel_status_text)
            .available(|d| d.len() >= 2 && d[1] != 0),
        ],
    );
    t.single(
        0x04,
        1,
        C::Engine,
        v(
            tx("Motorlast berechnet", "Calculated engine load"),
            "%",
            percent(0),
        )
        .range(0.0, 100.0),
    );
    t.single(
        0x05,
        1,
        C::Temperatures,
        v(
            tx("Kühlmitteltemperatur", "Coolant temperature"),
            "°C",
            temp8(0),
        )
        .range(-40.0, 130.0)
        .warn(-10.0, 110.0),
    );
    let trims = [
        tx(
            "Kurzzeit-Gemischkorrektur Bank 1",
            "Short-term fuel trim bank 1",
        ),
        tx(
            "Langzeit-Gemischkorrektur Bank 1",
            "Long-term fuel trim bank 1",
        ),
        tx(
            "Kurzzeit-Gemischkorrektur Bank 2",
            "Short-term fuel trim bank 2",
        ),
        tx(
            "Langzeit-Gemischkorrektur Bank 2",
            "Long-term fuel trim bank 2",
        ),
    ];
    for (i, name) in trims.into_iter().enumerate() {
        t.single(
            0x06 + i as i32,
            1,
            C::Fuel,
            v(name, "%", trim(0)).range(-30.0, 30.0).warn(-20.0, 20.0),
        );
    }
    t.single(
        0x0A,
        1,
        C::Fuel,
        v(
            tx("Kraftstoffdruck", "Fuel pressure"),
            "kPa",
            scaled(0, 1, 3.0, 1.0),
        )
        .range(0.0, 765.0),
    );
    t.single(
        0x0B,
        1,
        C::Air,
        v(
            tx("Saugrohrdruck", "Intake manifold pressure"),
            "kPa",
            raw8(0),
        )
        .range(0.0, 255.0),
    );
    t.single(
        0x0C,
        2,
        C::Engine,
        v(
            tx("Motordrehzahl", "Engine speed"),
            "1/min",
            scaled(0, 2, 1.0, 4.0),
        )
        .range(0.0, 8000.0)
        .warn(0.0, 6500.0),
    );
    t.single(
        0x0D,
        1,
        C::Vehicle,
        v(tx("Geschwindigkeit", "Vehicle speed"), "km/h", raw8(0)).range(0.0, 250.0),
    );
    t.single(
        0x0E,
        1,
        C::Engine,
        v(
            tx("Zündzeitpunkt", "Ignition timing advance"),
            "Grad v.OT",
            scaled_with(0, 1, 1.0, 2.0, -64.0, false),
        )
        .range(-20.0, 60.0)
        .unit_en("deg BTDC"),
    );
    t.single(
        0x0F,
        1,
        C::Temperatures,
        v(
            tx("Ansauglufttemperatur", "Intake air temperature"),
            "°C",
            temp8(0),
        )
        .range(-40.0, 100.0),
    );
    t.single(
        0x10,
        2,
        C::Air,
        v(
            tx("Luftmasse (LMM)", "Mass air flow (MAF)"),
            "g/s",
            scaled(0, 2, 1.0, 100.0),
        )
        .range(0.0, 300.0),
    );
    t.single(
        0x11,
        1,
        C::Air,
        v(
            tx("Drosselklappenstellung", "Throttle position"),
            "%",
            percent(0),
        )
        .range(0.0, 100.0),
    );
    t.single(
        0x12,
        1,
        C::Emissions,
        v(tx("Sekundärluft", "Secondary air status"), "", raw8(0)).text(secondary_air_text),
    );
    t.single(
        0x13,
        1,
        C::Diagnostics,
        v(
            tx("Verbaute Lambdasonden", "Oxygen sensors fitted"),
            "",
            raw8(0),
        )
        .text(|x| o2_present_text(x as i32, 2, 4)),
    );

    // Narrow-band oxygen sensors: voltage, and the short-term trim the sensor
    // feeds. B = 0xFF means the sensor is not used for the trim.
    for i in 0..8 {
        let de = format!("Lambdasonde {}", o2_position(i, false));
        let en = format!("O2 sensor {}", o2_position(i, true));
        t.add(
            0x14 + i,
            2,
            C::Exhaust,
            Some(keep_text(de.clone(), en.clone())),
            vec![
                v(
                    keep_text(format!("{de}, Spannung"), format!("{en}, voltage")),
                    "V",
                    scaled(0, 1, 1.0, 200.0),
                )
                .range(0.0, 1.275),
                v(
                    keep_text(
                        format!("{de}, Gemischkorrektur"),
                        format!("{en}, fuel trim"),
                    ),
                    "%",
                    trim(1),
                )
                .range(-30.0, 30.0)
                .available(|d| d.len() >= 2 && d[1] != 0xFF),
            ],
        );
    }

    t.single(
        0x1C,
        1,
        C::Diagnostics,
        v(
            tx("Abgasnorm des Steuergeräts", "OBD standard of the ECU"),
            "",
            raw8(0),
        )
        .text(obd_standard_text),
    );
    t.single(
        0x1D,
        1,
        C::Diagnostics,
        v(
            tx(
                "Verbaute Lambdasonden (4 Bänke)",
                "Oxygen sensors fitted (4 banks)",
            ),
            "",
            raw8(0),
        )
        .text(|x| o2_present_text(x as i32, 4, 2)),
    );
    t.single(
        0x1E,
        1,
        C::Vehicle,
        v(
            tx("Nebenantrieb (PTO)", "Power take-off (PTO)"),
            "",
            bit_of(0, 0),
        )
        .text(active_text),
    );
    t.single(
        0x1F,
        2,
        C::Vehicle,
        v(
            tx("Laufzeit seit Motorstart", "Run time since engine start"),
            "s",
            raw16(0),
        )
        .range(0.0, 7200.0),
    );

    // --- 0x21 to 0x3F --------------------------------------------------------
    t.single(
        0x21,
        2,
        C::Diagnostics,
        v(
            tx("Strecke mit Fehlerlampe an", "Distance with MIL on"),
            "km",
            raw16(0),
        )
        .range(0.0, 65535.0),
    );
    t.single(
        0x22,
        2,
        C::Fuel,
        v(
            tx(
                "Raildruck relativ zum Saugrohr",
                "Fuel rail pressure (relative to manifold)",
            ),
            "kPa",
            scaled(0, 2, 0.079, 1.0),
        )
        .range(0.0, 5177.265),
    );
    t.single(
        0x23,
        2,
        C::Fuel,
        v(
            tx("Raildruck", "Fuel rail pressure"),
            "kPa",
            scaled(0, 2, 10.0, 1.0),
        )
        .range(0.0, 250000.0),
    );

    // Wide-band oxygen sensors: lambda together with voltage (0x24 to 0x2B)
    // or with pump current (0x34 to 0x3B).
    for i in 0..8 {
        let de = format!("Breitbandsonde {}", o2_position(i, false));
        let en = format!("Wide-band O2 sensor {}", o2_position(i, true));
        t.add(
            0x24 + i,
            4,
            C::Exhaust,
            Some(keep_text(de.clone(), en.clone())),
            vec![
                v(
                    keep_text(format!("{de}, Lambda"), format!("{en}, lambda")),
                    "",
                    lambda(0),
                )
                .range(0.0, 2.0),
                v(
                    keep_text(format!("{de}, Spannung"), format!("{en}, voltage")),
                    "V",
                    scaled(2, 2, 8.0, 65536.0),
                )
                .range(0.0, 8.0),
            ],
        );
    }
    t.single(
        0x2C,
        1,
        C::Emissions,
        v(
            tx("Abgasrückführung angefordert", "Commanded EGR"),
            "%",
            percent(0),
        )
        .range(0.0, 100.0),
    );
    t.single(
        0x2D,
        1,
        C::Emissions,
        v(
            tx("Abgasrückführung Regelabweichung", "EGR error"),
            "%",
            trim(0),
        )
        .range(-100.0, 100.0)
        .warn(-20.0, 20.0),
    );
    t.single(
        0x2E,
        1,
        C::Emissions,
        v(
            tx("Tankentlüftung angefordert", "Commanded evaporative purge"),
            "%",
            percent(0),
        )
        .range(0.0, 100.0),
    );
    t.single(
        0x2F,
        1,
        C::Fuel,
        v(tx("Tankfüllstand", "Fuel tank level"), "%", percent(0))
            .range(0.0, 100.0)
            .warn(10.0, 101.0),
    );
    t.single(
        0x30,
        1,
        C::Diagnostics,
        v(
            tx("Warmläufe seit Löschen", "Warm-ups since codes cleared"),
            "",
            raw8(0),
        )
        .range(0.0, 255.0),
    );
    t.single(
        0x31,
        2,
        C::Diagnostics,
        v(
            tx("Strecke seit Löschen", "Distance since codes cleared"),
            "km",
            raw16(0),
        )
        .range(0.0, 65535.0),
    );
    t.single(
        0x32,
        2,
        C::Emissions,
        v(
            tx("Tankdruck", "Fuel tank vapor pressure"),
            "Pa",
            scaled_with(0, 2, 1.0, 4.0, 0.0, true),
        )
        .range(-8192.0, 8192.0),
    );
    t.single(
        0x33,
        1,
        C::Air,
        v(
            tx("Luftdruck (barometrisch)", "Barometric pressure"),
            "kPa",
            raw8(0),
        )
        .range(50.0, 110.0),
    );
    for i in 0..8 {
        let de = format!("Breitbandsonde {}", o2_position(i, false));
        let en = format!("Wide-band O2 sensor {}", o2_position(i, true));
        t.add(
            0x34 + i,
            4,
            C::Exhaust,
            Some(keep_text(
                format!("{de} (Strom)"),
                format!("{en} (current)"),
            )),
            vec![
                v(
                    keep_text(format!("{de}, Lambda"), format!("{en}, lambda")),
                    "",
                    lambda(0),
                )
                .range(0.0, 2.0),
                // Signed with offset 0x8000, 1/256 mA per bit.
                v(
                    keep_text(format!("{de}, Pumpstrom"), format!("{en}, current")),
                    "mA",
                    scaled_with(2, 2, 1.0, 256.0, -128.0, false),
                )
                .range(-5.0, 5.0),
            ],
        );
    }
    let cats = [
        tx(
            "Katalysatortemperatur Bank 1 Sonde 1",
            "Catalyst temperature bank 1 sensor 1",
        ),
        tx(
            "Katalysatortemperatur Bank 2 Sonde 1",
            "Catalyst temperature bank 2 sensor 1",
        ),
        tx(
            "Katalysatortemperatur Bank 1 Sonde 2",
            "Catalyst temperature bank 1 sensor 2",
        ),
        tx(
            "Katalysatortemperatur Bank 2 Sonde 2",
            "Catalyst temperature bank 2 sensor 2",
        ),
    ];
    for (i, name) in cats.into_iter().enumerate() {
        t.single(
            0x3C + i as i32,
            2,
            C::Exhaust,
            v(name, "°C", temp16(0))
                .range(0.0, 1000.0)
                .warn(-50.0, 900.0),
        );
    }

    // --- 0x41 to 0x5F --------------------------------------------------------
    t.single(
        0x42,
        2,
        C::Electrical,
        v(
            tx("Spannung Steuergerät", "Control unit voltage"),
            "V",
            scaled(0, 2, 1.0, 1000.0),
        )
        .range(8.0, 16.0)
        .warn(11.5, 15.0),
    );
    t.single(
        0x43,
        2,
        C::Engine,
        v(
            tx("Absolute Last", "Absolute load"),
            "%",
            scaled(0, 2, 100.0, 255.0),
        )
        .range(0.0, 200.0),
    );
    t.single(
        0x44,
        2,
        C::Fuel,
        v(tx("Lambda angefordert", "Commanded lambda"), "", lambda(0)).range(0.0, 2.0),
    );
    t.single(
        0x45,
        1,
        C::Air,
        v(
            tx(
                "Relative Drosselklappenstellung",
                "Relative throttle position",
            ),
            "%",
            percent(0),
        )
        .range(0.0, 100.0),
    );
    t.single(
        0x46,
        1,
        C::Temperatures,
        v(
            tx("Umgebungstemperatur", "Ambient air temperature"),
            "°C",
            temp8(0),
        )
        .range(-40.0, 60.0),
    );
    let positions = [
        tx("Drosselklappe absolut B", "Absolute throttle position B"),
        tx("Drosselklappe absolut C", "Absolute throttle position C"),
        tx("Fahrpedalstellung D", "Accelerator pedal position D"),
        tx("Fahrpedalstellung E", "Accelerator pedal position E"),
        tx("Fahrpedalstellung F", "Accelerator pedal position F"),
        tx("Drosselklappen-Ansteuerung", "Commanded throttle actuator"),
    ];
    for (i, name) in positions.into_iter().enumerate() {
        t.single(
            0x47 + i as i32,
            1,
            if (2..=4).contains(&i) {
                C::Engine
            } else {
                C::Air
            },
            v(name, "%", percent(0)).range(0.0, 100.0),
        );
    }
    t.single(
        0x4D,
        2,
        C::Diagnostics,
        v(
            tx("Laufzeit mit Fehlerlampe an", "Run time with MIL on"),
            "min",
            raw16(0),
        )
        .range(0.0, 65535.0),
    );
    t.single(
        0x4E,
        2,
        C::Diagnostics,
        v(
            tx("Zeit seit dem Löschen", "Time since codes cleared"),
            "min",
            raw16(0),
        )
        .range(0.0, 65535.0),
    );
    // Scaling limits the control unit uses for other PIDs. 0 means the
    // standard scaling applies.
    t.add(
        0x4F,
        4,
        C::Diagnostics,
        Some(tx("Skalierungsgrenzen", "Maximum scaling values")),
        vec![
            v(tx("Höchstwert Lambda", "Maximum lambda"), "", raw8(0)).range(0.0, 255.0),
            v(
                tx(
                    "Höchstwert Lambdasondenspannung",
                    "Maximum O2 sensor voltage",
                ),
                "V",
                raw8(1),
            )
            .range(0.0, 255.0),
            v(
                tx("Höchstwert Lambdasondenstrom", "Maximum O2 sensor current"),
                "mA",
                raw8(2),
            )
            .range(0.0, 255.0),
            v(
                tx(
                    "Höchstwert Saugrohrdruck",
                    "Maximum intake manifold pressure",
                ),
                "kPa",
                scaled(3, 1, 10.0, 1.0),
            )
            .range(0.0, 2550.0),
        ],
    );
    t.single(
        0x50,
        4,
        C::Diagnostics,
        v(
            tx("Höchstwert Luftmasse", "Maximum mass air flow"),
            "g/s",
            scaled(0, 1, 10.0, 1.0),
        )
        .range(0.0, 2550.0),
    );
    t.single(
        0x51,
        1,
        C::Fuel,
        v(tx("Kraftstoffart", "Fuel type"), "", raw8(0)).text(fuel_type_text),
    );
    t.single(
        0x52,
        1,
        C::Fuel,
        v(tx("Ethanolanteil", "Ethanol content"), "%", percent(0)).range(0.0, 100.0),
    );
    t.single(
        0x53,
        2,
        C::Emissions,
        v(
            tx("Tankdruck absolut", "Absolute fuel tank vapor pressure"),
            "kPa",
            scaled(0, 2, 1.0, 200.0),
        )
        .range(0.0, 327.675),
    );
    t.single(
        0x54,
        2,
        C::Emissions,
        v(
            tx(
                "Tankdruck (großer Bereich)",
                "Fuel tank vapor pressure (wide range)",
            ),
            "Pa",
            scaled_with(0, 2, 1.0, 1.0, 0.0, true),
        )
        .range(-32768.0, 32767.0),
    );
    let secondary = [
        [
            tx(
                "Kurzzeit-Korrektur hintere Lambdasonde Bank 1",
                "Short-term secondary O2 trim bank 1",
            ),
            tx(
                "Kurzzeit-Korrektur hintere Lambdasonde Bank 3",
                "Short-term secondary O2 trim bank 3",
            ),
        ],
        [
            tx(
                "Langzeit-Korrektur hintere Lambdasonde Bank 1",
                "Long-term secondary O2 trim bank 1",
            ),
            tx(
                "Langzeit-Korrektur hintere Lambdasonde Bank 3",
                "Long-term secondary O2 trim bank 3",
            ),
        ],
        [
            tx(
                "Kurzzeit-Korrektur hintere Lambdasonde Bank 2",
                "Short-term secondary O2 trim bank 2",
            ),
            tx(
                "Kurzzeit-Korrektur hintere Lambdasonde Bank 4",
                "Short-term secondary O2 trim bank 4",
            ),
        ],
        [
            tx(
                "Langzeit-Korrektur hintere Lambdasonde Bank 2",
                "Long-term secondary O2 trim bank 2",
            ),
            tx(
                "Langzeit-Korrektur hintere Lambdasonde Bank 4",
                "Long-term secondary O2 trim bank 4",
            ),
        ],
    ];
    for (i, names) in secondary.into_iter().enumerate() {
        t.add(
            0x55 + i as i32,
            2,
            C::Fuel,
            None,
            vec![
                v(names[0], "%", trim(0))
                    .range(-30.0, 30.0)
                    .warn(-20.0, 20.0),
                v(names[1], "%", trim(1))
                    .range(-30.0, 30.0)
                    .warn(-20.0, 20.0),
            ],
        );
    }
    t.single(
        0x59,
        2,
        C::Fuel,
        v(
            tx("Raildruck absolut", "Fuel rail pressure (absolute)"),
            "kPa",
            scaled(0, 2, 10.0, 1.0),
        )
        .range(0.0, 250000.0),
    );
    t.single(
        0x5A,
        1,
        C::Engine,
        v(
            tx(
                "Fahrpedalstellung relativ",
                "Relative accelerator pedal position",
            ),
            "%",
            percent(0),
        )
        .range(0.0, 100.0),
    );
    t.single(
        0x5B,
        1,
        C::HybridEv,
        v(
            tx(
                "Restkapazität Hybridbatterie",
                "Hybrid battery remaining life",
            ),
            "%",
            percent(0),
        )
        .range(0.0, 100.0),
    );
    t.single(
        0x5C,
        1,
        C::Temperatures,
        v(tx("Öltemperatur", "Engine oil temperature"), "°C", temp8(0))
            .range(-40.0, 150.0)
            .warn(-10.0, 130.0),
    );
    t.single(
        0x5D,
        2,
        C::Fuel,
        v(
            tx("Einspritzbeginn", "Fuel injection timing"),
            "Grad v.OT",
            scaled_with(0, 2, 1.0, 128.0, -210.0, false),
        )
        .range(-30.0, 60.0)
        .unit_en("deg BTDC"),
    );
    t.single(
        0x5E,
        2,
        C::Fuel,
        v(
            tx("Kraftstoffverbrauch", "Engine fuel rate"),
            "l/h",
            scaled(0, 2, 1.0, 20.0),
        )
        .range(0.0, 30.0),
    );
    t.single(
        0x5F,
        1,
        C::Diagnostics,
        v(
            tx(
                "Ausgelegt für Abgasvorschrift",
                "Emission requirements designed for",
            ),
            "",
            raw8(0),
        )
        .text(emission_design_text),
    );

    // --- 0x61 to 0x7F --------------------------------------------------------
    t.single(
        0x61,
        1,
        C::Engine,
        v(
            tx("Drehmoment angefordert", "Driver's demand engine torque"),
            "%",
            torque(0),
        )
        .range(-25.0, 100.0),
    );
    t.single(
        0x62,
        1,
        C::Engine,
        v(
            tx("Drehmoment tatsächlich", "Actual engine torque"),
            "%",
            torque(0),
        )
        .range(-25.0, 100.0),
    );
    t.single(
        0x63,
        2,
        C::Engine,
        v(
            tx("Bezugsdrehmoment des Motors", "Engine reference torque"),
            "Nm",
            raw16(0),
        )
        .range(0.0, 1000.0),
    );
    t.add(
        0x64,
        5,
        C::Engine,
        Some(tx("Drehmomentkennlinie", "Engine percent torque data")),
        vec![
            v(
                tx("Drehmoment im Leerlauf", "Torque at idle"),
                "%",
                torque(0),
            )
            .range(-25.0, 100.0),
            v(
                tx("Drehmoment Kennlinienpunkt 1", "Torque at engine point 1"),
                "%",
                torque(1),
            )
            .range(-25.0, 100.0),
            v(
                tx("Drehmoment Kennlinienpunkt 2", "Torque at engine point 2"),
                "%",
                torque(2),
            )
            .range(-25.0, 100.0),
            v(
                tx("Drehmoment Kennlinienpunkt 3", "Torque at engine point 3"),
                "%",
                torque(3),
            )
            .range(-25.0, 100.0),
            v(
                tx("Drehmoment Kennlinienpunkt 4", "Torque at engine point 4"),
                "%",
                torque(4),
            )
            .range(-25.0, 100.0),
        ],
    );
    t.add(
        0x65,
        2,
        C::Vehicle,
        Some(tx(
            "Zusätzliche Ein- und Ausgänge",
            "Auxiliary inputs and outputs",
        )),
        vec![
            v(
                tx("Nebenantrieb (PTO)", "Power take-off (PTO)"),
                "",
                bit_of(1, 0),
            )
            .bit(0)
            .text(active_text),
            v(
                tx(
                    "Automatikgetriebe, Wählhebel",
                    "Automatic transmission range",
                ),
                "",
                bit_of(1, 1),
            )
            .bit(1)
            .text(automatic_range_text),
            v(
                tx("Schaltgetriebe, Gang", "Manual transmission gear"),
                "",
                bit_of(1, 2),
            )
            .bit(2)
            .text(manual_gear_text),
            v(tx("Vorglühlampe", "Glow plug lamp"), "", bit_of(1, 3))
                .bit(3)
                .text(on_off_text),
        ],
    );
    t.add(
        0x66,
        5,
        C::Air,
        Some(tx(
            "Luftmassenmesser A und B",
            "Mass air flow sensors A and B",
        )),
        vec![
            v(
                tx("Luftmasse Sensor A", "Mass air flow sensor A"),
                "g/s",
                scaled(1, 2, 1.0, 32.0),
            )
            .bit(0)
            .range(0.0, 300.0),
            v(
                tx("Luftmasse Sensor B", "Mass air flow sensor B"),
                "g/s",
                scaled(3, 2, 1.0, 32.0),
            )
            .bit(1)
            .range(0.0, 300.0),
        ],
    );
    t.add(
        0x67,
        3,
        C::Temperatures,
        Some(tx(
            "Kühlmitteltemperatur (Sensoren)",
            "Coolant temperature sensors",
        )),
        vec![
            v(
                tx(
                    "Kühlmitteltemperatur Sensor 1",
                    "Coolant temperature sensor 1",
                ),
                "°C",
                temp8(1),
            )
            .bit(0)
            .range(-40.0, 130.0)
            .warn(-10.0, 110.0),
            v(
                tx(
                    "Kühlmitteltemperatur Sensor 2",
                    "Coolant temperature sensor 2",
                ),
                "°C",
                temp8(2),
            )
            .bit(1)
            .range(-40.0, 130.0)
            .warn(-10.0, 110.0),
        ],
    );
    // Older control units send three bytes (two sensors), newer ones seven.
    {
        let pos_de = [
            "Bank 1 Sensor 1",
            "Bank 1 Sensor 2",
            "Bank 1 Sensor 3",
            "Bank 2 Sensor 1",
            "Bank 2 Sensor 2",
            "Bank 2 Sensor 3",
        ];
        let pos_en = [
            "bank 1 sensor 1",
            "bank 1 sensor 2",
            "bank 1 sensor 3",
            "bank 2 sensor 1",
            "bank 2 sensor 2",
            "bank 2 sensor 3",
        ];
        let defs = (0..6)
            .map(|i| {
                v(
                    keep_text(
                        format!("Ansauglufttemperatur {}", pos_de[i]),
                        format!("Intake air temperature {}", pos_en[i]),
                    ),
                    "°C",
                    temp8(1 + i),
                )
                .bit(i as i32)
                .range(-40.0, 100.0)
            })
            .collect();
        t.add(
            0x68,
            7,
            C::Temperatures,
            Some(tx(
                "Ansauglufttemperatur (Sensoren)",
                "Intake air temperature sensors",
            )),
            defs,
        );
    }
    t.add(
        0x69,
        7,
        C::Emissions,
        Some(tx(
            "Abgasrückführung Soll und Ist",
            "EGR commanded and actual",
        )),
        vec![
            v(
                tx("Abgasrückführung A angefordert", "Commanded EGR A"),
                "%",
                percent(1),
            )
            .bit(0)
            .range(0.0, 100.0),
            v(
                tx("Abgasrückführung A tatsächlich", "Actual EGR A"),
                "%",
                percent(2),
            )
            .bit(1)
            .range(0.0, 100.0),
            v(
                tx("Abgasrückführung A Regelabweichung", "EGR A error"),
                "%",
                trim(3),
            )
            .bit(2)
            .range(-100.0, 100.0)
            .warn(-20.0, 20.0),
            v(
                tx("Abgasrückführung B angefordert", "Commanded EGR B"),
                "%",
                percent(4),
            )
            .bit(3)
            .range(0.0, 100.0),
            v(
                tx("Abgasrückführung B tatsächlich", "Actual EGR B"),
                "%",
                percent(5),
            )
            .bit(4)
            .range(0.0, 100.0),
            v(
                tx("Abgasrückführung B Regelabweichung", "EGR B error"),
                "%",
                trim(6),
            )
            .bit(5)
            .range(-100.0, 100.0)
            .warn(-20.0, 20.0),
        ],
    );
    t.add(
        0x6A,
        5,
        C::Air,
        Some(tx(
            "Ansaugluftklappe (Diesel)",
            "Diesel intake air flow control",
        )),
        vec![
            v(
                tx(
                    "Ansaugluftklappe A angefordert",
                    "Commanded intake air flow A",
                ),
                "%",
                percent(1),
            )
            .bit(0)
            .range(0.0, 100.0),
            v(
                tx("Ansaugluftklappe A Stellung", "Intake air flow position A"),
                "%",
                percent(2),
            )
            .bit(1)
            .range(0.0, 100.0),
            v(
                tx(
                    "Ansaugluftklappe B angefordert",
                    "Commanded intake air flow B",
                ),
                "%",
                percent(3),
            )
            .bit(2)
            .range(0.0, 100.0),
            v(
                tx("Ansaugluftklappe B Stellung", "Intake air flow position B"),
                "%",
                percent(4),
            )
            .bit(3)
            .range(0.0, 100.0),
        ],
    );
    t.add(
        0x6B,
        5,
        C::Emissions,
        Some(tx("Temperatur Abgasrückführung", "EGR temperature")),
        vec![
            v(
                tx(
                    "Temperatur Abgasrückführung Bank 1 Sensor 1",
                    "EGR temperature bank 1 sensor 1",
                ),
                "°C",
                temp8_wide(1, 4),
            )
            .bit(0)
            .range(-40.0, 600.0),
            v(
                tx(
                    "Temperatur Abgasrückführung Bank 1 Sensor 2",
                    "EGR temperature bank 1 sensor 2",
                ),
                "°C",
                temp8_wide(2, 5),
            )
            .bit(1)
            .range(-40.0, 600.0),
            v(
                tx(
                    "Temperatur Abgasrückführung Bank 2 Sensor 1",
                    "EGR temperature bank 2 sensor 1",
                ),
                "°C",
                temp8_wide(3, 6),
            )
            .bit(2)
            .range(-40.0, 600.0),
            v(
                tx(
                    "Temperatur Abgasrückführung Bank 2 Sensor 2",
                    "EGR temperature bank 2 sensor 2",
                ),
                "°C",
                temp8_wide(4, 7),
            )
            .bit(3)
            .range(-40.0, 600.0),
        ],
    );
    t.add(
        0x6C,
        5,
        C::Air,
        Some(tx(
            "Drosselklappen-Ansteuerung A und B",
            "Throttle actuator A and B",
        )),
        vec![
            v(
                tx(
                    "Drosselklappe A angefordert",
                    "Commanded throttle actuator A",
                ),
                "%",
                percent(1),
            )
            .bit(0)
            .range(0.0, 100.0),
            v(
                tx(
                    "Drosselklappe A Stellung relativ",
                    "Relative throttle position A",
                ),
                "%",
                percent(2),
            )
            .bit(1)
            .range(0.0, 100.0),
            v(
                tx(
                    "Drosselklappe B angefordert",
                    "Commanded throttle actuator B",
                ),
                "%",
                percent(3),
            )
            .bit(2)
            .range(0.0, 100.0),
            v(
                tx(
                    "Drosselklappe B Stellung relativ",
                    "Relative throttle position B",
                ),
                "%",
                percent(4),
            )
            .bit(3)
            .range(0.0, 100.0),
        ],
    );
    t.add(
        0x6D,
        11,
        C::Fuel,
        Some(tx(
            "Kraftstoffdruckregelung",
            "Fuel pressure control system",
        )),
        vec![
            v(
                tx("Raildruck A angefordert", "Commanded fuel rail pressure A"),
                "kPa",
                scaled(1, 2, 10.0, 1.0),
            )
            .bit(0)
            .range(0.0, 250000.0),
            v(
                tx("Raildruck A", "Fuel rail pressure A"),
                "kPa",
                scaled(3, 2, 10.0, 1.0),
            )
            .bit(1)
            .range(0.0, 250000.0),
            v(
                tx("Kraftstofftemperatur A", "Fuel temperature A"),
                "°C",
                temp8(5),
            )
            .bit(2)
            .range(-40.0, 120.0),
            v(
                tx("Raildruck B angefordert", "Commanded fuel rail pressure B"),
                "kPa",
                scaled(6, 2, 10.0, 1.0),
            )
            .bit(3)
            .range(0.0, 250000.0),
            v(
                tx("Raildruck B", "Fuel rail pressure B"),
                "kPa",
                scaled(8, 2, 10.0, 1.0),
            )
            .bit(4)
            .range(0.0, 250000.0),
            v(
                tx("Kraftstofftemperatur B", "Fuel temperature B"),
                "°C",
                temp8(10),
            )
            .bit(5)
            .range(-40.0, 120.0),
        ],
    );
    t.add(
        0x6E,
        9,
        C::Fuel,
        Some(tx(
            "Einspritzdruckregelung",
            "Injection pressure control system",
        )),
        vec![
            v(
                tx(
                    "Einspritzdruck A angefordert",
                    "Commanded injection control pressure A",
                ),
                "kPa",
                scaled(1, 2, 10.0, 1.0),
            )
            .bit(0)
            .range(0.0, 250000.0),
            v(
                tx("Einspritzdruck A", "Injection control pressure A"),
                "kPa",
                scaled(3, 2, 10.0, 1.0),
            )
            .bit(1)
            .range(0.0, 250000.0),
            v(
                tx(
                    "Einspritzdruck B angefordert",
                    "Commanded injection control pressure B",
                ),
                "kPa",
                scaled(5, 2, 10.0, 1.0),
            )
            .bit(2)
            .range(0.0, 250000.0),
            v(
                tx("Einspritzdruck B", "Injection control pressure B"),
                "kPa",
                scaled(7, 2, 10.0, 1.0),
            )
            .bit(3)
            .range(0.0, 250000.0),
        ],
    );
    t.add(
        0x6F,
        3,
        C::Air,
        Some(tx(
            "Druck vor dem Turbolader",
            "Turbocharger compressor inlet pressure",
        )),
        vec![
            v(
                tx("Druck vor Turbolader A", "Compressor inlet pressure A"),
                "kPa",
                raw8(1),
            )
            .bit(0)
            .range(0.0, 255.0),
            v(
                tx("Druck vor Turbolader B", "Compressor inlet pressure B"),
                "kPa",
                raw8(2),
            )
            .bit(1)
            .range(0.0, 255.0),
        ],
    );
    t.add(
        0x70,
        10,
        C::Air,
        Some(tx("Ladedruckregelung", "Boost pressure control")),
        vec![
            v(
                tx("Ladedruck A angefordert", "Commanded boost pressure A"),
                "kPa",
                scaled(1, 2, 1.0, 32.0),
            )
            .bit(0)
            .range(0.0, 300.0),
            v(
                tx("Ladedruck A", "Boost pressure A"),
                "kPa",
                scaled(3, 2, 1.0, 32.0),
            )
            .bit(1)
            .range(0.0, 300.0),
            v(
                tx("Ladedruck B angefordert", "Commanded boost pressure B"),
                "kPa",
                scaled(5, 2, 1.0, 32.0),
            )
            .bit(2)
            .range(0.0, 300.0),
            v(
                tx("Ladedruck B", "Boost pressure B"),
                "kPa",
                scaled(7, 2, 1.0, 32.0),
            )
            .bit(3)
            .range(0.0, 300.0),
            v(
                tx("Ladedruckregelung A", "Boost pressure control A"),
                "",
                bits_of(9, 0, 3),
            )
            .bit(4)
            .text(control_loop_text),
            v(
                tx("Ladedruckregelung B", "Boost pressure control B"),
                "",
                bits_of(9, 2, 3),
            )
            .bit(5)
            .text(control_loop_text),
        ],
    );
    t.add(
        0x71,
        6,
        C::Air,
        Some(tx(
            "Verstellbarer Turbolader (VTG)",
            "Variable geometry turbo control",
        )),
        vec![
            v(
                tx(
                    "Turbolader-Leitschaufeln A angefordert",
                    "Commanded VGT A position",
                ),
                "%",
                percent(1),
            )
            .bit(0)
            .range(0.0, 100.0),
            v(
                tx("Turbolader-Leitschaufeln A Stellung", "VGT A position"),
                "%",
                percent(2),
            )
            .bit(1)
            .range(0.0, 100.0),
            v(
                tx(
                    "Turbolader-Leitschaufeln B angefordert",
                    "Commanded VGT B position",
                ),
                "%",
                percent(3),
            )
            .bit(2)
            .range(0.0, 100.0),
            v(
                tx("Turbolader-Leitschaufeln B Stellung", "VGT B position"),
                "%",
                percent(4),
            )
            .bit(3)
            .range(0.0, 100.0),
            v(
                tx("Turbolader-Regelung A", "VGT A control"),
                "",
                bits_of(5, 0, 3),
            )
            .bit(4)
            .text(control_loop_text),
            v(
                tx("Turbolader-Regelung B", "VGT B control"),
                "",
                bits_of(5, 2, 3),
            )
            .bit(5)
            .text(control_loop_text),
        ],
    );
    t.add(
        0x72,
        5,
        C::Air,
        Some(tx("Wastegate (Ladedruckventil)", "Wastegate control")),
        vec![
            v(
                tx("Wastegate A angefordert", "Commanded wastegate A"),
                "%",
                percent(1),
            )
            .bit(0)
            .range(0.0, 100.0),
            v(
                tx("Wastegate A Stellung", "Wastegate A position"),
                "%",
                percent(2),
            )
            .bit(1)
            .range(0.0, 100.0),
            v(
                tx("Wastegate B angefordert", "Commanded wastegate B"),
                "%",
                percent(3),
            )
            .bit(2)
            .range(0.0, 100.0),
            v(
                tx("Wastegate B Stellung", "Wastegate B position"),
                "%",
                percent(4),
            )
            .bit(3)
            .range(0.0, 100.0),
        ],
    );
    t.add(
        0x73,
        5,
        C::Exhaust,
        Some(tx("Abgasdruck", "Exhaust pressure")),
        vec![
            v(
                tx("Abgasdruck Bank 1", "Exhaust pressure bank 1"),
                "kPa",
                scaled(1, 2, 1.0, 100.0),
            )
            .bit(0)
            .range(0.0, 300.0),
            v(
                tx("Abgasdruck Bank 2", "Exhaust pressure bank 2"),
                "kPa",
                scaled(3, 2, 1.0, 100.0),
            )
            .bit(1)
            .range(0.0, 300.0),
        ],
    );
    t.add(
        0x74,
        5,
        C::Air,
        Some(tx("Turboladerdrehzahl", "Turbocharger speed")),
        vec![
            v(
                tx("Drehzahl Turbolader A", "Turbocharger A speed"),
                "1/min",
                scaled(1, 2, 10.0, 1.0),
            )
            .bit(0)
            .range(0.0, 250000.0),
            v(
                tx("Drehzahl Turbolader B", "Turbocharger B speed"),
                "1/min",
                scaled(3, 2, 10.0, 1.0),
            )
            .bit(1)
            .range(0.0, 250000.0),
        ],
    );
    for i in 0..2 {
        let tc = if i == 0 { "A" } else { "B" };
        t.add(
            0x75 + i,
            7,
            C::Air,
            Some(keep_text(
                format!("Turboladertemperaturen {tc}"),
                format!("Turbocharger {tc} temperatures"),
            )),
            vec![
                v(
                    keep_text(
                        format!("Turbolader {tc}, Luft vor Verdichter"),
                        format!("Turbocharger {tc} compressor inlet temperature"),
                    ),
                    "°C",
                    temp8(1),
                )
                .bit(0)
                .range(-40.0, 100.0),
                v(
                    keep_text(
                        format!("Turbolader {tc}, Luft nach Verdichter"),
                        format!("Turbocharger {tc} compressor outlet temperature"),
                    ),
                    "°C",
                    temp8(2),
                )
                .bit(1)
                .range(-40.0, 250.0),
                v(
                    keep_text(
                        format!("Turbolader {tc}, Abgas vor Turbine"),
                        format!("Turbocharger {tc} turbine inlet temperature"),
                    ),
                    "°C",
                    temp16(3),
                )
                .bit(2)
                .range(0.0, 1000.0)
                .warn(-50.0, 950.0),
                v(
                    keep_text(
                        format!("Turbolader {tc}, Abgas nach Turbine"),
                        format!("Turbocharger {tc} turbine outlet temperature"),
                    ),
                    "°C",
                    temp16(5),
                )
                .bit(3)
                .range(0.0, 1000.0),
            ],
        );
    }
    t.add(
        0x77,
        5,
        C::Temperatures,
        Some(tx("Ladeluftkühler", "Charge air cooler temperature")),
        vec![
            v(
                tx(
                    "Ladelufttemperatur Bank 1 Sensor 1",
                    "Charge air temperature bank 1 sensor 1",
                ),
                "°C",
                temp8_wide(1, 4),
            )
            .bit(0)
            .range(-40.0, 150.0),
            v(
                tx(
                    "Ladelufttemperatur Bank 1 Sensor 2",
                    "Charge air temperature bank 1 sensor 2",
                ),
                "°C",
                temp8_wide(2, 5),
            )
            .bit(1)
            .range(-40.0, 150.0),
            v(
                tx(
                    "Ladelufttemperatur Bank 2 Sensor 1",
                    "Charge air temperature bank 2 sensor 1",
                ),
                "°C",
                temp8_wide(3, 6),
            )
            .bit(2)
            .range(-40.0, 150.0),
            v(
                tx(
                    "Ladelufttemperatur Bank 2 Sensor 2",
                    "Charge air temperature bank 2 sensor 2",
                ),
                "°C",
                temp8_wide(4, 7),
            )
            .bit(3)
            .range(-40.0, 150.0),
        ],
    );
    // Exhaust gas temperatures: 0x78/0x79 sensors 1 to 4, 0x98/0x99 sensors
    // 5 to 8, each with 0.1 C per bit and four support bits in byte A.
    let add_egt = |t: &mut TableBuilder, pid: i32, bank: i32, first_sensor: i32| {
        let defs = (0..4)
            .map(|s| {
                v(
                    keep_text(
                        format!("Abgastemperatur Bank {bank} Sensor {}", first_sensor + s),
                        format!(
                            "Exhaust gas temperature bank {bank} sensor {}",
                            first_sensor + s
                        ),
                    ),
                    "°C",
                    temp16(1 + 2 * s as usize),
                )
                .bit(s)
                .range(0.0, 1000.0)
                .warn(-50.0, 950.0)
            })
            .collect();
        t.add(
            pid,
            9,
            C::Exhaust,
            Some(keep_text(
                format!("Abgastemperatur Bank {bank}"),
                format!("Exhaust gas temperature bank {bank}"),
            )),
            defs,
        );
    };
    add_egt(&mut t, 0x78, 1, 1);
    add_egt(&mut t, 0x79, 2, 1);
    for bank in 1..=2 {
        t.add(
            0x79 + bank,
            7,
            C::Emissions,
            Some(keep_text(
                format!("Partikelfilter Bank {bank}, Drücke"),
                format!("Particulate filter bank {bank} pressures"),
            )),
            vec![
                // Signed, 0.01 kPa per bit: a clogged filter shows a high value.
                v(
                    keep_text(
                        format!("Partikelfilter Bank {bank}, Differenzdruck"),
                        format!("Particulate filter bank {bank}, differential pressure"),
                    ),
                    "kPa",
                    scaled_with(1, 2, 1.0, 100.0, 0.0, true),
                )
                .bit(0)
                .range(0.0, 50.0),
                v(
                    keep_text(
                        format!("Partikelfilter Bank {bank}, Druck davor"),
                        format!("Particulate filter bank {bank}, inlet pressure"),
                    ),
                    "kPa",
                    scaled(3, 2, 1.0, 100.0),
                )
                .bit(1)
                .range(0.0, 300.0),
                v(
                    keep_text(
                        format!("Partikelfilter Bank {bank}, Druck danach"),
                        format!("Particulate filter bank {bank}, outlet pressure"),
                    ),
                    "kPa",
                    scaled(5, 2, 1.0, 100.0),
                )
                .bit(2)
                .range(0.0, 300.0),
            ],
        );
    }
    t.add(
        0x7C,
        9,
        C::Emissions,
        Some(tx(
            "Partikelfilter, Temperaturen",
            "Particulate filter temperatures",
        )),
        vec![
            v(
                tx(
                    "Partikelfilter Bank 1, Temperatur davor",
                    "Particulate filter bank 1 inlet temperature",
                ),
                "°C",
                temp16(1),
            )
            .bit(0)
            .range(0.0, 1000.0),
            v(
                tx(
                    "Partikelfilter Bank 1, Temperatur danach",
                    "Particulate filter bank 1 outlet temperature",
                ),
                "°C",
                temp16(3),
            )
            .bit(1)
            .range(0.0, 1000.0),
            v(
                tx(
                    "Partikelfilter Bank 2, Temperatur davor",
                    "Particulate filter bank 2 inlet temperature",
                ),
                "°C",
                temp16(5),
            )
            .bit(2)
            .range(0.0, 1000.0),
            v(
                tx(
                    "Partikelfilter Bank 2, Temperatur danach",
                    "Particulate filter bank 2 outlet temperature",
                ),
                "°C",
                temp16(7),
            )
            .bit(3)
            .range(0.0, 1000.0),
        ],
    );
    t.single(
        0x7D,
        1,
        C::Emissions,
        v(
            tx(
                "Stickoxid-Kontrollbereich (NTE)",
                "NOx NTE control area status",
            ),
            "",
            raw8(0),
        )
        .text(nte_text),
    );
    t.single(
        0x7E,
        1,
        C::Emissions,
        v(
            tx(
                "Partikel-Kontrollbereich (NTE)",
                "PM NTE control area status",
            ),
            "",
            raw8(0),
        )
        .text(nte_text),
    );
    // Run time counters. The control unit counts seconds; hours read better.
    t.add(
        0x7F,
        13,
        C::Vehicle,
        Some(tx("Motorlaufzeit gesamt", "Engine run time")),
        vec![
            v(
                tx("Motorlaufzeit gesamt", "Total engine run time"),
                "h",
                scaled(1, 4, 1.0, 3600.0),
            )
            .bit(0)
            .range(0.0, 20000.0),
            v(
                tx("Motorlaufzeit im Leerlauf", "Total idle run time"),
                "h",
                scaled(5, 4, 1.0, 3600.0),
            )
            .bit(1)
            .range(0.0, 20000.0),
            v(
                tx(
                    "Laufzeit mit Nebenantrieb",
                    "Total run time with PTO active",
                ),
                "h",
                scaled(9, 4, 1.0, 3600.0),
            )
            .bit(2)
            .range(0.0, 20000.0),
        ],
    );

    // --- 0x81 to 0x9F --------------------------------------------------------
    // Run time of auxiliary emission control devices (AECD), two timers each.
    let add_aecd = |t: &mut TableBuilder, pid: i32, first: i32| {
        let mut defs = Vec::new();
        for i in 0..5 {
            for timer in 1..=2 {
                let n = first + i;
                defs.push(
                    v(
                        keep_text(
                            format!("Abgas-Sonderstrategie (AECD) {n}, Zeit {timer}"),
                            format!("Emission control device (AECD) {n}, timer {timer}"),
                        ),
                        "s",
                        raw32(1 + 8 * i as usize + 4 * (timer - 1) as usize),
                    )
                    .bit(i)
                    .range(0.0, 1e6),
                );
            }
        }
        t.add(
            pid,
            41,
            C::Diagnostics,
            Some(keep_text(
                format!("Laufzeit Abgas-Sonderstrategien {first} bis {}", first + 4),
                format!("Run time of AECD {first} to {}", first + 4),
            )),
            defs,
        );
    };
    add_aecd(&mut t, 0x81, 1);
    add_aecd(&mut t, 0x82, 6);
    let add_nox = |t: &mut TableBuilder, pid: i32, corrected: bool| {
        let pos_de = [
            "Bank 1 Sensor 1",
            "Bank 1 Sensor 2",
            "Bank 2 Sensor 1",
            "Bank 2 Sensor 2",
        ];
        let pos_en = [
            "bank 1 sensor 1",
            "bank 1 sensor 2",
            "bank 2 sensor 1",
            "bank 2 sensor 2",
        ];
        let de = if corrected {
            "Stickoxide korrigiert "
        } else {
            "Stickoxide "
        };
        let en = if corrected { "NOx corrected " } else { "NOx " };
        let defs = (0..4)
            .map(|i| {
                v(
                    keep_text(format!("{de}{}", pos_de[i]), format!("{en}{}", pos_en[i])),
                    "ppm",
                    raw16(1 + 2 * i),
                )
                .bit(i as i32)
                .range(0.0, 3000.0)
            })
            .collect();
        t.add(
            pid,
            9,
            C::Emissions,
            Some(if corrected {
                tx("Stickoxidsensoren, korrigiert", "NOx sensors, corrected")
            } else {
                tx("Stickoxidsensoren", "NOx sensors")
            }),
            defs,
        );
    };
    add_nox(&mut t, 0x83, false);
    t.single(
        0x84,
        1,
        C::Temperatures,
        v(
            tx(
                "Oberflächentemperatur Krümmer",
                "Manifold surface temperature",
            ),
            "°C",
            temp8(0),
        )
        .range(-40.0, 215.0),
    );
    t.add(
        0x85,
        10,
        C::Emissions,
        Some(tx("AdBlue-System", "NOx reagent system (AdBlue)")),
        vec![
            v(
                tx("AdBlue-Verbrauch im Schnitt", "Average reagent consumption"),
                "l/h",
                scaled(1, 2, 5.0, 1000.0),
            )
            .bit(0)
            .range(0.0, 5.0),
            v(
                tx(
                    "AdBlue-Bedarf im Schnitt",
                    "Average demanded reagent consumption",
                ),
                "l/h",
                scaled(3, 2, 5.0, 1000.0),
            )
            .bit(1)
            .range(0.0, 5.0),
            v(
                tx("AdBlue-Füllstand", "Reagent tank level"),
                "%",
                percent(5),
            )
            .bit(2)
            .range(0.0, 100.0)
            .warn(10.0, 101.0),
            v(
                tx("Zeit mit AdBlue-Warnung", "NOx warning indicator time"),
                "s",
                raw32(6),
            )
            .bit(3)
            .range(0.0, 1e6),
        ],
    );
    t.add(
        0x86,
        5,
        C::Emissions,
        Some(tx("Partikelsensor", "Particulate matter sensor")),
        vec![
            v(
                tx("Rußkonzentration Bank 1", "Particulate matter bank 1"),
                "mg/m3",
                scaled(1, 2, 1.0, 80.0),
            )
            .bit(0)
            .range(0.0, 50.0),
            v(
                tx("Rußkonzentration Bank 2", "Particulate matter bank 2"),
                "mg/m3",
                scaled(3, 2, 1.0, 80.0),
            )
            .bit(1)
            .range(0.0, 50.0),
        ],
    );
    t.add(
        0x87,
        5,
        C::Air,
        Some(tx(
            "Saugrohrdruck A und B",
            "Intake manifold pressure A and B",
        )),
        vec![
            v(
                tx("Saugrohrdruck A", "Intake manifold pressure A"),
                "kPa",
                scaled(1, 2, 1.0, 32.0),
            )
            .bit(0)
            .range(0.0, 300.0),
            v(
                tx("Saugrohrdruck B", "Intake manifold pressure B"),
                "kPa",
                scaled(3, 2, 1.0, 32.0),
            )
            .bit(1)
            .range(0.0, 300.0),
        ],
    );
    add_aecd(&mut t, 0x89, 11);
    add_aecd(&mut t, 0x8A, 16);
    t.add(
        0x8B,
        7,
        C::Emissions,
        Some(tx(
            "Abgasnachbehandlung (Diesel)",
            "Diesel aftertreatment status",
        )),
        vec![
            v(
                tx(
                    "Partikelfilter-Regeneration",
                    "Particulate filter regeneration",
                ),
                "",
                bit_of(1, 0),
            )
            .bit(0)
            .text(regeneration_text),
            v(
                tx("Art der Regeneration", "Regeneration type"),
                "",
                bit_of(1, 1),
            )
            .bit(1)
            .text(regeneration_type_text),
            v(
                tx("NOx-Speicherkat Regeneration", "NOx adsorber regeneration"),
                "",
                bit_of(1, 2),
            )
            .bit(2)
            .text(regeneration_text),
            v(
                tx(
                    "NOx-Speicherkat Entschwefelung",
                    "NOx adsorber desulfurization",
                ),
                "",
                bit_of(1, 3),
            )
            .bit(3)
            .text(regeneration_text),
            v(
                tx(
                    "Beladung bis zur nächsten Regeneration",
                    "Normalized trigger for regeneration",
                ),
                "%",
                percent(2),
            )
            .bit(4)
            .range(0.0, 100.0),
            v(
                tx(
                    "Zeit zwischen Regenerationen im Schnitt",
                    "Average time between regenerations",
                ),
                "min",
                raw16(3),
            )
            .bit(5)
            .range(0.0, 2000.0),
            v(
                tx(
                    "Strecke zwischen Regenerationen im Schnitt",
                    "Average distance between regenerations",
                ),
                "km",
                raw16(5),
            )
            .bit(6)
            .range(0.0, 2000.0),
        ],
    );
    t.single(
        0x8D,
        1,
        C::Air,
        v(
            tx("Drosselklappe absolut G", "Absolute throttle position G"),
            "%",
            percent(0),
        )
        .range(0.0, 100.0),
    );
    t.single(
        0x8E,
        1,
        C::Engine,
        v(
            tx("Reibmoment des Motors", "Engine friction torque"),
            "%",
            torque(0),
        )
        .range(-25.0, 25.0),
    );
    add_egt(&mut t, 0x98, 1, 5);
    add_egt(&mut t, 0x99, 2, 5);
    t.add(
        0x9A,
        6,
        C::HybridEv,
        Some(tx("Antriebsbatterie", "Hybrid/EV battery")),
        vec![
            v(
                tx("Spannung Antriebsbatterie", "Hybrid/EV battery voltage"),
                "V",
                scaled(2, 2, 1.0, 64.0),
            )
            .bit(1)
            .range(0.0, 1000.0),
            // Signed, 0.1 A per bit. The sign shows charging or discharging.
            v(
                tx("Strom Antriebsbatterie", "Hybrid/EV battery current"),
                "A",
                scaled_with(4, 2, 1.0, 10.0, 0.0, true),
            )
            .bit(2)
            .range(-400.0, 400.0),
        ],
    );
    t.single(
        0x9B,
        4,
        C::Emissions,
        v(
            tx(
                "AdBlue-Sensor, Füllstand",
                "Diesel exhaust fluid sensor, level",
            ),
            "%",
            percent(3),
        )
        .range(0.0, 100.0),
    );
    t.add(
        0x9D,
        4,
        C::Fuel,
        Some(tx("Kraftstoffmassenstrom", "Engine fuel rate (mass)")),
        vec![
            v(
                tx("Kraftstoffmassenstrom Motor", "Engine fuel rate"),
                "g/s",
                scaled(0, 2, 2.0, 100.0),
            )
            .range(0.0, 20.0),
            v(
                tx("Kraftstoffmassenstrom Fahrzeug", "Vehicle fuel rate"),
                "g/s",
                scaled(2, 2, 2.0, 100.0),
            )
            .range(0.0, 20.0),
        ],
    );
    t.single(
        0x9E,
        2,
        C::Exhaust,
        v(
            tx("Abgasmassenstrom", "Engine exhaust flow rate"),
            "kg/h",
            scaled(0, 2, 2.0, 10.0),
        )
        .range(0.0, 1000.0),
    );

    // --- 0xA1 to 0xC4 --------------------------------------------------------
    add_nox(&mut t, 0xA1, true);
    t.single(
        0xA2,
        2,
        C::Fuel,
        v(
            tx("Kraftstoffmenge je Hub", "Cylinder fuel rate"),
            "mg/Hub",
            scaled(0, 2, 1.0, 32.0),
        )
        .range(0.0, 100.0)
        .unit_en("mg/stroke"),
    );
    t.add(
        0xA4,
        4,
        C::Vehicle,
        Some(tx("Gang", "Transmission gear")),
        vec![
            v(
                tx("Eingelegter Gang", "Transmission actual gear"),
                "",
                bits_of(1, 4, 0x0F),
            )
            .bit(0)
            .range(0.0, 10.0)
            .text(gear_text),
            v(
                tx("Übersetzung des Gangs", "Actual gear ratio"),
                "",
                scaled(2, 2, 1.0, 1000.0),
            )
            .bit(1)
            .range(0.0, 6.0),
        ],
    );
    t.single(
        0xA5,
        4,
        C::Emissions,
        v(
            tx("AdBlue-Dosierung angefordert", "Commanded DEF dosing"),
            "%",
            scaled(1, 1, 1.0, 2.0),
        )
        .bit(0)
        .range(0.0, 127.5),
    );
    t.single(
        0xA6,
        4,
        C::Vehicle,
        v(
            tx("Kilometerstand", "Odometer"),
            "km",
            scaled(0, 4, 1.0, 10.0),
        )
        .range(0.0, 1e6),
    );

    t.table
}
