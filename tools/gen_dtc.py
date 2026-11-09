"""Generates src/data/DtcDatabase.gen.cpp.

Many EOBD codes follow fixed families (per cylinder, per bank, per sensor).
They are computed here instead of typed out, which avoids typos and covers
engines with up to twelve cylinders. On top of that comes a hand-maintained
list for everything that follows no rule, including VAG's own P1xxx codes.

Every code carries a German and an English description; the app picks the
active language at lookup time.

    python tools/gen_dtc.py
"""

from __future__ import annotations

import io
import os

# code -> (German, English)
CODES: dict[str, tuple[str, str]] = {}
THROTTLE: set[str] = set()


def put(code: str, de: str, en: str) -> None:
    CODES.setdefault(code, (de, en))


# --------------------------------------------------------------------------
# Family 1: oxygen sensor heater (P0030 to P0064)
# --------------------------------------------------------------------------
_HEATER_BASE = {(1, 1): 0x030, (1, 2): 0x036, (1, 3): 0x042,
                (2, 1): 0x050, (2, 2): 0x056, (2, 3): 0x062}
for (bank, sensor), base in _HEATER_BASE.items():
    where_de = f"Bank {bank}, Sonde {sensor}"
    where_en = f"bank {bank} sensor {sensor}"
    put(f"P{base:04X}", f"Lambdasondenheizung {where_de} - Stromkreis",
        f"O2 sensor heater {where_en} - circuit")
    put(f"P{base + 1:04X}", f"Lambdasondenheizung {where_de} - Kurzschluss nach Masse",
        f"O2 sensor heater {where_en} - short to ground")
    put(f"P{base + 2:04X}", f"Lambdasondenheizung {where_de} - Kurzschluss nach Plus",
        f"O2 sensor heater {where_en} - short to positive")

# --------------------------------------------------------------------------
# Family 2: oxygen sensor signal (P0130 to P0167)
# --------------------------------------------------------------------------
_O2_BASE = {(1, 1): 0x130, (1, 2): 0x136, (1, 3): 0x142,
            (2, 1): 0x150, (2, 2): 0x156, (2, 3): 0x162}
_O2_SUFFIX = [
    ("Stromkreis", "circuit"),
    ("Spannung zu niedrig (Gemisch zu mager oder Falschluft)",
     "voltage too low (mixture too lean or vacuum leak)"),
    ("Spannung zu hoch (Gemisch zu fett)", "voltage too high (mixture too rich)"),
    ("Regelung zu träge, Sonde reagiert langsam",
     "slow response, sensor reacts sluggishly"),
    ("kein Signal, keine Aktivität", "no signal, no activity"),
    ("Heizkreis - Funktion", "heater circuit - malfunction"),
]
for (bank, sensor), base in _O2_BASE.items():
    where_de = f"Bank {bank}, Sonde {sensor}"
    where_en = f"bank {bank} sensor {sensor}"
    for offset, (text_de, text_en) in enumerate(_O2_SUFFIX):
        put(f"P{base + offset:04X}", f"Lambdasonde {where_de} - {text_de}",
            f"O2 sensor {where_en} - {text_en}")

# --------------------------------------------------------------------------
# Family 3: injectors (P0201 to P0212 and P0261 to P0296)
# --------------------------------------------------------------------------
for cyl in range(1, 13):
    put(f"P{0x201 + cyl - 1:04X}", f"Einspritzventil Zylinder {cyl} - Stromkreis",
        f"Injector cylinder {cyl} - circuit")
    base = 0x261 + (cyl - 1) * 3
    put(f"P{base:04X}", f"Einspritzventil Zylinder {cyl} - Kurzschluss nach Masse",
        f"Injector cylinder {cyl} - short to ground")
    put(f"P{base + 1:04X}", f"Einspritzventil Zylinder {cyl} - Kurzschluss nach Plus",
        f"Injector cylinder {cyl} - short to positive")
    put(f"P{base + 2:04X}", f"Zylinder {cyl} - Beitrag zur Verbrennung zu gering",
        f"Cylinder {cyl} - contribution to combustion too low")

# --------------------------------------------------------------------------
# Family 4: misfires (P0300 to P0312)
# --------------------------------------------------------------------------
put("P0300", "Verbrennungsaussetzer - mehrere oder wechselnde Zylinder",
    "Random/multiple cylinder misfire detected")
for cyl in range(1, 13):
    put(f"P{0x300 + cyl:04X}", f"Verbrennungsaussetzer Zylinder {cyl}",
        f"Cylinder {cyl} misfire detected")

# --------------------------------------------------------------------------
# Family 5: ignition coils (P0350 to P0362)
# --------------------------------------------------------------------------
put("P0350", "Zündspule - Primär- oder Sekundärkreis",
    "Ignition coil - primary or secondary circuit")
for cyl in range(1, 13):
    put(f"P{0x350 + cyl:04X}",
        f"Zündspule Zylinder {cyl} - Primär- oder Sekundärkreis",
        f"Ignition coil cylinder {cyl} - primary or secondary circuit")

# --------------------------------------------------------------------------
# Family 6: variable camshaft timing (P0010 to P0028)
# --------------------------------------------------------------------------
_CAM = {0x010: ("Einlass", "Intake", 1), 0x013: ("Auslass", "Exhaust", 1),
        0x020: ("Einlass", "Intake", 2), 0x023: ("Auslass", "Exhaust", 2)}
