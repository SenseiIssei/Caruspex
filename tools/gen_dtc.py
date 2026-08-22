"""Erzeugt src/data/DtcDatabase.gen.cpp.

Viele EOBD-Codes folgen festen Familien (pro Zylinder, pro Bank, pro Sonde).
Die werden hier berechnet statt abgetippt, das vermeidet Tippfehler und deckt
Motoren bis zwoelf Zylinder ab. Dazu kommt eine handgepflegte Liste fuer alles,
was keiner Regel folgt, inklusive der VAG-eigenen P1xxx-Codes.

    python tools/gen_dtc.py
"""

from __future__ import annotations

import io
import os

CODES: dict[str, str] = {}
THROTTLE: set[str] = set()


def put(code: str, text: str) -> None:
    CODES.setdefault(code, text)


# --------------------------------------------------------------------------
# Familie 1: Lambdasonden-Heizung (P0030 bis P0064)
# --------------------------------------------------------------------------
_HEATER_BASE = {(1, 1): 0x030, (1, 2): 0x036, (1, 3): 0x042,
                (2, 1): 0x050, (2, 2): 0x056, (2, 3): 0x062}
for (bank, sensor), base in _HEATER_BASE.items():
    where = f"Bank {bank}, Sonde {sensor}"
    put(f"P{base:04X}", f"Lambdasondenheizung {where} - Stromkreis")
    put(f"P{base + 1:04X}", f"Lambdasondenheizung {where} - Kurzschluss nach Masse")
    put(f"P{base + 2:04X}", f"Lambdasondenheizung {where} - Kurzschluss nach Plus")

# --------------------------------------------------------------------------
# Familie 2: Lambdasonden-Signal (P0130 bis P0167)
# --------------------------------------------------------------------------
_O2_BASE = {(1, 1): 0x130, (1, 2): 0x136, (1, 3): 0x142,
            (2, 1): 0x150, (2, 2): 0x156, (2, 3): 0x162}
_O2_SUFFIX = [
    "Stromkreis",
    "Spannung zu niedrig (Gemisch zu mager oder Falschluft)",
    "Spannung zu hoch (Gemisch zu fett)",
    "Regelung zu traege, Sonde reagiert langsam",
    "kein Signal, keine Aktivitaet",
    "Heizkreis - Funktion",
]
for (bank, sensor), base in _O2_BASE.items():
    where = f"Bank {bank}, Sonde {sensor}"
    for offset, text in enumerate(_O2_SUFFIX):
        put(f"P{base + offset:04X}", f"Lambdasonde {where} - {text}")

# --------------------------------------------------------------------------
# Familie 3: Einspritzventile (P0201 bis P0212 und P0261 bis P0296)
# --------------------------------------------------------------------------
for cyl in range(1, 13):
    put(f"P{0x201 + cyl - 1:04X}", f"Einspritzventil Zylinder {cyl} - Stromkreis")
    base = 0x261 + (cyl - 1) * 3
    put(f"P{base:04X}", f"Einspritzventil Zylinder {cyl} - Kurzschluss nach Masse")
    put(f"P{base + 1:04X}", f"Einspritzventil Zylinder {cyl} - Kurzschluss nach Plus")
    put(f"P{base + 2:04X}", f"Zylinder {cyl} - Beitrag zur Verbrennung zu gering")

# --------------------------------------------------------------------------
# Familie 4: Verbrennungsaussetzer (P0300 bis P0312)
# --------------------------------------------------------------------------
put("P0300", "Verbrennungsaussetzer - mehrere oder wechselnde Zylinder")
for cyl in range(1, 13):
    put(f"P{0x300 + cyl:04X}", f"Verbrennungsaussetzer Zylinder {cyl}")

# --------------------------------------------------------------------------
# Familie 5: Zuendspulen (P0350 bis P0362)
# --------------------------------------------------------------------------
put("P0350", "Zuendspule - Primaer- oder Sekundaerkreis")
for cyl in range(1, 13):
    put(f"P{0x350 + cyl:04X}",
        f"Zuendspule Zylinder {cyl} - Primaer- oder Sekundaerkreis")

