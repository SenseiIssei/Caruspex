//! Used vehicle check for trucks and buses over SAE J1939: for someone
//! standing next to a truck they might buy.
//!
//! The caller asks every unit what J1939 offers on request and hands it in:
//! the VIN of every unit that keeps one, make, model, serial and software of
//! every unit, the odometer of every source (dashboard SPN 245, the fine
//! odometer SPN 917 of engine and tachograph), engine hours, idle hours,
//! total fuel, DM21 (distance with the lamp on and since the last clearing),
//! the active and previous faults per unit and a short health check. This
//! module compares them and answers with the same report as the car check:
//! a traffic light, the findings in plain words and the questions for the
//! seller.
//!
//! The texts say plainly what a dealer could have changed (an instrument
//! cluster can be swapped or reprogrammed, a fault memory cleared) and what
//! no control unit tells (the tachograph's own memory, maker specific
//! counters, the state of frame, brakes and tyres).

use crate::common::{Level, TrafficLight};
use crate::truck_health::{TruckFault, TruckUnit};
use crate::used_car::{
    HealthSummary, Item, Message, MileageReading, NotChecked, Section, UsedCarReport,
    normalise_vin, raw, text, vin_well_formed,
};

/// The SPNs of the odometers that are compared.
pub const SPN_DASHBOARD_DISTANCE: u32 = 245;
pub const SPN_FINE_DISTANCE: u32 = 917;

/// Thresholds of the rules.
pub mod limits {
    /// Units store the odometer at different moments, and engine and
    /// tachograph may be calibrated a little apart (tyre size); this much
    /// difference is normal.
    pub const MILEAGE_TOLERANCE_KM: f64 = 200.0;
    /// Or this share of the reading, whichever is larger.
    pub const MILEAGE_TOLERANCE_SHARE: f64 = 0.005;
    /// km/h over all engine hours. Above this the hours are too few for
    /// the distance: no truck averages more, stops and idling included.
    pub const SPEED_TOO_HIGH: f64 = 75.0;
    /// Below this the distance is very low for the hours: stationary work
    /// (crane, refuse, power take-off) or a turned back odometer.
    pub const SPEED_TOO_LOW: f64 = 8.0;
    /// Share of the engine hours at idle.
    pub const IDLE_HIGH: f64 = 0.40;
    pub const IDLE_NOTABLE: f64 = 0.25;
    /// l/100 km over the whole life outside this range does not fit a
    /// heavy truck or bus.
    pub const FUEL_LOW: f64 = 15.0;
    pub const FUEL_HIGH: f64 = 70.0;
    /// Less distance or time since clearing: cleared recently.
    pub const RECENT_KM: f64 = 50.0;
    pub const RECENT_MINUTES: f64 = 180.0;
    /// A fault that went active this often keeps coming back.
    pub const RECURRING: u8 = 10;
    /// This many previous faults in one unit are worth a question.
    pub const MANY_PREVIOUS: usize = 5;
}

/// The VIN one unit keeps.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct UnitVin {
    /// "Engine (00)", in the active language.
    pub label: String,
    pub vin: String,
}

/// How one unit identifies itself (component and software identification).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct UnitIdentity {
    pub label: String,
    pub make: String,
    pub model: String,
    pub serial: String,
    pub software: Vec<String>,
}

/// One odometer reading: which unit, which SPN, how far.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct OdometerReading {
    pub label: String,
    pub spn: u32,
    pub km: f64,
}

/// DM21 of the engine; every field only if the unit sent it.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct LampHistory {
    /// SPN 3069.
    pub mil_km: Option<f64>,
    /// SPN 3294.
    pub since_clear_km: Option<f64>,
    /// SPN 3295.
    pub mil_minutes: Option<f64>,
    /// SPN 3296.
    pub since_clear_minutes: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TruckUsedInput {
    /// Every unit that answered the VIN request, address order.
    pub vins: Vec<UnitVin>,
    /// The VIN from the registration papers as typed; empty if not entered.
    pub papers_vin: String,
    /// Every unit heard on the bus, with what it says about itself; make
    /// and model stay empty for a unit that did not answer.
    pub units: Vec<UnitIdentity>,
    pub odometers: Vec<OdometerReading>,
    /// SPN 247.
    pub engine_hours: Option<f64>,
    /// SPN 235.
    pub idle_hours: Option<f64>,
    /// SPN 250, l.
    pub fuel_total_l: Option<f64>,
    /// SPN 236, l.
    pub idle_fuel_l: Option<f64>,
    pub lamp_history: LampHistory,
    /// Active (DM1) and previous (DM2) faults with the lamps per unit.
    pub faults: Vec<TruckUnit>,
    pub faults_read: bool,
    pub previous_read: bool,
    pub health: Option<HealthSummary>,
}

/// The numbers the page shows as tiles.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct TruckUsedFacts {
    /// The highest odometer reading.
    pub odometer_km: Option<f64>,
    pub engine_hours: Option<f64>,
    /// 0..1.
    pub idle_share: Option<f64>,
    /// km per engine hour, idle included.
    pub average_speed: Option<f64>,
    /// l/100 km over the whole life.
    pub fuel_per_100km: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TruckUsedReport {
    pub report: UsedCarReport,
    pub facts: TruckUsedFacts,
}

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