for base, (side_de, side_en, bank) in _CAM.items():
    put(f"P{base:04X}", f"Nockenwellenversteller {side_de} Bank {bank} - Stromkreis",
        f"{side_en} camshaft adjuster bank {bank} - circuit")
    put(f"P{base + 1:04X}",
        f"Steuerzeiten {side_de} Bank {bank} - zu weit in Richtung früh "
        f"(Versteller hängt oder Kette gelängt)",
        f"{side_en} camshaft timing bank {bank} - over-advanced "
        f"(adjuster stuck or chain stretched)")
    put(f"P{base + 2:04X}",
        f"Steuerzeiten {side_de} Bank {bank} - zu weit in Richtung spät",
        f"{side_en} camshaft timing bank {bank} - over-retarded")

# --------------------------------------------------------------------------
# Family 7a: transmission (P07xx to P08xx)
# --------------------------------------------------------------------------
for gear in range(1, 9):
    put(f"P{0x730 + gear:04X}", f"Falsches Übersetzungsverhältnis im {gear}. Gang",
        f"Gear {gear} incorrect ratio")
for solenoid, letter in enumerate("ABCDEFGH"):
    base = 0x750 + solenoid * 5
    put(f"P{base:04X}", f"Schaltmagnetventil {letter} - Stromkreis",
        f"Shift solenoid {letter} - circuit")
    put(f"P{base + 1:04X}", f"Schaltmagnetventil {letter} - Funktion oder Rastung",
        f"Shift solenoid {letter} - performance or stuck")
    put(f"P{base + 2:04X}", f"Schaltmagnetventil {letter} - elektrischer Fehler",
        f"Shift solenoid {letter} - electrical fault")
    put(f"P{base + 3:04X}", f"Schaltmagnetventil {letter} - Signal setzt aus",
        f"Shift solenoid {letter} - intermittent")

# --------------------------------------------------------------------------
# Family 7b: chassis, wheels and brakes (C0xxx)
# --------------------------------------------------------------------------
_WHEELS = {0x035: ("vorne links", "front left"), 0x040: ("vorne rechts", "front right"),
           0x045: ("hinten links", "rear left"), 0x050: ("hinten rechts", "rear right")}
for base, (where_de, where_en) in _WHEELS.items():
    put(f"C{base:04X}", f"Raddrehzahlfühler {where_de} - Stromkreis",
        f"Wheel speed sensor {where_en} - circuit")
    put(f"C{base + 1:04X}", f"Raddrehzahlfühler {where_de} - Signal fehlt oder unplausibel",
        f"Wheel speed sensor {where_en} - signal missing or implausible")
    put(f"C{base + 2:04X}", f"Raddrehzahlfühler {where_de} - Signal setzt zeitweise aus",
        f"Wheel speed sensor {where_en} - signal intermittent")

_VALVES = {0x055: ("vorne links", "front left"), 0x060: ("vorne rechts", "front right"),
           0x065: ("hinten links", "rear left"), 0x070: ("hinten rechts", "rear right")}
for base, (where_de, where_en) in _VALVES.items():
    put(f"C{base:04X}", f"ABS-Einlassventil {where_de} - Stromkreis",
        f"ABS inlet valve {where_en} - circuit")
    put(f"C{base + 1:04X}", f"ABS-Auslassventil {where_de} - Stromkreis",
        f"ABS outlet valve {where_en} - circuit")

# --------------------------------------------------------------------------
# Family 7c: restraint systems (B0xxx)
# --------------------------------------------------------------------------
_AIRBAGS = {
    0x001: ("Fahrerairbag Stufe 1", "Driver airbag stage 1"),
    0x002: ("Fahrerairbag Stufe 2", "Driver airbag stage 2"),
    0x010: ("Beifahrerairbag Stufe 1", "Passenger airbag stage 1"),
    0x011: ("Beifahrerairbag Stufe 2", "Passenger airbag stage 2"),
    0x020: ("Seitenairbag vorne links", "Side airbag front left"),
    0x028: ("Seitenairbag vorne rechts", "Side airbag front right"),
    0x030: ("Kopfairbag links", "Curtain airbag left"),
    0x038: ("Kopfairbag rechts", "Curtain airbag right"),
    0x051: ("Gurtstraffer Fahrer", "Seat belt pretensioner driver"),
    0x052: ("Gurtstraffer Beifahrer", "Seat belt pretensioner passenger"),
}
for base, (what_de, what_en) in _AIRBAGS.items():
    put(f"B{base:04X}", f"{what_de} - Stromkreis", f"{what_en} - circuit")
    put(f"B{base + 3:04X}", f"{what_de} - Widerstand zu klein",
        f"{what_en} - resistance too low")
    put(f"B{base + 4:04X}", f"{what_de} - Widerstand zu groß",
        f"{what_en} - resistance too high")

