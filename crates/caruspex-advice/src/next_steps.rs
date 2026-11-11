//! "What should I do now?"
//!
//! Everything Caruspex has read about the car goes in: the fault codes, the
//! warning lamp, the readiness monitors, the findings of the health check,
//! the latest live values and what the driver sees on the dashboard. One list
//! comes out, in three steps:
//!
//! - stop now: driving on risks the engine, the catalytic converter or
//!   safety (a critical code, a flashing check engine light, overheating);
//! - visit a workshop soon: confirmed faults that cost fuel, emissions or
//!   comfort and get worse with time;
//! - can wait: pending codes, minor faults and open readiness monitors.
//!
//! A finding of the health check or a live value that belongs to the same
//! part of the car as a fault code does not get a line of its own: it
//! backs up the code (and can raise its urgency), so the list names every
//! problem once.

use std::cmp::Reverse;

use caruspex_analysis::common::Level;
use caruspex_analysis::health_check::limits;
use caruspex_data::dtc::{CostClass, DtcSystem, cost_class_hint, lookup_dtc};
use caruspex_data::dtc_location::{CarZone, locate_dtc};
use caruspex_data::severity::Severity;
use caruspex_i18n::l;

use crate::text::{args, number};

/// The three steps, most urgent first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Urgency {
    /// Driving on risks damage or safety.
    StopNow,
    /// Fine to drive, but have it looked at in the next weeks.
    Soon,
    /// Until the next service, or it sorts itself out.
    CanWait,
}

/// Rough repair cost at a workshop, parts and labour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Cost {
    /// Depends on the cause, which needs finding first.
    #[default]
    Unknown,
    /// Nothing to repair (readiness monitors).
    Free,
    /// Below about 200 euros.
    Cheap,
    /// About 200 to 800 euros.
    Medium,
    /// Above about 800 euros.
    Expensive,
}

/// Where an item comes from. Also the order inside one step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Source {
    /// What the driver sees: a flashing or a red lamp.
    Observation,
    Fault,
    /// A finding of the health check.
    Finding,
    LiveValue,
    /// The check engine light without a code.
    WarningLamp,
    Readiness,
}

/// A fault code as read from the car.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FaultInput {
    pub code: String,
    /// "gespeichert"/"stored", "sporadisch"/"intermittent",
    /// "dauerhaft"/"permanent", as the protocol layer reports it.
    pub kind: String,
    /// Confirmed (mode 03).
    pub stored: bool,
    /// The control unit that reported it, may be empty.
    pub ecu: String,
    /// Description as read, used when the database does not know the code.
    pub text: String,
}

/// One finding of the health check (`health_check::Finding`), already in
/// the active language.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FindingInput {
    /// The stable rule id, e.g. "fuel_trim_bank1".
    pub id: String,
    pub level: Level,
    pub title: String,
    pub explanation: String,
}

/// One readiness monitor: its name in the active language and whether it
/// has completed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReadinessInput {
    pub name: String,
    pub ready: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AdviceInput {
    /// Brand of the car for manufacturer codes, may be empty.
    pub brand: String,
    /// The fault memory has been read; tells "no faults" from "not read".
    pub faults_read: bool,
    pub faults: Vec<FaultInput>,
    /// Check engine light from PID 01, `None` if not read.
    pub mil_on: Option<bool>,
    pub readiness: Vec<ReadinessInput>,
    pub findings: Vec<FindingInput>,
    /// The latest value per PID (0x05 coolant, 0x0C engine speed, 0x42
    /// control unit voltage).
    pub live: Vec<(i32, f64)>,
    /// The driver says the check engine light flashes. OBD cannot tell a
    /// flashing lamp from a steady one.
    pub mil_flashing: bool,
    /// The driver says a red warning light is on.
    pub red_warning_lamp: bool,
}

