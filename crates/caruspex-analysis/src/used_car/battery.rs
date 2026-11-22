//! The high voltage battery of electric and hybrid cars.

use super::{Ctx, Section, UsedCarInput, limits, none};
use crate::common::Level;
use crate::message::{Message, text};

fn ask_battery_certificate() -> Message {
    text(
        "Gibt es ein Prüfprotokoll zum Akkuzustand (SOH) vom Hersteller oder einer Werkstatt, und gilt noch eine Akkugarantie?",
        "Is there a battery health (SOH) report from the maker or a workshop, and is a battery warranty still valid?",
    )
}

pub(super) fn check_battery(c: &mut Ctx, input: &UsedCarInput) {
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