# --------------------------------------------------------------------------
# Family 7: missing bus messages (U01xx)
# --------------------------------------------------------------------------
_MODULES = {
    0x100: ("Motorsteuergerät", "engine control unit"),
    0x101: ("Getriebesteuergerät", "transmission control unit"),
    0x102: ("Verteilergetriebe", "transfer case"),
    0x103: ("Schaltgetriebe", "gear shift module"),
    0x121: ("ABS-Steuergerät", "ABS control unit"),
    0x122: ("Fahrdynamiksensor", "vehicle dynamics sensor"),
    0x123: ("Gierratensensor", "yaw rate sensor"),
    0x124: ("Niveauregelung", "level control"),
    0x126: ("Lenkwinkelsensor", "steering angle sensor"),
    0x128: ("Feststellbremse", "parking brake"),
    0x131: ("Servolenkung", "power steering"),
    0x140: ("Bordnetzsteuergerät", "body control module"),
    0x151: ("Rückhaltesystem (Airbag)", "restraint system (airbag)"),
    0x155: ("Kombiinstrument", "instrument cluster"),
    0x164: ("Klimasteuergerät", "climate control unit"),
    0x184: ("Radio", "radio"),
    0x199: ("Türsteuergerät", "door control unit"),
    0x208: ("Reifendrucküberwachung", "tire pressure monitoring"),
}
for num, (name_de, name_en) in _MODULES.items():
    put(f"U{num:04X}", f"Keine Verbindung zum {name_de}",
        f"Lost communication with {name_en}")
    put(f"U{num + 0x300:04X}", f"Unplausible Daten vom {name_de}",
        f"Invalid data received from {name_en}")