/// One line of the list.
#[derive(Clone, Debug, PartialEq)]
pub struct AdviceItem {
    pub urgency: Urgency,
    pub source: Source,
    /// The fault code, empty for other items.
    pub code: String,
    pub title: String,
    /// One plain sentence: why it is in this step.
    pub why: String,
    pub cost: Cost,
    /// The rough price band as a sentence.
    pub cost_text: String,
    /// What to tell the workshop, ready to read out or copy.
    pub workshop: String,
    /// Measured values that back the item up (titles of merged findings).
    pub evidence: Vec<String>,
    /// The page with the details: "faults", "health", "battery" or empty.
    pub page: &'static str,
    /// Severity in the fault database, for the order inside a step.
    pub severity: Severity,
    /// The part of the car, `Unknown` when it belongs to none.
    pub system: DtcSystem,
    /// Workshop sentences before the evidence is added.
    workshop_parts: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AdviceReport {
    /// Most urgent first.
    pub items: Vec<AdviceItem>,
    pub headline: String,
    pub summary: String,
    /// Nothing has been read yet, the list says nothing about the car.
    pub nothing_read: bool,
}

impl AdviceReport {
    pub fn count(&self, urgency: Urgency) -> usize {
        self.items.iter().filter(|i| i.urgency == urgency).count()
    }
}

/// The rough price band as a sentence.
pub fn cost_text(cost: Cost) -> String {
    match cost {
        Cost::Free => l("Kostet nichts", "Costs nothing").to_string(),
        Cost::Cheap => cost_class_hint(CostClass::Cheap).to_string(),
        Cost::Medium => cost_class_hint(CostClass::Medium).to_string(),
        Cost::Expensive => cost_class_hint(CostClass::Expensive).to_string(),
        Cost::Unknown => l(
            "Kosten erst nach der Fehlersuche abschätzbar",
            "Cost can only be estimated once the cause is found",
        )
        .to_string(),
    }
}

fn cost_of(class: CostClass) -> Cost {
    match class {
        CostClass::Cheap => Cost::Cheap,
        CostClass::Medium => Cost::Medium,
        CostClass::Expensive => Cost::Expensive,
        CostClass::Unknown => Cost::Unknown,
    }
}

fn flashing_sentence() -> &'static str {
    l(
        "Eine blinkende Lampe heißt starke Zündaussetzer: unverbrannter Kraftstoff kann den Katalysator in wenigen Minuten zerstören.",
        "A flashing light means severe misfiring: unburnt fuel can destroy the catalytic converter within minutes.",
    )
}

/// P0300 (random misfire) to P0312 (cylinder 12).
fn is_misfire_code(code: &str) -> bool {
    code.len() == 5
        && code.starts_with("P03")
        && code[1..]
            .parse::<u32>()
            .is_ok_and(|n| (300..=312).contains(&n))
}

/// A pending code: seen once, not confirmed yet. Permanent codes count as
/// confirmed, they stay until the monitor has passed again.
fn is_pending(fault: &FaultInput) -> bool {
    !fault.stored
        && (fault.kind == l("sporadisch", "intermittent")
            || fault.kind == "sporadisch"
            || fault.kind == "intermittent")
}

/// How urgent a code is. Critical codes (oil pressure, overheating, fuel
/// leak) mean stop, also while pending. A misfire code with a flashing
/// lamp means stop. Pending codes of medium weight can wait.
fn fault_urgency(severity: Severity, confirmed: bool, misfire: bool, flashing: bool) -> Urgency {
    if misfire && flashing {
        return Urgency::StopNow;
    }
    match severity {
        Severity::Critical => Urgency::StopNow,
        Severity::High => Urgency::Soon,
        Severity::Medium if confirmed => Urgency::Soon,
        _ => Urgency::CanWait,
    }
}

fn item(urgency: Urgency, source: Source, title: String, why: String, cost: Cost) -> AdviceItem {
    AdviceItem {
        urgency,
        source,
        code: String::new(),
        title,
        why,
        cost,
        cost_text: String::new(),
        workshop: String::new(),
        evidence: Vec::new(),
        page: "",
        severity: Severity::Medium,
        system: DtcSystem::Unknown,
        workshop_parts: Vec::new(),
    }
}

