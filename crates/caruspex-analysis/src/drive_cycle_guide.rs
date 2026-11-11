//! Drive cycle help: what each readiness monitor checks and how to drive so
//! that it finishes.
//!
//! The exact enabling conditions (temperatures, speeds, times) are defined by
//! each manufacturer and often by each engine. The patterns here are the
//! generic ones that work on most cars; every guide carries that note so
//! nobody takes them for a factory specification.

use std::sync::OnceLock;

use caruspex_i18n::Text;

use crate::readiness::{Monitor, MonitorState, monitor_name};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonitorGuide {
    pub monitor: Monitor,
    /// Short name, e.g. "Tankentlüftung".
    pub name: Text,
    /// What the test checks, for non-mechanics.
    pub explanation: Text,
    /// A generic way to drive that usually completes it.
    pub drive_pattern: Text,
    /// Rough idea of how long it takes.
    pub typical_time: Text,
    /// Starts with an engine that sat for hours.
    pub needs_cold_start: bool,
}

/// All guides, in the order of the readiness list.
pub fn all_drive_cycle_guides() -> &'static [MonitorGuide] {
    static GUIDES: OnceLock<Vec<MonitorGuide>> = OnceLock::new();
    GUIDES.get_or_init(|| {
        vec![
            MonitorGuide {
                monitor: Monitor::Misfire,
                name: monitor_name(Monitor::Misfire),
                explanation: Text::new("Prüft, ob in jedem Zylinder das Gemisch sauber zündet. Ein Aussetzer schüttelt den Motor und schickt unverbranntes Benzin in den Katalysator, der davon überhitzen kann.", "Checks that the mixture fires cleanly in every cylinder. A misfire shakes the engine and sends unburnt fuel into the catalytic converter, which can overheat from it."),
                drive_pattern: Text::new("Läuft ständig mit. Ein paar Minuten normale Fahrt mit wechselnder Drehzahl reichen meistens.", "Runs all the time. A few minutes of normal driving at varying engine speeds is usually enough."),
                typical_time: Text::new("wenige Minuten", "a few minutes"),
                needs_cold_start: false,
            },
            MonitorGuide {
                monitor: Monitor::FuelSystem,
                name: monitor_name(Monitor::FuelSystem),
                explanation: Text::new("Prüft, ob die Gemischregelung das Verhältnis von Luft und Kraftstoff im erlaubten Bereich halten kann. Muss sie zu stark nachregeln, stimmt etwas nicht (Falschluft, Einspritzung, Kraftstoffdruck).", "Checks that the mixture control can keep the ratio of air and fuel within its allowed range. If it has to correct too much, something is off (air leak, injection, fuel pressure)."),
                drive_pattern: Text::new("Läuft ständig mit, sobald der Motor warm ist. Einige Minuten Leerlauf und gleichmäßige Fahrt.", "Runs all the time once the engine is warm. A few minutes of idling and steady driving."),
                typical_time: Text::new("5 bis 10 Minuten", "5 to 10 minutes"),
                needs_cold_start: false,
            },
            MonitorGuide {
                monitor: Monitor::Components,
                name: monitor_name(Monitor::Components),
                explanation: Text::new("Prüft die Sensoren und Stellglieder des Motors auf Kurzschluss, Unterbrechung und unplausible Werte, zum Beispiel Temperaturfühler und Drosselklappe.", "Checks the engine's sensors and actuators for short circuits, open circuits and implausible values, for example temperature sensors and the throttle."),
                drive_pattern: Text::new("Läuft ständig mit und ist meist schon kurz nach dem Start fertig.", "Runs all the time and is usually complete shortly after starting."),
                typical_time: Text::new("unter einer Minute bis wenige Minuten", "under a minute to a few minutes"),
                needs_cold_start: false,
            },
            MonitorGuide {
                monitor: Monitor::Catalyst,
                name: monitor_name(Monitor::Catalyst),
                explanation: Text::new("Prüft, ob der Katalysator die Abgase noch richtig reinigt. Dafür vergleicht das Steuergerät die Lambdasonde vor dem Katalysator mit der dahinter.", "Checks that the catalytic converter still cleans the exhaust properly. To do so, the control unit compares the oxygen sensor in front of the converter with the one behind it."),
                drive_pattern: Text::new("Motor ganz warm fahren, dann 5 bis 10 Minuten gleichmäßig zwischen 70 und 100 km/h auf ebener Strecke, mit ruhigem Gasfuß. Danach ein paar Mal vom Gas gehen und ausrollen lassen. Manche Autos brauchen dafür zwei oder drei Fahrten.", "Drive until the engine is fully warm, then 5 to 10 minutes at a steady 70 to 100 km/h (45 to 60 mph) on a level road with a light foot. Afterwards lift off the throttle a few times and let the car coast. Some cars need two or three trips."),
                typical_time: Text::new("15 bis 30 Minuten, oft über mehrere Fahrten", "15 to 30 minutes, often over several trips"),
                needs_cold_start: false,
            },
            MonitorGuide {
                monitor: Monitor::HeatedCatalyst,
                name: monitor_name(Monitor::HeatedCatalyst),
                explanation: Text::new("Prüft die Heizung, die den Katalysator nach dem Kaltstart schneller auf Betriebstemperatur bringt. Gibt es nur bei wenigen Autos.", "Checks the heater that brings the catalytic converter up to temperature faster after a cold start. Only a few cars have it."),
                drive_pattern: Text::new("Kaltstart, danach ein bis zwei Minuten Leerlauf und dann normal losfahren.", "Cold start, then idle for one to two minutes and drive off normally."),
                typical_time: Text::new("10 bis 20 Minuten nach einem Kaltstart", "10 to 20 minutes after a cold start"),
                needs_cold_start: true,
            },
            MonitorGuide {
                monitor: Monitor::Evap,
                name: monitor_name(Monitor::Evap),
                explanation: Text::new("Prüft, ob das Tanksystem dicht ist, damit keine Benzindämpfe entweichen. Schon ein schlecht geschlossener Tankdeckel kann den Test scheitern lassen.", "Checks that the fuel tank system is sealed so no fuel vapour escapes. Even a fuel cap that is not closed properly can make the test fail."),
                drive_pattern: Text::new("Der schwierigste Test. Das Auto mindestens 8 Stunden stehen lassen (am besten über Nacht), Tank zwischen einem Viertel und drei Vierteln voll, Tankdeckel fest zu. Kaltstart, dann ruhig fahren mit Stadt- und Landstraßenanteil und einer Minute Leerlauf am Ende. Viele Autos testen erst nach dem Abstellen, also danach wieder einige Stunden stehen lassen. Bei sehr kaltem oder sehr heißem Wetter läuft der Test oft gar nicht.", "The hardest one. Let the car sit for at least 8 hours (overnight is best), tank between a quarter and three quarters full, fuel cap tightly closed. Cold start, then drive calmly with some town and some country road and a minute of idling at the end. Many cars run the test only after being switched off, so let it sit for a few hours again afterwards. In very cold or very hot weather the test often does not run at all."),
                typical_time: Text::new("oft ein bis mehrere Tage normaler Nutzung", "often one to several days of normal use"),
                needs_cold_start: true,
            },
            MonitorGuide {
                monitor: Monitor::SecondaryAir,
                name: monitor_name(Monitor::SecondaryAir),
                explanation: Text::new("Prüft die Pumpe, die nach dem Kaltstart Frischluft in den Auspuff bläst, damit der Katalysator schneller warm wird.", "Checks the pump that blows fresh air into the exhaust after a cold start so the catalytic converter warms up faster."),
                drive_pattern: Text::new("Kaltstart nach mehreren Stunden Standzeit, dann ein bis zwei Minuten im Leerlauf laufen lassen, ohne Gas zu geben. Danach normal losfahren.", "Cold start after several hours parked, then let it idle for one to two minutes without touching the throttle. Then drive off normally."),
                typical_time: Text::new("wenige Minuten nach einem Kaltstart", "a few minutes after a cold start"),
                needs_cold_start: true,
            },
            MonitorGuide {
                monitor: Monitor::AcRefrigerant,
                name: monitor_name(Monitor::AcRefrigerant),
                explanation: Text::new("Ein alter, kaum genutzter Test für das Kältemittel der Klimaanlage. Heute meldet ihn fast kein Auto mehr.", "An old, rarely used test for the air conditioning refrigerant. Almost no car reports it any more."),
                drive_pattern: Text::new("Klimaanlage bei warmem Motor einige Minuten laufen lassen.", "Run the air conditioning for a few minutes with the engine warm."),
                typical_time: Text::new("wenige Minuten", "a few minutes"),
                needs_cold_start: false,
            },
            MonitorGuide {
                monitor: Monitor::OxygenSensor,
                name: monitor_name(Monitor::OxygenSensor),
                explanation: Text::new("Prüft, ob die Lambdasonden schnell und richtig auf den Sauerstoff im Abgas reagieren. Sie sind die Augen der Gemischregelung.", "Checks that the oxygen sensors react quickly and correctly to the oxygen in the exhaust. They are the eyes of the mixture control."),
                drive_pattern: Text::new("Motor warm fahren, dann einige Minuten gleichmäßig zwischen 50 und 90 km/h. Danach mehrmals vom Gas gehen und von etwa 90 auf 50 km/h ausrollen lassen, ohne zu bremsen. Zum Schluss eine Minute Leerlauf.", "Drive until warm, then a few minutes at a steady 50 to 90 km/h (30 to 55 mph). Afterwards lift off the throttle several times and coast from about 90 down to 50 km/h without braking. Finish with a minute of idling."),
                typical_time: Text::new("10 bis 20 Minuten", "10 to 20 minutes"),
                needs_cold_start: false,
            },
            MonitorGuide {
                monitor: Monitor::OxygenSensorHeater,
                name: monitor_name(Monitor::OxygenSensorHeater),
                explanation: Text::new("Prüft die Heizung in den Lambdasonden. Die Sonden messen erst ab etwa 300 Grad richtig, die Heizung bringt sie nach dem Start schnell dahin.", "Checks the heaters inside the oxygen sensors. The sensors only measure properly from about 300 degrees Celsius, and the heater gets them there quickly after starting."),
                drive_pattern: Text::new("Kaltstart, danach ein paar Minuten Leerlauf oder ruhige Fahrt.", "Cold start, then a few minutes of idling or gentle driving."),
                typical_time: Text::new("wenige Minuten nach einem Kaltstart", "a few minutes after a cold start"),
                needs_cold_start: true,
            },
            MonitorGuide {
                monitor: Monitor::Egr,
                name: monitor_name(Monitor::Egr),
                explanation: Text::new("Prüft die Abgasrückführung, die einen Teil des Abgases zurück in den Motor leitet und so Stickoxide senkt. Bei manchen Autos steckt hier auch die Nockenwellenverstellung mit drin.", "Checks the exhaust gas recirculation, which leads part of the exhaust back into the engine to lower nitrogen oxides. On some cars the variable valve timing is checked here as well."),
                drive_pattern: Text::new("Motor warm fahren, dann gleichmäßig um 70 bis 90 km/h fahren und mehrmals vom Gas gehen und lange ausrollen lassen, ohne zu bremsen. Beim Diesel helfen zusätzlich längere Abschnitte mit gleichmäßiger Teillast.", "Drive until warm, then cruise at about 70 to 90 km/h (45 to 55 mph) and lift off the throttle several times, letting the car coast for a long stretch without braking. On diesels, longer stretches at steady part load help as well."),
                typical_time: Text::new("10 bis 20 Minuten", "10 to 20 minutes"),
                needs_cold_start: false,
            },
            MonitorGuide {
                monitor: Monitor::NmhcCatalyst,
                name: monitor_name(Monitor::NmhcCatalyst),
                explanation: Text::new("Prüft den Oxidationskatalysator des Diesels, der Kohlenwasserstoffe und Kohlenmonoxid verbrennt.", "Checks the diesel oxidation catalyst, which burns off hydrocarbons and carbon monoxide."),
                drive_pattern: Text::new("Motor warm fahren, dann etwa 20 Minuten gleichmäßig über Land oder Autobahn, zum Beispiel mit 80 bis 110 km/h.", "Drive until warm, then about 20 minutes at a steady pace on country roads or the motorway, for example 80 to 110 km/h (50 to 70 mph)."),
                typical_time: Text::new("20 bis 30 Minuten", "20 to 30 minutes"),
                needs_cold_start: false,
            },
            MonitorGuide {
                monitor: Monitor::NoxScr,
                name: monitor_name(Monitor::NoxScr),
                explanation: Text::new("Prüft die Stickoxid-Reinigung, bei den meisten neueren Dieseln mit AdBlue (SCR) oder einem Speicherkatalysator.", "Checks the nitrogen oxide cleaning, on most newer diesels with AdBlue (SCR) or a storage catalyst."),
                drive_pattern: Text::new("AdBlue-Tank nicht leer, Motor warm fahren, dann 20 bis 30 Minuten gleichmäßig über Land oder Autobahn. Bei Frost dauert es oft länger.", "AdBlue tank not empty, drive until warm, then 20 to 30 minutes at a steady pace on country roads or the motorway. In freezing weather it often takes longer."),
                typical_time: Text::new("30 Minuten bis mehrere Fahrten", "30 minutes to several trips"),
                needs_cold_start: false,
            },
            MonitorGuide {
                monitor: Monitor::BoostPressure,
                name: monitor_name(Monitor::BoostPressure),
                explanation: Text::new("Prüft, ob der Turbolader den Ladedruck aufbaut, den das Steuergerät verlangt, und nicht zu viel davon.", "Checks that the turbocharger builds the boost pressure the control unit asks for, and not too much of it."),
                drive_pattern: Text::new("Bei warmem Motor ein paar Mal in einem mittleren Gang (3. oder 4.) zügig von etwa 50 auf 100 km/h beschleunigen, wo es sicher und erlaubt ist.", "With the engine warm, accelerate briskly a few times in a middle gear (3rd or 4th) from about 50 to 100 km/h (30 to 60 mph), where it is safe and legal."),
                typical_time: Text::new("10 bis 15 Minuten", "10 to 15 minutes"),
                needs_cold_start: false,
            },
            MonitorGuide {
                monitor: Monitor::ExhaustGasSensor,
                name: monitor_name(Monitor::ExhaustGasSensor),
                explanation: Text::new("Prüft die Sensoren im Abgas des Diesels, etwa Lambda- und NOx-Sensoren.", "Checks the sensors in the diesel exhaust, such as oxygen and NOx sensors."),
                drive_pattern: Text::new("Motor warm fahren, gleichmäßig fahren und dazwischen einige Male vom Gas gehen und ausrollen lassen.", "Drive until warm, drive steadily and in between lift off the throttle a few times and let the car coast."),
                typical_time: Text::new("15 bis 20 Minuten", "15 to 20 minutes"),
                needs_cold_start: false,
            },
            MonitorGuide {
                monitor: Monitor::ParticulateFilter,
                name: monitor_name(Monitor::ParticulateFilter),
                explanation: Text::new("Prüft den Rußpartikelfilter: ob er dicht ist und ob er sich beim Freibrennen (Regeneration) richtig leert.", "Checks the diesel particulate filter: that it is intact and that it empties properly when it burns off the soot (regeneration)."),
                drive_pattern: Text::new("Motor warm fahren, dann 15 bis 20 Minuten über 60 km/h, besser auf der Autobahn. Läuft gerade eine Regeneration (erhöhte Leerlaufdrehzahl, Lüfter läuft nach), die Fahrt nicht unterbrechen.", "Drive until warm, then 15 to 20 minutes above 60 km/h (40 mph), preferably on the motorway. If a regeneration is running (higher idle speed, fan keeps running), do not cut the trip short."),
                typical_time: Text::new("20 Minuten bis mehrere Fahrten", "20 minutes to several trips"),
                needs_cold_start: false,
            },
        ]
    })
}