# --------------------------------------------------------------------------
# Hand-maintained: everything that follows no rule
# --------------------------------------------------------------------------
MANUAL: dict[str, tuple[str, str]] = {
    # Mixture formation and air metering
    "P0068": ("Saugrohrdruck und Drosselklappenstellung passen nicht zusammen (Falschluft)",
              "Manifold pressure and throttle position do not match (vacuum leak)"),
    "P0070": ("Außentemperatursensor - Stromkreis",
              "Ambient air temperature sensor - circuit"),
    "P0087": ("Kraftstoffdruck im Rail zu niedrig (Pumpe, Filter, Druckregler)",
              "Fuel rail pressure too low (pump, filter, pressure regulator)"),
    "P0088": ("Kraftstoffdruck im Rail zu hoch", "Fuel rail pressure too high"),
    "P0089": ("Kraftstoffdruckregler - Funktion", "Fuel pressure regulator - performance"),
    "P0093": ("Kraftstoffsystem - großes Leck erkannt", "Fuel system - large leak detected"),
    "P0097": ("Ansauglufttemperatursensor 2 - Signal zu niedrig",
              "Intake air temperature sensor 2 - signal too low"),
    "P0100": ("Luftmassenmesser - Stromkreis", "Mass air flow sensor - circuit"),
    "P0101": ("Luftmassenmesser - Signal unplausibel (LMM defekt oder Falschluft)",
              "Mass air flow sensor - signal implausible (faulty MAF sensor or vacuum leak)"),
    "P0102": ("Luftmassenmesser - Signal zu niedrig", "Mass air flow sensor - signal too low"),
    "P0103": ("Luftmassenmesser - Signal zu hoch", "Mass air flow sensor - signal too high"),
    "P0105": ("Saugrohrdrucksensor - Stromkreis", "Intake manifold pressure sensor - circuit"),
    "P0106": ("Saugrohrdrucksensor - Signal unplausibel",
              "Intake manifold pressure sensor - signal implausible"),
    "P0107": ("Saugrohrdrucksensor - Signal zu niedrig",
              "Intake manifold pressure sensor - signal too low"),
    "P0108": ("Saugrohrdrucksensor - Signal zu hoch",
              "Intake manifold pressure sensor - signal too high"),
    "P0110": ("Ansauglufttemperatursensor - Stromkreis",
              "Intake air temperature sensor - circuit"),
    "P0111": ("Ansauglufttemperatursensor - Signal unplausibel",
              "Intake air temperature sensor - signal implausible"),
    "P0112": ("Ansauglufttemperatursensor - Signal zu niedrig",
              "Intake air temperature sensor - signal too low"),
    "P0113": ("Ansauglufttemperatursensor - Signal zu hoch",
              "Intake air temperature sensor - signal too high"),
    "P0115": ("Kühlmitteltemperatursensor - Stromkreis", "Coolant temperature sensor - circuit"),
    "P0116": ("Kühlmitteltemperatursensor - Signal unplausibel",
              "Coolant temperature sensor - signal implausible"),
    "P0117": ("Kühlmitteltemperatursensor - Signal zu niedrig",
              "Coolant temperature sensor - signal too low"),
    "P0118": ("Kühlmitteltemperatursensor - Signal zu hoch (klassisch: Geber G62 defekt)",
              "Coolant temperature sensor - signal too high (classic: sender G62 faulty)"),
    "P0120": ("Drosselklappenpotentiometer oder Fahrpedalgeber - Stromkreis",
              "Throttle position sensor or accelerator pedal sensor - circuit"),
    "P0121": ("Drosselklappenpotentiometer - Signal unplausibel",
              "Throttle position sensor - signal implausible"),
    "P0122": ("Drosselklappenpotentiometer - Signal zu niedrig",
              "Throttle position sensor - signal too low"),
    "P0123": ("Drosselklappenpotentiometer - Signal zu hoch",
              "Throttle position sensor - signal too high"),
    "P0125": ("Kühlmittel wird nicht warm genug für den Regelbetrieb (Thermostat)",
              "Coolant does not get warm enough for closed-loop control (thermostat)"),
    "P0128": ("Thermostat - Kühlmittel erreicht Solltemperatur nicht",
              "Thermostat - coolant does not reach operating temperature"),
    "P0170": ("Gemischadaption Bank 1 - Fehlfunktion", "Fuel trim bank 1 - malfunction"),
    "P0171": ("Gemisch Bank 1 zu mager (Falschluft, LMM, Kurbelgehäuseentlüftung)",
              "System too lean, bank 1 (vacuum leak, MAF sensor, crankcase ventilation)"),
    "P0172": ("Gemisch Bank 1 zu fett (Einspritzventile, Kraftstoffdruck, LMM)",
              "System too rich, bank 1 (injectors, fuel pressure, MAF sensor)"),
    "P0173": ("Gemischadaption Bank 2 - Fehlfunktion", "Fuel trim bank 2 - malfunction"),
    "P0174": ("Gemisch Bank 2 zu mager", "System too lean, bank 2"),
    "P0175": ("Gemisch Bank 2 zu fett", "System too rich, bank 2"),
    "P0180": ("Kraftstofftemperatursensor - Stromkreis", "Fuel temperature sensor - circuit"),
    "P0190": ("Kraftstoffdrucksensor - Stromkreis", "Fuel pressure sensor - circuit"),
    "P0191": ("Kraftstoffdrucksensor - Signal unplausibel",
              "Fuel pressure sensor - signal implausible"),
    # Boost pressure
    "P0221": ("Drosselklappensteller oder Fahrpedalgeber 2 - Signal unplausibel",
              "Throttle actuator or accelerator pedal sensor 2 - signal implausible"),
    "P0222": ("Drosselklappensteller Poti 2 - Signal zu niedrig",
              "Throttle actuator potentiometer 2 - signal too low"),
    "P0223": ("Drosselklappensteller Poti 2 - Signal zu hoch",
              "Throttle actuator potentiometer 2 - signal too high"),
    "P0234": ("Ladedruck zu hoch (Überladung, Ladedrucksteller hängt)",
              "Boost pressure too high (overboost, boost pressure actuator stuck)"),
    "P0235": ("Ladedrucksensor - Stromkreis", "Boost pressure sensor - circuit"),
    "P0236": ("Ladedrucksensor - Signal unplausibel",
              "Boost pressure sensor - signal implausible"),
    "P0238": ("Ladedrucksensor - Signal zu hoch", "Boost pressure sensor - signal too high"),
    "P0243": ("Magnetventil Ladedruckbegrenzung - Stromkreis",
              "Boost pressure control solenoid valve - circuit"),
    "P0245": ("Magnetventil Ladedruckbegrenzung - Signal zu niedrig",
              "Boost pressure control solenoid valve - signal too low"),
    "P0246": ("Magnetventil Ladedruckbegrenzung - Signal zu hoch",
              "Boost pressure control solenoid valve - signal too high"),
    "P0299": ("Ladedruck zu niedrig (Ladedruckdose, Unterdruckschläuche, VTG hängt)",
              "Boost pressure too low (wastegate actuator, vacuum hoses, VTG stuck)"),
    # Ignition and engine speed sensors
    "P0313": ("Aussetzer bei niedrigem Kraftstoffstand", "Misfire detected with low fuel level"),
    "P0321": ("Drehzahlgeber - Signal unplausibel", "Engine speed sensor - signal implausible"),
    "P0322": ("Drehzahlgeber - kein Signal", "Engine speed sensor - no signal"),
    "P0325": ("Klopfsensor 1 - Stromkreis", "Knock sensor 1 - circuit"),
    "P0327": ("Klopfsensor 1 - Signal zu niedrig", "Knock sensor 1 - signal too low"),
    "P0328": ("Klopfsensor 1 - Signal zu hoch", "Knock sensor 1 - signal too high"),
    "P0330": ("Klopfsensor 2 - Stromkreis", "Knock sensor 2 - circuit"),
    "P0335": ("Kurbelwellengeber - Stromkreis", "Crankshaft position sensor - circuit"),
    "P0336": ("Kurbelwellengeber - Signal unplausibel",
              "Crankshaft position sensor - signal implausible"),
    "P0340": ("Nockenwellengeber Bank 1 - Stromkreis",
              "Camshaft position sensor bank 1 - circuit"),
    "P0341": ("Nockenwellengeber Bank 1 - Signal unplausibel (Steuerzeiten prüfen)",
              "Camshaft position sensor bank 1 - signal implausible (check valve timing)"),
    "P0344": ("Nockenwellengeber Bank 1 - Signal setzt aus",
              "Camshaft position sensor bank 1 - signal intermittent"),
    "P0345": ("Nockenwellengeber Bank 2 - Stromkreis",
              "Camshaft position sensor bank 2 - circuit"),
    # Exhaust
    "P0400": ("Abgasrückführung - Funktion", "Exhaust gas recirculation - malfunction"),
    "P0401": ("Abgasrückführung - Durchfluss zu gering (AGR-Ventil verrußt)",
              "Exhaust gas recirculation - flow insufficient (EGR valve sooted up)"),
    "P0402": ("Abgasrückführung - Durchfluss zu hoch",
              "Exhaust gas recirculation - flow excessive"),
    "P0403": ("Abgasrückführventil - Stromkreis", "EGR valve - circuit"),
    "P0404": ("Abgasrückführung - Signal unplausibel",
              "Exhaust gas recirculation - signal implausible"),
    "P0405": ("Abgasrückführung Positionssensor - Signal zu niedrig",
              "EGR position sensor - signal too low"),
    "P0409": ("Abgasrückführung Positionssensor - Stromkreis", "EGR position sensor - circuit"),
    "P0410": ("Sekundärluftsystem - Funktion", "Secondary air injection system - malfunction"),
    "P0411": ("Sekundärluftsystem - Durchfluss falsch",
              "Secondary air injection system - incorrect flow"),
    "P0412": ("Sekundärluftventil - Stromkreis", "Secondary air valve - circuit"),
    "P0420": ("Katalysator Bank 1 - Wirkungsgrad zu gering",
              "Catalytic converter bank 1 - efficiency below threshold"),
    "P0421": ("Vorkatalysator Bank 1 - Wirkungsgrad zu gering",
              "Warm-up catalytic converter bank 1 - efficiency below threshold"),
    "P0430": ("Katalysator Bank 2 - Wirkungsgrad zu gering",
              "Catalytic converter bank 2 - efficiency below threshold"),
    "P0431": ("Vorkatalysator Bank 2 - Wirkungsgrad zu gering",
              "Warm-up catalytic converter bank 2 - efficiency below threshold"),
    "P0440": ("Tankentlüftung - Funktion", "EVAP system - malfunction"),
    "P0441": ("Tankentlüftung - Durchfluss falsch", "EVAP system - incorrect purge flow"),
    "P0442": ("Tankentlüftung - kleines Leck (oft nur der Tankdeckel)",
              "EVAP system - small leak (often just the fuel cap)"),
    "P0443": ("Magnetventil Tankentlüftung - Stromkreis", "EVAP purge solenoid valve - circuit"),
    "P0444": ("Magnetventil Tankentlüftung - Unterbrechung",
              "EVAP purge solenoid valve - open circuit"),
    "P0445": ("Magnetventil Tankentlüftung - Kurzschluss",
              "EVAP purge solenoid valve - short circuit"),
    "P0446": ("Tankentlüftung Belüftung - Funktion", "EVAP vent control - malfunction"),
    "P0455": ("Tankentlüftung - großes Leck (Tankdeckel, Schlauch)",
              "EVAP system - large leak (fuel cap, hose)"),
    "P0456": ("Tankentlüftung - sehr kleines Leck", "EVAP system - very small leak"),
    "P0480": ("Kühlerlüfter-Relais 1 - Stromkreis", "Cooling fan relay 1 - circuit"),
    "P0481": ("Kühlerlüfter-Relais 2 - Stromkreis", "Cooling fan relay 2 - circuit"),
    # Diesel and particulate filter
    "P0470": ("Abgasdrucksensor - Stromkreis", "Exhaust pressure sensor - circuit"),
    "P0471": ("Abgasdrucksensor - Signal unplausibel",
              "Exhaust pressure sensor - signal implausible"),
    "P046C": ("Differenzdrucksensor Partikelfilter - Signal unplausibel",
              "Particulate filter differential pressure sensor - signal implausible"),
    "P2002": ("Partikelfilter Bank 1 - Wirkungsgrad zu gering",
              "Particulate filter bank 1 - efficiency below threshold"),
    "P2003": ("Partikelfilter Bank 2 - Wirkungsgrad zu gering",
              "Particulate filter bank 2 - efficiency below threshold"),
    "P2452": ("Differenzdrucksensor Partikelfilter - Stromkreis",
              "Particulate filter differential pressure sensor - circuit"),
    "P2453": ("Differenzdrucksensor Partikelfilter - Signal unplausibel",
              "Particulate filter differential pressure sensor - signal implausible"),
    "P2454": ("Differenzdrucksensor Partikelfilter - Signal zu niedrig",
              "Particulate filter differential pressure sensor - signal too low"),
    "P2463": ("Partikelfilter - Rußbeladung zu hoch (Regeneration fällig)",
              "Particulate filter - soot load too high (regeneration due)"),
    "P244A": ("Partikelfilter - Differenzdruck zu niedrig",
              "Particulate filter - differential pressure too low"),
    "P244B": ("Partikelfilter - Differenzdruck zu hoch",
              "Particulate filter - differential pressure too high"),
    "P20EE": ("SCR-Katalysator - Wirkungsgrad zu gering (AdBlue)",
              "SCR catalyst - efficiency below threshold (AdBlue)"),
    "P204F": ("SCR-System - Abweichung", "SCR system - deviation"),
    "P2BAC": ("Stickoxid-Grenzwert überschritten", "NOx limit exceeded"),
    # Idle, auxiliary units, electrical system
    "P0500": ("Geschwindigkeitssignal - kein Signal", "Vehicle speed signal - no signal"),
    "P0501": ("Geschwindigkeitssignal - unplausibel", "Vehicle speed signal - implausible"),
    "P0505": ("Leerlaufregelung - Fehlfunktion (Drosselklappe verschmutzt)",
              "Idle control - malfunction (throttle body dirty)"),
    "P0506": ("Leerlaufdrehzahl zu niedrig (Drosselklappe oder Anlernwert)",
              "Idle speed too low (throttle body or learned value)"),
    "P0507": ("Leerlaufdrehzahl zu hoch (Falschluft oder Drosselklappe nicht angelernt)",
              "Idle speed too high (vacuum leak or throttle body not adapted)"),
    "P0510": ("Leerlaufschalter - Stromkreis", "Idle switch - circuit"),
    "P0520": ("Öldrucksensor - Stromkreis", "Oil pressure sensor - circuit"),
    "P0522": ("Öldruck zu niedrig", "Oil pressure too low"),
    "P0530": ("Klimadrucksensor - Stromkreis", "A/C pressure sensor - circuit"),
    "P0560": ("Bordspannung - unplausibel", "Battery voltage - implausible"),
    "P0562": ("Bordspannung zu niedrig (Batterie oder Lichtmaschine)",
              "Battery voltage too low (battery or alternator)"),
    "P0563": ("Bordspannung zu hoch", "Battery voltage too high"),
    "P0571": ("Bremslichtschalter - Stromkreis", "Brake light switch - circuit"),
    "P0575": ("Tempomat - Signal unplausibel", "Cruise control - signal implausible"),
    "P0597": ("Thermostat-Heizung - Stromkreis", "Thermostat heater - circuit"),
    # Control unit
    "P0600": ("Datenbus - Fehlfunktion", "Data bus - malfunction"),
    "P0601": ("Steuergerät - Prüfsummenfehler im Speicher",
              "Control unit - memory checksum error"),
    "P0602": ("Steuergerät - Programmierfehler", "Control unit - programming error"),
    "P0604": ("Steuergerät - RAM-Fehler", "Control unit - RAM error"),
    "P0605": ("Steuergerät - ROM-Fehler", "Control unit - ROM error"),
    "P0606": ("Steuergerät - Prozessorfehler", "Control unit - processor fault"),
    "P0627": ("Kraftstoffpumpen-Relais - Stromkreis", "Fuel pump relay - circuit"),
    "P0638": ("Drosselklappensteller - Regelgrenze überschritten (anlernen fällig)",
              "Throttle actuator - control limit exceeded (adaptation due)"),
    "P0645": ("Klimakompressor-Relais - Stromkreis", "A/C compressor relay - circuit"),
    "P0685": ("Hauptrelais Steuergerät - Stromkreis", "Control unit main relay - circuit"),
    # Transmission
    "P0700": ("Getriebesteuerung - Fehler hinterlegt (Getriebespeicher auslesen)",
              "Transmission control - fault stored (read the transmission fault memory)"),
    "P0701": ("Getriebesteuerung - Funktionsbereich", "Transmission control - range/performance"),
    "P0702": ("Getriebesteuerung - elektrischer Fehler",
              "Transmission control - electrical fault"),
    "P0705": ("Wählhebelsensor - Stromkreis", "Selector lever sensor - circuit"),
    "P0710": ("Getriebeoeltemperatursensor - Stromkreis",
              "Transmission fluid temperature sensor - circuit"),
    "P0715": ("Turbinendrehzahlsensor - Stromkreis", "Turbine speed sensor - circuit"),
    "P0720": ("Abtriebsdrehzahlsensor - Stromkreis", "Output speed sensor - circuit"),
    "P0730": ("Falsches Übersetzungsverhältnis", "Incorrect gear ratio"),
    "P0741": ("Wandlerüberbrückungskupplung - Schlupf", "Torque converter clutch - slipping"),
    "P0750": ("Schaltmagnetventil A - Stromkreis", "Shift solenoid A - circuit"),
    "P0755": ("Schaltmagnetventil B - Stromkreis", "Shift solenoid B - circuit"),
    "P0776": ("Druckregelventil B - Funktion", "Pressure control valve B - performance"),
    # Network
    "U0001": ("CAN-Bus - Fehlfunktion", "CAN bus - malfunction"),
    "U0002": ("CAN-Bus Antrieb - Funktionsfehler", "Powertrain CAN bus - performance fault"),
    "U0073": ("Steuergerät-Kommunikation Bus deaktiviert",
              "Control unit communication bus off"),
    "U0074": ("Kommunikationsbus 2 deaktiviert", "Communication bus 2 off"),
    "U0140": ("Keine Verbindung zum Bordnetzsteuergerät",
              "Lost communication with body control module"),
    # VAG-specific codes
    "P1102": ("Lambdasondenheizung vor Kat - Widerstand zu klein",
              "O2 sensor heater upstream of catalyst - resistance too low"),
    "P1105": ("Lambdasondenheizung nach Kat - Kurzschluss nach Plus",
              "O2 sensor heater downstream of catalyst - short to positive"),
    "P1128": ("Gemischadaption Teillast - Regelgrenze unterschritten (zu mager)",
              "Fuel trim at part load - below control limit (too lean)"),
    "P1129": ("Gemischadaption Teillast - Regelgrenze überschritten (zu fett)",
              "Fuel trim at part load - above control limit (too rich)"),
    "P1136": ("Gemischadaption Leerlauf - Regelgrenze unterschritten (Falschluft)",
              "Fuel trim at idle - below control limit (vacuum leak)"),
    "P1137": ("Gemischadaption Leerlauf - Regelgrenze überschritten",
              "Fuel trim at idle - above control limit"),
    "P1196": ("Lambdasondenheizung vor Kat - Stromkreis",
              "O2 sensor heater upstream of catalyst - circuit"),
    "P1296": ("Kühlsystem - Fehlfunktion (Thermostat oder Kühlmitteltemperatur)",
              "Cooling system - malfunction (thermostat or coolant temperature)"),
    "P1325": ("Klopfregelung Zylinder 1 - Regelgrenze",
              "Knock control cylinder 1 - control limit reached"),
    "P1340": ("Kurbel- und Nockenwellengeber zueinander unplausibel",
              "Crankshaft and camshaft position sensors implausible relative to each other"),
    "P1386": ("Steuergerät - interner Klopfsensortest fehlerhaft",
              "Control unit - internal knock sensor test failed"),
    "P1410": ("Tankentlüftungsventil - Kurzschluss nach Plus",
              "EVAP purge valve - short to positive"),
    "P1425": ("Tankentlüftungsventil - Kurzschluss nach Masse",
              "EVAP purge valve - short to ground"),
    "P1500": ("Kraftstoffpumpen-Relais - Stromkreis", "Fuel pump relay - circuit"),
    "P1543": ("Saugrohrdruck - Signal zu klein", "Intake manifold pressure - signal too low"),
    "P1545": ("Drosselklappensteuerung - Fehlfunktion", "Throttle body control - malfunction"),
    "P1546": ("Ladedruck-Magnetventil - Kurzschluss nach Plus",
              "Boost pressure solenoid valve - short to positive"),
    "P1547": ("Ladedruck-Magnetventil - Kurzschluss nach Masse",
              "Boost pressure solenoid valve - short to ground"),
    "P1550": ("Ladedruck - Regeldifferenz", "Boost pressure - control deviation"),
    "P1555": ("Ladedruck-Obergrenze überschritten", "Boost pressure upper limit exceeded"),
    "P1556": ("Ladedruckregelung - Regelgrenze unterschritten",
              "Boost pressure control - below control limit"),
    "P1557": ("Ladedruckregelung - Regelgrenze überschritten",
              "Boost pressure control - above control limit"),
    "P1558": ("Drosselklappensteller - elektrische Fehlfunktion",
              "Throttle actuator - electrical malfunction"),
    "P1559": ("Leerlaufdrehzahlregelung - Drosselklappenanpassung fehlerhaft (anlernen)",
              "Idle speed control - throttle body adaptation failed (run the adaptation)"),
    "P1560": ("Maximale Motordrehzahl überschritten", "Maximum engine speed exceeded"),
    "P1568": ("Leerlaufregelung - mechanische Fehlfunktion (Drosselklappe klemmt)",
              "Idle control - mechanical malfunction (throttle body sticking)"),
    "P1569": ("Tempomatschalter - Signal unplausibel", "Cruise control switch - signal implausible"),
    "P1570": ("Steuergerät durch Wegfahrsperre gesperrt", "Control unit locked by immobilizer"),
    "P1602": ("Spannungsversorgung Klemme 30 - Spannung zu klein",
              "Power supply terminal 30 - voltage too low"),
    "P1606": ("Fahrsignal rauhe Straße - Signal unplausibel",
              "Rough road signal - signal implausible"),
    "P1611": ("MIL-Anforderung Getriebesteuergerät - Kurzschluss nach Masse",
              "MIL request from transmission control unit - short to ground"),
    "P1613": ("MIL-Anforderung - Unterbrechung oder Kurzschluss nach Plus",
              "MIL request - open circuit or short to positive"),
    "P1624": ("MIL-Anforderung aktiv (Fehler in einem anderen Steuergerät)",
              "MIL request active (fault in another control unit)"),
    "P1626": ("Datenbus Antrieb - fehlende Botschaft vom Getriebesteuergerät",
              "Powertrain data bus - missing message from transmission control unit"),
    "P1630": ("Fahrpedalgeber 1 - Signal zu klein", "Accelerator pedal sensor 1 - signal too low"),
    "P1631": ("Fahrpedalgeber 1 - Signal zu groß", "Accelerator pedal sensor 1 - signal too high"),
    "P1633": ("Fahrpedalgeber 2 - Signal zu klein", "Accelerator pedal sensor 2 - signal too low"),
    "P1634": ("Fahrpedalgeber 2 - Signal zu groß", "Accelerator pedal sensor 2 - signal too high"),
    "P1639": ("Fahrpedalgeber 1 und 2 - Signale unplausibel",
              "Accelerator pedal sensors 1 and 2 - signals implausible"),
    "P1640": ("Steuergerät EEPROM - Fehler", "Control unit EEPROM - error"),
    "P1648": ("Datenbus Antrieb - Fehlfunktion", "Powertrain data bus - malfunction"),
    "P1649": ("Datenbus Antrieb - fehlende Botschaft vom ABS",
              "Powertrain data bus - missing message from ABS"),
    "P1653": ("Datenbus Antrieb - fehlende Botschaft vom Kombiinstrument",
              "Powertrain data bus - missing message from instrument cluster"),
    "P1690": ("Fehlerlampe - Fehlfunktion", "Malfunction indicator lamp - malfunction"),
    "P1780": ("Motoreingriff Getriebe - nicht plausibel",
              "Engine intervention by transmission - implausible"),
    # Chassis (C) and body (B), generic part
    "C0035": ("Raddrehzahlfühler vorne links - Stromkreis",
              "Wheel speed sensor front left - circuit"),
    "C0040": ("Raddrehzahlfühler vorne rechts - Stromkreis",
              "Wheel speed sensor front right - circuit"),
    "C0045": ("Raddrehzahlfühler hinten links - Stromkreis",
              "Wheel speed sensor rear left - circuit"),
    "C0050": ("Raddrehzahlfühler hinten rechts - Stromkreis",
              "Wheel speed sensor rear right - circuit"),
    "C0110": ("ABS-Pumpenmotor - Stromkreis", "ABS pump motor - circuit"),
    "C0121": ("ABS-Ventilrelais - Stromkreis", "ABS valve relay - circuit"),
    "C0161": ("ABS und Bremslichtschalter - Stromkreis", "ABS and brake light switch - circuit"),
    "C0265": ("ABS-Steuergerät Relais - Stromkreis", "ABS control unit relay - circuit"),
    "C1095": ("ABS-Hydraulikpumpe - Funktion", "ABS hydraulic pump - malfunction"),
    "B0001": ("Fahrerairbag - Stromkreis", "Driver airbag - circuit"),
    "B0002": ("Fahrerairbag Stufe 2 - Stromkreis", "Driver airbag stage 2 - circuit"),
    "B0010": ("Beifahrerairbag - Stromkreis", "Passenger airbag - circuit"),
    "B0020": ("Seitenairbag links - Stromkreis", "Side airbag left - circuit"),
    "B0028": ("Seitenairbag rechts - Stromkreis", "Side airbag right - circuit"),
    "B0051": ("Gurtstraffer Fahrer - Stromkreis", "Seat belt pretensioner driver - circuit"),
    "B0052": ("Gurtstraffer Beifahrer - Stromkreis",
              "Seat belt pretensioner passenger - circuit"),
}
for code, texts in MANUAL.items():
    CODES[code] = texts

