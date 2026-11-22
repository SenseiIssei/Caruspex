//! The VIN of every control unit against the papers and the catalog.

use super::{Ctx, Section, UsedCarInput, normalise_vin, vin_well_formed};
use crate::common::Level;
use crate::message::{none, text};

pub(super) fn check_identity(c: &mut Ctx, input: &UsedCarInput) {
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