fn ask_records() -> Message {
    text(
        "Gibt es Wartungsheft, Werkstattrechnungen und Prüfberichte (HU, SP), die Kilometer und Betriebsstunden belegen?",
        "Are there service records, workshop invoices and inspection reports that back up the kilometres and engine hours?",
    )
}

// --- Identity ------------------------------------------------------------------

fn check_identity(c: &mut Ctx, input: &TruckUsedInput) {
    let papers = normalise_vin(&input.papers_vin);
    let ask_plate = text(
        "Stimmt die Fahrgestellnummer am Rahmen und am Typschild mit den Papieren überein?",
        "Does the VIN on the frame and on the type plate match the papers?",
    );
    let vins: Vec<&UnitVin> = input.vins.iter().filter(|v| !v.vin.is_empty()).collect();
    let list = |vins: &[&UnitVin]| -> Vec<Message> {
        vins.iter()
            .map(|v| {
                text("%1: %2", "%1: %2")
                    .plain(v.label.clone())
                    .plain(v.vin.clone())
            })
            .collect()
    };
    if vins.is_empty() {
        c.add(
            "vin_missing",
            Section::Identity,
            Level::Info,
            text("Fahrgestellnummer nicht gemeldet", "VIN not reported"),
            text(
                "Kein Steuergerät nennt eine Fahrgestellnummer. Bei vielen älteren Lkw ist das normal; dann bleibt nur der Vergleich von Hand mit dem Rahmen.",
                "No control unit reports a VIN. That is normal on many older trucks; then only a comparison by hand with the frame is left.",
            ),
            ask_plate,
        );
        return;
    }
    let first = normalise_vin(&vins[0].vin);
    let differ = vins.iter().any(|v| normalise_vin(&v.vin) != first);
    if let Some(bad) = vins
        .iter()
        .find(|v| !vin_well_formed(&normalise_vin(&v.vin)))
    {
        c.add(
            "vin_invalid",
            Section::Identity,
            Level::Warning,
            text(
                "Ungültige Fahrgestellnummer im Steuergerät",
                "Invalid VIN in the control unit",
            ),
            text(
                "%1 meldet \"%2\". Das ist keine gültige Fahrgestellnummer (17 Zeichen, ohne I, O und Q). So etwas kommt vor, wenn ein Steuergerät getauscht und die Nummer nicht eingetragen wurde.",
                "%1 reports \"%2\". That is not a valid VIN (17 characters, without I, O and Q). This happens when a control unit was replaced and the number was not written in.",
            )
            .plain(bad.label.clone())
            .plain(bad.vin.clone()),
            text(
                "Welches Steuergerät wurde getauscht oder neu programmiert, und gibt es dafür eine Rechnung?",
                "Which control unit was replaced or reprogrammed, and is there an invoice for it?",
            ),
        );
    }
    if differ {
        c.add(
            "vins_differ",
            Section::Identity,
            Level::Critical,
            text(
                "Steuergeräte melden verschiedene Fahrgestellnummern",
                "Control units report different VINs",
            ),
            text(
                "%1. Mindestens ein Steuergerät stammt also aus einem anderen Fahrzeug oder wurde nicht richtig angelernt. Beim Kombiinstrument kann das einen anderen Kilometerstand bedeuten, beim Tachographen ist es ein Fall für die Prüfstelle.",
                "%1. So at least one control unit comes from another vehicle or was not set up properly. For the instrument cluster that can mean a different mileage, for the tachograph it is a matter for the inspection body.",
            )
            .list(list(&vins)),
            text(
                "Welches Steuergerät wurde getauscht, wann und warum? Gibt es dafür Rechnung und, beim Tachographen, eine Einbauprüfung?",
                "Which control unit was replaced, when and why? Is there an invoice and, for the tachograph, an installation inspection?",
            ),
        );
    } else {
        c.add(
            "vins_agree",
            Section::Identity,
            Level::Good,
            text(
                "Fahrgestellnummer %1",
                "VIN %1",
            )
            .plain(first.clone()),
            if vins.len() > 1 {
                text(
                    "Alle %1 Steuergeräte, die eine Fahrgestellnummer speichern, melden dieselbe: %2.",
                    "All %1 control units that store a VIN report the same one: %2.",
                )
                .num(vins.len() as f64, 0)
                .list(vins.iter().map(|v| raw(v.label.clone())).collect())
            } else {
                text(
                    "Nur %1 speichert eine Fahrgestellnummer; ein Vergleich zwischen Steuergeräten ist nicht möglich.",
                    "Only %1 stores a VIN; a comparison between control units is not possible.",
                )
                .plain(vins[0].label.clone())
            },
            none(),
        );
    }

    if papers.is_empty() {
        c.skip(
            Section::Identity,
            text("Fahrgestellnummer der Papiere", "VIN from the papers"),
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
    } else if vins.iter().all(|v| normalise_vin(&v.vin) == papers) {
        c.add(
            "vin_matches_papers",
            Section::Identity,
            Level::Good,
            text(
                "Fahrgestellnummer passt zu den Papieren",
                "VIN matches the papers",
            ),
            text(
                "Die Steuergeräte und die Papiere nennen dieselbe Nummer: %1.",
                "The control units and the papers carry the same number: %1.",
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
                "Die Papiere nennen %1, die Steuergeräte: %2. Entweder stammt ein Steuergerät aus einem anderen Fahrzeug, oder das Fahrzeug ist nicht das aus den Papieren. Das vor dem Kauf klären.",
                "The papers say %1, the control units: %2. Either a control unit comes from another vehicle, or the vehicle is not the one in the papers. Clear this up before buying.",
            )
            .plain(papers)
            .list(list(&vins)),
            text(
                "Warum melden die Steuergeräte eine andere Fahrgestellnummer als die Papiere?",
                "Why do the control units report a different VIN than the papers?",
            ),
        );
    }
}

fn check_units(c: &mut Ctx, input: &TruckUsedInput) {
    if input.units.is_empty() {
        return;
    }
    let lines: Vec<Message> = input
        .units
        .iter()
        .map(|u| {
            let mut parts: Vec<String> = [&u.make, &u.model]
                .iter()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();
            if !u.serial.is_empty() {
                parts.push(format!("S/N {}", u.serial));
            }
            if !u.software.is_empty() {
                parts.push(u.software.join(", "));
            }
            if parts.is_empty() {
                text("%1: keine Kennung", "%1: no identification").plain(u.label.clone())
            } else {
                text("%1: %2", "%1: %2")
                    .plain(u.label.clone())
                    .plain(parts.join(" "))
            }
        })
        .collect();
    let silent = input
        .units
        .iter()
        .filter(|u| u.make.is_empty() && u.model.is_empty() && u.software.is_empty())
        .count();
    c.add(
        "units",
        Section::Units,
        if silent > 0 { Level::Info } else { Level::Good },
        text("%1 Steuergeräte am Bus", "%1 control units on the bus").num(input.units.len() as f64, 0),
        text(
            "%1. Mit Rechnungen über getauschte Teile vergleichen: Seriennummer und Software ändern sich, wenn ein Steuergerät ersetzt oder neu programmiert wurde. Eine Werkstatt kann beides auch so einstellen, dass es nach Originalzustand aussieht.",
            "%1. Compare with invoices for replaced parts: serial number and software change when a control unit was replaced or reprogrammed. A workshop can also set both to look like the original.",
        )
        .list(lines),
        if silent > 0 {
            text(
                "Warum nennen manche Steuergeräte keine Kennung, wurden sie nachgerüstet oder getauscht?",
                "Why do some control units give no identification, were they added or replaced?",
            )
        } else {
            none()
        },
    );
}

// --- Mileage and usage ---------------------------------------------------------------

fn source_name(r: &OdometerReading) -> Message {
    if r.spn == SPN_DASHBOARD_DISTANCE {
        text("%1, Gesamtstrecke", "%1, total distance").plain(r.label.clone())
    } else {
        text("%1, Gesamtstrecke fein", "%1, fine total distance").plain(r.label.clone())
    }
}

fn mileage_tolerance(km: f64) -> f64 {
    (km * limits::MILEAGE_TOLERANCE_SHARE).max(limits::MILEAGE_TOLERANCE_KM)
}

fn check_mileage(c: &mut Ctx, input: &TruckUsedInput) -> Option<f64> {
    let readings = &input.odometers;
    if readings.is_empty() {
        c.add(
            "mileage_unknown",
            Section::Mileage,
            Level::Info,
            text("Kilometerstand nicht auslesbar", "Mileage cannot be read"),
            text(
                "Kein Steuergerät sendet eine Gesamtstrecke (SPN 245 oder 917). Den Tachostand mit Wartungsheft, Prüfberichten und Rechnungen vergleichen.",
                "No control unit sends a total distance (SPN 245 or 917). Compare the odometer with the service records, inspection reports and invoices.",
            ),
            ask_records(),
        );
        return None;
    }
    let highest = readings.iter().map(|r| r.km).fold(f64::MIN, f64::max);
    let lowest = readings.iter().map(|r| r.km).fold(f64::MAX, f64::min);
    let list: Vec<Message> = readings
        .iter()
        .map(|r| {
            text("%1: %2 km", "%1: %2 km")
                .list(vec![source_name(r)])
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
                "Nur eine Quelle: %1. Ein Vergleich zwischen Steuergeräten ist nicht möglich; der Wert muss zum Tacho und zu den Belegen passen.",
                "Only one source: %1. A comparison between control units is not possible; the value must match the dashboard and the records.",
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
                "Die Steuergeräte zählen verschieden: %1. Der Unterschied beträgt %2 km. Ein wenig ist normal, so viel nicht. Das Kombiinstrument lässt sich tauschen oder umprogrammieren, Motorsteuergerät und Tachograph zählen getrennt davon; meist stimmt der höhere Wert.",
                "The control units count differently: %1. The difference is %2 km. A little is normal, this much is not. The instrument cluster can be replaced or reprogrammed, the engine control unit and the tachograph count separately from it; usually the higher value is the true one.",
            )
            .list(list)
            .num(highest - lowest, 0),
            text(
                "Warum zeigen die Steuergeräte verschiedene Kilometerstände? Wurde das Kombiinstrument, das Motorsteuergerät oder der Tachograph getauscht, und gibt es dafür einen Beleg?",
                "Why do the control units show different mileages? Was the instrument cluster, the engine control unit or the tachograph replaced, and is there proof of it?",
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
                "Alle Quellen stimmen überein: %1. Ein gutes Zeichen, aber kein Beweis: wer alle Steuergeräte gleich verstellt, fällt hier nicht auf.",
                "All sources agree: %1. A good sign, but no proof: someone who alters every control unit alike does not show up here.",
            )
            .list(list),
            ask_records(),
        );
    }
    Some(highest)
}