# Codes where the throttle body is the usual suspect.
THROTTLE.update({
    "P0068", "P0120", "P0121", "P0122", "P0123", "P0220", "P0221", "P0222",
    "P0223", "P0225", "P0226", "P0227", "P0228", "P0505", "P0506", "P0507",
    "P0638", "P0639", "P1545", "P1558", "P1559", "P1568", "P2100", "P2101",
    "P2102", "P2103", "P2135", "P2138",
})

# Without these, the P2xxx throttle body codes would have no description.
for code, texts in {
    "P2100": ("Drosselklappensteller Motor - Stromkreis unterbrochen",
              "Throttle actuator motor - open circuit"),
    "P2101": ("Drosselklappensteller Motor - Regelbereich nicht erreicht",
              "Throttle actuator motor - control range not reached"),
    "P2102": ("Drosselklappensteller Motor - Kurzschluss nach Masse",
              "Throttle actuator motor - short to ground"),
    "P2103": ("Drosselklappensteller Motor - Kurzschluss nach Plus",
              "Throttle actuator motor - short to positive"),
    "P2135": ("Drosselklappenpotentiometer 1 und 2 - Signale unplausibel",
              "Throttle position sensors 1 and 2 - signals implausible"),
    "P2138": ("Fahrpedalgeber 1 und 2 - Signale unplausibel",
              "Accelerator pedal sensors 1 and 2 - signals implausible"),
    "P0220": ("Drosselklappensensor 2 - Stromkreis", "Throttle position sensor 2 - circuit"),
    "P0225": ("Drosselklappensensor 3 - Stromkreis", "Throttle position sensor 3 - circuit"),
    "P0226": ("Drosselklappensensor 3 - Signal unplausibel",
              "Throttle position sensor 3 - signal implausible"),
    "P0639": ("Drosselklappensteller Bank 2 - Regelgrenze überschritten",
              "Throttle actuator bank 2 - control limit exceeded"),
}.items():
    CODES[code] = texts