/// Guide for one monitor.
pub fn drive_cycle_guide(monitor: Monitor) -> &'static MonitorGuide {
    let guides = all_drive_cycle_guides();
    // Every monitor has a guide; the fallback is never reached.
    guides
        .iter()
        .find(|g| g.monitor == monitor)
        .unwrap_or(&guides[0])
}

/// The guides for all supported monitors that are not complete yet, cold
/// start ones first, because a cold start is the hardest to arrange.
pub fn guides_for_incomplete(monitors: &[MonitorState]) -> Vec<&'static MonitorGuide> {
    let mut out: Vec<&'static MonitorGuide> = Vec::new();
    for m in monitors {
        if !m.supported || m.complete {
            continue;
        }
        let g = drive_cycle_guide(m.monitor);
        if !out.iter().any(|x| std::ptr::eq(*x, g)) {
            out.push(g);
        }
    }
    // The sort is stable, so the readiness order is kept within each group.
    out.sort_by_key(|g| !g.needs_cold_start);
    out
}

/// Conditions that help almost every monitor (cold start, fuel level, no
/// stored faults, no clearing in between).
pub fn drive_cycle_basics() -> Text {
    Text::new(
        "So klappt es am ehesten: Auto über Nacht stehen lassen, damit der Motor ganz kalt ist (Kühlmittel ungefähr so warm wie die Außenluft). Tank zwischen einem Viertel und drei Vierteln voll. Vorher keine Fehler mehr im Speicher und zwischendurch nichts löschen, sonst beginnt alles von vorn. Während der Fahrt Zündung nicht ausschalten und Klimaanlage, Heckscheibenheizung und andere große Verbraucher möglichst aus lassen.",
        "This works best: let the car sit overnight so the engine is completely cold (coolant about as warm as the outside air). Tank between a quarter and three quarters full. No faults left in memory beforehand, and do not clear anything in between, or everything starts over. Do not switch the ignition off during the drive, and keep the air conditioning, rear window heating and other big consumers off if you can.",
    )
}