fn fault_item(fault: &FaultInput, input: &AdviceInput) -> AdviceItem {
    let info = lookup_dtc(&fault.code, &input.brand);
    let pending = is_pending(fault);
    let confirmed = !pending;
    let misfire = is_misfire_code(&info.code);
    let flashing = misfire && input.mil_flashing;
    let urgency = fault_urgency(info.severity, confirmed, misfire, input.mil_flashing);

    let title = if info.known || fault.text.is_empty() {
        info.text.clone()
    } else {
        fault.text.clone()
    };
    let why = if flashing {
        flashing_sentence().to_string()
    } else if pending && urgency == Urgency::CanWait {
        l(
            "Noch nicht bestätigt: das Steuergerät hat den Fehler einmal bemerkt und prüft bei den nächsten Fahrten weiter.",
            "Not confirmed yet: the control unit noticed the fault once and keeps checking on the next drives.",
        )
        .to_string()
    } else {
        info.advice.clone()
    };

    let mut parts = vec![args(
        l("Fehlercode %1, %2: %3.", "Trouble code %1, %2: %3."),
        &[&info.code, &fault.kind, &title],
    )];
    if !fault.ecu.is_empty() {
        parts.push(args(
            l("Gemeldet von: %1.", "Reported by: %1."),
            &[&fault.ecu],
        ));
    }
    if flashing {
        parts.push(
            l(
                "Die Motorkontrollleuchte hat geblinkt.",
                "The check engine light was flashing.",
            )
            .to_string(),
        );
    } else if confirmed && input.mil_on == Some(true) {
        parts.push(
            l(
                "Die Motorkontrollleuchte ist an.",
                "The check engine light is on.",
            )
            .to_string(),
        );
    }
    let zone = locate_dtc(&info.code);
    if zone.zone != CarZone::Unknown {
        parts.push(args(l("Bereich: %1.", "Area: %1."), &[&zone.name]));
    }
    if !info.causes.is_empty() {
        let first: Vec<&str> = info.causes.iter().take(2).map(String::as_str).collect();
        parts.push(args(
            l("Häufige Ursachen: %1.", "Common causes: %1."),
            &[&first.join("; ")],
        ));
    }

    let mut out = item(urgency, Source::Fault, title, why, cost_of(info.cost_class));
    out.code = info.code;
    out.page = "faults";
    out.severity = info.severity;
    out.system = info.system;
    out.workshop_parts = parts;
    out
}

/// The part of the car a health check rule is about, its rough cost, and
/// whether a critical result means stop. `None` for the rules that only
/// repeat what the fault memory and the lamp already say.
fn finding_topic(id: &str) -> Option<(DtcSystem, Cost, bool)> {
    let topic = match id {
        "faults" | "mil" | "readiness" | "cleared_recently" | "mil_distance" => return None,
        "misfire" => (DtcSystem::Ignition, Cost::Medium, true),
        "overheating" => (DtcSystem::Cooling, Cost::Unknown, true),
        "oil_temp" => (DtcSystem::Lubrication, Cost::Unknown, true),
        "thermostat" => (DtcSystem::Cooling, Cost::Medium, false),
        "sensor_coolant" => (DtcSystem::Cooling, Cost::Cheap, false),
        "sensor_intake" => (DtcSystem::AirIntake, Cost::Cheap, false),
        "charging" => (DtcSystem::Charging, Cost::Medium, false),
        "battery_rest" => (DtcSystem::Charging, Cost::Cheap, false),
        "catalyst_temp" => (DtcSystem::Exhaust, Cost::Unknown, false),
        _ if id.starts_with("fuel_trim") => (DtcSystem::FuelAir, Cost::Unknown, false),
        _ if id.starts_with("catalyst") => (DtcSystem::Exhaust, Cost::Expensive, false),
        _ if id.starts_with("o2_") => (DtcSystem::Exhaust, Cost::Medium, false),
        _ => (DtcSystem::Unknown, Cost::Unknown, false),
    };
    Some(topic)
}