def _quote(text: str) -> str:
    return '"' + text.replace("\\", "\\\\").replace('"', '\\"') + '"'


def main() -> None:
    for code, (de, en) in CODES.items():
        assert de and en, f"{code}: German and English text required"

    here = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    target = os.path.join(here, "src", "data", "DtcDatabase.gen.cpp")

    lines = [
        "// Generated by tools/gen_dtc.py. Do not edit by hand.",
        f"// {len(CODES)} trouble codes, {len(THROTTLE)} of them throttle-body related.",
        "",
        '#include "data/DtcDatabase.h"',
        "",
        "#include <unordered_map>",
        "#include <unordered_set>",
        "",
        "namespace caruspex {",
        "namespace {",
        "",
        "// Code -> {German, English}.",
        "const std::unordered_map<std::string, Text> kDescriptions = {",
    ]
    for code in sorted(CODES):
        de, en = CODES[code]
        lines.append(f'    {{"{code}", {{{_quote(de)}, {_quote(en)}}}}},')
    lines += [
        "};",
        "",
        "const std::unordered_set<std::string> kThrottleCodes = {",
    ]
    for code in sorted(THROTTLE):
        lines.append(f'    "{code}",')
    lines += [
        "};",
        "",
        "}  // namespace",
        "",
        "const std::unordered_map<std::string, Text>& dtcTexts() {",
        "    return kDescriptions;",
        "}",
        "",
        "const std::unordered_set<std::string>& dtcThrottleCodes() {",
        "    return kThrottleCodes;",
        "}",
        "",
        "}  // namespace caruspex",
        "",
    ]
    io.open(target, "w", encoding="utf-8", newline="\n").write("\n".join(lines))
    print(f"{target}\n{len(CODES)} codes, {len(THROTTLE)} throttle body codes")


if __name__ == "__main__":
    main()
