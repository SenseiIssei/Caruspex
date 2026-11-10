//! Guided battery and charging test: evaluation of the measured voltages.
//!
//! The UI walks the user through the steps ([`battery_test_steps`]) and
//! collects the voltage readings, from PID 42 or the adapter's own voltage
//! reading (AT RV). This module turns them into a verdict with plain
//! explanations.
//!
//! Thresholds are the common workshop values for 12 V lead-acid batteries at
//! room temperature. Cold batteries read lower, and cars with smart charging
//! (start-stop, AGM or EFB batteries) regulate the charging voltage on
//! purpose; the texts say so where it matters.

use caruspex_i18n::Text;

use crate::common::{Level, Phrase, format, number, worse};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum BatteryType {
    #[default]
    Unknown,
    Flooded,
    Efb,
    Agm,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct VoltageSample {
    /// Seconds.
    pub t: f64,
    pub volts: f64,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct BatteryInput {
    pub battery_type: BatteryType,
    /// Car has start-stop / smart charging.
    pub start_stop: bool,

    /// Step 1: engine off for a while, ignition on or adapter voltage.
    pub resting_volts: Option<f64>,
    /// To judge surface charge.
    pub minutes_since_engine_off: Option<f64>,

    /// Step 2: voltage during cranking, sampled as fast as possible.
    pub cranking: Vec<VoltageSample>,
    /// If the caller already has the minimum.
    pub cranking_minimum_volts: Option<f64>,

    /// Step 3: engine idling, no extra consumers.
    pub idle_volts: Option<f64>,
    /// Step 4: engine idling with headlights, blower and rear window heating on.
    pub loaded_volts: Option<f64>,

    /// Optional, softens cold readings.
    pub ambient_celsius: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BatteryStepResult {
    /// "resting", "cranking", "charging_idle", "charging_load".
    pub id: String,
    pub level: Level,
    pub volts: f64,
    pub title: Phrase,
    pub explanation: Phrase,
    pub advice: Phrase,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BatteryReport {
    pub overall: Level,
    /// Estimate from the resting voltage.
    pub state_of_charge_percent: Option<i32>,
    /// Only the steps that were measured.
    pub steps: Vec<BatteryStepResult>,
    /// Ids of steps without a reading.
    pub missing_steps: Vec<String>,
    pub summary: Phrase,
    pub cranking_sample_rate_hz: f64,
}

impl Default for BatteryReport {
    fn default() -> Self {
        Self {
            overall: Level::Good,
            state_of_charge_percent: None,
            steps: Vec::new(),
            missing_steps: Vec::new(),
            summary: Phrase::default(),
            cranking_sample_rate_hz: 0.0,
        }
    }
}

/// The instructions for the guided test, one per step, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BatteryStep {
    pub id: &'static str,
    pub instruction: Text,
}

pub mod battery {
    pub const FULL: f64 = 12.6;
    pub const THREE_QUARTERS: f64 = 12.4;
    pub const WEAK: f64 = 12.2;
    pub const FLAT: f64 = 11.9;
    pub const CRANK_GOOD: f64 = 10.0;
    pub const CRANK_WEAK: f64 = 9.6;
    pub const CRANK_BAD: f64 = 8.5;
    pub const CHARGE_LOW: f64 = 13.8;
    pub const CHARGE_HIGH: f64 = 14.8;
    pub const CHARGE_NONE: f64 = 13.2;
    pub const CHARGE_TOO_HIGH: f64 = 15.0;
    pub const CHARGE_DANGEROUS: f64 = 15.5;
}

fn smart_charging(input: &BatteryInput) -> bool {
    input.start_stop
        || input.battery_type == BatteryType::Agm
        || input.battery_type == BatteryType::Efb
}

/// AGM batteries rest about 0.1 V higher than flooded ones.
fn resting_offset(battery_type: BatteryType) -> f64 {
    if battery_type == BatteryType::Agm {
        0.1
    } else {
        0.0
    }
}

fn volts(v: f64) -> Phrase {
    format("{} V", "{} V", &[number(v, 2)])
}

fn add_step(
    r: &mut BatteryReport,
    id: &str,
    level: Level,
    v: f64,
    title: Phrase,
    explanation: Phrase,
    advice: Phrase,
) {
    r.steps.push(BatteryStepResult {
        id: id.to_string(),
        level,
        volts: v,
        title,
        explanation,
        advice,
    });
}

fn evaluate_resting(input: &BatteryInput, r: &mut BatteryReport) {
    let Some(v) = input.resting_volts else {
        r.missing_steps.push("resting".to_string());
        return;
    };
    let off = resting_offset(input.battery_type);
    let soc = estimate_state_of_charge(v, input.battery_type);
    r.state_of_charge_percent = Some(soc);

    let mut notes = Phrase::default();
    let recently_driven = input.minutes_since_engine_off.is_some_and(|m| m < 30.0);
    if recently_driven {
        notes = format(
            "Der Motor ist erst {} Minuten aus. Kurz nach der Fahrt zeigt die Batterie \
             eine Oberflächenladung und liest sich zu gut. Am aussagekräftigsten ist \
             der Wert nach einigen Stunden Standzeit.",
            "The engine has only been off for {} minutes. Shortly after driving the \
             battery carries a surface charge and reads too good. The value means \
             most after a few hours parked.",
            &[number(input.minutes_since_engine_off.unwrap_or(0.0), 0)],
        );
    } else if v > 12.9 + off {
        notes = format(
            "Über 12,9 V deutet auf eine Oberflächenladung kurz nach der Fahrt oder dem \
             Laden hin. Nach einigen Stunden Standzeit noch einmal messen.",
            "Above 12.9 V points to a surface charge shortly after driving or \
             charging. Measure again after a few hours parked.",
            &[],
        );
    }
    if input.ambient_celsius.is_some_and(|c| c < 5.0) {
        notes = notes
            + format(
                "Bei Kälte liegt die Ruhespannung etwas niedriger, ohne dass die \
                 Batterie schlechter ist.",
                "In the cold the resting voltage reads a little lower without \
                 the battery being worse.",
                &[],
            );
    }
    let soc_text = format(
        "Das entspricht grob {} % Ladung.",
        "That is roughly {}% charge.",
        &[number(f64::from(soc), 0)],
    );

    if v >= battery::FULL + off {
        add_step(
            r,
            "resting",
            Level::Good,
            v,
            format("Batterie voll", "Battery full", &[]),
            format(
                "Ruhespannung {}. Eine volle Batterie hat etwa 12,6 V (AGM etwa 12,7 V).",
                "Resting voltage {}. A full battery has about 12.6 V (AGM about 12.7 V).",
                &[volts(v)],
            ) + soc_text
                + notes,
            Phrase::default(),
        );
    } else if v >= battery::THREE_QUARTERS + off {
        add_step(
            r,
            "resting",
            Level::Good,
            v,
            format("Batterie gut geladen", "Battery well charged", &[]),
            format(
                "Ruhespannung {}. Nicht ganz voll, aber im grünen Bereich.",
                "Resting voltage {}. Not quite full, but fine.",
                &[volts(v)],
            ) + soc_text
                + notes,
            Phrase::default(),
        );
    } else if v >= battery::WEAK + off {
        add_step(
            r,
            "resting",
            Level::Info,
            v,
            format("Batterie halb geladen", "Battery half charged", &[]),
            format(
                "Ruhespannung {}. Die Batterie ist nur etwa halb voll. Auf Dauer schadet \
                 das, weil sich Sulfat an den Platten festsetzt.",
                "Resting voltage {}. The battery is only about half full. In the long \
                 run that harms it, because sulphate builds up on the plates.",
                &[volts(v)],
            ) + soc_text
                + notes,
            format(
                "Eine längere Fahrt machen oder mit einem Ladegerät nachladen.",
                "Take a longer drive or top it up with a charger.",
                &[],
            ),
        );
    } else if v >= battery::FLAT + off {
        add_step(
            r,
            "resting",
            Level::Warning,
            v,
            format("Batterie schwach", "Battery weak", &[]),
            format(
                "Ruhespannung {}. Unter 12,2 V ist die Batterie stark entladen oder \
                 gealtert. Bei Kälte kann das Auto damit schon schlecht anspringen.",
                "Resting voltage {}. Below 12.2 V the battery is heavily discharged or \
                 aged. In the cold the car may already struggle to start.",
                &[volts(v)],
            ) + soc_text
                + notes,
            format(
                "Mit einem Ladegerät voll laden und nach einem Tag erneut messen. Fällt \
                 die Spannung wieder ab, ist die Batterie verbraucht oder etwas zieht im \
                 Stand Strom.",
                "Charge it fully with a charger and measure again after a day. If the \
                 voltage drops again, the battery is worn out or something drains it \
                 while parked.",
                &[],
            ),
        );
    } else {
        add_step(
            r,
            "resting",
            Level::Critical,
            v,
            format("Batterie tiefentladen", "Battery deeply discharged", &[]),
            format(
                "Ruhespannung {}. Die Batterie ist so gut wie leer. Tiefentladung schadet \
                 Bleibatterien dauerhaft.",
                "Resting voltage {}. The battery is practically empty. Deep discharge \
                 permanently harms lead-acid batteries.",
                &[volts(v)],
            ) + notes,
            format(
                "Sofort laden. Danach prüfen lassen, ob sie die Ladung noch hält, und \
                 nach einem Stromverbraucher im Stand suchen.",
                "Charge it right away. Afterwards have it checked whether it still holds \
                 a charge, and look for something draining it while parked.",
                &[],
            ),
        );
    }
}

fn evaluate_cranking(input: &BatteryInput, r: &mut BatteryReport) {
    let mut minimum = input.cranking_minimum_volts;
    if let Some(first_sample) = input.cranking.first() {
        let mut lowest = first_sample.volts;
        let mut first = first_sample.t;
        let mut last = first;
        for s in &input.cranking {
            lowest = lowest.min(s.volts);
            first = first.min(s.t);
            last = last.max(s.t);
        }
        minimum = Some(match minimum {
            Some(m) => m.min(lowest),
            None => lowest,
        });
        if input.cranking.len() >= 2 && last > first {
            r.cranking_sample_rate_hz = (input.cranking.len() - 1) as f64 / (last - first);
        }
    }
    let Some(v) = minimum else {
        r.missing_steps.push("cranking".to_string());
        return;
    };

    let mut notes = Phrase::default();
    let slow = r.cranking_sample_rate_hz > 0.0 && r.cranking_sample_rate_hz < 10.0;
    if slow {
        notes = format(
            "Gemessen wurde mit {} Werten pro Sekunde. Der Einbruch beim Anlassen \
             dauert oft nur Sekundenbruchteile, der echte Tiefstwert kann also noch \
             etwas niedriger liegen.",
            "Measured at {} readings per second. The dip while cranking often lasts \
             only a fraction of a second, so the real minimum may be a little lower.",
            &[number(r.cranking_sample_rate_hz, 0)],
        );
    }
    if input.ambient_celsius.is_some_and(|c| c < 0.0) {
        notes = notes
            + format(
                "Bei Frost braucht der Anlasser mehr Strom, ein etwas tieferer \
                 Einbruch ist dann normal.",
                "In freezing weather the starter needs more current, so a \
                 slightly deeper dip is normal then.",
                &[],
            );
    }

    // A "dip" that is hardly below the resting voltage means the start was
    // not caught at all.
    if input.resting_volts.is_some_and(|rest| v > rest - 0.3) {
        add_step(
            r,
            "cranking",
            Level::Info,
            v,
            format(
                "Kein Einbruch beim Start gemessen",
                "No dip measured while cranking",
                &[],
            ),
            format(
                "Der tiefste Wert ({}) liegt kaum unter der Ruhespannung. Vermutlich \
                 wurde der Startvorgang nicht erfasst.",
                "The lowest value ({}) is hardly below the resting voltage. The start was \
                 probably not captured.",
                &[volts(v)],
            ) + notes,
            format(
                "Den Schritt wiederholen und erst nach dem Start der Messung den \
                 Schlüssel drehen.",
                "Repeat the step and only turn the key after the measurement has \
                 started.",
                &[],
            ),
        );
        return;
    }

    if v >= battery::CRANK_GOOD {
        add_step(
            r,
            "cranking",
            Level::Good,
            v,
            format("Startspannung gut", "Cranking voltage good", &[]),
            format(
                "Beim Anlassen fiel die Spannung nur auf {}. Über 10 V hat die Batterie \
                 genug Kraft für den Anlasser.",
                "While cranking the voltage only dropped to {}. Above 10 V the battery \
                 has enough strength for the starter.",
                &[volts(v)],
            ) + notes,
            Phrase::default(),
        );
    } else if v >= battery::CRANK_WEAK {
        add_step(
            r,
            "cranking",
            Level::Info,
            v,
            format("Startspannung knapp", "Cranking voltage borderline", &[]),
            format(
                "Beim Anlassen fiel die Spannung auf {}. Das reicht noch, aber unter 9,6 V \
                 wird es kritisch, vor allem im Winter.",
                "While cranking the voltage dropped to {}. That is still enough, but \
                 below 9.6 V it gets critical, especially in winter.",
                &[volts(v)],
            ) + notes,
            format(
                "Vor dem Winter die Batterie prüfen lassen.",
                "Have the battery tested before winter.",
                &[],
            ),
        );
    } else if v >= battery::CRANK_BAD {
        add_step(
            r,
            "cranking",
            Level::Warning,
            v,
            format("Startspannung zu niedrig", "Cranking voltage too low", &[]),
            format(
                "Beim Anlassen fiel die Spannung auf {}. Unter 9,6 V ist meist die \
                 Batterie schwach. Ist sie voll geladen und trotzdem so tief, kann auch \
                 der Anlasser zu viel Strom ziehen oder ein Kabel schlecht Kontakt haben.",
                "While cranking the voltage dropped to {}. Below 9.6 V the battery is \
                 usually weak. If it is fully charged and still drops this low, the \
                 starter may draw too much current or a cable may have a poor contact.",
                &[volts(v)],
            ) + notes,
            format(
                "Batterie laden und einen Belastungstest machen lassen. Polklemmen und \
                 Massekabel auf festen Sitz und Korrosion prüfen.",
                "Charge the battery and have a load test done. Check the terminals and \
                 ground cable for tightness and corrosion.",
                &[],
            ),
        );
    } else {
        add_step(
            r,
            "cranking",
            Level::Critical,
            v,
            format(
                "Batterie bricht beim Start ein",
                "Battery collapses while cranking",
                &[],
            ),
            format(
                "Beim Anlassen fiel die Spannung auf {}. So tief schalten sich manche \
                 Steuergeräte schon ab, und der nächste Start kann scheitern.",
                "While cranking the voltage dropped to {}. That low, some control units \
                 already switch off, and the next start may fail.",
                &[volts(v)],
            ) + notes,
            format(
                "Batterie bald ersetzen oder prüfen lassen, Starthilfekabel mitnehmen.",
                "Replace or test the battery soon, and carry jump leads.",
                &[],
            ),
        );
    }
}

fn smart_note() -> Phrase {
    format(
        "Dieses Auto hat ein intelligentes Lademanagement (Start-Stopp, AGM- oder EFB-Batterie). \
         Es lädt nur, wenn es nötig ist, und nutzt gern die Bremsphasen. Werte von etwa 12,5 bis \
         15 V sind dabei normal.",
        "This car has smart charging (start-stop, AGM or EFB battery). It only charges when \
         needed and likes to use braking phases. Values of about 12.5 to 15 V are normal then.",
        &[],
    )
}

fn evaluate_charging(input: &BatteryInput, r: &mut BatteryReport) {
    let smart = smart_charging(input);
    if let Some(v) = input.idle_volts {
        let base = format(
            "Im Leerlauf ohne zusätzliche Verbraucher liegen {} an. Normal \
             sind 13,8 bis 14,8 V.",
            "At idle without extra consumers there are {}. Normal is 13.8 \
             to 14.8 V.",
            &[volts(v)],
        );
        if v > battery::CHARGE_DANGEROUS {
            add_step(
                r,
                "charging_idle",
                Level::Critical,
                v,
                format(
                    "Ladespannung viel zu hoch",
                    "Charging voltage far too high",
                    &[],
                ),
                base + format(
                    "Über 15,5 V kocht die Batterie und Elektronik kann Schaden \
                     nehmen. Meist ist der Laderegler defekt.",
                    "Above 15.5 V the battery boils and electronics can be \
                     damaged. Usually the voltage regulator has failed.",
                    &[],
                ),
                format(
                    "Bald in die Werkstatt.",
                    "Have it seen by a garage soon.",
                    &[],
                ),
            );
        } else if v > battery::CHARGE_TOO_HIGH {
            add_step(
                r,
                "charging_idle",
                if smart { Level::Info } else { Level::Warning },
                v,
                format("Ladespannung hoch", "Charging voltage high", &[]),
                base + if smart {
                    smart_note()
                } else {
                    format(
                        "Dauerhaft über 15 V wird die Batterie überladen.",
                        "Permanently above 15 V the battery gets overcharged.",
                        &[],
                    )
                },
                format(
                    "Nach ein paar Minuten noch einmal messen. Bleibt es so, Regler prüfen \
                     lassen.",
                    "Measure again after a few minutes. If it stays like this, have the \
                     regulator checked.",
                    &[],
                ),
            );
        } else if v > battery::CHARGE_HIGH {
            add_step(
                r,
                "charging_idle",
                if smart { Level::Good } else { Level::Info },
                v,
                format(
                    "Ladespannung am oberen Rand",
                    "Charging voltage at the upper edge",
                    &[],
                ),
                base + if smart {
                    smart_note()
                } else {
                    format(
                        "Kurz nach dem Start oder bei Kälte ist das normal.",
                        "Shortly after starting or in the cold that is normal.",
                        &[],
                    )
                },
                Phrase::default(),
            );
        } else if v >= battery::CHARGE_LOW {
            add_step(
                r,
                "charging_idle",
                Level::Good,
                v,
                format(
                    "Lichtmaschine lädt richtig",
                    "Alternator charges properly",
                    &[],
                ),
                base,
                Phrase::default(),
            );
        } else if v >= battery::CHARGE_NONE {
            add_step(
                r,
                "charging_idle",
                if smart { Level::Good } else { Level::Info },
                v,
                format(
                    "Ladespannung eher niedrig",
                    "Charging voltage rather low",
                    &[],
                ),
                base + if smart {
                    smart_note()
                } else {
                    format(
                        "Etwas wenig; der Test mit eingeschalteten \
                         Verbrauchern zeigt, ob die Lichtmaschine mithält.",
                        "A little low; the test with consumers switched on \
                         shows whether the alternator keeps up.",
                        &[],
                    )
                },
                Phrase::default(),
            );
        } else if smart && v >= 12.4 {
            add_step(
                r,
                "charging_idle",
                Level::Info,
                v,
                format(
                    "Lademanagement pausiert vermutlich",
                    "Smart charging probably paused",
                    &[],
                ),
                base + smart_note(),
                format(
                    "Der nächste Schritt mit eingeschalteten Verbrauchern zeigt, ob die \
                     Lichtmaschine liefert.",
                    "The next step with consumers switched on shows whether the \
                     alternator delivers.",
                    &[],
                ),
            );
        } else if v >= 12.7 {
            add_step(
                r,
                "charging_idle",
                Level::Warning,
                v,
                format("Ladespannung zu niedrig", "Charging voltage too low", &[]),
                base + format(
                    "Unter 13,2 V lädt die Batterie kaum.",
                    "Below 13.2 V the battery barely charges.",
                    &[],
                ),
                format(
                    "Keilrippenriemen, Kabel und Lichtmaschine prüfen lassen.",
                    "Have the drive belt, cables and alternator checked.",
                    &[],
                ),
            );
        } else {
            add_step(
                r,
                "charging_idle",
                Level::Critical,
                v,
                format(
                    "Lichtmaschine lädt nicht",
                    "Alternator is not charging",
                    &[],
                ),
                base + format(
                    "Die Spannung liegt auf Batterieniveau, die Batterie wird also \
                     leer gefahren.",
                    "The voltage is at battery level, so the battery is being \
                     drained.",
                    &[],
                ),
                format(
                    "Bald prüfen lassen, sonst bleibt das Auto liegen.",
                    "Have it checked soon, or the car will stop on the road.",
                    &[],
                ),
            );
        }
    } else {
        r.missing_steps.push("charging_idle".to_string());
    }

    let Some(v) = input.loaded_volts else {
        r.missing_steps.push("charging_load".to_string());
        return;
    };
    let base = format(
        "Mit Licht, Gebläse und Heckscheibenheizung liegen {} an.",
        "With lights, blower and rear window heating on there are {}.",
        &[volts(v)],
    );
    let mut drop = Phrase::default();
    if let Some(idle) = input.idle_volts {
        drop = format(
            "Das sind {} V weniger als ohne Verbraucher.",
            "That is {} V less than without consumers.",
            &[number(idle - v, 2)],
        );
        if idle - v <= 0.05 {
            drop = Phrase::default();
        }
    }
    let floor_bad = if smart { 12.4 } else { 12.7 };
    let floor_weak = if smart { 12.8 } else { battery::CHARGE_NONE };
    if v < floor_bad {
        add_step(
            r,
            "charging_load",
            Level::Critical,
            v,
            format(
                "Lichtmaschine schafft die Last nicht",
                "Alternator cannot handle the load",
                &[],
            ),
            base + drop
                + format(
                    "Die Batterie muss mitliefern und wird dabei leer.",
                    "The battery has to help out and drains in the process.",
                    &[],
                ),
            format(
                "Lichtmaschine und Keilrippenriemen bald prüfen lassen.",
                "Have the alternator and drive belt checked soon.",
                &[],
            ),
        );
    } else if v < floor_weak {
        add_step(
            r,
            "charging_load",
            Level::Warning,
            v,
            format(
                "Ladespannung bricht unter Last ein",
                "Charging voltage sags under load",
                &[],
            ),
            base + drop
                + format(
                    "Eine gesunde Lichtmaschine hält auch unter Last über \
                     13,2 V.",
                    "A healthy alternator stays above 13.2 V even under load.",
                    &[],
                )
                + if smart {
                    smart_note()
                } else {
                    Phrase::default()
                },
            format(
                "Riemenspannung prüfen und die Lichtmaschine testen lassen.",
                "Check the belt tension and have the alternator tested.",
                &[],
            ),
        );
    } else if input.idle_volts.is_some_and(|idle| idle - v > 0.5) && !smart {
        add_step(
            r,
            "charging_load",
            Level::Info,
            v,
            format(
                "Ladespannung sinkt unter Last spürbar",
                "Charging voltage drops noticeably under load",
                &[],
            ),
            base + drop
                + format(
                    "Noch ausreichend, aber mehr als die üblichen 0,5 V Abfall.",
                    "Still enough, but more than the usual 0.5 V drop.",
                    &[],
                ),
            format(
                "Bei Gelegenheit Riemen und Lichtmaschine ansehen lassen.",
                "Have the belt and alternator looked at when convenient.",
                &[],
            ),
        );
    } else {
        add_step(
            r,
            "charging_load",
            Level::Good,
            v,
            format(
                "Lichtmaschine hält unter Last",
                "Alternator copes with the load",
                &[],
            ),
            base + drop,
            Phrase::default(),
        );
    }
}

/// Rough state of charge from the resting voltage (lead-acid, about 20 C).
/// AGM batteries sit about 0.1 V higher when full; that is taken into account.
pub fn estimate_state_of_charge(resting_volts: f64, battery_type: BatteryType) -> i32 {
    // Open-circuit voltage of a lead-acid battery against charge, roughly
    // linear between these points.
    const TABLE: [[f64; 2]; 5] = [
        [11.8, 0.0],
        [12.0, 25.0],
        [12.2, 50.0],
        [12.4, 75.0],
        [12.6, 100.0],
    ];
    let v = resting_volts - resting_offset(battery_type);
    if v <= TABLE[0][0] {
        return 0;
    }
    if v >= TABLE[4][0] {
        return 100;
    }
    for i in 1..5 {
        if v <= TABLE[i][0] {
            let f = (v - TABLE[i - 1][0]) / (TABLE[i][0] - TABLE[i - 1][0]);
            let soc = TABLE[i - 1][1] + f * (TABLE[i][1] - TABLE[i - 1][1]);
            return ((soc / 5.0).round() as i32) * 5; // steps of 5 %
        }
    }
    100
}

pub fn evaluate_battery(input: &BatteryInput) -> BatteryReport {
    let mut r = BatteryReport::default();
    evaluate_resting(input, &mut r);
    evaluate_cranking(input, &mut r);
    evaluate_charging(input, &mut r);

    r.overall = Level::Good;
    for s in &r.steps {
        r.overall = worse(r.overall, s.level);
    }

    if r.steps.is_empty() {
        r.summary = format(
            "Noch keine Messung. Bitte die Schritte des Tests durchgehen.",
            "No measurement yet. Please go through the steps of the test.",
            &[],
        );
        return r;
    }
    r.summary = match r.overall {
        Level::Good | Level::Info => format(
            "Batterie und Lichtmaschine sind in Ordnung.",
            "Battery and alternator are fine.",
            &[],
        ),
        Level::Warning => format(
            "Ein Teil des Bordnetzes schwächelt. Details stehen bei den \
             einzelnen Schritten.",
            "Part of the electrical system is weak. The details are with the \
             individual steps.",
            &[],
        ),
        Level::Critical => format(
            "Das Bordnetz hat ein ernstes Problem. Bitte bald darum kümmern, \
             sonst springt das Auto womöglich nicht mehr an.",
            "The electrical system has a serious problem. Please deal with it \
             soon, or the car may not start any more.",
            &[],
        ),
    };
    if !r.missing_steps.is_empty() {
        r.summary = r.summary
            + format(
                "{} von 4 Schritten fehlen noch.",
                "{} of 4 steps are still missing.",
                &[number(r.missing_steps.len() as f64, 0)],
            );
    }
    r
}

pub fn battery_test_steps() -> &'static [BatteryStep] {
    const STEPS: [BatteryStep; 4] = [
        BatteryStep {
            id: "resting",
            instruction: Text::new(
                "Schritt 1, Ruhespannung: Das Auto sollte mindestens 30 Minuten stehen, besser einige \
                 Stunden. Alle Verbraucher aus, Türen zu. Zündung nur kurz einschalten (nicht \
                 starten), damit der Adapter messen kann.",
                "Step 1, resting voltage: the car should have been parked for at least 30 minutes, \
                 better a few hours. All consumers off, doors closed. Switch the ignition on only \
                 briefly (do not start) so the adapter can measure.",
            ),
        },
        BatteryStep {
            id: "cranking",
            instruction: Text::new(
                "Schritt 2, Startspannung: Messung starten und dann den Motor anlassen. Caruspex \
                 fragt währenddessen nur die Spannung ab, so schnell es geht, und merkt sich den \
                 tiefsten Wert.",
                "Step 2, cranking voltage: start the measurement and then start the engine. During \
                 this Caruspex polls only the voltage, as fast as it can, and keeps the lowest \
                 value.",
            ),
        },
        BatteryStep {
            id: "charging_idle",
            instruction: Text::new(
                "Schritt 3, Ladespannung im Leerlauf: Motor im Leerlauf laufen lassen, alle \
                 Verbraucher aus. Etwa eine Minute warten.",
                "Step 3, charging voltage at idle: let the engine idle with all consumers off. Wait \
                 about a minute.",
            ),
        },
        BatteryStep {
            id: "charging_load",
            instruction: Text::new(
                "Schritt 4, Ladespannung unter Last: Abblendlicht, Gebläse auf höchster Stufe und \
                 Heckscheibenheizung einschalten. Etwa eine Minute warten.",
                "Step 4, charging voltage under load: switch on the low beams, the blower at full \
                 speed and the rear window heating. Wait about a minute.",
            ),
        },
    ];
    &STEPS
}
