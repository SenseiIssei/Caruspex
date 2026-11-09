"""Manufacturer-specific codes per brand family.

A family is a group of brands that share one engine management culture and
therefore the same meaning for their own codes. Only meanings we know
reliably are listed. A brand without entries still gets its family label, so
an unknown P1 code is shown as "manufacturer-specific (Renault/Dacia)" with
the group meaning, which is honest and useful.

Ford and Mazda shared engine management on some models, but by no means on
all of them. The Mazda family therefore only takes over Ford codes that are
explicitly marked as shared. Opel and Vauxhall get their own family for the
same reason: the GM codes below are North American GM definitions and are
not reliable on European Opel engines. Opel models built on PSA platforms
(from about 2018) should be looked up with the family key "PSA".
"""

from __future__ import annotations

from dataclasses import dataclass, field

from .model import Table


@dataclass
class Family:
    key: str
    label: tuple[str, str]
    # Brand names as the user or the VIN decoder writes them. Matching is
    # case-insensitive and ignores spaces, hyphens and dots.
    brands: list[str]
    table: Table = field(init=False)

    def __post_init__(self) -> None:
        self.table = Table(self.key)


FAMILIES: list[Family] = [
    Family("VAG", ("VAG (VW, Audi, Seat, Cupra, Škoda)", "VAG (VW, Audi, Seat, Cupra, Škoda)"),
           ["Volkswagen", "VW", "VW Nutzfahrzeuge", "Audi", "Seat", "Cupra", "Skoda", "Škoda"]),
    Family("BMW", ("BMW/Mini", "BMW/Mini"), ["BMW", "Mini", "BMW Mini", "Rolls-Royce"]),
    Family("MERCEDES", ("Mercedes-Benz/Smart", "Mercedes-Benz/Smart"),
           ["Mercedes-Benz", "Mercedes", "Mercedes Benz", "Smart", "Maybach"]),
    Family("FORD", ("Ford", "Ford"), ["Ford", "Lincoln", "Mercury"]),
    Family("MAZDA", ("Mazda", "Mazda"), ["Mazda"]),
    Family("GM", ("GM (Chevrolet, Buick, Cadillac, GMC)", "GM (Chevrolet, Buick, Cadillac, GMC)"),
           ["Chevrolet", "Chevy", "GMC", "Buick", "Cadillac", "Pontiac", "Saturn", "Oldsmobile", "Hummer",
            "Holden", "General Motors"]),
    Family("OPEL", ("Opel/Vauxhall", "Opel/Vauxhall"), ["Opel", "Vauxhall"]),
    Family("TOYOTA", ("Toyota/Lexus", "Toyota/Lexus"), ["Toyota", "Lexus", "Scion"]),
    Family("HONDA", ("Honda/Acura", "Honda/Acura"), ["Honda", "Acura"]),
    Family("NISSAN", ("Nissan/Infiniti", "Nissan/Infiniti"), ["Nissan", "Infiniti", "Datsun"]),
    Family("HYUNDAI", ("Hyundai/Kia", "Hyundai/Kia"), ["Hyundai", "Kia", "Genesis"]),
    Family("RENAULT", ("Renault/Dacia", "Renault/Dacia"), ["Renault", "Dacia", "Alpine"]),
    Family("PSA", ("PSA/Stellantis (Peugeot, Citroën, DS)", "PSA/Stellantis (Peugeot, Citroën, DS)"),
           ["Peugeot", "Citroen", "Citroën", "DS", "DS Automobiles", "Stellantis"]),
    Family("FIAT", ("Fiat/Alfa Romeo/Lancia", "Fiat/Alfa Romeo/Lancia"),
           ["Fiat", "Alfa Romeo", "Alfa", "Lancia", "Abarth"]),
    Family("VOLVO", ("Volvo", "Volvo"), ["Volvo"]),
    Family("SUBARU", ("Subaru", "Subaru"), ["Subaru"]),
    Family("MITSUBISHI", ("Mitsubishi", "Mitsubishi"), ["Mitsubishi"]),
]
BY_KEY = {f.key: f for f in FAMILIES}

# Codes the Mazda family takes over from Ford.
FORD_SHARED_WITH_MAZDA = {"P1000"}


def table(key: str) -> Table:
    return BY_KEY[key].table


# ==========================================================================
# VAG: Volkswagen, Audi, Seat, Cupra, Skoda
# ==========================================================================
vag = table("VAG").put
vag("P1102", "Lambdasondenheizung Bank 1, Sonde 1 - Kurzschluss nach Plus",
    "O2 sensor heater bank 1 sensor 1 - short to positive", "low", "exhaust", "o2_heater")
vag("P1105", "Lambdasondenheizung Bank 1, Sonde 2 - Kurzschluss nach Plus",
    "O2 sensor heater bank 1 sensor 2 - short to positive", "low", "exhaust", "o2_heater")
vag("P1127", "Gemischadaption Teillast Bank 1 - Regelgrenze überschritten (zu fett)",
    "Long term fuel trim at part load, bank 1 - system too rich", "medium", "fuel", "rich")
vag("P1128", "Gemischadaption Teillast Bank 1 - Regelgrenze unterschritten (zu mager)",
    "Long term fuel trim at part load, bank 1 - system too lean", "medium", "fuel", "lean")
vag("P1129", "Gemischadaption Teillast Bank 2 - Regelgrenze überschritten (zu fett)",
    "Long term fuel trim at part load, bank 2 - system too rich", "medium", "fuel", "rich")
vag("P1130", "Gemischadaption Teillast Bank 2 - Regelgrenze unterschritten (zu mager)",
    "Long term fuel trim at part load, bank 2 - system too lean", "medium", "fuel", "lean")
vag("P1136", "Gemischadaption Leerlauf Bank 1 - Regelgrenze unterschritten (zu mager, Falschluft)",
    "Long term fuel trim at idle, bank 1 - system too lean (vacuum leak)", "medium", "fuel", "lean")
vag("P1137", "Gemischadaption Leerlauf Bank 1 - Regelgrenze überschritten (zu fett)",
    "Long term fuel trim at idle, bank 1 - system too rich", "medium", "fuel", "rich")
vag("P1138", "Gemischadaption Leerlauf Bank 2 - Regelgrenze unterschritten (zu mager, Falschluft)",
    "Long term fuel trim at idle, bank 2 - system too lean (vacuum leak)", "medium", "fuel", "lean")
vag("P1139", "Gemischadaption Leerlauf Bank 2 - Regelgrenze überschritten (zu fett)",
    "Long term fuel trim at idle, bank 2 - system too rich", "medium", "fuel", "rich")
vag("P1176", "Lambdakorrektur hinter Katalysator Bank 1 - Regelgrenze erreicht",
    "O2 correction behind catalyst, bank 1 - limit attained", "medium", "exhaust", "o2_down")
vag("P1196", "Lambdasondenheizung Bank 1, Sonde 1 - elektrischer Fehler",
    "O2 sensor heater bank 1 sensor 1 - electrical malfunction", "low", "exhaust", "o2_heater")