# --------------------------------------------------------------------------
# Familie 6: Nockenwellenverstellung (P0010 bis P0028)
# --------------------------------------------------------------------------
_CAM = {0x010: ("Einlass", 1), 0x013: ("Auslass", 1),
        0x020: ("Einlass", 2), 0x023: ("Auslass", 2)}
for base, (side, bank) in _CAM.items():
    put(f"P{base:04X}", f"Nockenwellenversteller {side} Bank {bank} - Stromkreis")
    put(f"P{base + 1:04X}",
        f"Steuerzeiten {side} Bank {bank} - zu weit in Richtung frueh "
        f"(Versteller haengt oder Kette gelaengt)")
    put(f"P{base + 2:04X}",
        f"Steuerzeiten {side} Bank {bank} - zu weit in Richtung spaet")

# --------------------------------------------------------------------------
# Familie 7: Fehlende Busbotschaften (U01xx)
# --------------------------------------------------------------------------
_MODULES = {
    0x100: "Motorsteuergeraet", 0x101: "Getriebesteuergeraet",
    0x102: "Verteilergetriebe", 0x103: "Schaltgetriebe",
    0x121: "ABS-Steuergeraet", 0x122: "Fahrdynamiksensor",
    0x123: "Gierratensensor", 0x124: "Niveauregelung",
    0x126: "Lenkwinkelsensor", 0x128: "Feststellbremse",
    0x131: "Servolenkung", 0x140: "Bordnetzsteuergeraet",
    0x151: "Rueckhaltesystem (Airbag)", 0x155: "Kombiinstrument",
    0x164: "Klimasteuergeraet", 0x184: "Radio",
    0x199: "Tuersteuergeraet", 0x208: "Reifendruckueberwachung",
}
for num, name in _MODULES.items():
    put(f"U{num:04X}", f"Keine Verbindung zum {name}")
    put(f"U{num + 0x300:04X}", f"Unplausible Daten vom {name}")