/// The honest note: conditions vary by manufacturer.
pub fn manufacturer_note() -> Text {
    Text::new(
        "Wichtig: Die genauen Bedingungen legt jeder Hersteller selbst fest, oft sogar je Motor. Die Fahrmuster hier funktionieren bei den meisten Autos, sind aber keine Werksvorgabe. Wenn ein Test nach mehreren Tagen normaler Fahrt nicht fertig wird, lohnt ein Blick in die Werkstattunterlagen oder eine Frage an die Werkstatt.",
        "Important: the exact conditions are set by each manufacturer, often even per engine. The patterns here work on most cars but are not a factory specification. If a test is still not complete after several days of normal driving, check the workshop documentation or ask a garage.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::readiness::decode_readiness;

    fn has_long_dash(s: &str) -> bool {
        s.contains('\u{2014}') || s.contains('\u{2013}')
    }

    #[test]
    fn every_monitor_has_a_complete_guide_in_both_languages() {
        let guides = all_drive_cycle_guides();
        assert_eq!(guides.len(), 16);
        for g in guides {
            assert!(std::ptr::eq(drive_cycle_guide(g.monitor), g));
            for t in [&g.name, &g.explanation, &g.drive_pattern, &g.typical_time] {
                assert!(!t.de.is_empty());
                assert!(!t.en.is_empty());
                assert!(!has_long_dash(t.de));
                assert!(!has_long_dash(t.en));
            }
        }
        assert_eq!(drive_cycle_guide(Monitor::Evap).name.de, "Tankentlüftung");
        assert!(manufacturer_note().de.contains("Hersteller"));
        assert!(manufacturer_note().en.contains("manufacturer"));
        assert!(!has_long_dash(drive_cycle_basics().de));
    }

    #[test]
    fn open_monitors_are_listed_with_cold_start_ones_first() {
        let r = decode_readiness(0x00, 0x07, 0x6D, 0x25);
        // Open: catalyst (0x01), EVAP (0x04), O2 sensor (0x20); secondary air
        // (0x08) is supported and complete.
        let list = guides_for_incomplete(&r.monitors);
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].monitor, Monitor::Evap); // needs a cold start
        assert_eq!(list[1].monitor, Monitor::Catalyst);
        assert_eq!(list[2].monitor, Monitor::OxygenSensor);
    }
}