vag("P1250", "Kraftstoffstand zu niedrig (Tank fast leer gefahren)", "Fuel level too low", "low", "fuel")
vag("P1296", "Kühlsystem - Fehlfunktion (Thermostat oder Kühlmitteltemperatur)",
    "Cooling system - malfunction (thermostat or coolant temperature)", "medium", "cooling", "thermostat")
for cyl in range(1, 5):
    vag(f"P{1324 + cyl}", f"Klopfregelung Zylinder {cyl} - Regelgrenze erreicht",
        f"Knock control cylinder {cyl} - control limit attained", "medium", "ignition", "knock")
vag("P1336", "Motormomentenüberwachung - Adaption an der Grenze",
    "Engine torque monitoring - adaptation at limit", "medium", "engine")
vag("P1340", "Kurbel- und Nockenwellengeber zueinander unplausibel",
    "Crankshaft and camshaft position sensors implausible relative to each other", "high", "timing",
    "cam_crank_corr")
vag("P1386", "Steuergerät - interner Klopfsensortest fehlerhaft",
    "Control unit - internal knock sensor test failed", "medium", "engine", "ecu_internal")
vag("P1410", "Tankentlüftungsventil - Kurzschluss nach Plus", "EVAP purge valve - short to positive",
    "low", "evap", "evap_valve")
vag("P1425", "Tankentlüftungsventil - Kurzschluss nach Masse", "EVAP purge valve - short to ground",
    "low", "evap", "evap_valve")
vag("P1500", "Kraftstoffpumpen-Relais - Stromkreis", "Fuel pump relay - circuit", "high", "fuel", "fuel_pump")
vag("P1543", "Saugrohrdruck - Signal zu klein", "Intake manifold pressure - signal too low", "medium",
    "intake", "map")
vag("P1545", "Drosselklappensteuerung - Fehlfunktion", "Throttle body control - malfunction", "high",
    "throttle", "throttle", True)
vag("P1546", "Ladedruck-Magnetventil - Kurzschluss nach Plus", "Boost pressure solenoid valve - short to positive",
    "medium", "turbo", "boost_actuator")
vag("P1547", "Ladedruck-Magnetventil - Kurzschluss nach Masse", "Boost pressure solenoid valve - short to ground",
    "medium", "turbo", "boost_actuator")
vag("P1550", "Ladedruck - Regeldifferenz", "Boost pressure - control deviation", "medium", "turbo", "underboost")
vag("P1555", "Ladedruck-Obergrenze überschritten", "Boost pressure upper limit exceeded", "high", "turbo",
    "overboost")
vag("P1556", "Ladedruckregelung - Regelgrenze unterschritten", "Boost pressure control - below control limit",
    "medium", "turbo", "underboost")
vag("P1557", "Ladedruckregelung - Regelgrenze überschritten", "Boost pressure control - above control limit",
    "high", "turbo", "overboost")
vag("P1558", "Drosselklappensteller - elektrische Fehlfunktion", "Throttle actuator - electrical malfunction",
    "high", "throttle", "throttle", True)
vag("P1559", "Leerlaufdrehzahlregelung - Drosselklappenanpassung fehlerhaft (anlernen)",
    "Idle speed control - throttle body adaptation failed (run the adaptation)", "medium", "throttle",
    "throttle", True)
vag("P1560", "Maximale Motordrehzahl überschritten", "Maximum engine speed exceeded", "low", "engine")
vag("P1568", "Leerlaufregelung - mechanische Fehlfunktion (Drosselklappe klemmt)",
    "Idle control - mechanical malfunction (throttle body sticking)", "medium", "throttle", "throttle", True)
vag("P1569", "Tempomatschalter - Signal unplausibel", "Cruise control switch - signal implausible", "low",
    "engine", "cruise")
vag("P1570", "Steuergerät durch Wegfahrsperre gesperrt", "Control unit locked by immobilizer", "high",
    "security", "immobilizer")
vag("P1580", "Drosselklappensteller Bank 1 - Fehlfunktion", "Throttle actuator bank 1 - malfunction", "high",
    "throttle", "throttle", True)
vag("P1602", "Spannungsversorgung Klemme 30 - Spannung zu klein", "Power supply terminal 30 - voltage too low",
    "medium", "charging", "battery_low")
vag("P1606", "Schlechtwegsignal - unplausibel", "Rough road signal - implausible", "low", "brakes")
vag("P1611", "MIL-Anforderung Getriebesteuergerät - Kurzschluss nach Masse",
    "MIL request from transmission control unit - short to ground", "low", "engine")
vag("P1613", "MIL-Anforderung - Unterbrechung oder Kurzschluss nach Plus",
    "MIL request - open circuit or short to positive", "low", "engine")
vag("P1624", "MIL-Anforderung aktiv (Fehler in einem anderen Steuergerät)",
    "MIL request active (fault in another control unit)", "medium", "engine")
vag("P1626", "Datenbus Antrieb - fehlende Botschaft vom Getriebesteuergerät",
    "Powertrain data bus - missing message from transmission control unit", "medium", "network", "lost_comm")
vag("P1630", "Fahrpedalgeber 1 - Signal zu klein", "Accelerator pedal sensor 1 - signal too low", "high",
    "throttle", "pedal")
vag("P1631", "Fahrpedalgeber 1 - Signal zu groß", "Accelerator pedal sensor 1 - signal too high", "high",
    "throttle", "pedal")
vag("P1633", "Fahrpedalgeber 2 - Signal zu klein", "Accelerator pedal sensor 2 - signal too low", "high",
    "throttle", "pedal")
vag("P1634", "Fahrpedalgeber 2 - Signal zu groß", "Accelerator pedal sensor 2 - signal too high", "high",
    "throttle", "pedal")
vag("P1639", "Fahrpedalgeber 1 und 2 - Signale unplausibel",
    "Accelerator pedal sensors 1 and 2 - signals implausible", "high", "throttle", "pedal")
vag("P1640", "Steuergerät EEPROM - Fehler", "Control unit EEPROM - error", "high", "engine", "ecu_internal")
vag("P1648", "Datenbus Antrieb - Fehlfunktion", "Powertrain data bus - malfunction", "high", "network", "can_bus")
vag("P1649", "Datenbus Antrieb - fehlende Botschaft vom ABS", "Powertrain data bus - missing message from ABS",
    "medium", "network", "lost_comm")
vag("P1653", "Datenbus Antrieb - fehlende Botschaft vom Kombiinstrument",
    "Powertrain data bus - missing message from instrument cluster", "medium", "network", "lost_comm")
vag("P1681", "Steuergerät - Programmierung nicht abgeschlossen", "Control unit programming not finished",
    "high", "engine", "ecu_internal")
vag("P1690", "Motorkontrollleuchte - Fehlfunktion", "Malfunction indicator lamp - malfunction", "medium",
    "engine", "mil_lamp")
vag("P1780", "Motoreingriff Getriebe - nicht plausibel", "Engine intervention by transmission - implausible",
    "medium", "transmission")

# ==========================================================================
# Mercedes-Benz and Smart
# ==========================================================================
table("MERCEDES").put(
    "P1570", "Motorstart durch Fahrberechtigungssystem (DAS) gesperrt",
    "Engine start blocked by drive authorization system (DAS)", "high", "security", "immobilizer")