# --------------------------------------------------------------------------
# Handgepflegt: alles, was keiner Regel folgt
# --------------------------------------------------------------------------
MANUAL = {
    # Gemischbildung und Luftmessung
    "P0068": "Saugrohrdruck und Drosselklappenstellung passen nicht zusammen (Falschluft)",
    "P0070": "Aussentemperatursensor - Stromkreis",
    "P0087": "Kraftstoffdruck im Rail zu niedrig (Pumpe, Filter, Druckregler)",
    "P0088": "Kraftstoffdruck im Rail zu hoch",
    "P0089": "Kraftstoffdruckregler - Funktion",
    "P0093": "Kraftstoffsystem - grosses Leck erkannt",
    "P0097": "Ansauglufttemperatursensor 2 - Signal zu niedrig",
    "P0100": "Luftmassenmesser - Stromkreis",
    "P0101": "Luftmassenmesser - Signal unplausibel (LMM defekt oder Falschluft)",
    "P0102": "Luftmassenmesser - Signal zu niedrig",
    "P0103": "Luftmassenmesser - Signal zu hoch",
    "P0105": "Saugrohrdrucksensor - Stromkreis",
    "P0106": "Saugrohrdrucksensor - Signal unplausibel",
    "P0107": "Saugrohrdrucksensor - Signal zu niedrig",
    "P0108": "Saugrohrdrucksensor - Signal zu hoch",
    "P0110": "Ansauglufttemperatursensor - Stromkreis",
    "P0111": "Ansauglufttemperatursensor - Signal unplausibel",
    "P0112": "Ansauglufttemperatursensor - Signal zu niedrig",
    "P0113": "Ansauglufttemperatursensor - Signal zu hoch",
    "P0115": "Kuehlmitteltemperatursensor - Stromkreis",
    "P0116": "Kuehlmitteltemperatursensor - Signal unplausibel",
    "P0117": "Kuehlmitteltemperatursensor - Signal zu niedrig",
    "P0118": "Kuehlmitteltemperatursensor - Signal zu hoch (klassisch: Geber G62 defekt)",
    "P0120": "Drosselklappenpotentiometer oder Fahrpedalgeber - Stromkreis",
    "P0121": "Drosselklappenpotentiometer - Signal unplausibel",
    "P0122": "Drosselklappenpotentiometer - Signal zu niedrig",
    "P0123": "Drosselklappenpotentiometer - Signal zu hoch",
    "P0125": "Kuehlmittel wird nicht warm genug fuer den Regelbetrieb (Thermostat)",
    "P0128": "Thermostat - Kuehlmittel erreicht Solltemperatur nicht",
    "P0170": "Gemischadaption Bank 1 - Fehlfunktion",
    "P0171": "Gemisch Bank 1 zu mager (Falschluft, LMM, Kurbelgehaeuseentlueftung)",
    "P0172": "Gemisch Bank 1 zu fett (Einspritzventile, Kraftstoffdruck, LMM)",
    "P0173": "Gemischadaption Bank 2 - Fehlfunktion",
    "P0174": "Gemisch Bank 2 zu mager",
    "P0175": "Gemisch Bank 2 zu fett",
    "P0180": "Kraftstofftemperatursensor - Stromkreis",
    "P0190": "Kraftstoffdrucksensor - Stromkreis",
    "P0191": "Kraftstoffdrucksensor - Signal unplausibel",
    # Ladedruck
    "P0221": "Drosselklappensteller oder Fahrpedalgeber 2 - Signal unplausibel",
    "P0222": "Drosselklappensteller Poti 2 - Signal zu niedrig",
    "P0223": "Drosselklappensteller Poti 2 - Signal zu hoch",
    "P0234": "Ladedruck zu hoch (Ueberladung, Ladedrucksteller haengt)",
    "P0235": "Ladedrucksensor - Stromkreis",
    "P0236": "Ladedrucksensor - Signal unplausibel",
    "P0238": "Ladedrucksensor - Signal zu hoch",
    "P0243": "Magnetventil Ladedruckbegrenzung - Stromkreis",
    "P0245": "Magnetventil Ladedruckbegrenzung - Signal zu niedrig",
    "P0246": "Magnetventil Ladedruckbegrenzung - Signal zu hoch",
    "P0299": "Ladedruck zu niedrig (Ladedruckdose, Unterdruckschlaeuche, VTG haengt)",
    # Zuendung und Drehzahlgeber
    "P0313": "Aussetzer bei niedrigem Kraftstoffstand",
    "P0321": "Drehzahlgeber - Signal unplausibel",
    "P0322": "Drehzahlgeber - kein Signal",
    "P0325": "Klopfsensor 1 - Stromkreis",
    "P0327": "Klopfsensor 1 - Signal zu niedrig",
    "P0328": "Klopfsensor 1 - Signal zu hoch",
    "P0330": "Klopfsensor 2 - Stromkreis",
    "P0335": "Kurbelwellengeber - Stromkreis",
    "P0336": "Kurbelwellengeber - Signal unplausibel",
    "P0340": "Nockenwellengeber Bank 1 - Stromkreis",
    "P0341": "Nockenwellengeber Bank 1 - Signal unplausibel (Steuerzeiten pruefen)",
    "P0344": "Nockenwellengeber Bank 1 - Signal setzt aus",
    "P0345": "Nockenwellengeber Bank 2 - Stromkreis",
    # Abgas
    "P0400": "Abgasrueckfuehrung - Funktion",
    "P0401": "Abgasrueckfuehrung - Durchfluss zu gering (AGR-Ventil verrusst)",
    "P0402": "Abgasrueckfuehrung - Durchfluss zu hoch",
    "P0403": "Abgasrueckfuehrventil - Stromkreis",
    "P0404": "Abgasrueckfuehrung - Signal unplausibel",
    "P0405": "Abgasrueckfuehrung Positionssensor - Signal zu niedrig",
    "P0409": "Abgasrueckfuehrung Positionssensor - Stromkreis",
    "P0410": "Sekundaerluftsystem - Funktion",
    "P0411": "Sekundaerluftsystem - Durchfluss falsch",
    "P0412": "Sekundaerluftventil - Stromkreis",
    "P0420": "Katalysator Bank 1 - Wirkungsgrad zu gering",
    "P0421": "Vorkatalysator Bank 1 - Wirkungsgrad zu gering",
    "P0430": "Katalysator Bank 2 - Wirkungsgrad zu gering",
    "P0431": "Vorkatalysator Bank 2 - Wirkungsgrad zu gering",
    "P0440": "Tankentlueftung - Funktion",
    "P0441": "Tankentlueftung - Durchfluss falsch",
    "P0442": "Tankentlueftung - kleines Leck (oft nur der Tankdeckel)",
    "P0443": "Magnetventil Tankentlueftung - Stromkreis",
    "P0444": "Magnetventil Tankentlueftung - Unterbrechung",
    "P0445": "Magnetventil Tankentlueftung - Kurzschluss",
    "P0446": "Tankentlueftung Belueftung - Funktion",
    "P0455": "Tankentlueftung - grosses Leck (Tankdeckel, Schlauch)",
    "P0456": "Tankentlueftung - sehr kleines Leck",
    "P0480": "Kuehlerluefter-Relais 1 - Stromkreis",
    "P0481": "Kuehlerluefter-Relais 2 - Stromkreis",
    # Diesel und Partikelfilter
    "P0470": "Abgasdrucksensor - Stromkreis",
    "P0471": "Abgasdrucksensor - Signal unplausibel",
    "P046C": "Differenzdrucksensor Partikelfilter - Signal unplausibel",
    "P2002": "Partikelfilter Bank 1 - Wirkungsgrad zu gering",
    "P2003": "Partikelfilter Bank 2 - Wirkungsgrad zu gering",
    "P2452": "Differenzdrucksensor Partikelfilter - Stromkreis",
    "P2453": "Differenzdrucksensor Partikelfilter - Signal unplausibel",
    "P2454": "Differenzdrucksensor Partikelfilter - Signal zu niedrig",
    "P2463": "Partikelfilter - Russbeladung zu hoch (Regeneration faellig)",
    "P244A": "Partikelfilter - Differenzdruck zu niedrig",
    "P244B": "Partikelfilter - Differenzdruck zu hoch",
    "P20EE": "SCR-Katalysator - Wirkungsgrad zu gering (AdBlue)",
    "P204F": "SCR-System - Abweichung",
    "P2BAC": "Stickoxid-Grenzwert ueberschritten",
    # Leerlauf, Nebenaggregate, Bordnetz
    "P0500": "Geschwindigkeitssignal - kein Signal",
    "P0501": "Geschwindigkeitssignal - unplausibel",
    "P0505": "Leerlaufregelung - Fehlfunktion (Drosselklappe verschmutzt)",
    "P0506": "Leerlaufdrehzahl zu niedrig (Drosselklappe oder Anlernwert)",
    "P0507": "Leerlaufdrehzahl zu hoch (Falschluft oder Drosselklappe nicht angelernt)",
    "P0510": "Leerlaufschalter - Stromkreis",
    "P0520": "Oeldrucksensor - Stromkreis",
    "P0522": "Oeldruck zu niedrig",
    "P0530": "Klimadrucksensor - Stromkreis",
    "P0560": "Bordspannung - unplausibel",
    "P0562": "Bordspannung zu niedrig (Batterie oder Lichtmaschine)",
    "P0563": "Bordspannung zu hoch",
    "P0571": "Bremslichtschalter - Stromkreis",
    "P0575": "Tempomat - Signal unplausibel",
    "P0597": "Thermostat-Heizung - Stromkreis",
    # Steuergeraet
    "P0600": "Datenbus - Fehlfunktion",
    "P0601": "Steuergeraet - Pruefsummenfehler im Speicher",
    "P0602": "Steuergeraet - Programmierfehler",
    "P0604": "Steuergeraet - RAM-Fehler",
    "P0605": "Steuergeraet - ROM-Fehler",
    "P0606": "Steuergeraet - Prozessorfehler",
    "P0627": "Kraftstoffpumpen-Relais - Stromkreis",
    "P0638": "Drosselklappensteller - Regelgrenze ueberschritten (anlernen faellig)",
    "P0645": "Klimakompressor-Relais - Stromkreis",
    "P0685": "Hauptrelais Steuergeraet - Stromkreis",
    # Getriebe
    "P0700": "Getriebesteuerung - Fehler hinterlegt (Getriebespeicher auslesen)",
    "P0701": "Getriebesteuerung - Funktionsbereich",
    "P0702": "Getriebesteuerung - elektrischer Fehler",
    "P0705": "Waehlhebelsensor - Stromkreis",
    "P0710": "Getriebeoeltemperatursensor - Stromkreis",
    "P0715": "Turbinendrehzahlsensor - Stromkreis",
    "P0720": "Abtriebsdrehzahlsensor - Stromkreis",
    "P0730": "Falsches Uebersetzungsverhaeltnis",
    "P0741": "Wandlerueberbrueckungskupplung - Schlupf",
    "P0750": "Schaltmagnetventil A - Stromkreis",
    "P0755": "Schaltmagnetventil B - Stromkreis",
    "P0776": "Druckregelventil B - Funktion",
    # Netzwerk
    "U0001": "CAN-Bus - Fehlfunktion",
    "U0002": "CAN-Bus Antrieb - Funktionsfehler",
    "U0073": "Steuergeraet-Kommunikation Bus deaktiviert",
    "U0074": "Kommunikationsbus 2 deaktiviert",
    "U0140": "Keine Verbindung zum Bordnetzsteuergeraet",
    # VAG-eigene Codes
    "P1102": "Lambdasondenheizung vor Kat - Widerstand zu klein",
    "P1105": "Lambdasondenheizung nach Kat - Kurzschluss nach Plus",
    "P1128": "Gemischadaption Teillast - Regelgrenze unterschritten (zu mager)",
    "P1129": "Gemischadaption Teillast - Regelgrenze ueberschritten (zu fett)",
    "P1136": "Gemischadaption Leerlauf - Regelgrenze unterschritten (Falschluft)",
    "P1137": "Gemischadaption Leerlauf - Regelgrenze ueberschritten",
    "P1196": "Lambdasondenheizung vor Kat - Stromkreis",
    "P1296": "Kuehlsystem - Fehlfunktion (Thermostat oder Kuehlmitteltemperatur)",
    "P1325": "Klopfregelung Zylinder 1 - Regelgrenze",
    "P1340": "Kurbel- und Nockenwellengeber zueinander unplausibel",
    "P1386": "Steuergeraet - interner Klopfsensortest fehlerhaft",
    "P1410": "Tankentlueftungsventil - Kurzschluss nach Plus",
    "P1425": "Tankentlueftungsventil - Kurzschluss nach Masse",
    "P1500": "Kraftstoffpumpen-Relais - Stromkreis",
    "P1543": "Saugrohrdruck - Signal zu klein",
    "P1545": "Drosselklappensteuerung - Fehlfunktion",
    "P1546": "Ladedruck-Magnetventil - Kurzschluss nach Plus",
    "P1547": "Ladedruck-Magnetventil - Kurzschluss nach Masse",
    "P1550": "Ladedruck - Regeldifferenz",
    "P1555": "Ladedruck-Obergrenze ueberschritten",
    "P1556": "Ladedruckregelung - Regelgrenze unterschritten",
    "P1557": "Ladedruckregelung - Regelgrenze ueberschritten",
    "P1558": "Drosselklappensteller - elektrische Fehlfunktion",
    "P1559": "Leerlaufdrehzahlregelung - Drosselklappenanpassung fehlerhaft (anlernen)",
    "P1560": "Maximale Motordrehzahl ueberschritten",
    "P1568": "Leerlaufregelung - mechanische Fehlfunktion (Drosselklappe klemmt)",
    "P1569": "Tempomatschalter - Signal unplausibel",
    "P1570": "Steuergeraet durch Wegfahrsperre gesperrt",
    "P1602": "Spannungsversorgung Klemme 30 - Spannung zu klein",
    "P1606": "Fahrsignal rauhe Strasse - Signal unplausibel",
    "P1611": "MIL-Anforderung Getriebesteuergeraet - Kurzschluss nach Masse",
    "P1613": "MIL-Anforderung - Unterbrechung oder Kurzschluss nach Plus",
    "P1624": "MIL-Anforderung aktiv (Fehler in einem anderen Steuergeraet)",
    "P1626": "Datenbus Antrieb - fehlende Botschaft vom Getriebesteuergeraet",
    "P1630": "Fahrpedalgeber 1 - Signal zu klein",
    "P1631": "Fahrpedalgeber 1 - Signal zu gross",
    "P1633": "Fahrpedalgeber 2 - Signal zu klein",
    "P1634": "Fahrpedalgeber 2 - Signal zu gross",
    "P1639": "Fahrpedalgeber 1 und 2 - Signale unplausibel",
    "P1640": "Steuergeraet EEPROM - Fehler",
    "P1648": "Datenbus Antrieb - Fehlfunktion",
    "P1649": "Datenbus Antrieb - fehlende Botschaft vom ABS",
    "P1653": "Datenbus Antrieb - fehlende Botschaft vom Kombiinstrument",
    "P1690": "Fehlerlampe - Fehlfunktion",
    "P1780": "Motoreingriff Getriebe - nicht plausibel",
    # Fahrwerk (C) und Karosserie (B), generischer Teil
    "C0035": "Raddrehzahlfuehler vorne links - Stromkreis",
    "C0040": "Raddrehzahlfuehler vorne rechts - Stromkreis",
    "C0045": "Raddrehzahlfuehler hinten links - Stromkreis",
    "C0050": "Raddrehzahlfuehler hinten rechts - Stromkreis",
    "C0110": "ABS-Pumpenmotor - Stromkreis",
    "C0121": "ABS-Ventilrelais - Stromkreis",
    "C0161": "ABS und Bremslichtschalter - Stromkreis",
    "C0265": "ABS-Steuergeraet Relais - Stromkreis",
    "C1095": "ABS-Hydraulikpumpe - Funktion",
    "B0001": "Fahrerairbag - Stromkreis",
    "B0002": "Fahrerairbag Stufe 2 - Stromkreis",
    "B0010": "Beifahrerairbag - Stromkreis",
    "B0020": "Seitenairbag links - Stromkreis",
    "B0028": "Seitenairbag rechts - Stromkreis",
    "B0051": "Gurtstraffer Fahrer - Stromkreis",
    "B0052": "Gurtstraffer Beifahrer - Stromkreis",
}
for code, text in MANUAL.items():
    CODES[code] = text

