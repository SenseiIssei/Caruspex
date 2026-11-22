//! The VIN of every unit against the papers, and the identity of the units.

use super::{TruckUsedInput, UnitVin};
use crate::common::Level;
use crate::message::{Message, none, raw, text};
use crate::used_car::{Ctx, Section, normalise_vin, vin_well_formed};

pub(super) fn check_identity(c: &mut Ctx, input: &TruckUsedInput) {
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

pub(super) fn check_units(c: &mut Ctx, input: &TruckUsedInput) {
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
        .lines(lines),
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