# ==========================================================================
# Ford (EEC-V and later)
# ==========================================================================
ford = table("FORD").put
for code, de, en, sev, sys, detail in [
    ("P1000", "OBD-Bereitschaftstests noch nicht abgeschlossen (kein Fehler)",
     "OBD monitor testing not complete (not a fault)", "info", "engine", "readiness_info"),
    ("P1001", "Selbsttest bei laufendem Motor nicht abgeschlossen", "KOER self-test not able to complete",
     "info", "engine", ""),
    ("P1100", "Luftmassenmesser - Signal setzt zeitweise aus", "MAF sensor intermittent", "medium", "intake", "maf"),
    ("P1101", "Luftmassenmesser - außerhalb des Selbsttestbereichs", "MAF sensor out of self-test range",
     "medium", "intake", "maf"),
    ("P1112", "Ansauglufttemperatursensor - Signal setzt zeitweise aus", "IAT sensor intermittent", "low",
     "intake", "iat"),
    ("P1116", "Kühlmitteltemperatursensor - außerhalb des Selbsttestbereichs", "ECT sensor out of self-test range",
     "medium", "cooling", "ect_sensor"),
    ("P1117", "Kühlmitteltemperatursensor - Signal setzt zeitweise aus", "ECT sensor intermittent", "medium",
     "cooling", "ect_sensor"),
    ("P1127", "Abgas nicht warm genug, hintere Lambdasonde nicht geprüft",
     "Exhaust not warm enough, downstream sensor not tested", "info", "exhaust", ""),
    ("P1128", "Vordere Lambdasonden zwischen den Bänken vertauscht",
     "Upstream HO2S sensors swapped from bank to bank", "medium", "exhaust", ""),
    ("P1129", "Hintere Lambdasonden zwischen den Bänken vertauscht",
     "Downstream HO2S sensors swapped from bank to bank", "low", "exhaust", ""),
    ("P1130", "Vordere Lambdasonde Bank 1 schaltet nicht, Gemischadaption am Anschlag",
     "Lack of upstream HO2S switch - adaptive fuel at limit, bank 1", "medium", "exhaust", "o2_up"),
    ("P1131", "Vordere Lambdasonde Bank 1 schaltet nicht, meldet mager",
     "Lack of upstream HO2S switch - sensor indicates lean, bank 1", "medium", "fuel", "lean"),
    ("P1132", "Vordere Lambdasonde Bank 1 schaltet nicht, meldet fett",
     "Lack of upstream HO2S switch - sensor indicates rich, bank 1", "medium", "fuel", "rich"),
    ("P1137", "Hintere Lambdasonde Bank 1 schaltet nicht, meldet mager",
     "Lack of downstream HO2S switch - sensor indicates lean, bank 1", "low", "exhaust", "o2_down"),
    ("P1138", "Hintere Lambdasonde Bank 1 schaltet nicht, meldet fett",
     "Lack of downstream HO2S switch - sensor indicates rich, bank 1", "low", "exhaust", "o2_down"),
    ("P1150", "Vordere Lambdasonde Bank 2 schaltet nicht, Gemischadaption am Anschlag",
     "Lack of upstream HO2S switch - adaptive fuel at limit, bank 2", "medium", "exhaust", "o2_up"),
    ("P1151", "Vordere Lambdasonde Bank 2 schaltet nicht, meldet mager",
     "Lack of upstream HO2S switch - sensor indicates lean, bank 2", "medium", "fuel", "lean"),
    ("P1152", "Vordere Lambdasonde Bank 2 schaltet nicht, meldet fett",
     "Lack of upstream HO2S switch - sensor indicates rich, bank 2", "medium", "fuel", "rich"),
    ("P1157", "Hintere Lambdasonde Bank 2 schaltet nicht, meldet mager",
     "Lack of downstream HO2S switch - sensor indicates lean, bank 2", "low", "exhaust", "o2_down"),
    ("P1158", "Hintere Lambdasonde Bank 2 schaltet nicht, meldet fett",
     "Lack of downstream HO2S switch - sensor indicates rich, bank 2", "low", "exhaust", "o2_down"),
    ("P1233", "Kraftstoffpumpen-Steuermodul abgeschaltet oder nicht erreichbar",
     "Fuel pump driver module disabled or off line", "high", "fuel", "fuel_pump"),
    ("P1235", "Kraftstoffpumpensteuerung - außerhalb des Bereichs", "Fuel pump control out of range", "high",
     "fuel", "fuel_pump"),
    ("P1260", "Diebstahl erkannt, Motor gesperrt (Wegfahrsperre)", "Theft detected - vehicle immobilized",
     "high", "security", "immobilizer"),
    ("P1270", "Drehzahl- oder Geschwindigkeitsbegrenzer erreicht", "Engine RPM or vehicle speed limiter reached",
     "info", "engine", ""),
    ("P1285", "Zylinderkopf überhitzt", "Cylinder head over temperature sensed", "critical", "cooling",
     "overheat"),
    ("P1288", "Zylinderkopftemperatursensor - außerhalb des Selbsttestbereichs",
     "Cylinder head temperature sensor out of self-test range", "medium", "cooling", "ect_sensor"),
    ("P1289", "Zylinderkopftemperatursensor - Signal zu hoch", "Cylinder head temperature sensor circuit high input",
     "medium", "cooling", "ect_sensor"),
    ("P1290", "Zylinderkopftemperatursensor - Signal zu niedrig", "Cylinder head temperature sensor circuit low input",
     "medium", "cooling", "ect_sensor"),
    ("P1299", "Zylinderkopf überhitzt - Schutzfunktion aktiv", "Cylinder head over temperature protection active",
     "critical", "cooling", "overheat"),
    ("P1309", "Aussetzererkennung nicht aktiv", "Misfire detection monitor not enabled", "low", "ignition", ""),
    ("P1351", "Zündüberwachung (IDM) - Eingangssignal fehlerhaft", "Ignition diagnostic monitor input circuit",
     "high", "ignition", "coil"),
    ("P1400", "AGR-Differenzdrucksensor (DPFE) - Signal zu niedrig", "DPFE sensor circuit low input", "medium",
     "egr", "egr"),
    ("P1401", "AGR-Differenzdrucksensor (DPFE) - Signal zu hoch", "DPFE sensor circuit high input", "medium",
     "egr", "egr"),
    ("P1405", "AGR-Differenzdrucksensor - oberer Schlauch ab oder verstopft",
     "DPFE sensor upstream hose off or plugged", "medium", "egr", "egr"),
    ("P1406", "AGR-Differenzdrucksensor - unterer Schlauch ab oder verstopft",
     "DPFE sensor downstream hose off or plugged", "medium", "egr", "egr"),
    ("P1408", "AGR-Durchfluss - außerhalb des Selbsttestbereichs", "EGR flow out of self-test range", "medium",
     "egr", "egr"),
    ("P1409", "AGR-Unterdruckregler - Stromkreis", "EGR vacuum regulator solenoid circuit", "medium", "egr", "egr"),
    ("P1450", "Tankentlüftung - Unterdruck im Tank kann nicht abgebaut werden",
     "Unable to bleed up fuel tank vacuum", "low", "evap", "evap_valve"),
    ("P1451", "Belüftungsventil Aktivkohlefilter - Stromkreis", "EVAP canister vent solenoid circuit", "low",
     "evap", "evap_valve"),
    ("P1460", "Abschaltung der Klimaanlage bei Volllast - Stromkreis", "Wide open throttle A/C cutoff relay circuit",
     "low", "climate", ""),
    ("P1474", "Kühlerlüfter niedrige Stufe - Primärkreis", "Low fan control primary circuit", "high", "cooling",
     "fan"),
    ("P1500", "Geschwindigkeitssignal setzt zeitweise aus", "Vehicle speed sensor intermittent", "medium",
     "engine", "speed_sensor"),
    ("P1504", "Leerlaufsteller - Stromkreis", "Idle air control circuit", "low", "engine", "idle"),
    ("P1505", "Leerlaufsteller - Anpassung am Anschlag", "Idle air control system at adaptive clip", "low",
     "engine", "idle"),
    ("P1506", "Leerlaufregelung - Drehzahl zu hoch", "Idle air control overspeed error", "low", "engine", "idle"),
    ("P1507", "Leerlaufregelung - Drehzahl zu niedrig", "Idle air control underspeed error", "low", "engine",
     "idle"),
    ("P1550", "Servolenkungs-Drucksensor - außerhalb des Selbsttestbereichs",
     "Power steering pressure sensor out of self-test range", "low", "steering", ""),
    ("P1605", "Steuergerät - Dauerspeicher-Test fehlgeschlagen", "PCM keep alive memory test error", "medium",
     "engine", "ecu_internal"),
    ("P1633", "Dauerspeicher-Versorgung - Spannung zu niedrig", "Keep alive power voltage too low", "medium",
     "charging", "battery_low"),
    ("P1635", "Reifen- oder Achsübersetzung außerhalb des zulässigen Bereichs",
     "Tire/axle ratio out of acceptable range", "low", "engine", ""),
    ("P1639", "Fahrzeugkennungsblock fehlerhaft oder nicht programmiert", "Vehicle ID block corrupted or not programmed",
     "medium", "engine", ""),
    ("P1650", "Servolenkungs-Druckschalter - außerhalb des Selbsttestbereichs",
     "Power steering pressure switch out of self-test range", "low", "steering", ""),
    ("P1700", "Getriebe - unbestimmter Fehler (in Neutral geschaltet)",
     "Transmission indeterminate failure (failed to neutral)", "high", "transmission", "trans_general"),
    ("P1705", "Wählhebelsensor - außerhalb des Selbsttestbereichs (Test nicht in P oder N?)",
     "Transmission range sensor out of self-test range (test not in P or N?)", "medium", "transmission",
     "trans_sensor"),
    ("P1711", "Getriebeöltemperatursensor - außerhalb des Selbsttestbereichs",
     "Transmission fluid temperature sensor out of self-test range", "low", "transmission", "trans_sensor"),
    ("P1744", "Wandlerüberbrückung - Funktion", "Torque converter clutch system performance", "medium",
     "transmission", "trans_solenoid"),
    ("P1746", "Druckregelventil (EPC) - Unterbrechung", "Electronic pressure control solenoid open circuit",
     "high", "transmission", "trans_solenoid"),
    ("P1747", "Druckregelventil (EPC) - Kurzschluss", "Electronic pressure control solenoid short circuit",
     "high", "transmission", "trans_solenoid"),
    ("P1780", "Getriebe-Steuerschalter (O/D) - außerhalb des Selbsttestbereichs",
     "Transmission control switch circuit out of self-test range", "low", "transmission", ""),
    ("P1783", "Getriebe überhitzt", "Transmission over temperature condition", "high", "transmission",
     "trans_temp"),
    ("C1095", "ABS-Hydraulikpumpenmotor - Stromkreis", "ABS hydraulic pump motor circuit failure", "high",
     "brakes", "abs_unit"),
    ("C1145", "Raddrehzahlsensor vorne rechts - Eingangssignal fehlerhaft", "Speed wheel sensor RF input circuit failure",
     "high", "brakes", "wheel_speed"),
    ("C1155", "Raddrehzahlsensor vorne links - Eingangssignal fehlerhaft", "Speed wheel sensor LF input circuit failure",
     "high", "brakes", "wheel_speed"),
    ("C1165", "Raddrehzahlsensor hinten rechts - Eingangssignal fehlerhaft", "Speed wheel sensor RR input circuit failure",
     "high", "brakes", "wheel_speed"),
    ("C1175", "Raddrehzahlsensor hinten links - Eingangssignal fehlerhaft", "Speed wheel sensor LR input circuit failure",
     "high", "brakes", "wheel_speed"),
    ("C1233", "Raddrehzahlsensor vorne links - Signal fehlt", "Speed wheel LF input signal missing", "high",
     "brakes", "wheel_speed"),
    ("C1234", "Raddrehzahlsensor vorne rechts - Signal fehlt", "Speed wheel RF input signal missing", "high",
     "brakes", "wheel_speed"),
    ("C1235", "Raddrehzahlsensor hinten rechts - Signal fehlt", "Speed wheel RR input signal missing", "high",
     "brakes", "wheel_speed"),
    ("C1236", "Raddrehzahlsensor hinten links - Signal fehlt", "Speed wheel LR input signal missing", "high",
     "brakes", "wheel_speed"),
    ("B1317", "Batteriespannung zu hoch", "Battery voltage high", "high", "charging", "voltage_high"),
    ("B1318", "Batteriespannung zu niedrig", "Battery voltage low", "medium", "charging", "battery_low"),
    ("B1342", "Steuergerät defekt", "ECU is defective", "high", "body", "ecu_internal"),
    ("B1600", "Wegfahrsperre (PATS) - kein Signal vom Schlüssel-Transponder",
     "PATS ignition key transponder signal is not received", "medium", "security", "immobilizer"),
    ("B1601", "Wegfahrsperre (PATS) - falscher Schlüsselcode",
     "PATS received incorrect key-code from ignition key transponder", "medium", "security", "immobilizer"),
    ("B1602", "Wegfahrsperre (PATS) - ungültiges Format des Schlüsselcodes",
     "PATS received invalid format of key-code from ignition key transponder", "medium", "security",
     "immobilizer"),
    ("B1681", "Wegfahrsperre (PATS) - kein Signal vom Lesegerät", "PATS transceiver signal is not received",
     "medium", "security", "immobilizer"),
]:
    ford(code, de, en, sev, sys, detail,
         throttle=False)