fn check_usage(c: &mut Ctx, input: &TruckUsedInput, km: Option<f64>) -> TruckUsedFacts {
    let mut facts = TruckUsedFacts {
        odometer_km: km,
        engine_hours: input.engine_hours,
        ..TruckUsedFacts::default()
    };
    let Some(hours) = input.engine_hours.filter(|h| *h > 0.0) else {
        c.skip(
            Section::Usage,
            text("Motorbetriebsstunden", "Engine hours"),
            text(
                "Der Motor hat auf die Anfrage nach den Betriebsstunden (SPN 247) nicht geantwortet.",
                "The engine did not answer the request for its hours (SPN 247).",
            ),
        );
        return facts;
    };
    c.add(
        "engine_hours",
        Section::Usage,
        Level::Good,
        text("%1 Motorbetriebsstunden", "%1 engine hours").num(hours, 0),
        text(
            "Der Zähler steckt im Motorsteuergerät. Wird es getauscht, beginnt er neu oder bekommt einen Wert eingetragen; mit dem Wartungsheft vergleichen.",
            "The counter sits in the engine control unit. If it is replaced, it starts again or gets a value written in; compare with the service records.",
        ),
        none(),
    );

    if let Some(km) = km {
        let speed = km / hours;
        facts.average_speed = Some(speed);
        let (id, level, title, explanation, ask) = if speed > limits::SPEED_TOO_HIGH {
            (
                "hours_too_few",
                Level::Warning,
                text(
                    "Zu wenige Betriebsstunden für die Kilometer",
                    "Too few engine hours for the kilometres",
                ),
                text(
                    "Im Schnitt %1 km je Betriebsstunde. Das schafft kein Lkw, Stopps und Leerlauf eingerechnet; üblich sind 20 bis 60 km/h. Vermutlich wurde das Motorsteuergerät oder der Motor getauscht.",
                    "On average %1 km per engine hour. No truck manages that with stops and idling counted; 20 to 60 km/h is usual. Probably the engine control unit or the engine was replaced.",
                ),
                text(
                    "Wurde der Motor oder das Motorsteuergerät getauscht, und bei welchem Kilometerstand?",
                    "Was the engine or the engine control unit replaced, and at what mileage?",
                ),
            )
        } else if speed < limits::SPEED_TOO_LOW {
            (
                "km_too_few",
                Level::Warning,
                text(
                    "Wenige Kilometer für die Betriebsstunden",
                    "Few kilometres for the engine hours",
                ),
                text(
                    "Im Schnitt nur %1 km je Betriebsstunde. Das passt zu Arbeit im Stand, etwa mit Kran, Pumpe oder Müllaufbau, aber auch zu einem zurückgestellten Kilometerstand.",
                    "On average only %1 km per engine hour. That fits work while standing, such as with a crane, a pump or a refuse body, but also a turned back odometer.",
                ),
                text(
                    "Wofür wurde das Fahrzeug eingesetzt, und lief der Motor viel im Stand?",
                    "What was the vehicle used for, and did the engine run a lot while standing?",
                ),
            )
        } else {
            (
                "speed_plausible",
                Level::Good,
                text(
                    "Kilometer passen zu den Betriebsstunden",
                    "Kilometres fit the engine hours",
                ),
                text(
                    "Im Schnitt %1 km je Betriebsstunde, Stopps und Leerlauf eingerechnet. Fernverkehr liegt meist bei 45 bis 60, Verteilerverkehr bei 25 bis 40, Stadtbusse bei 15 bis 20.",
                    "On average %1 km per engine hour, stops and idling included. Long haul is mostly 45 to 60, distribution 25 to 40, city buses 15 to 20.",
                ),
                none(),
            )
        };
        c.add(
            id,
            Section::Usage,
            level,
            title,
            explanation.num(speed, 1),
            ask,
        );
    }

    if let Some(idle) = input.idle_hours.filter(|i| *i >= 0.0 && *i <= hours) {
        let share = idle / hours;
        facts.idle_share = Some(share);
        let level = if share > limits::IDLE_HIGH {
            Level::Warning
        } else if share > limits::IDLE_NOTABLE {
            Level::Info
        } else {
            Level::Good
        };
        c.add(
            "idle_share",
            Section::Usage,
            level,
            text("Leerlaufanteil %1 %", "Idle share %1 %").num(share * 100.0, 0),
            text(
                "%1 von %2 Betriebsstunden lief der Motor im Leerlauf. Leerlauf verschleißt den Motor ohne Kilometer, lässt den Partikelfilter schneller verrußen und verdünnt das Öl mit Diesel. Bis etwa ein Viertel ist üblich, Standheizung über den Motor treibt den Wert hoch.",
                "The engine idled for %1 of %2 engine hours. Idling wears the engine without kilometres, fills the particulate filter with soot faster and thins the oil with diesel. Up to about a quarter is usual; heating the cab with the engine pushes the share up.",
            )
            .num(idle, 0)
            .num(hours, 0),
            if level == Level::Good {
                none()
            } else {
                text(
                    "Warum lief der Motor so viel im Leerlauf, und wurde das Öl nach Stunden gewechselt?",
                    "Why did the engine idle so much, and was the oil changed by hours?",
                )
            },
        );
    } else {
        c.skip(
            Section::Usage,
            text("Leerlaufstunden", "Idle hours"),
            text(
                "Der Motor meldet keine Leerlaufstunden (SPN 235).",
                "The engine reports no idle hours (SPN 235).",
            ),
        );
    }

    match (input.fuel_total_l, km) {
        (Some(fuel), Some(km)) if km > 1000.0 && fuel > 0.0 => {
            let per_100 = fuel / km * 100.0;
            facts.fuel_per_100km = Some(per_100);
            let odd = !(limits::FUEL_LOW..=limits::FUEL_HIGH).contains(&per_100);
            c.add(
                "fuel_average",
                Section::Usage,
                if odd { Level::Warning } else { Level::Good },
                if odd {
                    text(
                        "Verbrauch passt nicht zum Kilometerstand: %1 l/100 km",
                        "Fuel does not fit the mileage: %1 l/100 km",
                    )
                } else {
                    text(
                        "Durchschnittsverbrauch %1 l/100 km",
                        "Average fuel use %1 l/100 km",
                    )
                }
                .num(per_100, 1),
                text(
                    "%1 Liter seit dem Bau des Motors auf %2 km. Schwere Lkw und Busse brauchen meist 25 bis 40 l/100 km. Der Zähler steckt im Motorsteuergerät; weicht der Schnitt stark ab, passen Motor und Kilometerstand nicht zusammen.",
                    "%1 litres since the engine was built over %2 km. Heavy trucks and buses mostly use 25 to 40 l/100 km. The counter sits in the engine control unit; if the average is far off, engine and mileage do not belong together.",
                )
                .num(fuel, 0)
                .num(km, 0),
                if odd {
                    text(
                        "Wurde der Motor oder das Motorsteuergerät getauscht, oder der Kilometerstand verändert?",
                        "Was the engine or the engine control unit replaced, or the mileage altered?",
                    )
                } else {
                    none()
                },
            );
        }
        _ => c.skip(
            Section::Usage,
            text("Kraftstoff gesamt", "Total fuel used"),
            text(
                "Ohne Gesamtverbrauch (SPN 250) und Kilometerstand lässt sich der Durchschnitt nicht rechnen.",
                "Without the total fuel used (SPN 250) and the mileage the average cannot be worked out.",
            ),
        ),
    }
    facts
}

