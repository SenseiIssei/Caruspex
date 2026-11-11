"""Record types and helpers shared by the fault code tables.

A trouble code has five characters: the system letter (P, C, B, U) and four
hex digits. The second character says who defines the meaning:

    P0, P2, P34..P3F, C0, C3, B0, B3, U0, U3   SAE / ISO, same on every car
    P1, P30..P33, C1, C2, B1, B2, U1, U2      the manufacturer

Even though the digits are hex, the standard numbers codes in decimal
sequences (P0109 is followed by P0110). The helpers below therefore count in
decimal. Codes that really use the letters A to F are written out by hand.
"""

from __future__ import annotations

import re
from dataclasses import dataclass

SEVERITIES = {
    "info": "Info",
    "low": "Low",
    "medium": "Medium",
    "high": "High",
    "critical": "Critical",
}

# Key used in the tables -> enumerator of caruspex::DtcSystem.
SYSTEMS = {
    "unknown": "Unknown",
    "fuel": "FuelAir",
    "intake": "AirIntake",
    "throttle": "Throttle",
    "ignition": "Ignition",
    "timing": "ValveTiming",
    "injection": "Injection",
    "turbo": "Turbo",
    "cooling": "Cooling",
    "lubrication": "Lubrication",
    "exhaust": "Exhaust",
    "egr": "Egr",
    "evap": "Evap",
    "diesel": "DieselAftertreatment",
    "charging": "Charging",
    "engine": "EngineControl",
    "transmission": "Transmission",
    "hybrid": "Hybrid",
    "network": "Network",
    "brakes": "Brakes",
    "steering": "Steering",
    "chassis": "Chassis",
    "airbag": "Airbag",
    "body": "Body",
    "security": "Immobilizer",
    "climate": "Climate",
}

COSTS = {"": "Unknown", "cheap": "Cheap", "medium": "Medium", "expensive": "Expensive"}

_CODE = re.compile(r"^[PCBU][0-3][0-9A-F]{3}$")
_FORBIDDEN = ("\u2014", "\u2013")  # em dash and en dash never reach the user


@dataclass
class Rec:
    de: str
    en: str
    sev: str
    sys: str
    detail: str = ""
    throttle: bool = False


def check_text(where: str, text: str) -> None:
    assert text and text.strip() == text, f"{where}: empty or padded text {text!r}"
    for bad in _FORBIDDEN:
        assert bad not in text, f"{where}: long dash in {text!r}"


def num(letter: str, n: int) -> str:
    """Code from a decimal number: num('P', 301) -> 'P0301'."""
    assert 0 <= n <= 3999, n
    return f"{letter}{n:04d}"


def is_manufacturer(code: str) -> bool:
    """True when the manufacturer, not SAE, defines the meaning."""
    letter, second = code[0], code[1]
    if letter == "P":
        if second == "1":
            return True
        if second == "3":
            return int(code[2], 16) <= 3
        return False
    return second in "12"


class Table:
    """An ordered set of codes with their meaning, refusing duplicates."""

    def __init__(self, name: str) -> None:
        self.name = name
        self.codes: dict[str, Rec] = {}

    def put(self, code: str, de: str, en: str, sev: str, sys: str,
            detail: str = "", throttle: bool = False) -> None:
        assert _CODE.match(code), f"{self.name}: malformed code {code}"
        assert code not in self.codes, f"{self.name}: {code} defined twice"
        assert sev in SEVERITIES, f"{code}: unknown severity {sev}"
        assert sys in SYSTEMS, f"{code}: unknown system {sys}"
        check_text(code, de)
        check_text(code, en)
        self.codes[code] = Rec(de, en, sev, sys, detail, throttle)

    def __len__(self) -> int:
        return len(self.codes)


# Suffixes in the order SAE uses them for a sensor: circuit, range or
# performance, low, high, intermittent.
SENSOR5 = [
    ("Stromkreis", "circuit"),
    ("Signal unplausibel", "circuit range/performance"),
    ("Signal zu niedrig", "circuit low"),
    ("Signal zu hoch", "circuit high"),
    ("Signal setzt zeitweise aus", "circuit intermittent"),
]

# Speed sensors: circuit, range or performance, no signal, intermittent.
SPEED4 = [
    ("Stromkreis", "circuit"),
    ("Signal unplausibel", "circuit range/performance"),
    ("kein Signal", "circuit no signal"),
    ("Signal setzt zeitweise aus", "circuit intermittent"),
]

# Actuators: control circuit or open, range or performance, low, high.
ACTUATOR4 = [
    ("Steuerkreis oder Unterbrechung", "control circuit/open"),
    ("Funktion", "control circuit range/performance"),
    ("Steuerkreis Spannung zu niedrig", "control circuit low"),
    ("Steuerkreis Spannung zu hoch", "control circuit high"),
]

# Actuators without the range/performance code: circuit or open, low, high.
ACTUATOR3 = [
    ("Steuerkreis oder Unterbrechung", "control circuit/open"),
    ("Steuerkreis Spannung zu niedrig", "control circuit low"),
    ("Steuerkreis Spannung zu hoch", "control circuit high"),
]

# Transmission solenoids: malfunction, performance or stuck off, stuck on,
# electrical, intermittent.
SOLENOID5 = [
    ("Fehlfunktion", "malfunction"),
    ("Funktion oder bleibt ausgeschaltet", "performance/stuck off"),
    ("bleibt eingeschaltet", "stuck on"),
    ("elektrischer Fehler", "electrical"),
    ("Signal setzt zeitweise aus", "intermittent"),
]


def family(table: Table, letter: str, start: int, de: str, en: str,
           suffixes: list[tuple[str, str]], sev, sys: str, detail: str = "",
           throttle: bool = False) -> None:
    """Puts one code per suffix, counting up from start.

    sev may be a single severity or a list with one entry per suffix.
    """
    for i, (sd, se) in enumerate(suffixes):
        severity = sev[i] if isinstance(sev, (list, tuple)) else sev
        table.put(num(letter, start + i), f"{de} - {sd}", f"{en} - {se}",
                  severity, sys, detail, throttle)