# Throttle position sensor codes, kept apart because they are throttle-body related.
for code, de, en, sev in [
    ("P1120", "Drosselklappensensor - Signal unter dem Messbereich", "Throttle position sensor out of range low",
     "high"),
    ("P1121", "Drosselklappensensor passt nicht zum Luftmassenmesser",
     "Throttle position sensor inconsistent with MAF sensor", "high"),
    ("P1124", "Drosselklappensensor - außerhalb des Selbsttestbereichs",
     "Throttle position sensor out of self-test range", "medium"),
    ("P1125", "Drosselklappensensor - Signal setzt zeitweise aus", "Throttle position sensor intermittent", "high"),
]:
    ford(code, de, en, sev, "throttle", "throttle", True)

# Mazda: only what is known to be shared with Ford.
for code in sorted(FORD_SHARED_WITH_MAZDA):
    rec = table("FORD").codes[code]
    table("MAZDA").put(code, rec.de, rec.en, rec.sev, rec.sys, rec.detail, rec.throttle)

# ==========================================================================
# GM North America (Chevrolet, GMC, Buick, Cadillac, Pontiac, Saturn)
# ==========================================================================
gm = table("GM").put
for code, de, en, sev, sys, detail, thr in [
    ("P1111", "Ansauglufttemperatursensor - Signal zeitweise zu hoch", "IAT sensor circuit intermittent high voltage",
     "low", "intake", "iat", False),
    ("P1112", "Ansauglufttemperatursensor - Signal zeitweise zu niedrig", "IAT sensor circuit intermittent low voltage",
     "low", "intake", "iat", False),
    ("P1114", "Kühlmitteltemperatursensor - Signal zeitweise zu niedrig",
     "ECT sensor circuit intermittent low voltage", "medium", "cooling", "ect_sensor", False),
    ("P1115", "Kühlmitteltemperatursensor - Signal zeitweise zu hoch",
     "ECT sensor circuit intermittent high voltage", "medium", "cooling", "ect_sensor", False),
    ("P1120", "Drosselklappensensor 1 - Stromkreis", "Throttle position sensor 1 circuit", "high", "throttle",
     "throttle", True),
    ("P1121", "Drosselklappensensor - Signal zeitweise zu hoch", "TP sensor circuit intermittent high voltage",
     "high", "throttle", "throttle", True),
    ("P1122", "Drosselklappensensor - Signal zeitweise zu niedrig", "TP sensor circuit intermittent low voltage",
     "high", "throttle", "throttle", True),
    ("P1125", "Fahrpedalsystem - Fehlfunktion", "Accelerator pedal position system", "high", "throttle", "pedal",
     False),
    ("P1133", "Lambdasonde Bank 1, Sonde 1 - zu wenige Wechsel", "HO2S insufficient switching - bank 1 sensor 1",
     "medium", "exhaust", "o2_up", False),
    ("P1134", "Lambdasonde Bank 1, Sonde 1 - Übergangszeit unplausibel",
     "HO2S transition time ratio - bank 1 sensor 1", "medium", "exhaust", "o2_up", False),
    ("P1153", "Lambdasonde Bank 2, Sonde 1 - zu wenige Wechsel", "HO2S insufficient switching - bank 2 sensor 1",
     "medium", "exhaust", "o2_up", False),
    ("P1154", "Lambdasonde Bank 2, Sonde 1 - Übergangszeit unplausibel",
     "HO2S transition time ratio - bank 2 sensor 1", "medium", "exhaust", "o2_up", False),
    ("P1220", "Drosselklappensensor 2 - Stromkreis", "Throttle position sensor 2 circuit", "high", "throttle",
     "throttle", True),
    ("P1221", "Drosselklappensensor 1 und 2 passen nicht zusammen", "TP sensor 1-2 correlation", "high",
     "throttle", "throttle", True),
    ("P1258", "Motor überhitzt - Schutzbetrieb aktiv", "Engine coolant over temperature - protection mode active",
     "critical", "cooling", "overheat", False),
    ("P1275", "Fahrpedalgeber 1 - Stromkreis", "Accelerator pedal position sensor 1 circuit", "high", "throttle",
     "pedal", False),
    ("P1280", "Fahrpedalgeber 2 - Stromkreis", "Accelerator pedal position sensor 2 circuit", "high", "throttle",
     "pedal", False),
    ("P1336", "Kurbelwellen-Geberrad nicht angelernt", "Crankshaft position system variation not learned", "low",
     "ignition", "", False),
    ("P1345", "Kurbelwelle und Nockenwelle passen nicht zusammen", "Crankshaft position - camshaft position correlation",
     "high", "timing", "cam_crank_corr", False),
    ("P1351", "Zündsteuerung - Spannung zu hoch", "Ignition control circuit high voltage", "high", "ignition",
     "coil", False),
    ("P1361", "Zündsteuerung - Spannung zu niedrig", "Ignition control circuit low voltage", "high", "ignition",
     "coil", False),
    ("P1374", "Kurbelwellengeber - Signale hoher und niedriger Auflösung passen nicht zusammen",
     "Crankshaft position high to low resolution frequency correlation", "high", "ignition", "crank", False),
    ("P1380", "Aussetzer erkannt - Schlechtwegdaten nicht verfügbar", "Misfire detected - rough road data not available",
     "low", "ignition", "", False),
    ("P1381", "Aussetzer erkannt - keine Verbindung zum ABS-Steuergerät",
     "Misfire detected - no communication with brake control module", "low", "ignition", "", False),
    ("P1404", "AGR-Ventil - geschlossene Stellung unplausibel", "EGR valve closed position performance", "medium",
     "egr", "egr", False),
    ("P1406", "AGR-Ventil Positionssensor - Stromkreis", "EGR valve pintle position circuit", "medium", "egr", "egr",
     False),
    ("P1415", "Sekundärluftsystem Bank 1 - Fehlfunktion", "AIR system - bank 1", "low", "egr", "secondary_air",
     False),
    ("P1416", "Sekundärluftsystem Bank 2 - Fehlfunktion", "AIR system - bank 2", "low", "egr", "secondary_air",
     False),
    ("P1441", "Tankentlüftung - Durchfluss ohne Spülbefehl", "EVAP system flow during non-purge", "low", "evap",
     "evap_valve", False),
    ("P1626", "Wegfahrsperre - Freigabe für Kraftstoff nicht empfangen",
     "Theft deterrent fuel enable signal not received", "high", "security", "immobilizer", False),
    ("P1631", "Wegfahrsperre - Startfreigabe falsch", "Theft deterrent start enable signal not correct", "high",
     "security", "immobilizer", False),
    ("P1635", "Sensor-Referenzspannung 5 Volt - Stromkreis", "5 volt reference circuit", "high", "engine",
     "ref_voltage", False),
    ("P1639", "Sensor-Referenzspannung 5 Volt Nr. 2 - Stromkreis", "5 volt reference 2 circuit", "high", "engine",
     "ref_voltage", False),
    ("P1860", "Wandlerüberbrückung - PWM-Magnetventil Stromkreis", "TCC PWM solenoid circuit", "medium",
     "transmission", "trans_solenoid", False),
    ("P1870", "Getriebebauteil rutscht", "Transmission component slipping", "high", "transmission",
     "trans_ratio", False),
    ("C0242", "Motorsteuergerät meldet Fehler der Antriebsschlupfregelung",
     "PCM indicated traction control malfunction", "medium", "brakes", "", False),
    ("C0265", "ABS-Steuergerät Relais - Stromkreis", "EBCM relay circuit", "high", "brakes", "abs_unit", False),
    ("C0550", "ABS-Steuergerät - interner Fehler", "Electronic control unit (EBCM) malfunction", "high", "brakes",
     "abs_unit", False),
    ("C0561", "ABS/ESP abgeschaltet, Information gespeichert (meist Folge eines anderen Fehlers)",
     "System disabled information stored (usually follows another fault)", "medium", "brakes", "", False),
]:
    gm(code, de, en, sev, sys, detail, thr)

