//! Plain warnings for every limit the curve touches.
//!
//! Each warning covers one rpm range in which its condition holds and keeps
//! the worst value of that range. Texts are built when asked for, in the
//! active language, so a later language switch shows them right.

use caruspex_i18n::{Language, l, language};

use crate::model::{BoostCap, Conditions, Gearbox, Point};
use crate::spec;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Worth knowing, no harm on its own.
    Note,
    /// Risk for the engine or a part.
    Warning,
    /// A hard limit: the result is capped there.
    Limit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WarningKind {
    KnockPulled,
    KnockReserve,
    TurboLimit,
    TurboNear,
    Spool,
    ExhaustTemperature,
    FuelPump,
    FuelPumpNear,
    Injectors,
    InjectorsNear,
    PreIgnition,
    Gearbox,
    GearboxNear,
    ChargeAir,
    Lean,
    Rich,
}

impl WarningKind {
    /// Short name for scripts and tests.
    pub fn key(self) -> &'static str {
        match self {
            WarningKind::KnockPulled => "knock",
            WarningKind::KnockReserve => "knock-reserve",
            WarningKind::TurboLimit => "turbo",
            WarningKind::TurboNear => "turbo-near",
            WarningKind::Spool => "spool",
            WarningKind::ExhaustTemperature => "egt",
            WarningKind::FuelPump => "pump",
            WarningKind::FuelPumpNear => "pump-near",
            WarningKind::Injectors => "injectors",
            WarningKind::InjectorsNear => "injectors-near",
            WarningKind::PreIgnition => "lspi",
            WarningKind::Gearbox => "gearbox",
            WarningKind::GearboxNear => "gearbox-near",
            WarningKind::ChargeAir => "charge-air",
            WarningKind::Lean => "lean",
            WarningKind::Rich => "rich",
        }
    }

    pub fn severity(self) -> Severity {
        match self {
            WarningKind::TurboLimit | WarningKind::FuelPump | WarningKind::Injectors => {
                Severity::Limit
            }
            WarningKind::KnockPulled
            | WarningKind::ExhaustTemperature
            | WarningKind::PreIgnition
            | WarningKind::Gearbox
            | WarningKind::Lean => Severity::Warning,
            _ => Severity::Note,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Warning {
    pub kind: WarningKind,
    pub severity: Severity,
    pub from_rpm: f64,
    pub to_rpm: f64,
    /// The worst value in the range, in the unit of the reason text.
    pub worst: f64,
    gearbox: Gearbox,
}

/// Decimal comma or point, as the language writes numbers.
fn number(value: f64, decimals: usize) -> String {
    let text = format!("{value:.decimals$}");
    let text = if text.starts_with('-')
        && text
            .trim_start_matches('-')
            .chars()
            .all(|c| c == '0' || c == '.')
    {
        text.trim_start_matches('-').to_string()
    } else {
        text
    };
    let comma = !matches!(
        language(),
        Language::English
            | Language::Chinese
            | Language::Japanese
            | Language::Korean
            | Language::Hindi
            | Language::Arabic
    );
    if comma { text.replace('.', ",") } else { text }
}

fn fill(pattern: &str, values: &[String]) -> String {
    let mut out = pattern.to_string();
    // Backwards, so %1 does not eat the start of a %10.
    for (i, value) in values.iter().enumerate().rev() {
        out = out.replace(&format!("%{}", i + 1), value);
    }
    out
}

impl Warning {
    /// A short title, such as "Knock limit".
    pub fn title(&self) -> &'static str {
        match self.kind {
            WarningKind::KnockPulled => l("Klopfgrenze", "Knock limit"),
            WarningKind::KnockReserve => l("Wenig Klopfreserve", "Little knock reserve"),
            WarningKind::TurboLimit => l("Grenze des Turbos", "Turbo limit"),
            WarningKind::TurboNear => l("Turbo nahe der Grenze", "Turbo near its limit"),
            WarningKind::Spool => l("Ladedruckaufbau", "Boost build-up"),
            WarningKind::ExhaustTemperature => l("Abgastemperatur", "Exhaust temperature"),
            WarningKind::FuelPump => l("Hochdruckpumpe", "High-pressure pump"),
            WarningKind::FuelPumpNear => {
                l("Hochdruckpumpe fast voll", "High-pressure pump nearly full")
            }
            WarningKind::Injectors => l("Einspritzventile", "Injectors"),
            WarningKind::InjectorsNear => l("Einspritzventile fast voll", "Injectors nearly full"),
            WarningKind::PreIgnition => l("Vorentflammung (LSPI)", "Pre-ignition (LSPI)"),
            WarningKind::Gearbox | WarningKind::GearboxNear => l("Getriebe", "Gearbox"),
            WarningKind::ChargeAir => l("Ladelufttemperatur", "Charge air temperature"),
            WarningKind::Lean => l("Mageres Gemisch", "Lean mixture"),
            WarningKind::Rich => l("Sehr fettes Gemisch", "Very rich mixture"),
        }
    }

    /// The reason in plain words, with the worst value of the range.
    pub fn reason(&self) -> String {
        let worst = self.worst;
        match self.kind {
            WarningKind::KnockPulled => fill(
                l(
                    "Die Zündung verlangt bis zu %1° mehr, als die Klopfgrenze erlaubt. Die Klopfregelung des Motors nimmt das zurück: kein Gewinn, aber Klopfen, bis sie eingreift.",
                    "The ignition asks for up to %1° more than the knock limit allows. The engine's knock control pulls it back: no gain, but knocking until it does.",
                ),
                // worst is the knock margin, negative here.
                &[number(-worst, 1)],
            ),
            WarningKind::KnockReserve => fill(
                l(
                    "Nur noch %1° Abstand zur Klopfgrenze. Wärmere Luft oder schlechterer Kraftstoff reichen, damit die Klopfregelung eingreift.",
                    "Only %1° left to the knock limit. Warmer air or poorer fuel is enough for the knock control to step in.",
                ),
                &[number(worst, 1)],
            ),
            WarningKind::TurboLimit => fill(
                l(
                    "Der Turbo ist am Ende (%1 % seiner Grenze): der Ladedruck bleibt unter dem Ziel. Ein höheres Ziel bringt hier nichts außer höherer Laderdrehzahl und heißerer Luft.",
                    "The turbo is at its end (%1 % of its limit): boost stays below the target. A higher target gains nothing here except more turbo speed and hotter air.",
                ),
                &[number(worst * 100.0, 0)],
            ),
            WarningKind::TurboNear => fill(
                l(
                    "Der Turbo arbeitet mit %1 % nahe an seiner Grenze. Die Grenze ist eine Annahme für einen kleinen Twin-Scroll-Lader, kein gemessenes Kennfeld.",
                    "The turbo works close to its limit at %1 %. The limit is an assumption for a small twin-scroll turbo, not a measured map.",
                ),
                &[number(worst * 100.0, 0)],
            ),
            WarningKind::Spool => fill(
                l(
                    "Bei dieser Drehzahl reicht die Abgasenergie nicht für das Ladedruckziel; möglich sind nur etwa %1 bar.",
                    "At this rpm the exhaust energy is not enough for the boost target; only about %1 bar is possible.",
                ),
                &[number(worst, 2)],
            ),
            WarningKind::ExhaustTemperature => fill(
                l(
                    "Abgas vor dem Turbo bis etwa %1 °C, über der angenommenen Grenze von %2 °C. Das Steuergerät würde anfetten oder Leistung zurücknehmen; auf Dauer leiden Turbo und Katalysator.",
                    "Exhaust before the turbo up to about %1 °C, above the assumed limit of %2 °C. The engine control would richen the mixture or cut power; over time turbo and catalyst suffer.",
                ),
                &[number(worst, 0), number(spec::EGT_LIMIT_C, 0)],
            ),
            WarningKind::FuelPump => fill(
                l(
                    "Die Hochdruckpumpe müsste %1 % ihrer angenommenen Fördermenge liefern. Der Raildruck fällt, das Gemisch magert ab.",
                    "The high-pressure pump would have to deliver %1 % of its assumed flow. Rail pressure drops and the mixture goes lean.",
                ),
                &[number(worst * 100.0, 0)],
            ),
            WarningKind::FuelPumpNear => fill(
                l(
                    "Die Hochdruckpumpe ist zu %1 % ausgelastet, kaum noch Reserve.",
                    "The high-pressure pump is %1 % loaded, hardly any reserve left.",
                ),
                &[number(worst * 100.0, 0)],
            ),
            WarningKind::Injectors => fill(
                l(
                    "Die Einspritzventile bräuchten %1 % des Einspritzfensters. Mehr Kraftstoff passt nicht in einen Arbeitstakt.",
                    "The injectors would need %1 % of the injection window. More fuel does not fit into one cycle.",
                ),
                &[number(worst * 100.0, 0)],
            ),
            WarningKind::InjectorsNear => fill(
                l(
                    "Die Einspritzventile nutzen %1 % des Einspritzfensters.",
                    "The injectors use %1 % of the injection window.",
                ),
                &[number(worst * 100.0, 0)],
            ),
            WarningKind::PreIgnition => fill(
                l(
                    "Unter %2 1/min steigt der Mitteldruck auf bis zu %1 bar (Serie etwa 17 bar). Dort wächst das Risiko der Vorentflammung bei niedriger Drehzahl (LSPI), die Kolben und Pleuel auf einen Schlag beschädigen kann.",
                    "Below %2 rpm the mean effective pressure rises to up to %1 bar (stock about 17 bar). There the risk of low-speed pre-ignition (LSPI) grows, which can damage pistons and con rods in one go.",
                ),
                &[number(worst, 1), number(spec::LSPI_RPM, 0)],
            ),
            WarningKind::Gearbox => {
                let pattern = match self.gearbox {
                    Gearbox::Automatic8hp45 => l(
                        "%1 Nm liegen über den %2 Nm, für die ZF das 8HP45 auslegt. Meist begrenzt das Getriebesteuergerät selbst, sonst leiden Kupplungen und Wandler.",
                        "%1 Nm is above the %2 Nm ZF rates the 8HP45 for. The gearbox control usually limits it by itself, otherwise clutches and converter suffer.",
                    ),
                    Gearbox::Manual6 => l(
                        "%1 Nm liegen über den etwa %2 Nm, die für Schaltgetriebe und Kupplung angenommen sind. Die Kupplung rutscht zuerst.",
                        "%1 Nm is above the roughly %2 Nm assumed for manual gearbox and clutch. The clutch slips first.",
                    ),
                };
                fill(
                    pattern,
                    &[number(worst, 0), number(self.gearbox.rated_nm(), 0)],
                )
            }
            WarningKind::GearboxNear => fill(
                l(
                    "Mit %1 Nm bleibt wenig Reserve zur Getriebegrenze von %2 Nm.",
                    "At %1 Nm little reserve is left to the gearbox limit of %2 Nm.",
                ),
                &[number(worst, 0), number(self.gearbox.rated_nm(), 0)],
            ),
            WarningKind::ChargeAir => fill(
                l(
                    "Ladeluft bis etwa %1 °C. Heiße Ladeluft senkt die Klopfgrenze; im Sommer und bei mehrfacher Volllast wird es mehr.",
                    "Charge air up to about %1 °C. Hot charge air lowers the knock limit; in summer and on repeated full load it gets more.",
                ),
                &[number(worst, 0)],
            ),
            WarningKind::Lean => fill(
                l(
                    "Lambda %1 unter Volllast ist mager: heißeres Abgas und weniger Abstand zur Klopfgrenze. Kein Serienmotor fährt unter Volllast mager.",
                    "Lambda %1 at full load is lean: hotter exhaust and less distance to the knock limit. No stock engine runs lean at full load.",
                ),
                &[number(worst, 2)],
            ),
            WarningKind::Rich => fill(
                l(
                    "Lambda %1 ist sehr fett: kaum mehr Leistung, aber mehr Verbrauch, Ruß und Last für den Katalysator.",
                    "Lambda %1 is very rich: hardly more power, but more fuel, soot and load on the catalyst.",
                ),
                &[number(worst, 2)],
            ),
        }
    }
}

/// Collects the rpm ranges in which `value` returns Some and keeps the worst
/// value of each range (`worse` decides which of two is worse). `value` also
/// says whether a point is past the limit itself: a range with such a point
/// becomes `over`, otherwise it stays `near`. One range per stretch of rpm,
/// so a value that hovers around the limit gives one warning, not a row of
/// alternating ones.
fn ranges(
    points: &[Point],
    (near, over): (WarningKind, WarningKind),
    gearbox: Gearbox,
    value: impl Fn(&Point) -> Option<(f64, bool)>,
    worse: fn(f64, f64) -> bool,
    out: &mut Vec<Warning>,
) {
    let close = |w: Warning, past: bool, out: &mut Vec<Warning>| {
        let kind = if past { over } else { near };
        out.push(Warning {
            kind,
            severity: kind.severity(),
            ..w
        });
    };
    let mut open: Option<(Warning, bool)> = None;
    for p in points {
        match (value(p), open.as_mut()) {
            (Some((v, past)), Some((w, any_past))) => {
                w.to_rpm = p.rpm;
                *any_past |= past;
                if worse(v, w.worst) {
                    w.worst = v;
                }
            }
            (Some((v, past)), None) => {
                open = Some((
                    Warning {
                        kind: near,
                        severity: near.severity(),
                        from_rpm: p.rpm,
                        to_rpm: p.rpm,
                        worst: v,
                        gearbox,
                    },
                    past,
                ));
            }
            (None, _) => {
                if let Some((w, past)) = open.take() {
                    close(w, past, out);
                }
            }
        }
    }
    if let Some((w, past)) = open {
        close(w, past, out);
    }
}

fn higher(a: f64, b: f64) -> bool {
    a > b
}

fn lower(a: f64, b: f64) -> bool {
    a < b
}

/// Every warning of a curve, the most serious first, then by rpm.
pub fn warnings(points: &[Point], conditions: &Conditions) -> Vec<Warning> {
    let g = conditions.gearbox;
    let mut out = Vec::new();
    use WarningKind as K;
    let one = |kind: K| (kind, kind);
    let plain = |c: bool, v: f64| c.then_some((v, false));

    // Little reserve, or past the knock limit (knock control pulls timing).
    ranges(
        points,
        (K::KnockReserve, K::KnockPulled),
        g,
        |p| (p.knock_margin_deg < 0.5).then_some((p.knock_margin_deg, p.knock_margin_deg < -0.05)),
        lower,
        &mut out,
    );
    let capped = |p: &Point| matches!(p.boost_cap, BoostCap::PressureRatio | BoostCap::Flow);
    ranges(
        points,
        (K::TurboNear, K::TurboLimit),
        g,
        |p| (capped(p) || p.turbo_load > 0.9).then_some((p.turbo_load, capped(p))),
        higher,
        &mut out,
    );
    ranges(
        points,
        one(K::Spool),
        g,
        // Only when the target is clearly out of reach, not for the stock
        // point at 1,000 rpm that is already at the spool limit.
        |p| {
            let missing = spec::P_STANDARD_BAR + p.boost_target_bar - p.manifold_kpa / 100.0;
            plain(
                p.boost_cap == BoostCap::Spool && missing > 0.05,
                p.boost_bar,
            )
        },
        lower,
        &mut out,
    );
    ranges(
        points,
        one(K::ExhaustTemperature),
        g,
        |p| plain(p.egt_c > spec::EGT_LIMIT_C, p.egt_c),
        higher,
        &mut out,
    );
    ranges(
        points,
        (K::FuelPumpNear, K::FuelPump),
        g,
        |p| (p.pump_load >= 0.9).then_some((p.pump_load, p.pump_load > 1.0)),
        higher,
        &mut out,
    );
    ranges(
        points,
        (K::InjectorsNear, K::Injectors),
        g,
        |p| (p.injector_load >= 0.9).then_some((p.injector_load, p.injector_load > 1.0)),
        higher,
        &mut out,
    );
    ranges(
        points,
        one(K::PreIgnition),
        g,
        |p| {
            plain(
                p.rpm <= spec::LSPI_RPM && p.bmep_bar > spec::LSPI_BMEP_BAR,
                p.bmep_bar,
            )
        },
        higher,
        &mut out,
    );
    ranges(
        points,
        (K::GearboxNear, K::Gearbox),
        g,
        |p| (p.gearbox_load >= 0.95).then_some((p.torque_nm, p.gearbox_load > 1.0)),
        higher,
        &mut out,
    );
    ranges(
        points,
        one(K::ChargeAir),
        g,
        |p| plain(p.charge_air_c > spec::CHARGE_AIR_NOTE_C, p.charge_air_c),
        higher,
        &mut out,
    );
    ranges(
        points,
        one(K::Lean),
        g,
        |p| plain(p.lambda > 1.005, p.lambda),
        higher,
        &mut out,
    );
    ranges(
        points,
        one(K::Rich),
        g,
        |p| plain(p.lambda < 0.78, p.lambda),
        lower,
        &mut out,
    );

    out.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then(a.from_rpm.total_cmp(&b.from_rpm))
    });
    out
}