# Codes, bei denen die Drosselklappe der uebliche Verdaechtige ist.
THROTTLE.update({
    "P0068", "P0120", "P0121", "P0122", "P0123", "P0220", "P0221", "P0222",
    "P0223", "P0225", "P0226", "P0227", "P0228", "P0505", "P0506", "P0507",
    "P0638", "P0639", "P1545", "P1558", "P1559", "P1568", "P2100", "P2101",
    "P2102", "P2103", "P2135", "P2138",
})

# Die Drosselklappen-Codes aus P2xxx fehlen sonst im Klartext.
for code, text in {
    "P2100": "Drosselklappensteller Motor - Stromkreis unterbrochen",
    "P2101": "Drosselklappensteller Motor - Regelbereich nicht erreicht",
    "P2102": "Drosselklappensteller Motor - Kurzschluss nach Masse",
    "P2103": "Drosselklappensteller Motor - Kurzschluss nach Plus",
    "P2135": "Drosselklappenpotentiometer 1 und 2 - Signale unplausibel",
    "P2138": "Fahrpedalgeber 1 und 2 - Signale unplausibel",
    "P0220": "Drosselklappensensor 2 - Stromkreis",
    "P0225": "Drosselklappensensor 3 - Stromkreis",
    "P0226": "Drosselklappensensor 3 - Signal unplausibel",
    "P0639": "Drosselklappensteller Bank 2 - Regelgrenze ueberschritten",
}.items():
    CODES[code] = text


def main() -> None:
    here = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    target = os.path.join(here, "src", "data", "DtcDatabase.gen.cpp")

    lines = [
        "// Automatisch erzeugt von tools/gen_dtc.py. Nicht von Hand aendern.",
        f"// {len(CODES)} Fehlercodes, {len(THROTTLE)} davon drosselklappenrelevant.",
        "",
        '#include "data/DtcDatabase.h"',
        "",
        "#include <unordered_map>",
        "#include <unordered_set>",
        "",
        "namespace caruspex {",
        "namespace {",
        "",
        "const std::unordered_map<std::string, std::string> kDescriptions = {",
    ]
    for code in sorted(CODES):
        text = CODES[code].replace("\\", "\\\\").replace('"', '\\"')
        lines.append(f'    {{"{code}", "{text}"}},')
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
        "const std::unordered_map<std::string, std::string>& dtcDescriptions() {",
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
    print(f"{target}\n{len(CODES)} Codes, {len(THROTTLE)} Drosselklappen-Codes")


if __name__ == "__main__":
    main()