# ==========================================================================
# Toyota and Lexus
# ==========================================================================
toyota = table("TOYOTA").put
for code, de, en, sev, sys, detail, thr in [
    ("P1100", "Luftdrucksensor - Stromkreis", "BARO sensor circuit", "low", "intake", "map", False),
    ("P1120", "Fahrpedalgeber - Stromkreis", "Accelerator pedal position sensor circuit", "high", "throttle",
     "pedal", False),
    ("P1121", "Fahrpedalgeber - Signal unplausibel", "Accelerator pedal position sensor range/performance", "high",
     "throttle", "pedal", False),
    ("P1125", "Drosselklappenmotor - Stromkreis", "Throttle control motor circuit", "high", "throttle", "throttle",
     True),
    ("P1126", "Magnetkupplung der Drosselklappe - Stromkreis", "Magnetic clutch circuit", "high", "throttle",
     "throttle", True),
    ("P1127", "Drosselklappensteller - Spannungsversorgung", "ETCS actuator power source circuit", "high",
     "throttle", "throttle", True),
    ("P1128", "Drosselklappenmotor blockiert", "Throttle control motor lock", "high", "throttle", "throttle", True),
    ("P1129", "Elektronische Drosselklappensteuerung - Fehlfunktion", "Electric throttle control system", "high",
     "throttle", "throttle", True),
    ("P1130", "Breitband-Lambdasonde Bank 1, Sonde 1 - Signal unplausibel",
     "A/F sensor circuit range/performance - bank 1 sensor 1", "medium", "exhaust", "o2_up", False),
    ("P1133", "Breitband-Lambdasonde Bank 1, Sonde 1 - reagiert zu langsam",
     "A/F sensor circuit response - bank 1 sensor 1", "medium", "exhaust", "o2_up", False),
    ("P1135", "Breitband-Lambdasonde Bank 1, Sonde 1 - Heizkreis", "A/F sensor heater circuit - bank 1 sensor 1",
     "medium", "exhaust", "o2_heater", False),
    ("P1150", "Breitband-Lambdasonde Bank 2, Sonde 1 - Signal unplausibel",
     "A/F sensor circuit range/performance - bank 2 sensor 1", "medium", "exhaust", "o2_up", False),
    ("P1153", "Breitband-Lambdasonde Bank 2, Sonde 1 - reagiert zu langsam",
     "A/F sensor circuit response - bank 2 sensor 1", "medium", "exhaust", "o2_up", False),
    ("P1155", "Breitband-Lambdasonde Bank 2, Sonde 1 - Heizkreis", "A/F sensor heater circuit - bank 2 sensor 1",
     "medium", "exhaust", "o2_heater", False),
    ("P1300", "Zündverstärker Nr. 1 - Stromkreis", "Igniter circuit malfunction (No. 1)", "high", "ignition", "coil",
     False),
    ("P1305", "Zündverstärker Nr. 2 - Stromkreis", "Igniter circuit malfunction (No. 2)", "high", "ignition", "coil",
     False),
    ("P1335", "Kein Kurbelwellensignal bei laufendem Motor", "No crankshaft position sensor signal - engine running",
     "high", "ignition", "crank", False),
    ("P1346", "Nockenwellenverstellung (VVT) Sensor Bank 1 - Signal unplausibel",
     "VVT sensor circuit range/performance - bank 1", "medium", "timing", "cam_sensor", False),
    ("P1349", "Nockenwellenverstellung (VVT) Bank 1 - Fehlfunktion", "VVT system malfunction - bank 1", "medium",
     "timing", "vvt", False),
    ("P1520", "Bremslichtschalter - Signal fehlerhaft", "Stop lamp switch signal malfunction", "medium", "brakes",
     "brake_switch", False),
    ("P1600", "Motorsteuergerät - Dauerplus (BATT) fehlerhaft", "ECM BATT malfunction", "medium", "engine",
     "ecu_power", False),
    ("P1604", "Motor springt schlecht an", "Startability malfunction", "medium", "engine", "", False),
    ("P1656", "Ölsteuerventil der Nockenwellenverstellung Bank 1 - Stromkreis", "OCV circuit malfunction - bank 1",
     "medium", "timing", "vvt", False),
    ("P1780", "Park-/Neutralschalter - Fehlfunktion", "Park/neutral position switch malfunction", "medium",
     "transmission", "trans_sensor", False),
    ("P3000", "Hybridbatterie - Fehlfunktion", "HV battery malfunction", "high", "hybrid", "hybrid_battery", False),
    ("P3190", "Hybrid: Leistung des Verbrennungsmotors zu gering", "Poor engine power", "high", "hybrid",
     "hybrid_system", False),
    ("P3191", "Hybrid: Verbrennungsmotor springt nicht an", "Engine does not start", "high", "hybrid",
     "hybrid_system", False),
    ("P3193", "Hybrid: Kraftstoff ausgegangen", "Fuel run out", "medium", "hybrid", "", False),
]:
    toyota(code, de, en, sev, sys, detail, thr)

