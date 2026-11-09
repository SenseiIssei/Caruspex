"""Group meanings for codes that are in no table.

SAE J2012 splits every system into number ranges with a fixed topic. A code
we do not know still tells us roughly where the problem sits and how
serious that area tends to be. The first matching rule wins, so narrow
ranges come before wide ones. Bounds are the hex value of the four digits
after the letter (P0A80 -> 0x0A80).

Manufacturer ranges carry their topic too, because most makers follow the
SAE grouping for their own codes. The runtime adds "manufacturer-specific"
and the brand family to the text.
"""

from __future__ import annotations

from .model import SEVERITIES, SYSTEMS, check_text

# (letter, lo, hi, German, English, severity, system, detail)
RANGES: list[tuple] = []


def rule(letter, lo, hi, de, en, sev, sys, detail=""):
    assert sev in SEVERITIES and sys in SYSTEMS
    check_text("range", de)
    check_text("range", en)
    RANGES.append((letter, lo, hi, de, en, sev, sys, detail))


_POWERTRAIN_GROUPS = [
    (0x000, 0x0FF, "Kraftstoff-/Luftgemisch und Zusatzemission",
     "Fuel/air metering and auxiliary emission controls", "medium", "fuel"),
    (0x100, 0x1FF, "Kraftstoff- und Luftmessung", "Fuel and air metering", "medium", "fuel"),
    (0x200, 0x2FF, "Kraftstoff- und Luftmessung (Einspritzung)", "Fuel and air metering (injector circuit)",
     "medium", "injection"),
    (0x300, 0x3FF, "Zündanlage oder Verbrennungsaussetzer", "Ignition system or misfire", "high", "ignition"),
    (0x400, 0x4FF, "Zusatzemission (AGR, Sekundärluft, Tankentlüftung, Katalysator)",
     "Auxiliary emission controls (EGR, secondary air, EVAP, catalyst)", "medium", "egr"),
    (0x500, 0x5FF, "Geschwindigkeit, Leerlaufregelung, Zusatzeingänge",
     "Vehicle speed, idle control and auxiliary inputs", "low", "engine"),
    (0x600, 0x6FF, "Steuergerät und Ausgänge", "Control module and outputs", "medium", "engine"),
    (0x700, 0x9FF, "Getriebe", "Transmission", "medium", "transmission"),
    (0xA00, 0xFFF, "Hybrid- und Elektroantrieb", "Hybrid and electric propulsion", "high", "hybrid"),
]

# P0: SAE, P1: manufacturer, both with the same topics.
for base in (0x0000, 0x1000):
    for lo, hi, de, en, sev, sys in _POWERTRAIN_GROUPS:
        rule("P", base + lo, base + hi, de, en, sev, sys)

rule("P", 0x2000, 0x22FF, "Kraftstoff-/Luftgemisch und Abgasnachbehandlung",
     "Fuel/air metering and exhaust aftertreatment", "medium", "fuel")
rule("P", 0x2300, 0x23FF, "Zündanlage", "Ignition system", "high", "ignition")
rule("P", 0x2400, 0x24FF, "Zusatzemission und Partikelfilter", "Auxiliary emission controls and particulate filter",
     "medium", "egr")
rule("P", 0x2500, 0x25FF, "Zusatzeingänge und Bordnetz", "Auxiliary inputs and charging", "medium", "engine")
rule("P", 0x2600, 0x26FF, "Steuergerät und Ausgänge", "Control module and outputs", "medium", "engine")
rule("P", 0x2700, 0x27FF, "Getriebe", "Transmission", "medium", "transmission")
rule("P", 0x2800, 0x2FFF, "Antrieb und Abgasnachbehandlung", "Powertrain and exhaust aftertreatment",
     "medium", "engine")
rule("P", 0x3000, 0x33FF, "Antrieb", "Powertrain", "medium", "engine")
rule("P", 0x3400, 0x34FF, "Zylinderabschaltung", "Cylinder deactivation", "medium", "timing", "cyl_deactivation")
rule("P", 0x3500, 0x3FFF, "Antrieb (von SAE reserviert)", "Powertrain (reserved by SAE)", "medium", "engine")

rule("C", 0x0000, 0x0FFF, "Fahrwerk: Bremsen, ABS, ESP, Lenkung", "Chassis: brakes, ABS, stability control, steering",
     "medium", "brakes")
rule("C", 0x1000, 0x2FFF, "Fahrwerk: Bremsen, ABS, ESP, Lenkung", "Chassis: brakes, ABS, stability control, steering",
     "medium", "chassis")
rule("C", 0x3000, 0x3FFF, "Fahrwerk (von SAE reserviert)", "Chassis (reserved by SAE)", "medium", "chassis")

rule("B", 0x0000, 0x00FF, "Rückhaltesysteme: Airbag und Gurtstraffer",
     "Restraint systems: airbags and seat belt pretensioners", "high", "airbag", "airbag")
rule("B", 0x0100, 0x0FFF, "Karosserie und Komfortelektronik", "Body and comfort electronics", "low", "body")
rule("B", 0x1000, 0x2FFF, "Karosserie, Komfort und Rückhaltesysteme", "Body, comfort and restraint systems",
     "medium", "body")
rule("B", 0x3000, 0x3FFF, "Karosserie (von SAE reserviert)", "Body (reserved by SAE)", "low", "body")

rule("U", 0x0000, 0x00FF, "Datenbus: Leitungen und Busfunktion", "Network: wiring and bus operation", "high",
     "network", "can_bus")
rule("U", 0x0100, 0x02FF, "Keine Verbindung zu einem Steuergerät", "Lost communication with a control module",
     "medium", "network", "lost_comm")
rule("U", 0x0300, 0x03FF, "Software oder Codierung eines Steuergeräts passt nicht",
     "Control module software or coding incompatibility", "medium", "network")
rule("U", 0x0400, 0x05FF, "Unplausible Daten von einem Steuergerät", "Invalid data received from a control module",
     "medium", "network", "invalid_data")
rule("U", 0x0600, 0x0FFF, "Datenbus und Vernetzung", "Network and vehicle communication", "medium", "network")
rule("U", 0x1000, 0x2FFF, "Datenbus und Vernetzung", "Network and vehicle communication", "medium", "network")
rule("U", 0x3000, 0x3FFF, "Datenbus (von SAE reserviert)", "Network (reserved by SAE)", "medium", "network")