fn finding_item(f: &FindingInput) -> Option<AdviceItem> {
    let (system, cost, stop_when_critical) = finding_topic(&f.id)?;
    let urgency = match f.level {
        Level::Good => return None,
        Level::Info => Urgency::CanWait,
        Level::Warning => Urgency::Soon,
        Level::Critical if stop_when_critical => Urgency::StopNow,
        Level::Critical => Urgency::Soon,
    };
    let mut out = item(
        urgency,
        Source::Finding,
        f.title.clone(),
        f.explanation.clone(),
        cost,
    );
    out.page = "health";
    out.system = system;
    out.workshop_parts = vec![args(
        l(
            "Der Gesundheitscheck meldet: %1.",
            "The health check reports: %1.",
        ),
        &[&f.title],
    )];
    Some(out)
}

fn live_items(input: &AdviceInput) -> Vec<AdviceItem> {
    let value = |pid: i32| {
        input
            .live
            .iter()
            .rev()
            .find(|(p, v)| *p == pid && v.is_finite())
            .map(|(_, v)| *v)
    };
    let measured = |mut it: AdviceItem, system: DtcSystem, page: &'static str| {
        it.system = system;
        it.page = page;
        it.workshop_parts = vec![args(
            l(
                "Gemessen bei laufendem Motor: %1.",
                "Measured with the engine running: %1.",
            ),
            &[&it.title],
        )];
        it
    };
    let mut out = Vec::new();
    if let Some(coolant) = value(0x05)
        && coolant >= limits::COOLANT_HOT
    {
        let stop = coolant >= limits::COOLANT_OVERHEAT;
        let why = if stop {
            l(
                "Über 115 °C kann die Zylinderkopfdichtung Schaden nehmen; Motor abstellen und abkühlen lassen.",
                "Above 115 °C the head gasket can be damaged; switch off the engine and let it cool down.",
            )
        } else {
            l(
                "Die Kühlmitteltemperatur liegt über dem normalen Bereich; Kühlmittelstand und Lüfter bald prüfen lassen.",
                "The coolant temperature is above the normal range; have the coolant level and the fan checked soon.",
            )
        };
        let it = item(
            if stop {
                Urgency::StopNow
            } else {
                Urgency::Soon
            },
            Source::LiveValue,
            args(
                l("Kühlmittel sehr heiß: %1 °C", "Coolant very hot: %1 °C"),
                &[&number(coolant, 0)],
            ),
            why.to_string(),
            Cost::Unknown,
        );
        out.push(measured(it, DtcSystem::Cooling, "health"));
    }
    let running = value(0x0C).is_some_and(|rpm| rpm >= 500.0);
    if let Some(volts) = value(0x42)
        && running
    {
        let shown = number(volts, 1);
        let charging = if volts < limits::CHARGING_NONE {
            Some((
                args(
                    l(
                        "Lichtmaschine lädt nicht: %1 V",
                        "Alternator not charging: %1 V",
                    ),
                    &[&shown],
                ),
                l(
                    "Bei laufendem Motor sollten mindestens 13,2 V anliegen, sonst wird die Batterie leer und das Auto bleibt stehen.",
                    "With the engine running there should be at least 13.2 V, otherwise the battery goes flat and the car stops.",
                ),
            ))
        } else if volts > limits::CHARGING_TOO_HIGH {
            Some((
                args(
                    l(
                        "Ladespannung zu hoch: %1 V",
                        "Charging voltage too high: %1 V",
                    ),
                    &[&shown],
                ),
                l(
                    "Über 15,5 V leiden Batterie und Elektronik; meist ist der Regler der Lichtmaschine defekt.",
                    "Above 15.5 V the battery and electronics suffer; usually the alternator's regulator is faulty.",
                ),
            ))
        } else {
            None
        };
        if let Some((title, why)) = charging {
            let it = item(
                Urgency::Soon,
                Source::LiveValue,
                title,
                why.to_string(),
                Cost::Medium,
            );
            out.push(measured(it, DtcSystem::Charging, "battery"));
        }
    }
    out
}

/// Folds `other` into `into`: the more urgent step wins together with its
/// reason, the title of the other becomes evidence.
fn absorb(into: &mut AdviceItem, other: AdviceItem) {
    if other.urgency < into.urgency {
        into.urgency = other.urgency;
        into.why = other.why;
    }
    if into.cost == Cost::Unknown {
        into.cost = other.cost;
    }
    into.evidence.push(other.title);
    into.evidence.extend(other.evidence);
}