// --- Clearing, lamps, faults ---------------------------------------------------------

fn check_clearing(c: &mut Ctx, input: &TruckUsedInput) {
    let h = input.lamp_history;
    let mut facts = Vec::new();
    if let Some(km) = h.since_clear_km {
        facts.push(text("%1 km", "%1 km").num(km, 0));
    }
    if let Some(minutes) = h.since_clear_minutes {
        facts.push(text("%1 Stunden Betrieb", "%1 hours of operation").num(minutes / 60.0, 0));
    }
    if facts.is_empty() {
        c.skip(
            Section::Clearing,
            text("Zuletzt gelöscht", "Last cleared"),
            text(
                "Der Lkw beantwortet DM21 nicht und meldet so nicht, wann der Fehlerspeicher zuletzt gelöscht wurde.",
                "The truck does not answer DM21, so it does not tell when the fault memory was last cleared.",
            ),
        );
    } else {
        let recent = h.since_clear_km.is_some_and(|k| k < limits::RECENT_KM)
            || h.since_clear_minutes
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
                    "Seit dem letzten Löschen: %1. Wer löscht, schaltet auch die Warnlampen aus; Fehler, die erst nach einigen Fahrten wiederkommen, sind bei der Besichtigung dann nicht zu sehen. Es gibt harmlose Gründe, etwa eine Reparatur oder eine Inspektion.",
                    "Since the last clearing: %1. Clearing also switches off the warning lamps; faults that only come back after a few drives are then not visible at the viewing. There are harmless reasons too, such as a repair or a service.",
                )
                .list(facts),
                text(
                    "Wann und warum wurde der Fehlerspeicher zuletzt gelöscht? Gibt es dazu eine Rechnung?",
                    "When and why was the fault memory last cleared? Is there an invoice for it?",
                ),
            );
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
    if let Some(km) = h.mil_km.filter(|k| *k > 0.0) {
        c.add(
            "mil_distance",
            Section::Clearing,
            Level::Info,
            text(
                "Mit Abgaslampe gefahren: %1 km",
                "Driven with the emission lamp on: %1 km",
            )
            .num(km, 0),
            text(
                "Seit dem letzten Löschen ist der Lkw %1 km mit leuchtender Abgaslampe gefahren. J1939 führt keinen Verlauf der roten und gelben Lampe; nur diese Strecke und die Zeit mit Abgaslampe bleiben gespeichert.",
                "Since the last clearing the truck has been driven %1 km with the emission lamp on. J1939 keeps no history of the red and amber lamps; only this distance and the time with the emission lamp are stored.",
            )
            .num(km, 0),
            text(
                "Warum wurde mit leuchtender Abgaslampe weitergefahren?",
                "Why was the truck driven on with the emission lamp on?",
            ),
        );
    }
}