# ==========================================================================
# Honda and Acura
# ==========================================================================
honda = table("HONDA").put
for code, de, en, sev, sys, detail in [
    ("P1009", "Nockenwellenverstellung (VTC) - Fehlfunktion bei Frühverstellung",
     "Variable valve timing control advance malfunction", "medium", "timing", "vvt"),
    ("P1106", "Luftdrucksensor - Signal unplausibel", "BARO sensor circuit range/performance", "low", "intake", "map"),
    ("P1107", "Luftdrucksensor - Signal zu niedrig", "BARO sensor circuit low input", "low", "intake", "map"),
    ("P1108", "Luftdrucksensor - Signal zu hoch", "BARO sensor circuit high input", "low", "intake", "map"),
    ("P1128", "Saugrohrdrucksensor - Signal niedriger als erwartet", "MAP sensor signal lower than expected",
     "medium", "intake", "map"),
    ("P1129", "Saugrohrdrucksensor - Signal höher als erwartet", "MAP sensor signal higher than expected",
     "medium", "intake", "map"),
    ("P1149", "Vordere Lambdasonde (Sonde 1) - Signal unplausibel", "Primary HO2S (sensor 1) circuit range/performance",
     "medium", "exhaust", "o2_up"),
    ("P1162", "Vordere Lambdasonde (Sonde 1) - Stromkreis", "Primary HO2S (sensor 1) circuit malfunction", "medium",
     "exhaust", "o2_up"),
    ("P1163", "Vordere Lambdasonde (Sonde 1) - reagiert zu langsam", "Primary HO2S (sensor 1) slow response",
     "medium", "exhaust", "o2_up"),
    ("P1166", "Vordere Lambdasonde - Heizung elektrisch fehlerhaft", "Primary HO2S heater system electrical", "low",
     "exhaust", "o2_heater"),
    ("P1167", "Vordere Lambdasonde - Heizung Funktion", "Primary HO2S heater system", "low", "exhaust", "o2_heater"),
    ("P1259", "VTEC-System - Fehlfunktion", "VTEC system malfunction", "medium", "timing", "vvt"),
    ("P1297", "Lastsensor Bordnetz (ELD) - Spannung zu niedrig", "Electrical load detector circuit low voltage",
     "low", "charging", ""),
    ("P1298", "Lastsensor Bordnetz (ELD) - Spannung zu hoch", "Electrical load detector circuit high voltage",
     "low", "charging", ""),
    ("P1361", "OT-Geber - Signal setzt zeitweise aus", "TDC sensor intermittent interruption", "high", "ignition",
     "crank"),
    ("P1362", "OT-Geber - kein Signal", "TDC sensor no signal", "high", "ignition", "crank"),
    ("P1381", "Zylinderpositionsgeber - Signal setzt zeitweise aus", "Cylinder position sensor intermittent interruption",
     "medium", "timing", "cam_sensor"),
    ("P1382", "Zylinderpositionsgeber - kein Signal", "Cylinder position sensor no signal", "medium", "timing",
     "cam_sensor"),
    ("P1456", "Tankentlüftung - Leck im Bereich des Tanks", "EVAP control system leakage (fuel tank system)", "low",
     "evap", "evap_small"),
    ("P1457", "Tankentlüftung - Leck im Bereich des Aktivkohlefilters",
     "EVAP control system leakage (EVAP canister system)", "low", "evap", "evap_small"),
    ("P1491", "AGR-Ventil - Hub zu gering", "EGR valve lift insufficient detected", "medium", "egr", "egr"),
    ("P1498", "AGR-Ventil Hubsensor - Spannung zu hoch", "EGR valve lift sensor high voltage", "medium", "egr", "egr"),
    ("P1508", "Leerlaufsteller - Stromkreis", "Idle air control valve circuit failure", "low", "engine", "idle"),
    ("P1607", "Motorsteuergerät - interner Schaltkreis A fehlerhaft", "ECM/PCM internal circuit malfunction A",
     "high", "engine", "ecu_internal"),
    ("P1705", "Automatikgetriebe Wählhebelschalter - Kurzschluss", "A/T gear position switch short", "medium",
     "transmission", "trans_sensor"),
    ("P1706", "Automatikgetriebe Wählhebelschalter - Unterbrechung", "A/T gear position switch open", "medium",
     "transmission", "trans_sensor"),
    ("P1753", "Automatikgetriebe Überbrückungs-Magnetventil A - Fehlfunktion",
     "A/T lock-up control solenoid valve A malfunction", "medium", "transmission", "trans_solenoid"),
    ("P1768", "Automatikgetriebe Kupplungsdruck-Magnetventil A - Fehlfunktion",
     "A/T clutch pressure control solenoid valve A malfunction", "high", "transmission", "trans_solenoid"),
]:
    honda(code, de, en, sev, sys, detail)