/// The list for the car as far as it has been read.
pub fn what_to_do(input: &AdviceInput) -> AdviceReport {
    let mut items: Vec<AdviceItem> = input
        .faults
        .iter()
        .filter(|f| !f.code.trim().is_empty())
        .map(|f| fault_item(f, input))
        .collect();

    // Health check findings and live values, one per part of the car.
    let mut measured: Vec<AdviceItem> = Vec::new();
    let candidates = input
        .findings
        .iter()
        .filter_map(finding_item)
        .chain(live_items(input));
    for it in candidates {
        let same = (it.system != DtcSystem::Unknown)
            .then(|| measured.iter().position(|m| m.system == it.system))
            .flatten();
        match same {
            Some(i) if it.urgency < measured[i].urgency => {
                let weaker = std::mem::replace(&mut measured[i], it);
                absorb(&mut measured[i], weaker);
            }
            Some(i) => absorb(&mut measured[i], it),
            None => measured.push(it),
        }
    }
    // A measured value that belongs to a fault code backs that code up.
    for m in measured {
        let fault = (m.system != DtcSystem::Unknown)
            .then(|| {
                items
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| f.source == Source::Fault && f.system == m.system)
                    .min_by_key(|(_, f)| f.urgency)
                    .map(|(i, _)| i)
            })
            .flatten();
        match fault {
            Some(i) => absorb(&mut items[i], m),
            None => items.push(m),
        }
    }

    let confirmed_fault = input.faults.iter().any(|f| !is_pending(f));
    if input.mil_on == Some(true) && !confirmed_fault {
        let mut it = item(
            Urgency::Soon,
            Source::WarningLamp,
            l(
                "Motorkontrollleuchte an, aber kein Fehlercode",
                "Check engine light on, but no trouble code",
            )
            .to_string(),
            l(
                "Die Lampe meldet einen abgasrelevanten Fehler; liefert das Motorsteuergerät keinen Code, steht er oft in einem anderen Steuergerät.",
                "The light reports an emissions fault; if the engine control unit gives no code, it is often stored in another control unit.",
            )
            .to_string(),
            Cost::Unknown,
        );
        it.page = "faults";
        it.workshop_parts = vec![
            l(
                "Die Motorkontrollleuchte leuchtet, über OBD liefert das Motorsteuergerät aber keinen Fehlercode. Bitte alle Steuergeräte auslesen.",
                "The check engine light is on, but the engine control unit gives no trouble code over OBD. Please read all control units.",
            )
            .to_string(),
        ];
        items.push(it);
    }

    let misfire_listed = items
        .iter()
        .any(|i| i.source == Source::Fault && is_misfire_code(&i.code));
    if input.mil_flashing && !misfire_listed {
        let mut it = item(
            Urgency::StopNow,
            Source::Observation,
            l(
                "Motorkontrollleuchte blinkt",
                "Check engine light is flashing",
            )
            .to_string(),
            flashing_sentence().to_string(),
            Cost::Unknown,
        );
        it.page = "faults";
        it.severity = Severity::Critical;
        it.system = DtcSystem::Ignition;
        it.workshop_parts = vec![
            l(
                "Die Motorkontrollleuchte hat während der Fahrt geblinkt, Verdacht auf starke Zündaussetzer. Das Auto wurde abgestellt.",
                "The check engine light was flashing while driving, suspected severe misfiring. The car has been parked.",
            )
            .to_string(),
        ];
        items.push(it);
    }
    if input.red_warning_lamp {
        let mut it = item(
            Urgency::StopNow,
            Source::Observation,
            l("Rote Warnlampe leuchtet", "A red warning light is on").to_string(),
            l(
                "Rot heißt anhalten: meist geht es um Öldruck, Kühlmittel, Bremse oder Ladestrom, und Weiterfahren kann Motor oder Bremsen kosten.",
                "Red means stop: it is usually oil pressure, coolant, brakes or charging, and driving on can cost the engine or the brakes.",
            )
            .to_string(),
            Cost::Unknown,
        );
        it.severity = Severity::Critical;
        it.workshop_parts = vec![
            l(
                "Eine rote Warnlampe leuchtet (Symbol fotografieren und zeigen). Das Auto wurde abgestellt.",
                "A red warning light is on (take a photo of the symbol and show it). The car has been parked.",
            )
            .to_string(),
        ];
        items.push(it);
    }

    let open: Vec<&str> = input
        .readiness
        .iter()
        .filter(|r| !r.ready)
        .map(|r| r.name.as_str())
        .collect();
    if !open.is_empty() {
        let names = open.join(", ");
        let mut it = item(
            Urgency::CanWait,
            Source::Readiness,
            args(
                l(
                    "Bereitschaftstests offen: %1",
                    "Readiness monitors incomplete: %1",
                ),
                &[&names],
            ),
            l(
                "Das Auto hat seine Selbsttests noch nicht beendet; nach dem Löschen oder einem Batteriewechsel ist das normal und erledigt sich durch ein paar Tage normales Fahren.",
                "The car has not finished its self-tests yet; after clearing codes or a battery change that is normal and sorts itself out after a few days of normal driving.",
            )
            .to_string(),
            Cost::Free,
        );
        it.page = "health";
        it.severity = Severity::Info;
        it.workshop_parts = vec![args(
            l(
                "Bereitschaftstests noch offen: %1. Vor der Abgasuntersuchung erst fertig fahren lassen.",
                "Readiness monitors still incomplete: %1. Let them complete before the emissions test.",
            ),
            &[&names],
        )];
        items.push(it);
    }

    for it in &mut items {
        it.cost_text = cost_text(it.cost);
        let mut parts = std::mem::take(&mut it.workshop_parts);
        if !it.evidence.is_empty() {
            parts.push(args(
                l("Messwerte dazu: %1.", "Measured values: %1."),
                &[&it.evidence.join("; ")],
            ));
        }
        it.workshop = parts.join(" ");
    }
    // Stable: inside one step and source the order of the car stays.
    items.sort_by_key(|i| (i.urgency, i.source, Reverse(i.severity)));

    let nothing_read = !input.faults_read
        && input.faults.is_empty()
        && input.mil_on.is_none()
        && input.readiness.is_empty()
        && input.findings.is_empty()
        && input.live.is_empty()
        && !input.mil_flashing
        && !input.red_warning_lamp;
    let mut report = AdviceReport {
        items,
        nothing_read,
        ..AdviceReport::default()
    };
    let (headline, summary) = if nothing_read {
        (
            l("Noch nichts gelesen", "Nothing read yet"),
            l(
                "Verbinden und das Auto prüfen, dann steht hier, was zu tun ist.",
                "Connect and check the car, then this page tells you what to do.",
            ),
        )
    } else if report.count(Urgency::StopNow) > 0 {
        (
            l("Bitte anhalten", "Please stop"),
            l(
                "Mindestens ein Punkt verträgt keine Weiterfahrt. Anhalten, sobald es sicher geht, und Hilfe holen.",
                "At least one point does not allow driving on. Stop as soon as it is safe and get help.",
            ),
        )
    } else if report.count(Urgency::Soon) > 0 {
        (
            l("Bald in die Werkstatt", "Visit a workshop soon"),
            l(
                "Weiterfahren geht, aber lass die Punkte unten in den nächsten Wochen ansehen.",
                "You can keep driving, but have the points below looked at in the next few weeks.",
            ),
        )
    } else if report.count(Urgency::CanWait) > 0 {
        (
            l("Nichts Dringendes", "Nothing urgent"),
            l(
                "Was unten steht, kann bis zur nächsten Inspektion warten.",
                "What is listed below can wait until the next service.",
            ),
        )
    } else {
        (
            l("Alles in Ordnung", "All good"),
            l(
                "Kein Fehler gespeichert und nichts Auffälliges gefunden.",
                "No fault stored and nothing unusual found.",
            ),
        )
    };
    report.headline = headline.to_string();
    report.summary = summary.to_string();
    report
}

#[cfg(test)]
mod tests;
