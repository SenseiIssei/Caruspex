//! Used car check: for someone standing next to a car they might buy.
//!
//! The caller reads what the car tells about itself (the VIN of the engine
//! control unit, the odometer of every control unit that has a documented
//! identifier for it, when the fault memory was last cleared, readiness,
//! faults, a short health check and, for electric cars, the battery) and
//! hands it in; this module judges it and answers with a traffic light, the
//! findings in plain words and the questions to ask the seller.
//!
//! Like the health check, every rule only runs on what the car really
//! answered; whatever could not be checked is listed separately.
//!
//! Texts are kept as [`Message`]: the German and English pattern with its
//! arguments, rendered in the active language when shown. The patterns are
//! looked up in the catalog like every other core text, so the result is
//! translated into all languages, numbers included.

use caruspex_i18n::{Language, Text, l, language};

use crate::common::{Level, TrafficLight, format_number};
use crate::readiness::{MonitorState, monitor_name};

// --- Texts -------------------------------------------------------------------

/// One argument of a [`Message`].
#[derive(Clone, Debug, PartialEq)]
pub enum Arg {
    /// A number with this many decimals, written the way the language does.
    Number(f64, i32),
    /// A text that is the same in every language (a VIN, a fault code) or
    /// already in the active language.
    Plain(String),
    /// A fixed name in both languages (a readiness monitor).
    Name(Text),
    /// Several messages, joined with a comma.
    List(Vec<Message>),
}

/// A text with the placeholders %1 to %9, rendered in the active language.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Message {
    pub de: &'static str,
    pub en: &'static str,
    pub args: Vec<Arg>,
}

/// A message from its German and English pattern.
pub fn text(de: &'static str, en: &'static str) -> Message {
    Message {
        de,
        en,
        args: Vec::new(),
    }
}

/// A message that only carries a ready text (the health check summary).
pub fn raw(value: impl Into<String>) -> Message {
    Message {
        de: "",
        en: "",
        args: vec![Arg::Plain(value.into())],
    }
}

impl Message {
    pub fn num(mut self, value: f64, decimals: i32) -> Self {
        self.args.push(Arg::Number(value, decimals));
        self
    }

    pub fn plain(mut self, value: impl Into<String>) -> Self {
        self.args.push(Arg::Plain(value.into()));
        self
    }

    pub fn name(mut self, value: Text) -> Self {
        self.args.push(Arg::Name(value));
        self
    }

    pub fn list(mut self, items: Vec<Message>) -> Self {
        self.args.push(Arg::List(items));
        self
    }

    pub fn is_empty(&self) -> bool {
        self.en.is_empty() && self.args.is_empty()
    }

    /// The text in the active language.
    pub fn text(&self) -> String {
        let rendered: Vec<String> = self.args.iter().map(render_arg).collect();
        if self.en.is_empty() {
            return rendered.join(" ");
        }
        fill(l(self.de, self.en), &rendered)
    }
}

fn render_arg(arg: &Arg) -> String {
    match arg {
        Arg::Number(value, decimals) => {
            format_number(*value, *decimals, language() == Language::German)
        }
        Arg::Plain(s) => s.clone(),
        Arg::Name(t) => t.get().to_string(),
        Arg::List(items) => items
            .iter()
            .map(Message::text)
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// Fills %1 to %9 in one pass, like QString::arg.
fn fill(pattern: &str, values: &[String]) -> String {
    let mut out = String::with_capacity(pattern.len() + 16);
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%'
            && let Some(&d) = chars.peek()
            && let Some(n) = d.to_digit(10)
            && n >= 1
            && (n as usize) <= values.len()
        {
            chars.next();
            out.push_str(&values[n as usize - 1]);
        } else {
            out.push(c);
        }
    }
    out
}

// --- Input -------------------------------------------------------------------

/// What the VIN says, decoded and matched against the catalog by the caller
/// (caruspex-vehicles decode_vin and match_vin).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct VinFacts {
    /// Normalised, as the control unit reported it.
    pub vin: String,
    /// 17 characters, none of I, O, Q.
    pub valid: bool,
    /// The ninth character is a check digit (North America, China).
    pub check_digit_applies: bool,
    pub check_digit_ok: bool,
    /// Empty if the maker code is unknown.
    pub brand: String,
    /// 0 = unknown.
    pub model_year: i32,
    /// Display name of the catalog generation, "Audi A2 (8Z)"; empty if none.
    pub vehicle: String,
    /// Production years of that generation; `year_to` 0 = still built.
    pub year_from: i32,
    pub year_to: i32,
}

/// The odometer as one control unit stores it.
#[derive(Clone, Debug, PartialEq)]
pub struct MileageReading {
    /// Where it comes from, in the active language ("Engine (OBD, PID A6)").
    pub source: String,
    pub km: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FaultKind {
    /// Confirmed (mode 03).
    #[default]
    Stored,
    /// Seen once, not confirmed yet (mode 07).
    Pending,
    /// Survives clearing until the car has seen the repair (mode 0A).
    Permanent,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct UsedCarFault {
    pub code: String,
    /// Meaning in the active language, may be empty.
    pub text: String,
    pub kind: FaultKind,
    /// How serious (from the fault database).
    pub level: Level,
}

/// How the car is driven, from the catalog and from PID 51.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Drive {
    #[default]
    Unknown,
    Combustion,
    Hybrid,
    Electric,
}

impl Drive {
    /// Has a traction battery.
    pub fn electric(self) -> bool {
        matches!(self, Drive::Hybrid | Drive::Electric)
    }
}

/// Drive from the fuel type of PID 51 (SAE J1979 coding); Unknown for codes
/// that say nothing about it.
pub fn drive_from_fuel_type(code: i32) -> Drive {
    match code {
        8 => Drive::Electric,
        // bifuel with electricity, hybrids of every kind
        0x0F..=0x16 => Drive::Hybrid,
        1..=7 | 9..=0x0E | 0x17 => Drive::Combustion,
        _ => Drive::Unknown,
    }
}

/// The traction battery, read from a documented pack of manufacturer values.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct BatteryFacts {
    /// Title of the pack in the active language.
    pub pack: String,
    /// The battery control unit answered.
    pub read: bool,
    /// Why not, if it did not.
    pub error: String,
    /// State of health in %.
    pub soh: Option<f64>,
    /// State of charge in %.
    pub soc: Option<f64>,
    /// Highest minus lowest cell voltage in V.
    pub cell_spread_v: Option<f64>,
}

/// The result of the short health check that runs along.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct HealthSummary {
    pub score: i32,
    pub grade: TrafficLight,
    /// In the active language.
    pub summary: String,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct UsedCarInput {
    /// The VIN of the engine control unit (mode 09); `None` if it gave none.
    pub car_vin: Option<VinFacts>,
    /// The VIN from the registration papers as typed; empty if not entered.
    pub papers_vin: String,
    /// PID A6, the odometer of the engine control unit.
    pub odometer_km: Option<f64>,
    /// The odometer of other control units (documented identifiers only).
    pub other_mileage: Vec<MileageReading>,
    /// PID 30.
    pub warm_ups: Option<f64>,
    /// PID 31.
    pub distance_since_clear_km: Option<f64>,
    /// PID 4E.
    pub minutes_since_clear: Option<f64>,
    /// PID 21.
    pub mil_distance_km: Option<f64>,
    pub mil_on: Option<bool>,
    /// Supported monitors only.
    pub readiness: Vec<MonitorState>,
    pub readiness_read: bool,
    pub faults: Vec<UsedCarFault>,
    pub faults_read: bool,
    pub health: Option<HealthSummary>,
    pub drive: Drive,
    /// `None`: no documented pack for this model.
    pub battery: Option<BatteryFacts>,
    /// For the mileage per year.
    pub current_year: i32,
}

// --- Output ------------------------------------------------------------------

/// The part of the check a finding belongs to, in the order of the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Section {
    #[default]
    Identity,
    Mileage,
    Clearing,
    Readiness,
    Faults,
    Health,
    Battery,
}