# ==========================================================================
# Nissan and Infiniti
# ==========================================================================
nissan = table("NISSAN").put
for code, de, en, sev, sys, detail, thr in [
    ("P1111", "Einlassnockenwellenverstellung Magnetventil - Stromkreis",
     "Intake valve timing control solenoid valve circuit", "medium", "timing", "vvt", False),
    ("P1121", "Elektronischer Drosselklappensteller - Fehlfunktion", "Electric throttle control actuator", "high",
     "throttle", "throttle", True),
    ("P1122", "Elektronische Drosselklappensteuerung - Funktion", "Electric throttle control function", "high",
     "throttle", "throttle", True),
    ("P1124", "Relais Drosselklappenmotor - Kurzschluss", "Throttle control motor relay circuit short", "high",
     "throttle", "throttle", True),
    ("P1126", "Relais Drosselklappenmotor - Unterbrechung", "Throttle control motor relay circuit open", "high",
     "throttle", "throttle", True),
    ("P1128", "Drosselklappenmotor - Kurzschluss", "Throttle control motor circuit short", "high", "throttle",
     "throttle", True),
    ("P1148", "Lambdaregelung Bank 1 - Fehlfunktion", "Closed loop control - bank 1", "medium", "fuel", "fuel_trim",
     False),
    ("P1168", "Lambdaregelung Bank 2 - Fehlfunktion", "Closed loop control - bank 2", "medium", "fuel", "fuel_trim",
     False),
    ("P1217", "Motor überhitzt", "Engine over temperature (overheat)", "critical", "cooling", "overheat", False),
    ("P1225", "Anlernen der geschlossenen Drosselklappenstellung - Funktion",
     "Closed throttle position learning performance", "medium", "throttle", "throttle", True),
    ("P1273", "Breitband-Lambdasonde 1 Bank 1 - Verschiebung nach mager", "A/F sensor 1 (bank 1) lean shift monitoring",
     "medium", "exhaust", "o2_up", False),
    ("P1274", "Breitband-Lambdasonde 1 Bank 1 - Verschiebung nach fett", "A/F sensor 1 (bank 1) rich shift monitoring",
     "medium", "exhaust", "o2_up", False),
    ("P1283", "Breitband-Lambdasonde 1 Bank 2 - Verschiebung nach mager", "A/F sensor 1 (bank 2) lean shift monitoring",
     "medium", "exhaust", "o2_up", False),
    ("P1284", "Breitband-Lambdasonde 1 Bank 2 - Verschiebung nach fett", "A/F sensor 1 (bank 2) rich shift monitoring",
     "medium", "exhaust", "o2_up", False),
    ("P1320", "Zündsignal - Fehlfunktion", "Ignition signal", "high", "ignition", "coil", False),
    ("P1335", "Kurbelwellengeber (REF) - Fehlfunktion", "Crankshaft position sensor (REF)", "high", "ignition",
     "crank", False),
    ("P1336", "Kurbelwellengeber (Zahnsignal) - Fehlfunktion", "Crankshaft position sensor (COG)", "high", "ignition",
     "crank", False),
    ("P1400", "AGR-Magnetventil - Stromkreis", "EGRC solenoid valve", "medium", "egr", "egr", False),
    ("P1402", "Abgasrückführung - Funktion (offen)", "EGR function (open)", "medium", "egr", "egr", False),
    ("P1440", "Tankentlüftung - kleines Leck (Überdruck)", "EVAP control system small leak (positive pressure)",
     "low", "evap", "evap_small", False),
    ("P1444", "Spülmengen-Magnetventil Aktivkohlefilter - Fehlfunktion",
     "Canister purge volume control solenoid valve", "low", "evap", "evap_valve", False),
    ("P1446", "Belüftungsventil Aktivkohlefilter - hängt geschlossen", "EVAP canister vent control valve close",
     "low", "evap", "evap_valve", False),
    ("P1447", "Tankentlüftung - Überwachung des Spüldurchflusses", "EVAP control system purge flow monitoring",
     "low", "evap", "evap_valve", False),
    ("P1448", "Belüftungsventil Aktivkohlefilter - bleibt offen", "EVAP canister vent control valve open", "low",
     "evap", "evap_valve", False),
    ("P1490", "Bypassventil des Unterdruck-Trennventils - Stromkreis", "Vacuum cut valve bypass valve circuit",
     "low", "evap", "evap_valve", False),
    ("P1491", "Bypassventil des Unterdruck-Trennventils - Funktion", "Vacuum cut valve bypass valve", "low", "evap",
     "evap_valve", False),
    ("P1564", "Tempomat-Lenkradschalter - Fehlfunktion", "ASCD steering switch", "low", "engine", "cruise", False),
    ("P1572", "Tempomat-Bremsschalter - Fehlfunktion", "ASCD brake switch", "medium", "brakes", "brake_switch", False),
    ("P1574", "Tempomat-Geschwindigkeitssignal - Fehlfunktion", "ASCD vehicle speed sensor", "low", "engine", "cruise",
     False),
    ("P1610", "Wegfahrsperre (NATS) - Sperrmodus", "NATS lock mode", "high", "security", "immobilizer", False),
    ("P1611", "Wegfahrsperre (NATS) - Kennung zwischen Wegfahrsperre und Motorsteuergerät passt nicht",
     "NATS ID discord, IMMU-ECM", "high", "security", "immobilizer", False),
    ("P1612", "Wegfahrsperre (NATS) - Verbindung Motorsteuergerät zur Wegfahrsperre gestört",
     "NATS chain of ECM-IMMU", "high", "security", "immobilizer", False),
    ("P1613", "Wegfahrsperre (NATS) - Motorsteuergerät fehlerhaft", "NATS ECM", "high", "security", "immobilizer",
     False),
    ("P1614", "Wegfahrsperre (NATS) - Verbindung zum Schlüssel gestört", "NATS chain of IMMU-key", "medium",
     "security", "immobilizer", False),
    ("P1615", "Wegfahrsperre (NATS) - Schlüssel nicht erkannt", "NATS difference of key", "medium", "security",
     "immobilizer", False),
    ("P1705", "Drosselklappensensor (Automatikgetriebe) - Fehlfunktion", "Throttle position sensor (A/T)", "medium",
     "transmission", "trans_sensor", False),
    ("P1706", "Park-/Neutralschalter - Fehlfunktion", "Park/neutral position switch", "medium", "transmission",
     "trans_sensor", False),
    ("P1715", "Eingangsdrehzahlsensor (Primär) - Fehlfunktion", "Input speed sensor (primary speed sensor)", "medium",
     "transmission", "trans_sensor", False),
    ("P1716", "Turbinendrehzahlsensor - Fehlfunktion", "Turbine revolution sensor", "medium", "transmission",
     "trans_sensor", False),
    ("P1720", "Geschwindigkeitssensor (Kombiinstrument) - Fehlfunktion", "Vehicle speed sensor (MTR)", "medium",
     "engine", "speed_sensor", False),
    ("P1777", "CVT-Schrittmotor - Stromkreis", "Step motor circuit", "high", "transmission", "trans_solenoid", False),
    ("P1778", "CVT-Schrittmotor - Funktion", "Step motor function", "high", "transmission", "trans_solenoid", False),
    ("P1800", "Saugrohrumschaltung (VIAS) Magnetventil - Stromkreis", "VIAS control solenoid valve", "low", "intake",
     "imrc", False),
    ("P1805", "Bremsschalter - Fehlfunktion", "Brake switch", "medium", "brakes", "brake_switch", False),
]:
    nissan(code, de, en, sev, sys, detail, thr)