fn fault_lines(unit: &TruckUnit, faults: &[TruckFault]) -> Message {
    let lines: Vec<Message> = faults
        .iter()
        .map(|f| {
            let base = if f.text.is_empty() {
                raw(f.code())
            } else {
                text("%1 (%2)", "%1 (%2)")
                    .plain(f.code())
                    .plain(f.text.clone())
            };
            match f.occurrences {
                Some(n) if n > 1 => text("%1, %2-mal", "%1, %2 times")
                    .list(vec![base])
                    .num(f64::from(n), 0),
                _ => base,
            }
        })
        .collect();
    text("%1: %2", "%1: %2")
        .plain(unit.label.clone())
        .list(lines)
}

fn check_faults(c: &mut Ctx, input: &TruckUsedInput) {
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
    let red: Vec<Message> = input
        .faults
        .iter()
        .filter(|u| u.lamps.red_stop)
        .map(|u| raw(u.label.clone()))
        .collect();
    if !red.is_empty() {
        c.add(
            "lamp_red_stop",
            Section::Faults,
            Level::Critical,
            text("Rote Stopplampe an", "Red stop lamp on"),
            text(
                "Die rote Stopplampe leuchtet bei: %1. Mit so einem Fehler sollte der Lkw nicht weiterfahren.",
                "The red stop lamp is lit for: %1. With such a fault the truck should not drive on.",
            )
            .list(red),
            text(
                "Was ist die Ursache der roten Stopplampe, und wer zahlt die Reparatur?",
                "What is the cause of the red stop lamp, and who pays for the repair?",
            ),
        );
    }
    let active: Vec<&TruckUnit> = input
        .faults
        .iter()
        .filter(|u| !u.active.is_empty())
        .collect();
    if active.is_empty() {
        c.add(
            "no_active_faults",
            Section::Faults,
            Level::Good,
            text("Keine aktiven Fehler", "No active faults"),
            text(
                "Kein Steuergerät meldet gerade einen aktiven Fehler.",
                "No control unit reports an active fault right now.",
            ),
            none(),
        );
    } else {
        let lit = active
            .iter()
            .any(|u| u.lamps.amber || u.lamps.mil || u.lamps.red_stop);
        let count: usize = active.iter().map(|u| u.active.len()).sum();
        c.add(
            "faults_active",
            Section::Faults,
            if lit { Level::Warning } else { Level::Info },
            text("Aktive Fehler: %1", "Active faults: %1").num(count as f64, 0),
            text(
                "%1. Aktive Fehler liegen jetzt an; sie gehen erst weg, wenn die Ursache behoben ist.",
                "%1. Active faults are present now; they only go away once the cause is fixed.",
            )
            .list(active.iter().map(|u| fault_lines(u, &u.active)).collect()),
            text(
                "Was ist die Ursache der aktiven Fehler, und was würde die Reparatur kosten?",
                "What is the cause of the active faults, and what would the repair cost?",
            ),
        );
    }

    if !input.previous_read {
        c.skip(
            Section::Faults,
            text("Frühere Fehler", "Previous faults"),
            text(
                "Kein Steuergerät hat seine früheren Fehler (DM2) genannt.",
                "No control unit gave its previous faults (DM2).",
            ),
        );
        return;
    }
    let previous: Vec<&TruckUnit> = input
        .faults
        .iter()
        .filter(|u| !u.previous.is_empty())
        .collect();
    if previous.is_empty() {
        c.add(
            "no_previous_faults",
            Section::Faults,
            Level::Good,
            text("Keine früheren Fehler", "No previous faults"),
            text(
                "Kein Steuergerät hat frühere Fehler gespeichert. Das ist gut, kann aber auch ein frisch gelöschter Speicher sein.",
                "No control unit has previous faults stored. That is good, but it can also be a freshly cleared memory.",
            ),
            none(),
        );
        return;
    }
    let count: usize = previous.iter().map(|u| u.previous.len()).sum();
    let recurring = previous.iter().any(|u| {
        u.previous.len() >= limits::MANY_PREVIOUS
            || u.previous
                .iter()
                .any(|f| f.occurrences.is_some_and(|n| n >= limits::RECURRING))
    });
    c.add(
        "faults_previous",
        Section::Faults,
        if recurring { Level::Warning } else { Level::Info },
        text("Frühere Fehler: %1", "Previous faults: %1").num(count as f64, 0),
        text(
            "%1. Diese Fehler waren aktiv und sind es nicht mehr. Kommt einer immer wieder, steckt oft ein Kabel- oder Steckerproblem dahinter.",
            "%1. These faults were active and are no longer. If one keeps coming back, a wiring or plug problem is often behind it.",
        )
        .list(previous.iter().map(|u| fault_lines(u, &u.previous)).collect()),
        if recurring {
            text(
                "Welche Fehler kamen immer wieder, und was wurde dagegen gemacht?",
                "Which faults kept coming back, and what was done about them?",
            )
        } else {
            none()
        },
    );
}