impl Section {
    pub const ALL: [Section; 7] = [
        Section::Identity,
        Section::Mileage,
        Section::Clearing,
        Section::Readiness,
        Section::Faults,
        Section::Health,
        Section::Battery,
    ];

    /// Stable key for the interface ("identity", "mileage", ...).
    pub fn key(self) -> &'static str {
        match self {
            Section::Identity => "identity",
            Section::Mileage => "mileage",
            Section::Clearing => "clearing",
            Section::Readiness => "readiness",
            Section::Faults => "faults",
            Section::Health => "health",
            Section::Battery => "battery",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    /// Stable rule id, e.g. "mileage_mismatch".
    pub id: &'static str,
    pub section: Section,
    pub level: Level,
    pub title: Message,
    pub explanation: Message,
    /// What to ask the seller; empty if nothing.
    pub ask: Message,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NotChecked {
    pub section: Section,
    pub title: Message,
    pub reason: Message,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UsedCarReport {
    pub light: TrafficLight,
    pub headline: Message,
    pub summary: Message,
    /// In the order of [`Section`].
    pub items: Vec<Item>,
    pub not_checked: Vec<NotChecked>,
    /// The odometer readings that were compared, engine first.
    pub mileage: Vec<MileageReading>,
}

impl UsedCarReport {
    pub fn find(&self, id: &str) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }

    /// The questions for the seller, the most serious first.
    pub fn questions(&self) -> Vec<&Message> {
        let mut asked: Vec<&Item> = self.items.iter().filter(|i| !i.ask.is_empty()).collect();
        asked.sort_by_key(|a| std::cmp::Reverse(a.level));
        asked.into_iter().map(|i| &i.ask).collect()
    }
}

/// Thresholds of the rules.
pub mod limits {
    /// Fewer warm-ups than this since clearing: cleared recently.
    pub const RECENT_WARM_UPS: f64 = 5.0;
    /// Less distance since clearing: cleared recently.
    pub const RECENT_DISTANCE_KM: f64 = 50.0;
    /// Less time since clearing: cleared recently.
    pub const RECENT_MINUTES: f64 = 180.0;
    /// Control units store the odometer at different moments and in steps;
    /// this much difference is normal.
    pub const MILEAGE_TOLERANCE_KM: f64 = 100.0;
    /// Or this share of the reading, whichever is larger.
    pub const MILEAGE_TOLERANCE_SHARE: f64 = 0.01;
    /// Below this per year (cars of three years and more): remarkably little.
    pub const LOW_KM_PER_YEAR: f64 = 3000.0;
    /// Above this per year: a lot.
    pub const HIGH_KM_PER_YEAR: f64 = 40000.0;
    /// State of health in %.
    pub const SOH_WORN: f64 = 80.0;
    /// Many makers guarantee this much for eight years.
    pub const SOH_WARRANTY: f64 = 70.0;
    /// V between the highest and the lowest cell.
    pub const CELL_SPREAD_V: f64 = 0.08;
}

// --- Rules -------------------------------------------------------------------

struct Ctx {
    items: Vec<Item>,
    not_checked: Vec<NotChecked>,
}

impl Ctx {
    fn add(
        &mut self,
        id: &'static str,
        section: Section,
        level: Level,
        title: Message,
        explanation: Message,
        ask: Message,
    ) {
        self.items.push(Item {
            id,
            section,
            level,
            title,
            explanation,
            ask,
        });
    }

    fn skip(&mut self, section: Section, title: Message, reason: Message) {
        self.not_checked.push(NotChecked {
            section,
            title,
            reason,
        });
    }
}

fn none() -> Message {
    Message::default()
}

/// Upper case without the separators people type along.
pub fn normalise_vin(input: &str) -> String {
    input
        .chars()
        .filter(|c| !matches!(c, ' ' | '-' | '.' | '\t' | '*' | '\r' | '\n'))
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// 17 letters and digits without I, O and Q.
pub fn vin_well_formed(vin: &str) -> bool {
    vin.len() == 17
        && vin
            .chars()
            .all(|c| c.is_ascii_alphanumeric() && !matches!(c, 'I' | 'O' | 'Q'))
}

fn check_identity(c: &mut Ctx, input: &UsedCarInput) {
    let papers = normalise_vin(&input.papers_vin);
    let ask_plate = text(
        "Stimmt die Fahrgestellnummer an der Frontscheibe und am Typschild mit den Papieren überein?",
        "Does the VIN on the windscreen and on the type plate match the papers?",
    );

    let Some(car) = &input.car_vin else {
        c.add(
            "vin_missing",
            Section::Identity,
            Level::Info,
            text(
                "Fahrgestellnummer nicht gemeldet",
                "VIN not reported",
            ),
            text(
                "Das Motorsteuergerät nennt keine Fahrgestellnummer. Bei Autos bis etwa 2005 ist das normal, bei neueren seltener. Dann bleibt nur der Vergleich von Hand.",
                "The engine control unit does not report a VIN. That is normal for cars up to about 2005 and rarer for newer ones. Then only a comparison by hand is left.",
            ),
            ask_plate,
        );
        return;
    };

    if !car.valid {
        c.add(
            "vin_invalid",
            Section::Identity,
            Level::Warning,
            text(
                "Ungültige Fahrgestellnummer im Steuergerät",
                "Invalid VIN in the control unit",
            ),
            text(
                "Das Motorsteuergerät meldet \"%1\". Das ist keine gültige Fahrgestellnummer (17 Zeichen, ohne I, O und Q). So etwas kommt vor, wenn ein Steuergerät getauscht oder neu programmiert und die Nummer nicht eingetragen wurde.",
                "The engine control unit reports \"%1\". That is not a valid VIN (17 characters, without I, O and Q). This happens when a control unit was replaced or reprogrammed and the number was not written in.",
            )
            .plain(car.vin.clone()),
            text(
                "Wurde das Motorsteuergerät getauscht oder neu programmiert, und gibt es dafür eine Rechnung?",
                "Was the engine control unit replaced or reprogrammed, and is there an invoice for it?",
            ),
        );
    } else if car.check_digit_applies && !car.check_digit_ok {
        c.add(
            "vin_check_digit",
            Section::Identity,
            Level::Warning,
            text(
                "Prüfziffer der Fahrgestellnummer stimmt nicht",
                "VIN check digit does not match",
            ),
            text(
                "Bei %1 muss die neunte Stelle eine Prüfziffer sein, die zu den anderen Zeichen passt. Hier passt sie nicht. Das kann ein Lesefehler sein, aber auch eine veränderte Nummer.",
                "For %1 the ninth character must be a check digit that fits the other characters. Here it does not. That can be a reading error, but also an altered number.",
            )
            .plain(car.vin.clone()),
            ask_plate.clone(),
        );
    }

    // The car itself: maker, model year and generation from the catalog.
    if !car.vehicle.is_empty() {
        let years = if car.year_to > 0 {
            text("%1 bis %2", "%1 to %2")
                .num(f64::from(car.year_from), 0)
                .num(f64::from(car.year_to), 0)
        } else {
            text("seit %1", "since %1").num(f64::from(car.year_from), 0)
        };
        let explanation = if car.model_year > 0 {
            text(
                "Laut Fahrgestellnummer ein %1, Modelljahr %2 (die Baureihe wurde %3 gebaut). Passt das zu Anzeige und Papieren?",
                "According to the VIN a %1, model year %2 (the series was built %3). Does that match the advert and the papers?",
            )
            .plain(car.vehicle.clone())
            .num(f64::from(car.model_year), 0)
            .list(vec![years])
        } else {
            text(
                "Laut Fahrgestellnummer ein %1 (die Baureihe wurde %2 gebaut). Passt das zu Anzeige und Papieren?",
                "According to the VIN a %1 (the series was built %2). Does that match the advert and the papers?",
            )
            .plain(car.vehicle.clone())
            .list(vec![years])
        };
        c.add(
            "vehicle",
            Section::Identity,
            Level::Good,
            text("Erkannt: %1", "Identified: %1").plain(car.vehicle.clone()),
            explanation,
            none(),
        );
    } else if !car.brand.is_empty() {
        let explanation = if car.model_year > 0 {
            text(
                "Die Fahrgestellnummer gehört zu %1, Modelljahr %2. Das genaue Modell steht nicht im Katalog; es muss zu Anzeige und Papieren passen.",
                "The VIN belongs to %1, model year %2. The exact model is not in the catalog; it has to match the advert and the papers.",
            )
            .plain(car.brand.clone())
            .num(f64::from(car.model_year), 0)
        } else {
            text(
                "Die Fahrgestellnummer gehört zu %1. Das genaue Modell steht nicht im Katalog; es muss zu Anzeige und Papieren passen.",
                "The VIN belongs to %1. The exact model is not in the catalog; it has to match the advert and the papers.",
            )
            .plain(car.brand.clone())
        };
        c.add(
            "vehicle",
            Section::Identity,
            Level::Good,
            text("Hersteller: %1", "Maker: %1").plain(car.brand.clone()),
            explanation,
            none(),
        );
    } else if car.valid {
        c.add(
            "vehicle_unknown",
            Section::Identity,
            Level::Info,
            text("Hersteller nicht erkannt", "Maker not recognised"),
            text(
                "Den Herstellercode %1 kennt Caruspex nicht. Das spricht nicht gegen das Auto, nur lässt sich Modell und Baujahr nicht aus der Nummer prüfen.",
                "Caruspex does not know the maker code %1. That says nothing against the car, only model and year cannot be checked from the number.",
            )
            .plain(car.vin.chars().take(3).collect::<String>()),
            none(),
        );
    }

    // Against the papers.
    if papers.is_empty() {
        c.skip(
            Section::Identity,
            text(
                "Fahrgestellnummer der Papiere",
                "VIN from the papers",
            ),
            text(
                "Nicht eingegeben. Für den Vergleich die Nummer aus der Zulassungsbescheinigung Teil I (Feld E) eintragen.",
                "Not entered. For the comparison, type in the number from the registration certificate.",
            ),
        );
    } else if !vin_well_formed(&papers) {
        c.add(
            "papers_vin_invalid",
            Section::Identity,
            Level::Info,
            text(
                "Eingegebene Nummer bitte prüfen",
                "Please check the number entered",
            ),
            text(
                "\"%1\" ist keine gültige Fahrgestellnummer (17 Zeichen, ohne I, O und Q). Vermutlich ein Tippfehler; bitte noch einmal von den Papieren abschreiben.",
                "\"%1\" is not a valid VIN (17 characters, without I, O and Q). Probably a typing error; please copy it from the papers again.",
            )
            .plain(papers),
            none(),
        );
    } else if car.valid && papers == car.vin {
        c.add(
            "vin_matches_papers",
            Section::Identity,
            Level::Good,
            text(
                "Fahrgestellnummer passt zu den Papieren",
                "VIN matches the papers",
            ),
            text(
                "Im Motorsteuergerät und in den Papieren steht dieselbe Nummer: %1.",
                "The engine control unit and the papers carry the same number: %1.",
            )
            .plain(papers),
            ask_plate,
        );
    } else {
        c.add(
            "vin_differs_from_papers",
            Section::Identity,
            Level::Critical,
            text(
                "Fahrgestellnummer weicht von den Papieren ab",
                "VIN differs from the papers",
            ),
            text(
                "Im Motorsteuergerät steht %1, in den Papieren %2. Entweder stammt das Steuergerät aus einem anderen Auto, oder das Auto ist nicht das aus den Papieren. Das vor dem Kauf klären, im Zweifel bei der Zulassungsstelle.",
                "The engine control unit says %1, the papers say %2. Either the control unit comes from another car, or the car is not the one in the papers. Clear this up before buying, if in doubt with the registration office.",
            )
            .plain(car.vin.clone())
            .plain(papers),
            text(
                "Warum steht im Motorsteuergerät eine andere Fahrgestellnummer als in den Papieren?",
                "Why does the engine control unit carry a different VIN than the papers?",
            ),
        );
    }
}

fn mileage_tolerance(km: f64) -> f64 {
    (km * limits::MILEAGE_TOLERANCE_SHARE).max(limits::MILEAGE_TOLERANCE_KM)
}

fn ask_records() -> Message {
    text(
        "Gibt es Serviceheft, Rechnungen und Prüfberichte, die den Kilometerstand belegen?",
        "Are there a service book, invoices and inspection reports that back up the mileage?",
    )
}

fn check_mileage(c: &mut Ctx, input: &UsedCarInput, readings: &[MileageReading]) {
    if readings.is_empty() {
        c.add(
            "mileage_unknown",
            Section::Mileage,
            Level::Info,
            text(
                "Kilometerstand nicht auslesbar",
                "Mileage cannot be read",
            ),
            text(
                "Das Auto meldet seinen Kilometerstand nicht über OBD (PID A6 gibt es erst bei neueren Autos), und für dieses Modell sind keine weiteren Kennungen belegt. Den Tachostand mit Serviceheft, Prüfberichten und Rechnungen vergleichen.",
                "The car does not report its mileage over OBD (PID A6 only exists on newer cars), and no further identifiers are documented for this model. Compare the odometer with the service book, inspection reports and invoices.",
            ),
            ask_records(),
        );
        return;
    }

    let highest = readings.iter().map(|r| r.km).fold(f64::MIN, f64::max);
    let lowest = readings.iter().map(|r| r.km).fold(f64::MAX, f64::min);
    let list: Vec<Message> = readings
        .iter()
        .map(|r| {
            text("%1: %2 km", "%1: %2 km")
                .plain(r.source.clone())
                .num(r.km, 0)
        })
        .collect();

    if readings.len() == 1 {
        c.add(
            "mileage_single",
            Section::Mileage,
            Level::Info,
            text("Kilometerstand: %1 km", "Mileage: %1 km").num(highest, 0),
            text(
                "Nur eine Quelle: %1. Für dieses Modell sind keine weiteren Steuergeräte mit belegter Kennung bekannt, ein Vergleich ist also nicht möglich. Der Wert muss zum Tacho passen.",
                "Only one source: %1. No further control units with a documented identifier are known for this model, so no comparison is possible. The value must match the dashboard.",
            )
            .list(list),
            ask_records(),
        );
    } else if highest - lowest > mileage_tolerance(highest) {
        c.add(
            "mileage_mismatch",
            Section::Mileage,
            Level::Critical,
            text(
                "Kilometerstände passen nicht zusammen",
                "Mileage readings do not match",
            ),
            text(
                "Die Steuergeräte speichern verschiedene Werte: %1. Der Unterschied beträgt %2 km. Ein wenig ist normal, weil nicht jedes Gerät gleich oft speichert. So viel deutet aber darauf hin, dass der Tacho verstellt oder ein Steuergerät aus einem anderen Auto eingebaut wurde.",
                "The control units store different values: %1. The difference is %2 km. A little is normal, because not every unit stores equally often. This much points to an altered odometer or a control unit from another car.",
            )
            .list(list)
            .num(highest - lowest, 0),
            text(
                "Warum zeigen die Steuergeräte verschiedene Kilometerstände? Wurde der Tacho oder ein Steuergerät getauscht, und gibt es dafür einen Beleg?",
                "Why do the control units show different mileages? Was the instrument cluster or a control unit replaced, and is there proof of it?",
            ),
        );
    } else {
        c.add(
            "mileage_consistent",
            Section::Mileage,
            Level::Good,
            text(
                "Kilometerstände passen zusammen",
                "Mileage readings agree",
            ),
            text(
                "Alle Quellen stimmen überein: %1. Kleine Unterschiede sind normal, weil nicht jedes Gerät gleich oft speichert.",
                "All sources agree: %1. Small differences are normal, because not every unit stores equally often.",
            )
            .list(list),
            ask_records(),
        );
    }

    // More driven since clearing than the odometer shows: the engine
    // control unit has counted kilometres that the odometer does not have.
    if let Some(since) = input.distance_since_clear_km
        && since > highest + mileage_tolerance(highest)
    {
        c.add(
            "mileage_below_clear_distance",
            Section::Mileage,
            Level::Warning,
            text(
                "Kilometerstand kleiner als die Strecke seit dem Löschen",
                "Mileage lower than the distance since clearing",
            ),
            text(
                "Seit dem letzten Löschen hat das Motorsteuergerät %1 km gezählt, der Kilometerstand zeigt aber nur %2 km. Das geht nur, wenn der Tacho zurückgestellt oder ein Steuergerät getauscht wurde.",
                "Since the last clearing the engine control unit has counted %1 km, but the mileage shows only %2 km. That is only possible if the odometer was turned back or a control unit was replaced.",
            )
            .num(since, 0)
            .num(highest, 0),
            text(
                "Wurde der Tacho oder das Motorsteuergerät getauscht?",
                "Was the instrument cluster or the engine control unit replaced?",
            ),
        );
    }

    // Kilometres per year, if the age is known.
    let model_year = input.car_vin.as_ref().map_or(0, |v| v.model_year);
    if model_year > 0 && input.current_year >= model_year {
        let age = f64::from(input.current_year - model_year) + 0.5;
        let per_year = highest / age;
        if age >= 3.0 && per_year < limits::LOW_KM_PER_YEAR {
            c.add(
                "mileage_low_for_age",
                Section::Mileage,
                Level::Info,
                text(
                    "Auffallend wenig gefahren",
                    "Remarkably low mileage",
                ),
                text(
                    "Im Schnitt etwa %1 km pro Jahr. Das gibt es, etwa bei Zweitwagen, ist aber auch das, was ein zurückgedrehter Tacho zeigt.",
                    "On average about %1 km per year. That happens, for a second car for instance, but it is also what a turned back odometer shows.",
                )
                .num((per_year / 100.0).round() * 100.0, 0),
                ask_records(),
            );
        } else if per_year > limits::HIGH_KM_PER_YEAR {
            c.add(
                "mileage_high_for_age",
                Section::Mileage,
                Level::Info,
                text("Viel gefahren", "High mileage"),
                text(
                    "Im Schnitt etwa %1 km pro Jahr, meist Langstrecke. Das schont den Motor, aber Verschleißteile wie Kupplung, Bremsen und Fahrwerk sind entsprechend beansprucht.",
                    "On average about %1 km per year, mostly long distances. That is easy on the engine, but wear parts such as clutch, brakes and suspension have worked accordingly.",
                )
                .num((per_year / 100.0).round() * 100.0, 0),
                text(
                    "Wurden die Wartungen nach Plan gemacht, und wann wurden Kupplung, Bremsen und Zahnriemen zuletzt erneuert?",
                    "Was the car serviced to plan, and when were the clutch, brakes and timing belt last renewed?",
                ),
            );
        } else {
            c.add(
                "mileage_fits_age",
                Section::Mileage,
                Level::Good,
                text("Kilometerstand passt zum Alter", "Mileage fits the age"),
                text(
                    "Im Schnitt etwa %1 km pro Jahr, ein üblicher Wert.",
                    "On average about %1 km per year, a usual figure.",
                )
                .num((per_year / 100.0).round() * 100.0, 0),
                none(),
            );
        }
    }
}

fn check_clearing(c: &mut Ctx, input: &UsedCarInput) {
    let mut facts = Vec::new();
    if let Some(km) = input.distance_since_clear_km {
        facts.push(text("%1 km", "%1 km").num(km, 0));
    }
    if let Some(n) = input.warm_ups {
        facts.push(text("%1 Warmläufe", "%1 warm-ups").num(n, 0));
    }
    if let Some(minutes) = input.minutes_since_clear {
        facts.push(text("%1 Stunden Betrieb", "%1 hours of operation").num(minutes / 60.0, 0));
    }
    if facts.is_empty() {
        c.skip(
            Section::Clearing,
            text("Zuletzt gelöscht", "Last cleared"),
            text(
                "Das Auto meldet nicht, wann der Fehlerspeicher zuletzt gelöscht wurde (PID 30, 31 und 4E).",
                "The car does not report when the fault memory was last cleared (PID 30, 31 and 4E).",
            ),
        );
    } else {
        let recent = input.warm_ups.is_some_and(|w| w < limits::RECENT_WARM_UPS)
            || input
                .distance_since_clear_km
                .is_some_and(|d| d < limits::RECENT_DISTANCE_KM)
            || input
                .minutes_since_clear
                .is_some_and(|m| m < limits::RECENT_MINUTES);
        if recent {
            c.add(
                "cleared_recently",
                Section::Clearing,
                Level::Warning,
                text(
                    "Fehlerspeicher vor Kurzem gelöscht",
                    "Fault memory cleared recently",
                ),
                text(
                    "Seit dem letzten Löschen: %1. Wer den Fehlerspeicher löscht, schaltet auch die Warnlampe aus und setzt die Selbsttests des Autos zurück. Fehler, die erst nach einigen Fahrten wiederkommen, sind bei der Besichtigung dann noch nicht zu sehen. Kurz vor einem Verkauf kann das heißen, dass ein Fehler versteckt werden soll. Es gibt auch harmlose Gründe, etwa eine Reparatur, eine Inspektion oder eine abgeklemmte Batterie.",
                    "Since the last clearing: %1. Clearing the fault memory also switches off the warning lamp and resets the car's self-tests. Faults that only come back after a few drives are then not visible at the viewing. Shortly before a sale this can mean that a fault is being hidden. There are harmless reasons too, such as a repair, a service or a disconnected battery.",
                )
                .list(facts),
                text(
                    "Wann und warum wurde der Fehlerspeicher zuletzt gelöscht? Gibt es dazu eine Rechnung?",
                    "When and why was the fault memory last cleared? Is there an invoice for it?",
                ),
            );
            let permanent: Vec<&UsedCarFault> = input
                .faults
                .iter()
                .filter(|f| f.kind == FaultKind::Permanent)
                .collect();
            if !permanent.is_empty() {
                c.add(
                    "cleared_but_permanent",
                    Section::Clearing,
                    Level::Critical,
                    text(
                        "Gelöscht, aber nicht repariert",
                        "Cleared, but not repaired",
                    ),
                    text(
                        "Trotz des Löschens meldet das Auto dauerhafte Fehler: %1. Diese Codes lassen sich nicht löschen; sie verschwinden erst, wenn das Auto selbst festgestellt hat, dass der Fehler behoben ist. Der Fehler war also vor dem Löschen da und ist es vermutlich noch.",
                        "Despite the clearing the car reports permanent faults: %1. These codes cannot be cleared; they only go away once the car itself has seen that the fault is fixed. So the fault was there before the clearing and probably still is.",
                    )
                    .list(fault_list(&permanent)),
                    text(
                        "Warum wurde %1 gelöscht, ohne den Fehler zu reparieren?",
                        "Why was %1 cleared without repairing the fault?",
                    )
                    .plain(codes(&permanent)),
                );
            }
        } else {
            c.add(
                "not_cleared_recently",
                Section::Clearing,
                Level::Good,
                text("Nicht frisch gelöscht", "Not freshly cleared"),
                text(
                    "Seit dem letzten Löschen: %1. Kein Hinweis darauf, dass der Fehlerspeicher kurz vor der Besichtigung gelöscht wurde.",
                    "Since the last clearing: %1. No sign that the fault memory was cleared shortly before the viewing.",
                )
                .list(facts),
                none(),
            );
        }
    }

    if let Some(km) = input.mil_distance_km
        && km > 0.0
    {
        c.add(
            "mil_distance",
            Section::Clearing,
            Level::Info,
            text(
                "Mit Warnlampe gefahren: %1 km",
                "Driven with the warning lamp on: %1 km",
            )
            .num(km, 0),
            text(
                "Seit dem letzten Löschen ist das Auto %1 km mit leuchtender Motorkontrollleuchte gefahren. Je länger ein Fehler ignoriert wird, desto eher entstehen Folgeschäden.",
                "Since the last clearing the car has been driven %1 km with the check engine light on. The longer a fault is ignored, the more likely it causes further damage.",
            )
            .num(km, 0),
            text(
                "Warum wurde mit leuchtender Motorkontrollleuchte weitergefahren?",
                "Why was the car driven on with the check engine light on?",
            ),
        );
    }
}

fn check_readiness(c: &mut Ctx, input: &UsedCarInput) {
    if !input.readiness_read {
        c.skip(
            Section::Readiness,
            text("Bereitschaftstests", "Readiness monitors"),
            text(
                "Das Auto hat den Stand der Selbsttests (PID 01) nicht gemeldet.",
                "The car did not report the state of its self-tests (PID 01).",
            ),
        );
        return;
    }
    let supported: Vec<&MonitorState> = input.readiness.iter().filter(|m| m.supported).collect();
    if supported.is_empty() {
        if input.drive == Drive::Electric {
            c.add(
                "readiness_none_electric",
                Section::Readiness,
                Level::Good,
                text(
                    "Keine Abgas-Selbsttests",
                    "No emission self-tests",
                ),
                text(
                    "Ein reines E-Auto hat keinen Auspuff und damit keine Bereitschaftstests. Das ist richtig so.",
                    "A fully electric car has no exhaust and so no readiness monitors. That is as it should be.",
                ),
                none(),
            );
        } else {
            c.add(
                "readiness_none",
                Section::Readiness,
                Level::Info,
                text(
                    "Keine Bereitschaftstests gemeldet",
                    "No readiness monitors reported",
                ),
                text(
                    "Das Auto meldet keine Selbsttests. Manche Steuergeräte tun das nicht; dann lässt sich nicht sehen, ob die Tests seit dem letzten Löschen gelaufen sind.",
                    "The car reports no self-tests. Some control units do not; then it cannot be seen whether the tests have run since the last clearing.",
                ),
                none(),
            );
        }
        return;
    }
    let open: Vec<&MonitorState> = supported.iter().copied().filter(|m| !m.complete).collect();
    if open.is_empty() {
        c.add(
            "readiness_complete",
            Section::Readiness,
            Level::Good,
            text(
                "Alle Bereitschaftstests abgeschlossen",
                "All readiness monitors complete",
            ),
            text(
                "Das Auto hat alle %1 Selbsttests seit dem letzten Löschen beendet. Das spricht dagegen, dass kurz vor der Besichtigung gelöscht wurde.",
                "The car has finished all %1 self-tests since the last clearing. That speaks against a clearing shortly before the viewing.",
            )
            .num(supported.len() as f64, 0),
            none(),
        );
        return;
    }
    let names: Vec<Message> = open
        .iter()
        .map(|m| Message::default().name(monitor_name(m.monitor)))
        .collect();
    let level = if open.len() == 1 {
        Level::Info
    } else {
        Level::Warning
    };
    c.add(
        "readiness_open",
        Section::Readiness,
        level,
        text(
            "Offene Bereitschaftstests: %1 von %2",
            "Readiness monitors not complete: %1 of %2",
        )
        .num(open.len() as f64, 0)
        .num(supported.len() as f64, 0),
        text(
            "Noch nicht fertig: %1. Nach dem Löschen des Fehlerspeichers oder dem Abklemmen der Batterie braucht das Auto einige Fahrten, bis diese Selbsttests wieder durch sind. Einzelne offene Tests sind normal, viele passen zu einem frischen Löschen.",
            "Not complete yet: %1. After the fault memory was cleared or the battery disconnected, the car needs a few drives until these self-tests are done again. Single open tests are normal, many fit a fresh clearing.",
        )
        .list(names),
        if open.len() == 1 {
            none()
        } else {
            text(
                "Wurde in letzter Zeit die Batterie abgeklemmt oder der Fehlerspeicher gelöscht?",
                "Was the battery disconnected or the fault memory cleared recently?",
            )
        },
    );
}

fn fault_list(faults: &[&UsedCarFault]) -> Vec<Message> {
    faults
        .iter()
        .map(|f| {
            if f.text.is_empty() {
                raw(f.code.clone())
            } else {
                text("%1 (%2)", "%1 (%2)")
                    .plain(f.code.clone())
                    .plain(f.text.clone())
            }
        })
        .collect()
}

fn codes(faults: &[&UsedCarFault]) -> String {
    faults
        .iter()
        .map(|f| f.code.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn check_faults(c: &mut Ctx, input: &UsedCarInput) {
    if input.mil_on == Some(true) {
        c.add(
            "mil_on",
            Section::Faults,
            Level::Warning,
            text(
                "Motorkontrollleuchte an",
                "Check engine light on",
            ),
            text(
                "Das Steuergerät meldet die Motorkontrollleuchte als eingeschaltet. Bei laufendem Motor muss sie aus sein; leuchtet sie, liegt ein bestätigter Fehler an der Abgasreinigung oder am Motor vor.",
                "The control unit reports the check engine light as on. With the engine running it must be off; if it is lit, there is a confirmed fault in the emission control or the engine.",
            ),
            text(
                "Seit wann leuchtet die Motorkontrollleuchte, und was hat die Werkstatt dazu gesagt?",
                "Since when has the check engine light been on, and what did the workshop say about it?",
            ),
        );
    }
    if !input.faults_read {
        c.skip(
            Section::Faults,
            text("Fehlerspeicher", "Fault memory"),
            text(
                "Der Fehlerspeicher ließ sich nicht lesen.",
                "The fault memory could not be read.",
            ),
        );
        return;
    }
    let of = |kind: FaultKind| -> Vec<&UsedCarFault> {
        input.faults.iter().filter(|f| f.kind == kind).collect()
    };
    let stored = of(FaultKind::Stored);
    let pending = of(FaultKind::Pending);
    let permanent = of(FaultKind::Permanent);

    if stored.is_empty() && pending.is_empty() && permanent.is_empty() {
        c.add(
            "no_faults",
            Section::Faults,
            Level::Good,
            text("Kein Fehler gespeichert", "No faults stored"),
            text(
                "Weder gespeicherte noch vorläufige noch dauerhafte Fehler im Motorsteuergerät.",
                "No stored, pending or permanent faults in the engine control unit.",
            ),
            none(),
        );
        return;
    }
    if !stored.is_empty() {
        let level = stored
            .iter()
            .map(|f| f.level)
            .max()
            .unwrap_or(Level::Warning)
            .max(Level::Info);
        c.add(
            "faults_stored",
            Section::Faults,
            level,
            text("Gespeicherte Fehler: %1", "Stored faults: %1").num(stored.len() as f64, 0),
            text(
                "%1. Diese Fehler hat das Steuergerät bestätigt. Was sie bedeuten und was eine Reparatur ungefähr kostet, steht im Fehlerspeicher bei jedem Code.",
                "%1. The control unit has confirmed these faults. What they mean and roughly what a repair costs is shown with each code in the fault memory.",
            )
            .list(fault_list(&stored)),
            text(
                "Was ist die Ursache für %1, und was würde die Reparatur kosten?",
                "What is the cause of %1, and what would the repair cost?",
            )
            .plain(codes(&stored)),
        );
    }
    if !pending.is_empty() {
        c.add(
            "faults_pending",
            Section::Faults,
            Level::Info,
            text("Vorläufige Fehler: %1", "Pending faults: %1").num(pending.len() as f64, 0),
            text(
                "%1. Diese Fehler hat das Steuergerät einmal gesehen, aber noch nicht bestätigt. Sie können verschwinden oder zu gespeicherten Fehlern werden.",
                "%1. The control unit has seen these faults once but not confirmed them yet. They can disappear or become stored faults.",
            )
            .list(fault_list(&pending)),
            none(),
        );
    }
    if !permanent.is_empty() {
        c.add(
            "faults_permanent",
            Section::Faults,
            Level::Warning,
            text("Dauerhafte Fehler: %1", "Permanent faults: %1").num(permanent.len() as f64, 0),
            text(
                "%1. Dauerhafte Fehler lassen sich nicht löschen. Sie verschwinden erst, wenn das Auto selbst festgestellt hat, dass der Fehler behoben ist.",
                "%1. Permanent faults cannot be cleared. They only go away once the car itself has seen that the fault is fixed.",
            )
            .list(fault_list(&permanent)),
            text(
                "Wurde der Fehler hinter %1 repariert, und gibt es dafür eine Rechnung?",
                "Was the fault behind %1 repaired, and is there an invoice for it?",
            )
            .plain(codes(&permanent)),
        );
    }
}

fn check_health(c: &mut Ctx, input: &UsedCarInput) {
    let Some(health) = &input.health else {
        return;
    };
    let level = match health.grade {
        TrafficLight::Green => Level::Good,
        TrafficLight::Yellow => Level::Info,
        TrafficLight::Red => Level::Warning,
    };
    c.add(
        "health",
        Section::Health,
        level,
        text(
            "Gesundheitscheck: %1 von 100 Punkten",
            "Health check: %1 of 100 points",
        )
        .num(f64::from(health.score), 0),
        raw(health.summary.clone()),
        none(),
    );
}

fn ask_battery_certificate() -> Message {
    text(
        "Gibt es ein Prüfprotokoll zum Akkuzustand (SOH) vom Hersteller oder einer Werkstatt, und gilt noch eine Akkugarantie?",
        "Is there a battery health (SOH) report from the maker or a workshop, and is a battery warranty still valid?",
    )
}

fn check_battery(c: &mut Ctx, input: &UsedCarInput) {
    if !input.drive.electric() {
        return;
    }
    let Some(battery) = &input.battery else {
        c.add(
            "battery_unknown",
            Section::Battery,
            Level::Info,
            text(
                "Akkuzustand nicht auslesbar",
                "Battery health cannot be read",
            ),
            text(
                "Für dieses Modell sind keine Akkuwerte belegt. Beim E-Auto und Hybrid ist der Akku das teuerste Bauteil; seinen Zustand am besten mit einem Prüfprotokoll belegen lassen.",
                "No battery values are documented for this model. On an electric car or hybrid the battery is the most expensive part; best have its health confirmed by a test report.",
            ),
            ask_battery_certificate(),
        );
        return;
    };
    if !battery.read {
        c.add(
            "battery_unread",
            Section::Battery,
            Level::Info,
            text(
                "Akku hat nicht geantwortet",
                "Battery did not answer",
            ),
            text(
                "Das Batteriesteuergerät (%1) hat nicht geantwortet: %2. Ist die Zündung an, und passt das Paket zum Auto?",
                "The battery control unit (%1) did not answer: %2. Is the ignition on, and does the pack fit the car?",
            )
            .plain(battery.pack.clone())
            .plain(battery.error.clone()),
            ask_battery_certificate(),
        );
        return;
    }
    if let Some(soh) = battery.soh {
        let (id, level, title) = if soh < limits::SOH_WARRANTY {
            (
                "battery_worn_out",
                Level::Critical,
                text("Akku stark gealtert: %1 %", "Battery badly aged: %1 %"),
            )
        } else if soh < limits::SOH_WORN {
            (
                "battery_worn",
                Level::Warning,
                text("Akku gealtert: %1 %", "Battery aged: %1 %"),
            )
        } else {
            (
                "battery_good",
                Level::Good,
                text(
                    "Akku in gutem Zustand: %1 %",
                    "Battery in good health: %1 %",
                ),
            )
        };
        c.add(
            id,
            Section::Battery,
            level,
            title.num(soh, 0),
            text(
                "Der Akku hat noch %1 % seiner ursprünglichen Kapazität; die Reichweite ist entsprechend kleiner als neu. Viele Hersteller garantieren 70 % für acht Jahre oder 160 000 km.",
                "The battery still has %1 % of its original capacity; the range is smaller than new by the same share. Many makers guarantee 70 % for eight years or 160 000 km.",
            )
            .num(soh, 0),
            if level == Level::Good {
                none()
            } else {
                ask_battery_certificate()
            },
        );
    } else {
        c.add(
            "battery_soh_unknown",
            Section::Battery,
            Level::Info,
            text(
                "Akkugesundheit nicht gemeldet",
                "Battery health not reported",
            ),
            text(
                "Das Batteriesteuergerät hat geantwortet, aber keinen Wert für die Akkugesundheit (SOH) geliefert.",
                "The battery control unit answered, but gave no value for the battery health (SOH).",
            ),
            ask_battery_certificate(),
        );
    }
    if let Some(spread) = battery.cell_spread_v
        && spread > limits::CELL_SPREAD_V
    {
        c.add(
            "battery_cells_uneven",
            Section::Battery,
            Level::Warning,
            text(
                "Akkuzellen ungleich geladen",
                "Battery cells unevenly charged",
            ),
            text(
                "Zwischen der höchsten und der tiefsten Zellspannung liegen %1 mV. Bei einem gesunden Akku sind es wenige hundertstel Volt. Einmal langsam voll laden kann helfen; bleibt es so, ist eine Zelle schwach.",
                "There are %1 mV between the highest and the lowest cell voltage. A healthy battery stays within a few hundredths of a volt. One slow full charge can help; if it stays like this, a cell is weak.",
            )
            .num(spread * 1000.0, 0),
            text(
                "Wurde am Akku schon etwas repariert oder getauscht?",
                "Has anything on the battery been repaired or replaced?",
            ),
        );
    }
}

/// The odometer readings to compare: the engine (PID A6) first, then the
/// other control units.
pub fn mileage_readings(input: &UsedCarInput) -> Vec<MileageReading> {
    let mut out = Vec::new();
    if let Some(km) = input.odometer_km {
        out.push(MileageReading {
            source: l("Motorsteuergerät (OBD)", "Engine control unit (OBD)").to_string(),
            km,
        });
    }
    out.extend(input.other_mileage.iter().cloned());
    out
}

/// Runs every rule and sums up.
pub fn evaluate_used_car(input: &UsedCarInput) -> UsedCarReport {
    let mut c = Ctx {
        items: Vec::new(),
        not_checked: Vec::new(),
    };
    let readings = mileage_readings(input);
    check_identity(&mut c, input);
    check_mileage(&mut c, input, &readings);
    check_clearing(&mut c, input);
    check_readiness(&mut c, input);
    check_faults(&mut c, input);
    check_health(&mut c, input);
    check_battery(&mut c, input);
    // Stable: the rules add in section order already, this keeps it so.
    c.items.sort_by_key(|i| i.section);

    let count = |level: Level| c.items.iter().filter(|i| i.level == level).count() as f64;
    let critical = count(Level::Critical);
    let warnings = count(Level::Warning);
    let (light, headline, summary) = if critical > 0.0 {
        (
            TrafficLight::Red,
            text(
                "Vorsicht: vor dem Kauf klären",
                "Careful: clear up before buying",
            ),
            text(
                "Punkte, die gegen einen Kauf ohne weitere Prüfung sprechen: %1. Die Fragen unten zuerst mit dem Verkäufer klären und das Auto in einer Werkstatt ansehen lassen.",
                "Points that speak against buying without a closer look: %1. Clear up the questions below with the seller first and have the car looked at by a workshop.",
            )
            .num(critical, 0),
        )
    } else if warnings > 0.0 {
        (
            TrafficLight::Yellow,
            text("Mit Fragen kaufen", "Buy with questions"),
            text(
                "Punkte, die Sie mit dem Verkäufer besprechen sollten: %1. Nichts davon schließt einen Kauf aus, aber es gehört in den Preis oder in den Kaufvertrag.",
                "Points to talk through with the seller: %1. None of them rules out a purchase, but they belong in the price or in the contract.",
            )
            .num(warnings, 0),
        )
    } else {
        (
            TrafficLight::Green,
            text("Unauffällig", "Nothing unusual"),
            text(
                "Die Steuergeräte zeigen nichts, was gegen das Auto spricht. Eine Probefahrt und ein Blick in die Wartungsnachweise ersetzt das nicht.",
                "The control units show nothing that speaks against the car. That does not replace a test drive and a look at the service records.",
            ),
        )
    };
    UsedCarReport {
        light,
        headline,
        summary,
        items: c.items,
        not_checked: c.not_checked,
        mileage: readings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::readiness::Monitor;

    fn monitor(monitor: Monitor, complete: bool) -> MonitorState {
        MonitorState {
            monitor,
            supported: true,
            complete,
        }
    }

    fn fault(code: &str, kind: FaultKind, level: Level) -> UsedCarFault {
        UsedCarFault {
            code: code.into(),
            text: String::new(),
            kind,
            level,
        }
    }

    const DEMO_VIN: &str = "YCXDE15C7MC048217";

    /// The demo car as the worker reads it before anything was cleared:
    /// PID A6 48 213 km, the cluster and the ABS agree, 1842 km and 36
    /// warm-ups since the last clearing, evaporative system and EGR open,
    /// P0171 and P0420 stored, P0300 pending, the lamp on.
    fn demo() -> UsedCarInput {
        UsedCarInput {
            car_vin: Some(VinFacts {
                vin: DEMO_VIN.into(),
                valid: true,
                model_year: 2021,
                vehicle: "Caruspex Demo".into(),
                year_from: 2019,
                year_to: 0,
                ..VinFacts::default()
            }),
            papers_vin: String::new(),
            odometer_km: Some(48213.4),
            other_mileage: vec![
                MileageReading {
                    source: "Instrument cluster".into(),
                    km: 48213.0,
                },
                MileageReading {
                    source: "ABS".into(),
                    km: 48210.0,
                },
            ],
            warm_ups: Some(36.0),
            distance_since_clear_km: Some(1842.0),
            minutes_since_clear: Some(2580.0),
            mil_distance_km: Some(312.0),
            mil_on: Some(true),
            readiness: vec![
                monitor(Monitor::Misfire, true),
                monitor(Monitor::FuelSystem, true),
                monitor(Monitor::Components, true),
                monitor(Monitor::Catalyst, true),
                monitor(Monitor::Evap, false),
                monitor(Monitor::OxygenSensor, true),
                monitor(Monitor::OxygenSensorHeater, true),
                monitor(Monitor::Egr, false),
            ],
            readiness_read: true,
            faults: vec![
                fault("P0171", FaultKind::Stored, Level::Warning),
                fault("P0420", FaultKind::Stored, Level::Warning),
                fault("P0300", FaultKind::Pending, Level::Info),
            ],
            faults_read: true,
            health: Some(HealthSummary {
                score: 58,
                grade: TrafficLight::Yellow,
                summary: "Needs care".into(),
            }),
            drive: Drive::Combustion,
            battery: None,
            current_year: 2026,
        }
    }

    fn ids(report: &UsedCarReport) -> Vec<&'static str> {
        report.items.iter().map(|i| i.id).collect()
    }

    #[test]
    fn the_demo_car_needs_questions_but_nothing_is_hidden() {
        let report = evaluate_used_car(&demo());
        assert_eq!(report.light, TrafficLight::Yellow, "{:?}", ids(&report));
        assert_eq!(report.find("vehicle").unwrap().level, Level::Good);
        assert_eq!(
            report.find("mileage_consistent").unwrap().level,
            Level::Good
        );
        assert_eq!(report.find("mileage_fits_age").unwrap().level, Level::Good);
        assert_eq!(
            report.find("not_cleared_recently").unwrap().level,
            Level::Good
        );
        assert_eq!(report.find("readiness_open").unwrap().level, Level::Warning);
        assert_eq!(report.find("faults_stored").unwrap().level, Level::Warning);
        assert_eq!(report.find("faults_pending").unwrap().level, Level::Info);
        assert_eq!(report.find("mil_on").unwrap().level, Level::Warning);
        assert_eq!(report.find("mil_distance").unwrap().level, Level::Info);
        assert_eq!(report.find("health").unwrap().level, Level::Info);
        assert!(report.find("cleared_recently").is_none());
        // A petrol car: no battery section.
        assert!(report.items.iter().all(|i| i.section != Section::Battery));
        // The papers were not entered: listed as not checked.
        assert_eq!(report.not_checked.len(), 1);
        assert_eq!(report.not_checked[0].section, Section::Identity);
        // Three readings, engine first.
        assert_eq!(report.mileage.len(), 3);
        assert_eq!(report.mileage[0].km, 48213.4);
        // Questions: the most serious first, every one has a text.
        let questions = report.questions();
        assert!(questions.len() >= 4);
        assert!(questions.iter().all(|q| !q.text().is_empty()));
        // Items come in section order.
        let sections: Vec<Section> = report.items.iter().map(|i| i.section).collect();
        let mut sorted = sections.clone();
        sorted.sort();
        assert_eq!(sections, sorted);
    }

    #[test]
    fn a_fresh_clearing_is_a_warning_and_permanent_codes_make_it_red() {
        let mut input = demo();
        input.warm_ups = Some(1.0);
        input.distance_since_clear_km = Some(3.0);
        input.minutes_since_clear = Some(12.0);
        input.mil_distance_km = Some(0.0);
        input.mil_on = Some(false);
        input.faults = vec![
            fault("P0171", FaultKind::Permanent, Level::Warning),
            fault("P0420", FaultKind::Permanent, Level::Warning),
        ];
        input.readiness = input
            .readiness
            .iter()
            .map(|m| MonitorState {
                complete: matches!(
                    m.monitor,
                    Monitor::Misfire | Monitor::FuelSystem | Monitor::Components
                ),
                ..*m
            })
            .collect();
        let report = evaluate_used_car(&input);
        assert_eq!(
            report.find("cleared_recently").unwrap().level,
            Level::Warning
        );
        assert_eq!(
            report.find("cleared_but_permanent").unwrap().level,
            Level::Critical
        );
        assert_eq!(
            report.find("faults_permanent").unwrap().level,
            Level::Warning
        );
        assert!(report.find("mil_distance").is_none());
        assert!(report.find("mil_on").is_none());
        let open = report.find("readiness_open").unwrap();
        assert_eq!(open.level, Level::Warning);
        assert_eq!(open.title.args[0], Arg::Number(5.0, 0));
        assert_eq!(report.light, TrafficLight::Red);
        // The critical question comes first.
        let first = report.questions()[0];
        assert_eq!(first, &report.find("cleared_but_permanent").unwrap().ask);

        // Each counter alone is enough for "recently".
        for (w, d, m) in [
            (Some(2.0), None, None),
            (None, Some(20.0), None),
            (None, None, Some(30.0)),
        ] {
            let mut one = demo();
            one.warm_ups = w;
            one.distance_since_clear_km = d;
            one.minutes_since_clear = m;
            let report = evaluate_used_car(&one);
            assert!(
                report.find("cleared_recently").is_some(),
                "{w:?} {d:?} {m:?}"
            );
        }
    }

    #[test]
    fn mileage_that_does_not_match_is_red() {
        let mut input = demo();
        input.other_mileage[0].km = 131_870.0;
        let report = evaluate_used_car(&input);
        let item = report.find("mileage_mismatch").unwrap();
        assert_eq!(item.level, Level::Critical);
        assert!(!item.ask.is_empty());
        assert_eq!(report.light, TrafficLight::Red);

        // A few kilometres apart is normal: units store at different times.
        let mut close = demo();
        close.other_mileage[1].km = 48_150.0;
        assert!(
            evaluate_used_car(&close)
                .find("mileage_consistent")
                .is_some()
        );
        // The tolerance grows with the reading: 1 % of 200 000 km.
        let mut high = demo();
        high.odometer_km = Some(200_000.0);
        high.other_mileage = vec![MileageReading {
            source: "Cluster".into(),
            km: 198_500.0,
        }];
        assert!(
            evaluate_used_car(&high)
                .find("mileage_consistent")
                .is_some()
        );

        // More counted since clearing than the odometer shows.
        let mut turned_back = demo();
        turned_back.distance_since_clear_km = Some(60_000.0);
        let report = evaluate_used_car(&turned_back);
        assert_eq!(
            report.find("mileage_below_clear_distance").unwrap().level,
            Level::Warning
        );
    }

    #[test]
    fn mileage_per_year_and_missing_sources() {
        let mut low = demo();
        low.odometer_km = Some(9000.0);
        low.other_mileage.clear();
        let report = evaluate_used_car(&low);
        assert!(report.find("mileage_single").is_some());
        assert_eq!(
            report.find("mileage_low_for_age").unwrap().level,
            Level::Info
        );

        let mut high = demo();
        high.odometer_km = Some(300_000.0);
        high.other_mileage.clear();
        assert!(
            evaluate_used_car(&high)
                .find("mileage_high_for_age")
                .is_some()
        );

        let mut none = demo();
        none.odometer_km = None;
        none.other_mileage.clear();
        let report = evaluate_used_car(&none);
        let item = report.find("mileage_unknown").unwrap();
        assert_eq!(item.level, Level::Info);
        assert!(!item.ask.is_empty());
        assert!(report.mileage.is_empty());
        assert!(report.find("mileage_fits_age").is_none());
    }

    #[test]
    fn vin_against_the_papers() {
        let mut same = demo();
        same.papers_vin = "ycx de15c7 mc048217".into();
        let report = evaluate_used_car(&same);
        assert_eq!(
            report.find("vin_matches_papers").unwrap().level,
            Level::Good
        );
        assert!(report.not_checked.is_empty());

        let mut other = demo();
        other.papers_vin = "WVWZZZ1JZ3W000001".into();
        let report = evaluate_used_car(&other);
        assert_eq!(
            report.find("vin_differs_from_papers").unwrap().level,
            Level::Critical
        );
        assert_eq!(report.light, TrafficLight::Red);

        let mut typo = demo();
        typo.papers_vin = "YCXDE15C7MC04821".into();
        assert_eq!(
            evaluate_used_car(&typo)
                .find("papers_vin_invalid")
                .unwrap()
                .level,
            Level::Info
        );

        let mut silent = demo();
        silent.car_vin = None;
        let report = evaluate_used_car(&silent);
        assert!(report.find("vin_missing").is_some());
        assert!(report.find("vehicle").is_none());
        assert!(report.find("mileage_fits_age").is_none());

        let mut garbage = demo();
        garbage.car_vin = Some(VinFacts {
            vin: "000000000000".into(),
            ..VinFacts::default()
        });
        assert_eq!(
            evaluate_used_car(&garbage)
                .find("vin_invalid")
                .unwrap()
                .level,
            Level::Warning
        );

        let mut digit = demo();
        if let Some(v) = digit.car_vin.as_mut() {
            v.check_digit_applies = true;
            v.check_digit_ok = false;
        }
        assert!(evaluate_used_car(&digit).find("vin_check_digit").is_some());
    }

    #[test]
    fn no_readiness_and_unread_parts() {
        let mut none = demo();
        none.readiness.clear();
        let report = evaluate_used_car(&none);
        assert_eq!(report.find("readiness_none").unwrap().level, Level::Info);

        let mut unread = demo();
        unread.readiness_read = false;
        unread.faults_read = false;
        unread.warm_ups = None;
        unread.distance_since_clear_km = None;
        unread.minutes_since_clear = None;
        unread.health = None;
        let report = evaluate_used_car(&unread);
        let skipped: Vec<Section> = report.not_checked.iter().map(|n| n.section).collect();
        assert!(skipped.contains(&Section::Readiness));
        assert!(skipped.contains(&Section::Faults));
        assert!(skipped.contains(&Section::Clearing));
        assert!(report.find("health").is_none());
        assert!(report.find("no_faults").is_none());

        let mut complete = demo();
        for m in &mut complete.readiness {
            m.complete = true;
        }
        complete.faults.clear();
        complete.mil_on = Some(false);
        complete.mil_distance_km = Some(0.0);
        complete.health = Some(HealthSummary {
            score: 96,
            grade: TrafficLight::Green,
            summary: String::new(),
        });
        let report = evaluate_used_car(&complete);
        assert!(report.find("readiness_complete").is_some());
        assert!(report.find("no_faults").is_some());
        assert_eq!(report.light, TrafficLight::Green);
    }

    #[test]
    fn electric_cars_get_the_battery_checked() {
        let mut ev = demo();
        ev.drive = Drive::Electric;
        ev.readiness.clear();
        ev.faults.clear();
        ev.mil_on = Some(false);
        ev.mil_distance_km = None;
        ev.health = None;
        ev.battery = Some(BatteryFacts {
            pack: "Hyundai Kona Electric".into(),
            read: true,
            soh: Some(91.0),
            soc: Some(64.0),
            cell_spread_v: Some(0.02),
            ..BatteryFacts::default()
        });
        let report = evaluate_used_car(&ev);
        assert_eq!(
            report.find("readiness_none_electric").unwrap().level,
            Level::Good
        );
        assert_eq!(report.find("battery_good").unwrap().level, Level::Good);
        assert_eq!(report.light, TrafficLight::Green);

        let mut worn = ev.clone();
        if let Some(b) = worn.battery.as_mut() {
            b.soh = Some(76.0);
            b.cell_spread_v = Some(0.15);
        }
        let report = evaluate_used_car(&worn);
        assert_eq!(report.find("battery_worn").unwrap().level, Level::Warning);
        assert!(report.find("battery_cells_uneven").is_some());
        assert_eq!(report.light, TrafficLight::Yellow);

        let mut dead = ev.clone();
        if let Some(b) = dead.battery.as_mut() {
            b.soh = Some(64.0);
        }
        assert_eq!(evaluate_used_car(&dead).light, TrafficLight::Red);

        let mut silent = ev.clone();
        silent.battery = Some(BatteryFacts {
            pack: "Renault Zoe".into(),
            read: false,
            error: "NO DATA".into(),
            ..BatteryFacts::default()
        });
        assert!(evaluate_used_car(&silent).find("battery_unread").is_some());

        let mut unknown = ev.clone();
        unknown.drive = Drive::Hybrid;
        unknown.battery = None;
        let report = evaluate_used_car(&unknown);
        assert!(!report.find("battery_unknown").unwrap().ask.is_empty());

        assert_eq!(drive_from_fuel_type(8), Drive::Electric);
        assert_eq!(drive_from_fuel_type(0x11), Drive::Hybrid);
        assert_eq!(drive_from_fuel_type(1), Drive::Combustion);
        assert_eq!(drive_from_fuel_type(0), Drive::Unknown);
    }

    #[test]
    fn messages_fill_their_arguments() {
        let m = text("%1 von %2", "%1 of %2").num(3.0, 0).num(8.0, 0);
        assert_eq!(fill(m.en, &["3".into(), "8".into()]), "3 of 8");
        // Arguments that contain a placeholder themselves stay as they are.
        assert_eq!(fill("%1 / %2", &["%2".into(), "x".into()]), "%2 / x");
        assert_eq!(raw("Fertig").args.len(), 1);
        assert!(Message::default().is_empty());
        assert!(!raw("x").is_empty());
        assert_eq!(normalise_vin(" wvw-zzz1jz 3w000001 "), "WVWZZZ1JZ3W000001");
        assert!(vin_well_formed("WVWZZZ1JZ3W000001"));
        assert!(!vin_well_formed("WVWZZZ1JZ3W00000O"));
    }
}