# ==========================================================================
# Hyundai, Kia, Genesis
# ==========================================================================
hyundai = table("HYUNDAI").put
hyundai("P1326", "Klopfsensor-Erkennungssystem (KSDS): Verdacht auf Pleuellagerschaden",
        "Knock sensor detection system (KSDS): suspected connecting rod bearing wear", "critical",
        "lubrication", "ksds")
for code, coil, lvl_de, lvl_en in [("P1505", 1, "zu niedrig", "low"), ("P1506", 1, "zu hoch", "high"),
                                   ("P1507", 2, "zu niedrig", "low"), ("P1508", 2, "zu hoch", "high")]:
    hyundai(code, f"Leerlaufsteller Spule {coil} - Signal {lvl_de}",
            f"Idle charge actuator signal {lvl_en} (coil {coil})", "low", "engine", "idle")
hyundai("P1529", "MIL-Anforderung vom Getriebesteuergerät (Getriebespeicher auslesen)",
        "MIL request from transmission control module (read the transmission fault memory)", "medium",
        "transmission", "trans_general")
hyundai("P1690", "Wegfahrsperre: SMARTRA-Modul antwortet nicht", "Immobilizer: SMARTRA no response", "medium",
        "security", "immobilizer")

# ==========================================================================
# Subaru
# ==========================================================================
subaru = table("SUBARU").put
subaru("P1152", "Lambdasonde Bank 1, Sonde 1 - Signal unplausibel (niedrig)",
       "O2 sensor circuit range/performance (low) - bank 1 sensor 1", "medium", "exhaust", "o2_up")
subaru("P1153", "Lambdasonde Bank 1, Sonde 1 - Signal unplausibel (hoch)",
       "O2 sensor circuit range/performance (high) - bank 1 sensor 1", "medium", "exhaust", "o2_up")
subaru("P1400", "Tankdruck-Regelventil - Spannung zu niedrig", "Fuel tank pressure control solenoid valve circuit low",
       "low", "evap", "evap_valve")
subaru("P1443", "Belüftungsventil Tankentlüftung - Funktion", "Vent control solenoid valve function problem",
       "low", "evap", "evap_valve")
subaru("P1491", "Kurbelgehäuseentlüftung - Funktion (Falschluft?)",
       "Positive crankcase ventilation (blow-by) function problem", "medium", "intake", "lean")
subaru("P1507", "Leerlaufregelung - Fehlfunktion (Notlauf)", "Idle control system malfunction (fail-safe)",
       "medium", "engine", "idle")
subaru("P1518", "Anlasserschalter - Signal zu niedrig", "Starter switch circuit low input", "medium", "engine")
subaru("P1560", "Pufferspannung (Dauerplus) - Fehlfunktion", "Back-up voltage circuit malfunction", "medium",
       "charging", "battery_low")

# ==========================================================================
# Mitsubishi
# ==========================================================================
mitsubishi = table("MITSUBISHI").put
mitsubishi("P1400", "Saugrohr-Differenzdrucksensor - Stromkreis", "Manifold differential pressure sensor circuit",
           "medium", "intake", "map")
mitsubishi("P1500", "Lichtmaschine Klemme FR - Stromkreis", "Alternator FR terminal circuit", "medium", "charging",
           "alternator")
mitsubishi("P1600", "Datenverbindung Motor- und Getriebesteuergerät", "Serial communication link (ECM - TCM)",
           "medium", "network", "can_bus")
mitsubishi("P1603", "Pufferleitung Batterie (Dauerplus) - Fehlfunktion", "Battery back-up line malfunction", "low",
           "charging", "battery_low")
mitsubishi("P1751", "Automatikgetriebe Steuerrelais - Fehlfunktion", "A/T control relay malfunction", "high",
           "transmission")