fn check_health(c: &mut Ctx, input: &TruckUsedInput) {
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

/// What no control unit tells, so the buyer knows where to look further.
fn blind_spots(c: &mut Ctx) {
    c.skip(
        Section::Mileage,
        text("Speicher des Tachographen", "Tachograph memory"),
        text(
            "Der Kilometerverlauf im Massenspeicher des Tachographen ist nur mit Unternehmens- oder Werkstattkarte lesbar, nicht über J1939.",
            "The mileage history in the tachograph's mass memory can only be read with a company or workshop card, not over J1939.",
        ),
    );
    c.skip(
        Section::Faults,
        text("Herstellerspeicher", "Maker specific memory"),
        text(
            "Herstellereigene Zähler (Bremsbeläge, Kupplung, Getriebe, Retarder) und Fehlerspeicher außerhalb von J1939 liest diese Prüfung nicht.",
            "Maker specific counters (brake pads, clutch, gearbox, retarder) and fault memories outside J1939 are not read by this check.",
        ),
    );
    c.skip(
        Section::Health,
        text("Mechanischer Zustand", "Mechanical condition"),
        text(
            "Rahmen, Achsen, Bremsen, Reifen, Aufbau und Unfallschäden sieht kein Steuergerät.",
            "No control unit sees the frame, axles, brakes, tyres, body or accident damage.",
        ),
    );
}

/// Runs every rule and sums up.
pub fn evaluate_used_truck(input: &TruckUsedInput) -> TruckUsedReport {
    let mut c = Ctx {
        items: Vec::new(),
        not_checked: Vec::new(),
    };
    check_identity(&mut c, input);
    check_units(&mut c, input);
    let km = check_mileage(&mut c, input);
    let facts = check_usage(&mut c, input, km);
    check_clearing(&mut c, input);
    check_faults(&mut c, input);
    check_health(&mut c, input);
    blind_spots(&mut c);
    c.items.sort_by_key(|i| i.section);
    c.not_checked.sort_by_key(|n| n.section);

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
                "Punkte, die gegen einen Kauf ohne weitere Prüfung sprechen: %1. Die Fragen zuerst mit dem Verkäufer klären und das Fahrzeug in einer Lkw-Werkstatt ansehen lassen.",
                "Points that speak against buying without a closer look: %1. Clear up the questions with the seller first and have the vehicle looked at by a truck workshop.",
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
                "Die Steuergeräte zeigen nichts, was gegen das Fahrzeug spricht. Probefahrt, Wartungsnachweise und ein Blick unter das Fahrzeug ersetzt das nicht.",
                "The control units show nothing that speaks against the vehicle. That does not replace a test drive, the service records and a look underneath.",
            ),
        )
    };
    let mileage = input
        .odometers
        .iter()
        .map(|r| MileageReading {
            source: source_name(r).text(),
            km: r.km,
        })
        .collect();
    TruckUsedReport {
        report: UsedCarReport {
            light,
            headline,
            summary,
            items: c.items,
            not_checked: c.not_checked,
            mileage,
        },
        facts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::truck_health::TruckLamps;
    use caruspex_i18n::{Language, with_language};

    const VIN: &str = "YCXTR42H4NC051739";

    fn unit(address: u8, label: &str) -> TruckUnit {
        TruckUnit {
            address,
            label: label.into(),
            ..TruckUnit::default()
        }
    }

    /// The demo truck as the worker reads it: three VINs alike, three
    /// odometers within a few km, 12 346 hours with a third at idle, DM21
    /// 8 412 km since clearing, two active and two previous faults.
    fn demo() -> TruckUsedInput {
        let mut engine = unit(0x00, "Engine (00)");
        engine.lamps = TruckLamps {
            mil: true,
            amber: true,
            ..TruckLamps::default()
        };
        engine.active = vec![
            TruckFault {
                spn: 105,
                fmi: 3,
                occurrences: Some(7),
                ..TruckFault::default()
            },
            TruckFault {
                spn: 111,
                fmi: 17,
                occurrences: Some(2),
                ..TruckFault::default()
            },
        ];
        engine.previous = vec![TruckFault {
            spn: 1761,
            fmi: 18,
            occurrences: Some(2),
            ..TruckFault::default()
        }];
        let mut brakes = unit(0x0B, "Brakes (0B)");
        brakes.previous = vec![TruckFault {
            spn: 904,
            fmi: 2,
            occurrences: Some(4),
            ..TruckFault::default()
        }];
        TruckUsedInput {
            vins: ["Engine (00)", "Instrument cluster (17)", "Tachograph (EE)"]
                .iter()
                .map(|l| UnitVin {
                    label: l.to_string(),
                    vin: VIN.into(),
                })
                .collect(),
            units: vec![UnitIdentity {
                label: "Engine (00)".into(),
                make: "CARUSPEX".into(),
                model: "CX-D13".into(),
                serial: "E7712093".into(),
                software: vec!["ECM 4.2.1".into()],
            }],
            odometers: vec![
                OdometerReading {
                    label: "Engine (00)".into(),
                    spn: SPN_FINE_DISTANCE,
                    km: 512_358.4,
                },
                OdometerReading {
                    label: "Instrument cluster (17)".into(),
                    spn: SPN_DASHBOARD_DISTANCE,
                    km: 512_352.6,
                },
                OdometerReading {
                    label: "Tachograph (EE)".into(),
                    spn: SPN_FINE_DISTANCE,
                    km: 512_352.1,
                },
            ],
            engine_hours: Some(12_345.6),
            idle_hours: Some(3_950.25),
            fuel_total_l: Some(187_654.5),
            idle_fuel_l: Some(9_875.5),
            lamp_history: LampHistory {
                mil_km: Some(37.0),
                since_clear_km: Some(8_412.0),
                mil_minutes: Some(55.0),
                since_clear_minutes: Some(9_870.0),
            },
            faults: vec![engine, unit(0x03, "Transmission (03)"), brakes],
            faults_read: true,
            previous_read: true,
            ..TruckUsedInput::default()
        }
    }

    fn level(report: &TruckUsedReport, id: &str) -> Level {
        report
            .report
            .find(id)
            .unwrap_or_else(|| panic!("no item {id}"))
            .level
    }

    #[test]
    fn the_demo_truck_is_bought_with_questions() {
        let r = evaluate_used_truck(&demo());
        assert_eq!(r.report.light, TrafficLight::Yellow);
        assert_eq!(level(&r, "vins_agree"), Level::Good);
        assert_eq!(level(&r, "units"), Level::Good);
        assert_eq!(level(&r, "mileage_consistent"), Level::Good);
        assert_eq!(level(&r, "speed_plausible"), Level::Good);
        assert_eq!(level(&r, "idle_share"), Level::Info);
        assert_eq!(level(&r, "fuel_average"), Level::Good);
        assert_eq!(level(&r, "not_cleared_recently"), Level::Good);
        assert_eq!(level(&r, "mil_distance"), Level::Info);
        assert_eq!(level(&r, "faults_active"), Level::Warning);
        assert_eq!(level(&r, "faults_previous"), Level::Info);
        let speed = r.facts.average_speed.unwrap();
        assert!((speed - 41.5).abs() < 0.1, "{speed}");
        assert!((r.facts.idle_share.unwrap() - 0.32).abs() < 0.01);
        assert!((r.facts.fuel_per_100km.unwrap() - 36.6).abs() < 0.1);
        assert_eq!(r.report.mileage.len(), 3);
        // Sections in page order, and the blind spots are named.
        let sections: Vec<Section> = r.report.items.iter().map(|i| i.section).collect();
        assert!(sections.windows(2).all(|w| w[0] <= w[1]));
        assert!(r.report.not_checked.len() >= 3);
        let english = with_language(Language::English, || {
            r.report.find("faults_active").unwrap().explanation.text()
        });
        assert!(
            english.starts_with("Engine (00): SPN 105 FMI 3, 7 times, SPN 111 FMI 17, 2 times."),
            "{english}"
        );
    }

    #[test]
    fn a_turned_back_dashboard_is_red() {
        let mut input = demo();
        input.odometers[1].km -= 200_000.0;
        let r = evaluate_used_truck(&input);
        assert_eq!(r.report.light, TrafficLight::Red);
        assert_eq!(level(&r, "mileage_mismatch"), Level::Critical);
        // The higher reading is the one the hours are measured against.
        assert!((r.facts.odometer_km.unwrap() - 512_358.4).abs() < 0.01);
        let text = with_language(Language::English, || {
            r.report
                .find("mileage_mismatch")
                .unwrap()
                .explanation
                .text()
        });
        assert!(
            text.contains("The difference is 200,006 km") || text.contains("200006 km"),
            "{text}"
        );
        // A few hundred km between sources stay within the tolerance.
        let mut input = demo();
        input.odometers[2].km += 2_000.0;
        assert_eq!(
            level(&evaluate_used_truck(&input), "mileage_consistent"),
            Level::Good
        );
    }

    #[test]
    fn hours_idle_and_fuel_against_the_kilometres() {
        let mut input = demo();
        input.engine_hours = Some(5_000.0);
        input.idle_hours = Some(2_500.0);
        let r = evaluate_used_truck(&input);
        assert_eq!(level(&r, "hours_too_few"), Level::Warning);
        assert_eq!(level(&r, "idle_share"), Level::Warning);
        let mut input = demo();
        input.engine_hours = Some(80_000.0);
        input.fuel_total_l = Some(40_000.0);
        let r = evaluate_used_truck(&input);
        assert_eq!(level(&r, "km_too_few"), Level::Warning);
        assert_eq!(level(&r, "fuel_average"), Level::Warning);
        let mut input = demo();
        input.engine_hours = None;
        let r = evaluate_used_truck(&input);
        assert!(r.report.find("engine_hours").is_none());
        assert!(r.facts.average_speed.is_none());
    }

    #[test]
    fn vins_between_units_and_against_the_papers() {
        let mut input = demo();
        input.vins[2].vin = "YCXTR42H0MC099999".into();
        let r = evaluate_used_truck(&input);
        assert_eq!(level(&r, "vins_differ"), Level::Critical);
        let mut input = demo();
        input.papers_vin = VIN.to_lowercase();
        assert_eq!(
            level(&evaluate_used_truck(&input), "vin_matches_papers"),
            Level::Good
        );
        input.papers_vin = "YCXTR42H4NC051730".into();
        assert_eq!(
            level(&evaluate_used_truck(&input), "vin_differs_from_papers"),
            Level::Critical
        );
        input.vins.clear();
        assert_eq!(
            level(&evaluate_used_truck(&input), "vin_missing"),
            Level::Info
        );
    }

    #[test]
    fn a_fresh_clearing_and_recurring_faults() {
        let mut input = demo();
        input.lamp_history = LampHistory {
            mil_km: Some(0.0),
            since_clear_km: Some(12.0),
            mil_minutes: Some(0.0),
            since_clear_minutes: Some(25.0),
        };
        input.faults[2].previous[0].occurrences = Some(14);
        let r = evaluate_used_truck(&input);
        assert_eq!(level(&r, "cleared_recently"), Level::Warning);
        assert!(r.report.find("mil_distance").is_none());
        assert_eq!(level(&r, "faults_previous"), Level::Warning);
        input.faults[0].lamps.red_stop = true;
        let r = evaluate_used_truck(&input);
        assert_eq!(level(&r, "lamp_red_stop"), Level::Critical);
        assert_eq!(r.report.light, TrafficLight::Red);
    }
}
