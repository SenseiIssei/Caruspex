"""SAE J2012 / ISO 15031-6 codes that mean the same on every car.

Families (per cylinder, per bank, per sensor) are computed; everything else is
listed by hand. Only meanings we are sure about go in; a missing code still
gets its group meaning at runtime.
"""

from __future__ import annotations

from .model import (ACTUATOR3, ACTUATOR4, SENSOR5, SOLENOID5, SPEED4, Table,
                    family, num)

T = Table("generic")
put = T.put


def p(start, de, en, suffixes, sev, sys, detail="", throttle=False):
    family(T, "P", start, de, en, suffixes, sev, sys, detail, throttle)


LOW_HIGH = [("Spannung zu niedrig", "low"), ("Spannung zu hoch", "high")]
BANK_SENSORS = [(1, 1), (1, 2), (1, 3), (2, 1), (2, 2), (2, 3)]


def bs(bank: int, sensor: int) -> tuple[str, str]:
    return f"Bank {bank}, Sonde {sensor}", f"bank {bank} sensor {sensor}"


# ==========================================================================
# P0000 to P0099: fuel and air metering, auxiliary emission controls
# ==========================================================================
p(1, "Kraftstoffmengenregler", "Fuel volume regulator", ACTUATOR4, "high", "fuel", "fuel_pressure")
p(5, "Kraftstoffabsperrventil A", "Fuel shutoff valve A", ACTUATOR3, "high", "fuel")
put("P0008", "Motorpositionssystem Bank 1 - Funktion", "Engine position system performance - bank 1",
    "high", "timing", "cam_crank_corr")
put("P0009", "Motorpositionssystem Bank 2 - Funktion", "Engine position system performance - bank 2",
    "high", "timing", "cam_crank_corr")
for code, cam, bank in [("P000A", "A", 1), ("P000B", "B", 1), ("P000C", "A", 2), ("P000D", "B", 2)]:
    side_de = "Einlass" if cam == "A" else "Auslass"
    side_en = "intake" if cam == "A" else "exhaust"
    put(code, f"Nockenwelle {cam} ({side_de}) Bank {bank} - Verstellung reagiert zu langsam",
        f"{cam} camshaft position ({side_en}) bank {bank} - slow response", "medium", "timing", "vvt")

# Camshaft timing, A is usually intake and B exhaust.
for base, cam, bank in [(10, "A", 1), (13, "B", 1), (20, "A", 2), (23, "B", 2)]:
    side_de = "Einlass" if cam == "A" else "Auslass"
    side_en = "intake" if cam == "A" else "exhaust"
    put(num("P", base), f"Nockenwellenversteller {cam} ({side_de}) Bank {bank} - Stromkreis",
        f"{cam} camshaft position actuator ({side_en}) bank {bank} - circuit", "medium", "timing", "vvt")
    put(num("P", base + 1),
        f"Nockenwelle {cam} ({side_de}) Bank {bank} - Steuerzeit zu früh oder Systemfunktion",
        f"{cam} camshaft position ({side_en}) bank {bank} - timing over-advanced or system performance",
        "medium", "timing", "vvt")
    put(num("P", base + 2), f"Nockenwelle {cam} ({side_de}) Bank {bank} - Steuerzeit zu spät",
        f"{cam} camshaft position ({side_en}) bank {bank} - timing over-retarded", "medium", "timing", "vvt")
for code, bank, sensor in [(16, 1, "A"), (17, 1, "B"), (18, 2, "A"), (19, 2, "B")]:
    put(num("P", code),
        f"Kurbelwelle und Nockenwelle passen nicht zusammen, Bank {bank} Geber {sensor}",
        f"Crankshaft position - camshaft position correlation, bank {bank} sensor {sensor}",
        "high", "timing", "cam_crank_corr")
for code, side_de, side_en, bank in [(26, "Einlass", "Intake", 1), (27, "Auslass", "Exhaust", 1),
                                     (28, "Einlass", "Intake", 2), (29, "Auslass", "Exhaust", 2)]:
    put(num("P", code), f"Ventilsteuerung {side_de} Magnetventil Bank {bank} - Funktion",
        f"{side_en} valve control solenoid circuit range/performance - bank {bank}",
        "medium", "timing", "vvt")

# Oxygen sensor heaters.
_HEATER = {(1, 1): 30, (1, 2): 36, (1, 3): 42, (2, 1): 50, (2, 2): 56, (2, 3): 62}
_HEATER_RES = {(1, 1): 53, (1, 2): 54, (1, 3): 55, (2, 1): 59, (2, 2): 60, (2, 3): 61}
for (bank, sensor), base in _HEATER.items():
    wd, we = bs(bank, sensor)
    put(num("P", base), f"Lambdasondenheizung {wd} - Steuerkreis",
        f"O2 sensor heater control circuit - {we}", "low", "exhaust", "o2_heater")
    put(num("P", base + 1), f"Lambdasondenheizung {wd} - Steuerkreis Spannung zu niedrig",
        f"O2 sensor heater control circuit low - {we}", "low", "exhaust", "o2_heater")
    put(num("P", base + 2), f"Lambdasondenheizung {wd} - Steuerkreis Spannung zu hoch",
        f"O2 sensor heater control circuit high - {we}", "low", "exhaust", "o2_heater")
    put(num("P", _HEATER_RES[(bank, sensor)]), f"Lambdasondenheizung {wd} - Widerstand",
        f"O2 sensor heater resistance - {we}", "low", "exhaust", "o2_heater")

put("P0033", "Schubumluftventil (Turbo-Bypass) - Steuerkreis",
    "Turbocharger bypass valve control circuit", "medium", "turbo", "boost_actuator")
put("P0034", "Schubumluftventil (Turbo-Bypass) - Steuerkreis Spannung zu niedrig",
    "Turbocharger bypass valve control circuit low", "medium", "turbo", "boost_actuator")
put("P0035", "Schubumluftventil (Turbo-Bypass) - Steuerkreis Spannung zu hoch",
    "Turbocharger bypass valve control circuit high", "medium", "turbo", "boost_actuator")
put("P0039", "Schubumluftventil (Turbo-Bypass) - Funktion",
    "Turbocharger bypass valve control circuit range/performance", "medium", "turbo", "boost_actuator")
put("P0040", "Lambdasonden vertauscht: Bank 1 Sonde 1 und Bank 2 Sonde 1",
    "O2 sensor signals swapped: bank 1 sensor 1 / bank 2 sensor 1", "medium", "exhaust")
put("P0041", "Lambdasonden vertauscht: Bank 1 Sonde 2 und Bank 2 Sonde 2",
    "O2 sensor signals swapped: bank 1 sensor 2 / bank 2 sensor 2", "low", "exhaust")
p(45, "Ladedruckregelung A", "Turbo/super charger boost control A",
  [("Stromkreis oder Unterbrechung", "circuit/open"), ("Funktion", "circuit range/performance"),
   ("Spannung zu niedrig", "circuit low"), ("Spannung zu hoch", "circuit high")],
  "medium", "turbo", "boost_actuator")
put("P0049", "Turbolader - Turbine dreht zu schnell", "Turbo/super charger turbine overspeed",
    "high", "turbo", "overboost")
put("P0065", "Luftunterstützte Einspritzung - Funktion", "Air assisted injector control range/performance",
    "low", "injection")
put("P0066", "Luftunterstützte Einspritzung - Steuerkreis oder Spannung zu niedrig",
    "Air assisted injector control circuit or circuit low", "low", "injection")
put("P0067", "Luftunterstützte Einspritzung - Steuerkreis Spannung zu hoch",
    "Air assisted injector control circuit high", "low", "injection")
put("P0068", "Saugrohrdruck oder Luftmasse passt nicht zur Drosselklappenstellung (oft Falschluft)",
    "MAP/MAF - throttle position correlation (often a vacuum leak)", "medium", "intake", "lean", True)
put("P0069", "Saugrohrdruck und Luftdruck passen nicht zusammen",
    "Manifold absolute pressure - barometric pressure correlation", "medium", "intake", "map")
p(70, "Außenlufttemperatursensor", "Ambient air temperature sensor", SENSOR5, "low", "intake")
for base, side_de, side_en, bank in [(75, "Einlass", "Intake", 1), (78, "Auslass", "Exhaust", 1),
                                     (81, "Einlass", "Intake", 2), (84, "Auslass", "Exhaust", 2)]:
    p(base, f"Magnetventil Ventilsteuerung {side_de} Bank {bank}",
      f"{side_en} valve control solenoid bank {bank}",
      [("Stromkreis", "circuit"), ("Spannung zu niedrig", "circuit low"),
       ("Spannung zu hoch", "circuit high")], "medium", "timing", "vvt")
put("P0087", "Kraftstoffdruck im Rail oder System zu niedrig", "Fuel rail/system pressure - too low",
    "high", "fuel", "fuel_pressure")
put("P0088", "Kraftstoffdruck im Rail oder System zu hoch", "Fuel rail/system pressure - too high",
    "high", "fuel", "fuel_pressure")
put("P0089", "Kraftstoffdruckregler 1 - Funktion", "Fuel pressure regulator 1 performance",
    "high", "fuel", "fuel_pressure")
p(90, "Kraftstoffdruckregler 1", "Fuel pressure regulator 1",
  [("Steuerkreis", "control circuit"), ("Steuerkreis Spannung zu niedrig", "control circuit low"),
   ("Steuerkreis Spannung zu hoch", "control circuit high")], "high", "fuel", "fuel_pressure")
put("P0093", "Kraftstoffsystem - großes Leck erkannt", "Fuel system leak detected - large leak",
    "critical", "fuel", "fuel_leak")
put("P0094", "Kraftstoffsystem - kleines Leck erkannt", "Fuel system leak detected - small leak",
    "high", "fuel", "fuel_leak")
p(95, "Ansauglufttemperatursensor 2", "Intake air temperature sensor 2", SENSOR5, "low", "intake", "iat")

# ==========================================================================
# P0100 to P0199: fuel and air metering
# ==========================================================================
p(100, "Luftmassenmesser", "Mass or volume air flow sensor A", SENSOR5, "medium", "intake", "maf")
p(105, "Saugrohrdruck-/Luftdrucksensor", "Manifold absolute pressure/barometric pressure sensor",
  SENSOR5, "medium", "intake", "map")
p(110, "Ansauglufttemperatursensor 1", "Intake air temperature sensor 1", SENSOR5, "low", "intake", "iat")
p(115, "Kühlmitteltemperatursensor", "Engine coolant temperature sensor", SENSOR5, "medium", "cooling",
  "ect_sensor")
p(120, "Drosselklappen-/Fahrpedalgeber A", "Throttle/pedal position sensor A", SENSOR5, "high",
  "throttle", "throttle", True)
put("P0125", "Kühlmittel zu kalt für die Gemischregelung",
    "Insufficient coolant temperature for closed loop fuel control", "low", "cooling", "thermostat")
put("P0126", "Kühlmittel zu kalt für stabilen Betrieb",
    "Insufficient coolant temperature for stable operation", "low", "cooling", "thermostat")
put("P0127", "Ansauglufttemperatur zu hoch", "Intake air temperature too high", "medium", "intake")
put("P0128", "Thermostat - Kühlmitteltemperatur unter der Regeltemperatur",
    "Coolant thermostat (coolant temperature below thermostat regulating temperature)",
    "low", "cooling", "thermostat")
put("P0129", "Luftdruck zu niedrig", "Barometric pressure too low", "low", "intake", "map")

# Oxygen sensors.
_O2 = {(1, 1): 130, (1, 2): 136, (1, 3): 142, (2, 1): 150, (2, 2): 156, (2, 3): 162}
_O2_SUFFIX = [
    ("Stromkreis", "circuit"),
    ("Spannung zu niedrig", "circuit low voltage"),
    ("Spannung zu hoch", "circuit high voltage"),
    ("reagiert zu langsam", "circuit slow response"),
    ("keine Aktivität", "circuit no activity detected"),
    ("Heizkreis", "heater circuit"),
]
for (bank, sensor), base in _O2.items():
    wd, we = bs(bank, sensor)
    for i, (sd, se) in enumerate(_O2_SUFFIX):
        if i == 5:
            sev, detail = "low", "o2_heater"
        elif sensor == 1:
            sev, detail = "medium", "o2_up"
        else:
            sev, detail = "low", "o2_down"
        put(num("P", base + i), f"Lambdasonde {wd} - {sd}", f"O2 sensor {se} - {we}",
            sev, "exhaust", detail)
# Newer response codes, these really use hex digits.
for code, kind_de, kind_en, dir_de, dir_en, bank, sensor in [
    ("P013A", "reagiert zu langsam", "slow response", "fett nach mager", "rich to lean", 1, 2),
    ("P013B", "reagiert zu langsam", "slow response", "mager nach fett", "lean to rich", 1, 2),
    ("P013C", "reagiert zu langsam", "slow response", "fett nach mager", "rich to lean", 2, 2),
    ("P013D", "reagiert zu langsam", "slow response", "mager nach fett", "lean to rich", 2, 2),
    ("P013E", "reagiert verzögert", "delayed response", "fett nach mager", "rich to lean", 1, 2),
    ("P013F", "reagiert verzögert", "delayed response", "mager nach fett", "lean to rich", 1, 2),
    ("P014A", "reagiert verzögert", "delayed response", "fett nach mager", "rich to lean", 2, 2),
    ("P014B", "reagiert verzögert", "delayed response", "mager nach fett", "lean to rich", 2, 2),
    ("P014C", "reagiert zu langsam", "slow response", "fett nach mager", "rich to lean", 1, 1),
    ("P014D", "reagiert zu langsam", "slow response", "mager nach fett", "lean to rich", 1, 1),
    ("P014E", "reagiert zu langsam", "slow response", "fett nach mager", "rich to lean", 2, 1),
    ("P014F", "reagiert zu langsam", "slow response", "mager nach fett", "lean to rich", 2, 1),
    ("P015A", "reagiert verzögert", "delayed response", "fett nach mager", "rich to lean", 1, 1),
    ("P015B", "reagiert verzögert", "delayed response", "mager nach fett", "lean to rich", 1, 1),
    ("P015C", "reagiert verzögert", "delayed response", "fett nach mager", "rich to lean", 2, 1),
    ("P015D", "reagiert verzögert", "delayed response", "mager nach fett", "lean to rich", 2, 1),
]:
    wd, we = bs(bank, sensor)
    sev, detail = ("medium", "o2_up") if sensor == 1 else ("low", "o2_down")
    put(code, f"Lambdasonde {wd} - {kind_de} von {dir_de}", f"O2 sensor {kind_en} - {dir_en} - {we}",
        sev, "exhaust", detail)

put("P0148", "Fehler bei der Kraftstoffförderung", "Fuel delivery error", "high", "fuel", "fuel_pressure")
put("P0149", "Fehler beim Einspritzzeitpunkt", "Fuel timing error", "medium", "injection")
put("P0168", "Kraftstofftemperatur zu hoch", "Fuel temperature too high", "medium", "fuel")
put("P0169", "Falsche Kraftstoffzusammensetzung", "Incorrect fuel composition", "medium", "fuel")
for bank, base in [(1, 170), (2, 173)]:
    put(num("P", base), f"Gemischadaption Bank {bank} - Fehlfunktion", f"Fuel trim malfunction - bank {bank}",
        "medium", "fuel", "fuel_trim")
    put(num("P", base + 1), f"Gemisch Bank {bank} zu mager", f"System too lean - bank {bank}",
        "medium", "fuel", "lean")
    put(num("P", base + 2), f"Gemisch Bank {bank} zu fett", f"System too rich - bank {bank}",
        "medium", "fuel", "rich")
p(176, "Kraftstoffzusammensetzungssensor", "Fuel composition sensor", SENSOR5[:4], "low", "fuel")
p(180, "Kraftstofftemperatursensor A", "Fuel temperature sensor A", SENSOR5, "low", "fuel")
p(185, "Kraftstofftemperatursensor B", "Fuel temperature sensor B", SENSOR5, "low", "fuel")
p(190, "Kraftstoffdrucksensor A (Rail)", "Fuel rail pressure sensor A", SENSOR5, "high", "fuel",
  "fuel_pressure")
p(195, "Motoröltemperatursensor", "Engine oil temperature sensor", SENSOR5, "low", "lubrication")

# ==========================================================================
# P0200 to P0299: fuel and air metering, injector circuits, boost
# ==========================================================================
put("P0200", "Einspritzventile - Stromkreis oder Unterbrechung", "Injector circuit/open",
    "high", "injection", "injector_circuit")
for cyl in range(1, 13):
    put(num("P", 200 + cyl), f"Einspritzventil Zylinder {cyl} - Stromkreis oder Unterbrechung",
        f"Injector circuit/open - cylinder {cyl}", "high", "injection", "injector_circuit")
    base = 261 + (cyl - 1) * 3
    put(num("P", base), f"Einspritzventil Zylinder {cyl} - Stromkreis Spannung zu niedrig",
        f"Cylinder {cyl} injector circuit low", "high", "injection", "injector_circuit")
    put(num("P", base + 1), f"Einspritzventil Zylinder {cyl} - Stromkreis Spannung zu hoch",
        f"Cylinder {cyl} injector circuit high", "high", "injection", "injector_circuit")
    put(num("P", base + 2), f"Zylinder {cyl} - Beitrag zur Verbrennung oder Laufruhe abweichend",
        f"Cylinder {cyl} contribution/balance", "medium", "injection", "injector_balance")
put("P0213", "Kaltstartventil 1 - Fehlfunktion", "Cold start injector 1", "low", "injection")
put("P0214", "Kaltstartventil 2 - Fehlfunktion", "Cold start injector 2", "low", "injection")
put("P0215", "Motorabstellmagnet - Fehlfunktion", "Engine shutoff solenoid", "high", "engine")
put("P0216", "Einspritzzeitpunkt-Steuerung - Stromkreis", "Injector/injection timing control circuit",
    "high", "injection")
put("P0217", "Motor überhitzt (Kühlmitteltemperatur zu hoch)", "Engine coolant over temperature condition",
    "critical", "cooling", "overheat")
put("P0218", "Getriebeöl überhitzt", "Transmission fluid over temperature condition",
    "high", "transmission", "trans_temp")
put("P0219", "Motordrehzahl zu hoch (Überdrehzahl)", "Engine overspeed condition", "low", "engine")
p(220, "Drosselklappen-/Fahrpedalgeber B", "Throttle/pedal position sensor B", SENSOR5, "high",
  "throttle", "throttle", True)
p(225, "Drosselklappen-/Fahrpedalgeber C", "Throttle/pedal position sensor C", SENSOR5, "high",
  "throttle", "throttle", True)
p(230, "Kraftstoffpumpe", "Fuel pump",
  [("Primärkreis", "primary circuit"), ("Sekundärkreis Spannung zu niedrig", "secondary circuit low"),
   ("Sekundärkreis Spannung zu hoch", "secondary circuit high"),
   ("Sekundärkreis setzt zeitweise aus", "secondary circuit intermittent")],
  "high", "fuel", "fuel_pump")
put("P0234", "Ladedruck zu hoch (Overboost)", "Turbo/super charger A overboost condition",
    "high", "turbo", "overboost")
p(235, "Ladedrucksensor A", "Turbocharger boost sensor A", SENSOR5[:4], "medium", "turbo", "boost_sensor")
p(239, "Ladedrucksensor B", "Turbocharger boost sensor B", SENSOR5[:4], "medium", "turbo", "boost_sensor")
_WASTEGATE = [("Fehlfunktion", "malfunction"), ("Funktion", "range/performance"),
              ("Spannung zu niedrig", "low"), ("Spannung zu hoch", "high")]
p(243, "Ladedruckregelventil A (Wastegate)", "Turbo/super charger wastegate solenoid A", _WASTEGATE,
  "medium", "turbo", "boost_actuator")
p(247, "Ladedruckregelventil B (Wastegate)", "Turbo/super charger wastegate solenoid B", _WASTEGATE,
  "medium", "turbo", "boost_actuator")
_METERING = [("Fehlfunktion", "malfunction"), ("Funktion", "range/performance"),
             ("Spannung zu niedrig", "low"), ("Spannung zu hoch", "high"),
             ("Signal setzt zeitweise aus", "intermittent")]
p(251, "Einspritzpumpe Mengenregelung A", "Injection pump fuel metering control A", _METERING,
  "high", "injection", "fuel_pressure")
p(256, "Einspritzpumpe Mengenregelung B", "Injection pump fuel metering control B", _METERING,
  "high", "injection", "fuel_pressure")
put("P0297", "Fahrzeug zu schnell", "Vehicle overspeed condition", "info", "engine")
put("P0298", "Motoröl überhitzt", "Engine oil over temperature", "high", "lubrication")
put("P0299", "Ladedruck zu niedrig (Underboost)", "Turbo/super charger A underboost condition",
    "medium", "turbo", "underboost")

# ==========================================================================
# P0300 to P0399: ignition system and misfire
# ==========================================================================
put("P0300", "Verbrennungsaussetzer - mehrere oder wechselnde Zylinder",
    "Random/multiple cylinder misfire detected", "high", "ignition", "misfire")
for cyl in range(1, 13):
    put(num("P", 300 + cyl), f"Verbrennungsaussetzer Zylinder {cyl}",
        f"Cylinder {cyl} misfire detected", "high", "ignition", "misfire")
put("P0313", "Verbrennungsaussetzer bei niedrigem Tankinhalt", "Misfire detected with low fuel",
    "high", "ignition", "misfire")
put("P0314", "Verbrennungsaussetzer an einem Zylinder (nicht angegeben)",
    "Single cylinder misfire (cylinder not specified)", "high", "ignition", "misfire")
put("P0315", "Kurbelwellen-Geberrad nicht angelernt", "Crankshaft position system variation not learned",
    "low", "ignition")
put("P0316", "Verbrennungsaussetzer beim Start (erste 1000 Umdrehungen)",
    "Engine misfire detected on startup (first 1000 revolutions)", "high", "ignition", "misfire")
put("P0317", "Schlechtwegerkennung - Hardware fehlt", "Rough road hardware not present", "low", "engine")
put("P0318", "Schlechtwegsensor A - Signalkreis", "Rough road sensor A signal circuit", "low", "engine")
put("P0319", "Schlechtwegsensor B - Signalkreis", "Rough road sensor B signal circuit", "low", "engine")
p(320, "Zünd-/Motordrehzahlsignal", "Ignition/distributor engine speed input", SPEED4, "high",
  "ignition", "crank")
put("P0324", "Klopfregelung - Systemfehler", "Knock control system error", "medium", "ignition", "knock")
p(325, "Klopfsensor 1 (Bank 1 oder einziger Sensor)", "Knock sensor 1 (bank 1 or single sensor)",
  SENSOR5, "medium", "ignition", "knock")
p(330, "Klopfsensor 2 (Bank 2)", "Knock sensor 2 (bank 2)", SENSOR5, "medium", "ignition", "knock")
p(335, "Kurbelwellengeber A", "Crankshaft position sensor A", SENSOR5, "high", "ignition", "crank")
p(340, "Nockenwellengeber A Bank 1", "Camshaft position sensor A bank 1 (or single sensor)", SENSOR5,
  "medium", "timing", "cam_sensor")
p(345, "Nockenwellengeber A Bank 2", "Camshaft position sensor A bank 2", SENSOR5, "medium", "timing",
  "cam_sensor")
put("P0350", "Zündspule - Primär- oder Sekundärkreis", "Ignition coil primary/secondary circuit",
    "high", "ignition", "coil")
for cyl in range(1, 13):
    letter = "ABCDEFGHIJKL"[cyl - 1]
    put(num("P", 350 + cyl), f"Zündspule {letter} (meist Zylinder {cyl}) - Primär- oder Sekundärkreis",
        f"Ignition coil {letter} (usually cylinder {cyl}) primary/secondary circuit",
        "high", "ignition", "coil")
p(365, "Nockenwellengeber B Bank 1", "Camshaft position sensor B bank 1", SENSOR5, "medium", "timing",
  "cam_sensor")
_PULSES = [("Fehlfunktion", "malfunction"), ("zu viele Impulse", "too many pulses"),
           ("zu wenige Impulse", "too few pulses"), ("Impulse unregelmäßig", "intermittent/erratic pulses"),
           ("keine Impulse", "no pulse")]
p(370, "Hochauflösendes Bezugssignal A", "Timing reference high resolution signal A", _PULSES,
  "medium", "ignition")
p(375, "Hochauflösendes Bezugssignal B", "Timing reference high resolution signal B", _PULSES,
  "medium", "ignition")
put("P0380", "Glühkerzen/Heizung Kreis A - Fehlfunktion", "Glow plug/heater circuit A",
    "low", "diesel", "glow_plug")
put("P0381", "Glühkontrollleuchte - Stromkreis", "Glow plug/heater indicator circuit", "low", "diesel")
put("P0382", "Glühkerzen/Heizung Kreis B - Fehlfunktion", "Glow plug/heater circuit B",
    "low", "diesel", "glow_plug")
put("P0383", "Glühzeitsteuergerät - Steuerkreis Spannung zu niedrig",
    "Glow plug control module control circuit low", "medium", "diesel", "glow_module")
put("P0384", "Glühzeitsteuergerät - Steuerkreis Spannung zu hoch",
    "Glow plug control module control circuit high", "medium", "diesel", "glow_module")
p(385, "Kurbelwellengeber B", "Crankshaft position sensor B", SENSOR5, "high", "ignition", "crank")
p(390, "Nockenwellengeber B Bank 2", "Camshaft position sensor B bank 2", SENSOR5, "medium", "timing",
  "cam_sensor")

# ==========================================================================
# P0400 to P0499: auxiliary emission controls
# ==========================================================================
for code, de, en in [
    ("P0400", "Abgasrückführung - Durchfluss fehlerhaft", "Exhaust gas recirculation flow"),
    ("P0401", "Abgasrückführung - Durchfluss zu gering", "Exhaust gas recirculation flow insufficient detected"),
    ("P0402", "Abgasrückführung - Durchfluss zu hoch", "Exhaust gas recirculation flow excessive detected"),
    ("P0403", "Abgasrückführung - Steuerkreis", "Exhaust gas recirculation control circuit"),
    ("P0404", "Abgasrückführung - Steuerkreis Funktion",
     "Exhaust gas recirculation control circuit range/performance"),
    ("P0405", "AGR-Sensor A - Signal zu niedrig", "Exhaust gas recirculation sensor A circuit low"),
    ("P0406", "AGR-Sensor A - Signal zu hoch", "Exhaust gas recirculation sensor A circuit high"),
    ("P0407", "AGR-Sensor B - Signal zu niedrig", "Exhaust gas recirculation sensor B circuit low"),
    ("P0408", "AGR-Sensor B - Signal zu hoch", "Exhaust gas recirculation sensor B circuit high"),
    ("P0409", "AGR-Sensor A - Stromkreis", "Exhaust gas recirculation sensor A circuit"),
    ("P046C", "AGR-Sensor A - Signal unplausibel",
     "Exhaust gas recirculation sensor A circuit range/performance"),
    ("P0486", "AGR-Sensor B - Stromkreis", "Exhaust gas recirculation sensor B circuit"),
    ("P0487", "AGR-Drosselklappe - Steuerkreis", "Exhaust gas recirculation throttle position control circuit"),
    ("P0488", "AGR-Drosselklappe - Funktion",
     "Exhaust gas recirculation throttle position control range/performance"),
    ("P0489", "Abgasrückführung A - Steuerkreis Spannung zu niedrig",
     "Exhaust gas recirculation A control circuit low"),
    ("P0490", "Abgasrückführung A - Steuerkreis Spannung zu hoch",
     "Exhaust gas recirculation A control circuit high"),
]:
    put(code, de, en, "medium", "egr", "egr")

for code, de, en in [
    ("P0410", "Sekundärluftsystem - Fehlfunktion", "Secondary air injection system"),
    ("P0411", "Sekundärluftsystem - falscher Durchfluss erkannt",
     "Secondary air injection system incorrect flow detected"),
    ("P0412", "Sekundärluft-Umschaltventil A - Stromkreis",
     "Secondary air injection system switching valve A circuit"),
    ("P0413", "Sekundärluft-Umschaltventil A - Unterbrechung",
     "Secondary air injection system switching valve A circuit open"),
    ("P0414", "Sekundärluft-Umschaltventil A - Kurzschluss",
     "Secondary air injection system switching valve A circuit shorted"),
    ("P0415", "Sekundärluft-Umschaltventil B - Stromkreis",
     "Secondary air injection system switching valve B circuit"),
    ("P0416", "Sekundärluft-Umschaltventil B - Unterbrechung",
     "Secondary air injection system switching valve B circuit open"),
    ("P0417", "Sekundärluft-Umschaltventil B - Kurzschluss",
     "Secondary air injection system switching valve B circuit shorted"),
    ("P0418", "Sekundärluftpumpe oder Relais A - Steuerkreis",
     "Secondary air injection system control A circuit"),
    ("P0419", "Sekundärluftpumpe oder Relais B - Steuerkreis",
     "Secondary air injection system control B circuit"),
    ("P0491", "Sekundärluftsystem Bank 1 - Durchfluss zu gering",
     "Secondary air injection system insufficient flow - bank 1"),
    ("P0492", "Sekundärluftsystem Bank 2 - Durchfluss zu gering",
     "Secondary air injection system insufficient flow - bank 2"),
]:
    put(code, de, en, "low", "egr", "secondary_air")

for bank, base in [(1, 420), (2, 430)]:
    for i, (de, en) in enumerate([
        ("Katalysator", "Catalyst system efficiency below threshold"),
        ("Vorkatalysator", "Warm up catalyst efficiency below threshold"),
        ("Hauptkatalysator", "Main catalyst efficiency below threshold"),
        ("Beheizter Katalysator", "Heated catalyst efficiency below threshold"),
    ]):
        put(num("P", base + i), f"{de} Bank {bank} - Wirkungsgrad zu gering", f"{en} - bank {bank}",
            "medium", "exhaust", "catalyst")
    put(num("P", base + 4), f"Beheizter Katalysator Bank {bank} - Temperatur zu niedrig",
        f"Heated catalyst temperature below threshold - bank {bank}", "medium", "exhaust", "catalyst")
    p(base + 5, f"Katalysator-Temperatursensor Bank {bank}", f"Catalyst temperature sensor bank {bank}",
      SENSOR5[:4], "low", "exhaust")
    put(num("P", base + 9), f"Katalysatorheizung Bank {bank} - Steuerkreis",
        f"Catalyst heater control circuit - bank {bank}", "low", "exhaust")

for code, de, en, detail in [
    ("P0440", "Tankentlüftung - Fehlfunktion", "Evaporative emission system", "evap_valve"),
    ("P0441", "Tankentlüftung - falscher Spüldurchfluss", "Evaporative emission system incorrect purge flow",
     "evap_valve"),
    ("P0442", "Tankentlüftung - kleines Leck erkannt", "Evaporative emission system leak detected (small leak)",
     "evap_small"),
    ("P0443", "Tankentlüftungsventil A - Stromkreis", "EVAP system purge control valve A circuit", "evap_valve"),
    ("P0444", "Tankentlüftungsventil A - Unterbrechung", "EVAP system purge control valve A circuit open",
     "evap_valve"),
    ("P0445", "Tankentlüftungsventil A - Kurzschluss", "EVAP system purge control valve A circuit shorted",
     "evap_valve"),
    ("P0446", "Tankentlüftung Belüftungsventil - Steuerkreis", "EVAP system vent control circuit", "evap_valve"),
    ("P0447", "Tankentlüftung Belüftungsventil - Unterbrechung", "EVAP system vent control circuit open",
     "evap_valve"),
    ("P0448", "Tankentlüftung Belüftungsventil - Kurzschluss", "EVAP system vent control circuit shorted",
     "evap_valve"),
    ("P0449", "Tankentlüftung Belüftungsventil - Stromkreis", "EVAP system vent valve/solenoid circuit",
     "evap_valve"),
    ("P0455", "Tankentlüftung - großes Leck erkannt (Tankdeckel, Schlauch)",
     "Evaporative emission system leak detected (large leak)", "evap_large"),
    ("P0456", "Tankentlüftung - sehr kleines Leck erkannt",
     "Evaporative emission system leak detected (very small leak)", "evap_small"),
    ("P0457", "Tankentlüftung - Leck erkannt (Tankdeckel lose oder fehlt)",
     "Evaporative emission system leak detected (fuel cap loose/off)", "evap_large"),
    ("P0458", "Tankentlüftungsventil A - Spannung zu niedrig", "EVAP system purge control valve A circuit low",
     "evap_valve"),
    ("P0459", "Tankentlüftungsventil A - Spannung zu hoch", "EVAP system purge control valve A circuit high",
     "evap_valve"),
    ("P0496", "Tankentlüftung - Spüldurchfluss zu hoch", "Evaporative emission system high purge flow",
     "evap_valve"),
    ("P0497", "Tankentlüftung - Spüldurchfluss zu gering", "Evaporative emission system low purge flow",
     "evap_valve"),
    ("P0498", "Tankentlüftung Belüftungsventil - Spannung zu niedrig",
     "EVAP system vent valve control circuit low", "evap_valve"),
    ("P0499", "Tankentlüftung Belüftungsventil - Spannung zu hoch",
     "EVAP system vent valve control circuit high", "evap_valve"),
]:
    put(code, de, en, "low", "evap", detail)
p(450, "Drucksensor Tankentlüftung", "EVAP system pressure sensor/switch", SENSOR5, "low", "evap",
  "evap_valve")
p(460, "Tankfüllstandsgeber A", "Fuel level sensor A", SENSOR5, "low", "fuel", "fuel_level")
p(465, "Spüldurchflusssensor Tankentlüftung", "EVAP purge flow sensor", SENSOR5, "low", "evap")
p(470, "Abgasdrucksensor A", "Exhaust pressure sensor A", SENSOR5, "medium", "diesel", "dpf_sensor")
p(475, "Abgasdruckregelventil A", "Exhaust pressure control valve A",
  [("Fehlfunktion", "malfunction"), ("Funktion", "range/performance"), ("Spannung zu niedrig", "low"),
   ("Spannung zu hoch", "high"), ("Signal setzt zeitweise aus", "intermittent")], "medium", "diesel")
for code, de, en in [
    ("P0480", "Kühlerlüfter 1 - Steuerkreis", "Fan 1 control circuit"),
    ("P0481", "Kühlerlüfter 2 - Steuerkreis", "Fan 2 control circuit"),
    ("P0482", "Kühlerlüfter 3 - Steuerkreis", "Fan 3 control circuit"),
    ("P0483", "Kühlerlüfter - Plausibilitätsprüfung fehlgeschlagen", "Fan rationality check"),
    ("P0484", "Kühlerlüfter - Überstrom", "Fan circuit over current"),
    ("P0485", "Kühlerlüfter - Spannungsversorgung oder Masse", "Fan power/ground circuit"),
    ("P0691", "Kühlerlüfter 1 - Steuerkreis Spannung zu niedrig", "Fan 1 control circuit low"),
    ("P0692", "Kühlerlüfter 1 - Steuerkreis Spannung zu hoch", "Fan 1 control circuit high"),
    ("P0693", "Kühlerlüfter 2 - Steuerkreis Spannung zu niedrig", "Fan 2 control circuit low"),
    ("P0694", "Kühlerlüfter 2 - Steuerkreis Spannung zu hoch", "Fan 2 control circuit high"),
    ("P0695", "Kühlerlüfter 3 - Steuerkreis Spannung zu niedrig", "Fan 3 control circuit low"),
    ("P0696", "Kühlerlüfter 3 - Steuerkreis Spannung zu hoch", "Fan 3 control circuit high"),
]:
    put(code, de, en, "high", "cooling", "fan")
put("P0493", "Kühlerlüfter dreht zu schnell (Lüfterkupplung)", "Fan overspeed", "medium", "cooling", "fan")
put("P0494", "Kühlerlüfter dreht zu langsam", "Fan speed low", "medium", "cooling", "fan")
put("P0495", "Kühlerlüfter dreht zu schnell", "Fan speed high", "medium", "cooling", "fan")

# ==========================================================================
# P0500 to P0599: vehicle speed, idle control, auxiliary inputs
# ==========================================================================
p(500, "Geschwindigkeitssensor A", "Vehicle speed sensor A",
  [("Fehlfunktion", "malfunction"), ("Signal unplausibel", "range/performance"),
   ("Signal zu niedrig", "circuit low input"),
   ("Signal setzt aus oder zu hoch", "intermittent/erratic/high")], "medium", "engine", "speed_sensor")
put("P0504", "Bremslichtschalter A und B passen nicht zusammen", "Brake switch A/B correlation",
    "medium", "brakes", "brake_switch")
put("P0505", "Leerlaufregelung - Fehlfunktion", "Idle control system", "low", "engine", "idle", True)
put("P0506", "Leerlaufdrehzahl niedriger als erwartet", "Idle control system RPM lower than expected",
    "low", "engine", "idle", True)
put("P0507", "Leerlaufdrehzahl höher als erwartet (Falschluft oder Drosselklappe nicht angelernt)",
    "Idle control system RPM higher than expected (vacuum leak or throttle body not adapted)",
    "low", "engine", "idle", True)
put("P0508", "Leerlaufsteller - Stromkreis Spannung zu niedrig", "Idle air control system circuit low",
    "low", "engine", "idle")
put("P0509", "Leerlaufsteller - Stromkreis Spannung zu hoch", "Idle air control system circuit high",
    "low", "engine", "idle")
put("P0510", "Leerlaufschalter (Drosselklappe geschlossen) - Fehlfunktion", "Closed throttle position switch",
    "low", "throttle", "idle")
put("P0511", "Leerlaufsteller - Stromkreis", "Idle air control circuit", "low", "engine", "idle")
put("P0512", "Startanforderung - Stromkreis", "Starter request circuit", "medium", "engine")
put("P0513", "Ungültiger Schlüssel (Wegfahrsperre)", "Invalid immobilizer key", "medium", "security",
    "immobilizer")
put("P0514", "Batterietemperatursensor - Signal unplausibel",
    "Battery temperature sensor circuit range/performance", "low", "charging")
p(515, "Batterietemperatursensor", "Battery temperature sensor",
  [("Stromkreis", "circuit"), ("Signal zu niedrig", "circuit low"), ("Signal zu hoch", "circuit high")],
  "low", "charging")
put("P0518", "Leerlaufsteller - Signal setzt zeitweise aus", "Idle air control circuit intermittent",
    "low", "engine", "idle")
put("P0519", "Leerlaufregelung - Funktion", "Idle air control system performance", "low", "engine", "idle")
p(520, "Öldrucksensor/-schalter", "Engine oil pressure sensor/switch",
  [("Stromkreis", "circuit"), ("Signal unplausibel", "range/performance"),
   ("Spannung zu niedrig", "low voltage"), ("Spannung zu hoch", "high voltage")],
  "high", "lubrication", "oil_pressure")
put("P0524", "Öldruck zu niedrig", "Engine oil pressure too low", "critical", "lubrication", "oil_pressure")
put("P0525", "Tempomat-Stellglied - Funktion", "Cruise control servo control circuit range/performance",
    "low", "engine", "cruise")
p(526, "Lüfterdrehzahlsensor", "Fan speed sensor", SPEED4, "medium", "cooling", "fan")
p(530, "Kältemittel-Drucksensor A", "A/C refrigerant pressure sensor A", SENSOR5[:4], "low", "climate", "ac")
put("P0534", "Klimaanlage - Kältemittelverlust", "Air conditioner refrigerant charge loss", "low",
    "climate", "ac")
p(535, "Verdampfer-Temperatursensor", "A/C evaporator temperature sensor", SENSOR5, "low", "climate", "ac")
p(540, "Ansaugluftheizung A", "Intake air heater A",
  [("Stromkreis", "circuit"), ("Spannung zu niedrig", "circuit low"), ("Spannung zu hoch", "circuit high"),
   ("Unterbrechung", "circuit open")], "low", "diesel")
for base, sensor_de, sensor_en in [(544, "Bank 1 Sensor 1", "bank 1 sensor 1"),
                                   (547, "Bank 2 Sensor 1", "bank 2 sensor 1")]:
    p(base, f"Abgastemperatursensor {sensor_de}", f"Exhaust gas temperature sensor {sensor_en}",
      [("Stromkreis", "circuit"), ("Signal zu niedrig", "circuit low"), ("Signal zu hoch", "circuit high")],
      "medium", "exhaust", "egt_sensor")
p(550, "Servolenkungs-Drucksensor/-schalter", "Power steering pressure sensor/switch", SENSOR5, "low",
  "steering")
p(555, "Bremskraftverstärker-Drucksensor", "Brake booster pressure sensor", SENSOR5, "medium", "brakes")
put("P0560", "Bordnetzspannung - Fehlfunktion", "System voltage", "medium", "charging", "battery_low")
put("P0561", "Bordnetzspannung instabil", "System voltage unstable", "medium", "charging", "battery_low")
put("P0562", "Bordnetzspannung zu niedrig (Batterie oder Lichtmaschine)", "System voltage low",
    "medium", "charging", "battery_low")
put("P0563", "Bordnetzspannung zu hoch", "System voltage high", "high", "charging", "voltage_high")
for code, de, en in [
    ("P0564", "Tempomat-Bedienhebel A - Stromkreis", "Cruise control multi-function input A circuit"),
    ("P0565", "Tempomat - Signal Ein", "Cruise control on signal"),
    ("P0566", "Tempomat - Signal Aus", "Cruise control off signal"),
    ("P0567", "Tempomat - Signal Wiederaufnahme", "Cruise control resume signal"),
    ("P0568", "Tempomat - Signal Setzen", "Cruise control set signal"),
    ("P0569", "Tempomat - Signal Verzögern", "Cruise control coast signal"),
    ("P0570", "Tempomat - Signal Beschleunigen", "Cruise control accel signal"),
    ("P0574", "Tempomat - Geschwindigkeit zu hoch", "Cruise control system - vehicle speed too high"),
    ("P0575", "Tempomat - Eingangssignal", "Cruise control input circuit"),
    ("P0576", "Tempomat - Eingangssignal zu niedrig", "Cruise control input circuit low"),
    ("P0577", "Tempomat - Eingangssignal zu hoch", "Cruise control input circuit high"),
    ("P0578", "Tempomat-Bedienhebel A - Signal hängt", "Cruise control multi-function input A circuit stuck"),
    ("P0579", "Tempomat-Bedienhebel A - Signal unplausibel",
     "Cruise control multi-function input A circuit range/performance"),
    ("P0580", "Tempomat-Bedienhebel A - Signal zu niedrig", "Cruise control multi-function input A circuit low"),
    ("P0581", "Tempomat-Bedienhebel A - Signal zu hoch", "Cruise control multi-function input A circuit high"),
]:
    put(code, de, en, "low", "engine", "cruise")
p(571, "Bremslichtschalter A", "Brake switch A",
  [("Stromkreis", "circuit"), ("Signal zu niedrig", "circuit low"), ("Signal zu hoch", "circuit high")],
  "medium", "brakes", "brake_switch")
p(597, "Thermostatheizung", "Thermostat heater", ACTUATOR3, "low", "cooling")

# ==========================================================================
# P0600 to P0699: control unit and outputs
# ==========================================================================
put("P0600", "Serielle Datenverbindung - Fehlfunktion", "Serial communication link", "medium", "network",
    "can_bus")
for code, de, en, sev in [
    ("P0601", "Steuergerät - Prüfsummenfehler im Speicher", "Internal control module memory check sum error",
     "high"),
    ("P0602", "Steuergerät - Programmierfehler", "Control module programming error", "high"),
    ("P0603", "Steuergerät - Dauerspeicher (KAM) fehlerhaft",
     "Internal control module keep alive memory (KAM) error", "medium"),
    ("P0604", "Steuergerät - Arbeitsspeicher (RAM) fehlerhaft",
     "Internal control module random access memory (RAM) error", "high"),
    ("P0605", "Steuergerät - Programmspeicher (ROM) fehlerhaft",
     "Internal control module read only memory (ROM) error", "high"),
    ("P0606", "Steuergerät - Prozessorfehler", "Control module processor", "high"),
    ("P0607", "Steuergerät - Funktion", "Control module performance", "high"),
    ("P060A", "Steuergerät - Überwachungsprozessor Funktion", "Internal control module monitoring processor performance",
     "high"),
    ("P060B", "Steuergerät - Analog-Digital-Wandlung Funktion", "Internal control module A/D processing performance",
     "high"),
    ("P060C", "Steuergerät - Hauptprozessor Funktion", "Internal control module main processor performance", "high"),
    ("P060D", "Steuergerät - Fahrpedalauswertung Funktion",
     "Internal control module accelerator pedal position performance", "high"),
    ("P061B", "Steuergerät - Drehmomentberechnung Funktion", "Internal control module torque calculation performance",
     "high"),
    ("P061C", "Steuergerät - Motordrehzahlauswertung Funktion", "Internal control module engine RPM performance",
     "high"),
    ("P0610", "Steuergerät - Fahrzeugvarianten-Fehler (Codierung)", "Control module vehicle options error",
     "medium"),
]:
    put(code, de, en, sev, "engine", "ecu_internal")
put("P0608", "Steuergerät - Geschwindigkeitsausgang A", "Control module VSS output A", "low", "engine")
put("P0609", "Steuergerät - Geschwindigkeitsausgang B", "Control module VSS output B", "low", "engine")
put("P0611", "Einspritzsteuergerät - Funktion", "Fuel injector control module performance", "high", "injection")
put("P0612", "Einspritzsteuergerät - Relaisansteuerung", "Fuel injector control module relay control",
    "high", "injection")
put("P0613", "Getriebesteuergerät - Prozessor", "TCM processor", "high", "transmission", "ecu_internal")
put("P0614", "Motor- und Getriebesteuergerät passen nicht zusammen", "ECM/TCM incompatible", "medium", "engine")
p(615, "Anlasserrelais", "Starter relay",
  [("Stromkreis", "circuit"), ("Spannung zu niedrig", "circuit low"), ("Spannung zu hoch", "circuit high")],
  "medium", "engine")
put("P0618", "Steuergerät Alternativkraftstoff - Dauerspeicher fehlerhaft",
    "Alternative fuel control module KAM error", "medium", "fuel")
put("P0619", "Steuergerät Alternativkraftstoff - RAM/ROM fehlerhaft",
    "Alternative fuel control module RAM/ROM error", "medium", "fuel")
for code, de, en, sev in [
    ("P0620", "Lichtmaschine - Steuerkreis", "Generator control circuit", "high"),
    ("P0621", "Lichtmaschine Ladekontrolle (Klemme L) - Steuerkreis", "Generator lamp/L terminal control circuit",
     "high"),
    ("P0622", "Lichtmaschine Erregung (Klemme F) - Steuerkreis", "Generator field/F terminal control circuit",
     "high"),
    ("P0625", "Lichtmaschine Erregung (Klemme F) - Spannung zu niedrig", "Generator field/F terminal circuit low",
     "high"),
    ("P0626", "Lichtmaschine Erregung (Klemme F) - Spannung zu hoch", "Generator field/F terminal circuit high",
     "high"),
]:
    put(code, de, en, sev, "charging", "alternator")
put("P0623", "Ladekontrollleuchte - Steuerkreis", "Generator lamp control circuit", "medium", "charging")
put("P0624", "Tankdeckel-Warnleuchte - Steuerkreis", "Fuel cap lamp control circuit", "low", "evap")
p(627, "Kraftstoffpumpe A", "Fuel pump A", ACTUATOR3, "high", "fuel", "fuel_pump")
put("P0630", "Fahrgestellnummer im Motorsteuergerät nicht programmiert oder falsch",
    "VIN not programmed or incompatible - ECM/PCM", "medium", "engine")
put("P0631", "Fahrgestellnummer im Getriebesteuergerät nicht programmiert oder falsch",
    "VIN not programmed or incompatible - TCM", "medium", "transmission")
put("P0632", "Kilometerstand im Motorsteuergerät nicht programmiert", "Odometer not programmed - ECM/PCM",
    "low", "engine")
put("P0633", "Schlüssel im Motorsteuergerät nicht angelernt (Wegfahrsperre)",
    "Immobilizer key not programmed - ECM/PCM", "medium", "security", "immobilizer")
put("P0634", "Steuergerät - Innentemperatur zu hoch", "PCM/ECM/TCM internal temperature too high",
    "high", "engine")
p(635, "Servolenkung", "Power steering",
  [("Steuerkreis", "control circuit"), ("Steuerkreis Spannung zu niedrig", "control circuit low"),
   ("Steuerkreis Spannung zu hoch", "control circuit high")], "medium", "steering")
put("P0638", "Drosselklappensteller Bank 1 - Funktion (Anlernen fällig?)",
    "Throttle actuator control range/performance - bank 1", "high", "throttle", "throttle", True)
put("P0639", "Drosselklappensteller Bank 2 - Funktion", "Throttle actuator control range/performance - bank 2",
    "high", "throttle", "throttle", True)
put("P0640", "Ansaugluftheizung - Steuerkreis", "Intake air heater control circuit", "low", "diesel")
for base, letter in [(641, "A"), (651, "B"), (697, "C")]:
    p(base, f"Sensor-Referenzspannung {letter}", f"Sensor reference voltage {letter}",
      [("Stromkreis oder Unterbrechung", "circuit/open"), ("Spannung zu niedrig", "circuit low"),
       ("Spannung zu hoch", "circuit high")], "high", "engine", "ref_voltage")
put("P0644", "Fahrer-Display - serielle Verbindung", "Driver display serial communication circuit",
    "low", "network")
p(645, "Klimakompressor-Relais", "A/C clutch relay",
  [("Steuerkreis", "control circuit"), ("Steuerkreis Spannung zu niedrig", "control circuit low"),
   ("Steuerkreis Spannung zu hoch", "control circuit high")], "low", "climate", "ac")
put("P0648", "Wegfahrsperrenleuchte - Steuerkreis", "Immobilizer lamp control circuit", "low", "security")
put("P0649", "Tempomatleuchte - Steuerkreis", "Speed control lamp control circuit", "low", "engine")
put("P0650", "Motorkontrollleuchte (MIL) - Steuerkreis", "Malfunction indicator lamp (MIL) control circuit",
    "medium", "engine", "mil_lamp")
put("P0654", "Drehzahlausgang - Stromkreis", "Engine RPM output circuit", "low", "engine")
put("P0655", "Warnleuchte Motortemperatur - Steuerkreis", "Engine hot lamp output control circuit",
    "medium", "cooling")
put("P0656", "Tankanzeige-Ausgang - Stromkreis", "Fuel level output circuit", "low", "fuel")
p(657, "Stellglied-Versorgungsspannung A", "Actuator supply voltage A",
  [("Stromkreis oder Unterbrechung", "circuit/open"), ("Spannung zu niedrig", "circuit low"),
   ("Spannung zu hoch", "circuit high")], "high", "engine", "ecu_power")
for base, bank in [(660, 1), (663, 2)]:
    p(base, f"Saugrohrumschaltung Bank {bank}", f"Intake manifold tuning valve bank {bank}", ACTUATOR3,
      "low", "intake", "imrc")
p(666, "Steuergerät-Innentemperatursensor A", "PCM/ECM/TCM internal temperature sensor A", SENSOR5[:4],
  "low", "engine")
put("P0670", "Glühzeitsteuergerät - Steuerkreis", "Glow plug module control circuit", "medium", "diesel",
    "glow_module")
for cyl in range(1, 13):
    put(num("P", 670 + cyl), f"Glühkerze Zylinder {cyl} - Stromkreis", f"Cylinder {cyl} glow plug circuit",
        "low", "diesel", "glow_plug")
put("P0683", "Glühzeitsteuergerät - Verbindung zum Motorsteuergerät",
    "Glow plug control module to PCM communication circuit", "medium", "diesel", "glow_module")
put("P0684", "Glühzeitsteuergerät - Verbindung zum Motorsteuergerät unplausibel",
    "Glow plug control module to PCM communication circuit range/performance", "medium", "diesel",
    "glow_module")
p(685, "Hauptrelais Motorsteuergerät", "ECM/PCM power relay",
  [("Steuerkreis oder Unterbrechung", "control circuit/open"),
   ("Steuerkreis Spannung zu niedrig", "control circuit low"),
   ("Steuerkreis Spannung zu hoch", "control circuit high"),
   ("Rückmeldung, Stromkreis oder Unterbrechung", "sense circuit/open"),
   ("Rückmeldung Spannung zu niedrig", "sense circuit low"),
   ("Rückmeldung Spannung zu hoch", "sense circuit high")], "high", "engine", "ecu_power")

# ==========================================================================
# P0700 to P0999: transmission
# ==========================================================================
put("P0700", "Getriebesteuerung - Fehler gespeichert (Getriebespeicher auslesen)",
    "Transmission control system (MIL request)", "medium", "transmission", "trans_general")
put("P0701", "Getriebesteuerung - Funktion", "Transmission control system range/performance",
    "medium", "transmission", "trans_general")
put("P0702", "Getriebesteuerung - elektrischer Fehler", "Transmission control system electrical",
    "medium", "transmission", "trans_general")
put("P0703", "Bremsschalter B - Stromkreis", "Brake switch B circuit", "medium", "brakes", "brake_switch")
put("P0704", "Kupplungsschalter - Eingangssignal", "Clutch switch input circuit malfunction",
    "low", "transmission")
p(705, "Getriebebereichssensor A (Wählhebel)", "Transmission range sensor A (PRNDL input)", SENSOR5,
  "medium", "transmission", "trans_sensor")
p(710, "Getriebeöltemperatursensor A", "Transmission fluid temperature sensor A", SENSOR5, "medium",
  "transmission", "trans_sensor")
p(715, "Turbinen-/Eingangsdrehzahlsensor A", "Input/turbine speed sensor A", SPEED4, "medium",
  "transmission", "trans_sensor")
put("P0719", "Bremsschalter B - Signal zu niedrig", "Brake switch B circuit low", "medium", "brakes",
    "brake_switch")
p(720, "Abtriebsdrehzahlsensor", "Output speed sensor", SPEED4, "medium", "transmission", "trans_sensor")
put("P0724", "Bremsschalter B - Signal zu hoch", "Brake switch B circuit high", "medium", "brakes",
    "brake_switch")
p(725, "Motordrehzahl-Eingang (Getriebe)", "Engine speed input", SPEED4, "medium", "transmission",
  "trans_sensor")
put("P0729", "6. Gang - falsche Übersetzung", "Gear 6 incorrect ratio", "high", "transmission", "trans_ratio")
put("P0730", "Falsche Übersetzung", "Incorrect gear ratio", "high", "transmission", "trans_ratio")
for gear in range(1, 6):
    put(num("P", 730 + gear), f"{gear}. Gang - falsche Übersetzung", f"Gear {gear} incorrect ratio",
        "high", "transmission", "trans_ratio")
put("P0736", "Rückwärtsgang - falsche Übersetzung", "Reverse incorrect ratio", "high", "transmission",
    "trans_ratio")
p(737, "Getriebesteuergerät Drehzahlausgang", "TCM engine speed output",
  [("Stromkreis", "circuit"), ("Spannung zu niedrig", "circuit low"), ("Spannung zu hoch", "circuit high")],
  "low", "transmission")
p(740, "Wandlerüberbrückungskupplung", "Torque converter clutch",
  [("Magnetventil Stromkreis oder Unterbrechung", "solenoid circuit/open"),
   ("Funktion oder bleibt offen", "circuit performance/stuck off"),
   ("bleibt geschlossen", "circuit stuck on"), ("elektrischer Fehler", "circuit electrical"),
   ("Signal setzt zeitweise aus", "circuit intermittent")],
  ["medium", "medium", "high", "medium", "medium"], "transmission", "trans_solenoid")
for base, letter in [(745, "A"), (775, "B"), (795, "C")]:
    p(base, f"Druckregelventil {letter}", f"Pressure control solenoid {letter}", SOLENOID5, "high",
      "transmission", "trans_solenoid")
for i, letter in enumerate("ABCDE"):
    p(750 + 5 * i, f"Schaltmagnetventil {letter}", f"Shift solenoid {letter}", SOLENOID5, "high",
      "transmission", "trans_solenoid")
put("P0780", "Schaltfehler", "Shift error", "high", "transmission", "trans_ratio")
for i, (a, b) in enumerate([(1, 2), (2, 3), (3, 4), (4, 5)]):
    put(num("P", 781 + i), f"Schaltung {a}-{b} fehlerhaft", f"{a}-{b} shift", "high", "transmission",
        "trans_ratio")
p(785, "Schalt-/Zeitsteuerungs-Magnetventil", "Shift/timing solenoid",
  [("Fehlfunktion", "malfunction"), ("Funktion", "range/performance"), ("Spannung zu niedrig", "low"),
   ("Spannung zu hoch", "high"), ("Signal setzt zeitweise aus", "intermittent")],
  "medium", "transmission", "trans_solenoid")
put("P0790", "Schalter Normal/Sport - Stromkreis", "Normal/performance switch circuit", "low", "transmission")
p(791, "Zwischenwellen-Drehzahlsensor A", "Intermediate shaft speed sensor A", SPEED4, "medium",
  "transmission", "trans_sensor")
for code, de, en, sev in [
    ("P0801", "Rückwärtsgangsperre - Steuerkreis", "Reverse inhibit control circuit", "low"),
    ("P0803", "Gangsprung 1-4 Magnetventil - Steuerkreis", "1-4 upshift (skip shift) solenoid control circuit",
     "low"),
    ("P0804", "Gangsprung 1-4 Kontrollleuchte - Steuerkreis", "1-4 upshift (skip shift) lamp control circuit",
     "low"),
    ("P0810", "Kupplungspositionsregelung - Fehler", "Clutch position control error", "medium"),
    ("P0811", "Kupplung rutscht zu stark", "Excessive clutch slippage", "high"),
    ("P0812", "Rückwärtsgang-Eingang - Stromkreis", "Reverse input circuit", "low"),
    ("P0813", "Rückwärtsgang-Ausgang - Stromkreis", "Reverse output circuit", "low"),
    ("P0814", "Ganganzeige - Stromkreis", "Transmission range display circuit", "low"),
    ("P0815", "Hochschaltschalter - Stromkreis", "Upshift switch circuit", "low"),
    ("P0816", "Herunterschaltschalter - Stromkreis", "Downshift switch circuit", "low"),
    ("P0817", "Anlasssperre - Stromkreis", "Starter disable circuit", "medium"),
    ("P0818", "Antriebsstrang-Trennschalter - Eingangssignal", "Driveline disconnect switch input circuit",
     "low"),
    ("P0826", "Schalter Hoch-/Herunterschalten - Stromkreis", "Up and down shift switch circuit", "low"),
    ("P0827", "Schalter Hoch-/Herunterschalten - Signal zu niedrig", "Up and down shift switch circuit low",
     "low"),
    ("P0828", "Schalter Hoch-/Herunterschalten - Signal zu hoch", "Up and down shift switch circuit high",
     "low"),
    ("P0850", "Park-/Neutralschalter - Eingangssignal", "Park/neutral switch input circuit", "medium"),
    ("P0894", "Getriebebauteil rutscht", "Transmission component slipping", "high"),
    ("P0897", "Getriebeöl gealtert", "Transmission fluid deteriorated", "medium"),
]:
    detail = "trans_ratio" if code in ("P0811", "P0894") else ""
    put(code, de, en, sev, "transmission", detail)
p(805, "Kupplungspositionssensor", "Clutch position sensor", SENSOR5, "medium", "transmission")
for base, letter in [(830, "A"), (833, "B")]:
    p(base, f"Kupplungspedalschalter {letter}", f"Clutch pedal switch {letter}",
      [("Stromkreis", "circuit"), ("Signal zu niedrig", "circuit low"), ("Signal zu hoch", "circuit high")],
      "low", "transmission")
p(836, "Allradschalter", "Four wheel drive (4WD) switch", SENSOR5[:4], "low", "transmission")
p(840, "Getriebeöldrucksensor/-schalter A", "Transmission fluid pressure sensor/switch A", SENSOR5,
  "medium", "transmission", "trans_sensor")
p(845, "Getriebeöldrucksensor/-schalter B", "Transmission fluid pressure sensor/switch B", SENSOR5,
  "medium", "transmission", "trans_sensor")
put("P0856", "Antriebsschlupfregelung - Eingangssignal", "Traction control input signal", "low", "brakes")
p(880, "Getriebesteuergerät - Spannungsversorgung", "TCM power input signal",
  [("Fehlfunktion", "malfunction"), ("Signal unplausibel", "range/performance"),
   ("Spannung zu niedrig", "low"), ("Spannung zu hoch", "high"),
   ("Signal setzt zeitweise aus", "intermittent")], "medium", "transmission")
p(885, "Hauptrelais Getriebesteuergerät", "TCM power relay", ACTUATOR3, "medium", "transmission")
p(900, "Kupplungssteller", "Clutch actuator",
  [("Stromkreis oder Unterbrechung", "circuit/open"), ("Funktion", "circuit range/performance"),
   ("Spannung zu niedrig", "circuit low"), ("Spannung zu hoch", "circuit high")], "medium", "transmission")
for base, letter in [(960, "A"), (964, "B"), (968, "C")]:
    p(base, f"Druckregelventil {letter}", f"Pressure control solenoid {letter}", ACTUATOR4, "medium",
      "transmission", "trans_solenoid")
for i, letter in enumerate("ABCDE"):
    p(972 + 3 * i, f"Schaltmagnetventil {letter}", f"Shift solenoid {letter}",
      [("Steuerkreis Funktion", "control circuit range/performance"),
       ("Steuerkreis Spannung zu niedrig", "control circuit low"),
       ("Steuerkreis Spannung zu hoch", "control circuit high")], "medium", "transmission", "trans_solenoid")

# ==========================================================================
# P0A00 and up: hybrid and electric propulsion
# ==========================================================================
for code, de, en, sev, detail in [
    ("P0A0A", "Hochvolt-Sicherheitsschleife (Interlock) - Stromkreis", "High voltage system interlock circuit",
     "high", "hybrid_system"),
    ("P0A0D", "Hochvolt-Sicherheitsschleife (Interlock) - Spannung zu hoch",
     "High voltage system interlock circuit high", "high", "hybrid_system"),
    ("P0A0F", "Verbrennungsmotor springt nicht an", "Engine failed to start", "high", "hybrid_system"),
    ("P0A78", "Fahrmotor A - Wechselrichter Funktion", "Drive motor A inverter performance", "high",
     "hybrid_system"),
    ("P0A7E", "Hybridbatterie - Übertemperatur", "Hybrid battery pack over temperature", "high",
     "hybrid_battery"),
    ("P0A7F", "Hybridbatterie - Alterung", "Hybrid battery pack deterioration", "high", "hybrid_battery"),
    ("P0A80", "Hybridbatterie ersetzen", "Replace hybrid battery pack", "high", "hybrid_battery"),
    ("P0A93", "Wechselrichter A - Kühlung Funktion", "Inverter A cooling system performance", "high",
     "hybrid_system"),
    ("P0A94", "Gleichspannungswandler (DC/DC) - Funktion", "DC/DC converter performance", "high",
     "hybrid_system"),
    ("P0AA6", "Hybridbatterie - Isolationsfehler im Hochvoltsystem",
     "Hybrid battery voltage system isolation fault", "critical", "hv_isolation"),
]:
    put(code, de, en, sev, "hybrid", detail)

# ==========================================================================
# P2000 to P2FFF: SAE controlled, second block
# ==========================================================================
put("P2000", "NOx-Speicherkatalysator Bank 1 - Wirkungsgrad zu gering",
    "NOx adsorber efficiency below threshold - bank 1", "medium", "exhaust")
put("P2001", "NOx-Speicherkatalysator Bank 2 - Wirkungsgrad zu gering",
    "NOx adsorber efficiency below threshold - bank 2", "medium", "exhaust")
put("P2002", "Partikelfilter Bank 1 - Wirkungsgrad zu gering",
    "Particulate filter efficiency below threshold - bank 1", "medium", "diesel", "dpf_sensor")
put("P2003", "Partikelfilter Bank 2 - Wirkungsgrad zu gering",
    "Particulate filter efficiency below threshold - bank 2", "medium", "diesel", "dpf_sensor")
for code, state_de, state_en, bank in [("P2004", "hängen offen", "stuck open", 1),
                                       ("P2005", "hängen offen", "stuck open", 2),
                                       ("P2006", "hängen geschlossen", "stuck closed", 1),
                                       ("P2007", "hängen geschlossen", "stuck closed", 2)]:
    put(code, f"Saugrohrklappen Bank {bank} - {state_de}",
        f"Intake manifold runner control {state_en} - bank {bank}", "low", "intake", "imrc")
for base, bank in [(2008, 1), (2011, 2)]:
    p(base, f"Saugrohrklappen Bank {bank}", f"Intake manifold runner control bank {bank}", ACTUATOR3,
      "low", "intake", "imrc")
for base, bank in [(2014, 1), (2019, 2)]:
    p(base, f"Saugrohrklappen-Positionssensor Bank {bank}",
      f"Intake manifold runner position sensor/switch bank {bank}", SENSOR5, "low", "intake", "imrc")
p(2024, "Kraftstoffdampf-Temperatursensor", "EVAP fuel vapor temperature sensor", SENSOR5, "low", "evap")
put("P2029", "Kraftstoffzuheizer abgeschaltet", "Fuel fired heater disabled", "low", "climate")
put("P2030", "Kraftstoffzuheizer - Funktion", "Fuel fired heater performance", "low", "climate")
for base, bank in [(2031, 1), (2034, 2)]:
    p(base, f"Abgastemperatursensor Bank {bank} Sensor 2", f"Exhaust gas temperature sensor bank {bank} sensor 2",
      [("Stromkreis", "circuit"), ("Signal zu niedrig", "circuit low"), ("Signal zu hoch", "circuit high")],
      "medium", "exhaust", "egt_sensor")
p(2047, "AdBlue-Dosierventil Bank 1 Einheit 1", "Reductant injector bank 1 unit 1", ACTUATOR3, "high",
  "diesel", "adblue_dosing")
put("P203A", "AdBlue-Füllstandssensor - Stromkreis", "Reductant level sensor circuit", "medium", "diesel",
    "adblue_level")
put("P203F", "AdBlue-Füllstand zu niedrig", "Reductant level too low", "high", "diesel", "adblue_level")
put("P204F", "AdBlue-System - Funktion", "Reductant system performance", "high", "diesel", "scr")
put("P20E8", "AdBlue-Druck zu niedrig", "Reductant pressure too low", "high", "diesel", "adblue_dosing")
put("P20E9", "AdBlue-Druck zu hoch", "Reductant pressure too high", "high", "diesel", "adblue_dosing")
put("P20EE", "SCR-Katalysator Bank 1 - Wirkungsgrad zu gering",
    "SCR NOx catalyst efficiency below threshold - bank 1", "medium", "diesel", "scr")
put("P2080", "Abgastemperatursensor Bank 1 Sensor 1 - Signal unplausibel",
    "Exhaust gas temperature sensor circuit range/performance - bank 1 sensor 1", "medium", "exhaust",
    "egt_sensor")
put("P2081", "Abgastemperatursensor Bank 1 Sensor 1 - Signal setzt zeitweise aus",
    "Exhaust gas temperature sensor circuit intermittent - bank 1 sensor 1", "medium", "exhaust", "egt_sensor")
put("P2084", "Abgastemperatursensor Bank 1 Sensor 2 - Signal unplausibel",
    "Exhaust gas temperature sensor circuit range/performance - bank 1 sensor 2", "medium", "exhaust",
    "egt_sensor")
for i, (cam, bank) in enumerate([("A", 1), ("B", 1), ("A", 2), ("B", 2)]):
    for j, (lvl_de, lvl_en) in enumerate(LOW_HIGH):
        put(num("P", 2088 + 2 * i + j),
            f"Nockenwellenversteller {cam} Bank {bank} - Steuerkreis {lvl_de}",
            f"{cam} camshaft position actuator control circuit {lvl_en} - bank {bank}",
            "medium", "timing", "vvt")
for code, mix_de, mix_en, bank, detail in [("P2096", "mager", "lean", 1, "lean"),
                                           ("P2097", "fett", "rich", 1, "rich"),
                                           ("P2098", "mager", "lean", 2, "lean"),
                                           ("P2099", "fett", "rich", 2, "rich")]:
    put(code, f"Gemischkorrektur nach Katalysator Bank {bank} - zu {mix_de}",
        f"Post catalyst fuel trim system too {mix_en} - bank {bank}", "medium", "fuel", detail)
p(2100, "Drosselklappensteller Motor", "Throttle actuator control motor",
  [("Stromkreis oder Unterbrechung", "circuit/open"), ("Funktion", "circuit range/performance"),
   ("Stromkreis Spannung zu niedrig", "circuit low"), ("Stromkreis Spannung zu hoch", "circuit high")],
  "high", "throttle", "throttle", True)
for code, de, en, sev in [
    ("P2104", "Drosselklappensteuerung - Leerlauf erzwungen (Notlauf)",
     "Throttle actuator control system - forced idle", "high"),
    ("P2105", "Drosselklappensteuerung - Motor zwangsweise abgestellt",
     "Throttle actuator control system - forced engine shutdown", "critical"),
    ("P2106", "Drosselklappensteuerung - Leistung zwangsweise begrenzt",
     "Throttle actuator control system - forced limited power", "high"),
    ("P2107", "Drosselklappen-Steuermodul - Prozessor", "Throttle actuator control module processor", "high"),
    ("P2108", "Drosselklappen-Steuermodul - Funktion", "Throttle actuator control module performance", "high"),
    ("P2109", "Drosselklappen-/Fahrpedalgeber A - unterer Anschlag unplausibel",
     "Throttle/pedal position sensor A minimum stop performance", "high"),
    ("P2110", "Drosselklappensteuerung - Drehzahl zwangsweise begrenzt",
     "Throttle actuator control system - forced limited RPM", "high"),
    ("P2111", "Drosselklappe hängt offen", "Throttle actuator control system - stuck open", "high"),
    ("P2112", "Drosselklappe hängt geschlossen", "Throttle actuator control system - stuck closed", "high"),
    ("P2118", "Drosselklappensteller Motorstrom - Funktion",
     "Throttle actuator control motor current range/performance", "high"),
    ("P2119", "Drosselklappengehäuse - Funktion", "Throttle actuator control throttle body range/performance",
     "high"),
    ("P2135", "Drosselklappen-/Fahrpedalgeber A und B - Spannungen passen nicht zusammen",
     "Throttle/pedal position sensor A/B voltage correlation", "high"),
    ("P2136", "Drosselklappen-/Fahrpedalgeber A und C - Spannungen passen nicht zusammen",
     "Throttle/pedal position sensor A/C voltage correlation", "high"),
    ("P2137", "Drosselklappen-/Fahrpedalgeber B und C - Spannungen passen nicht zusammen",
     "Throttle/pedal position sensor B/C voltage correlation", "high"),
    ("P2163", "Drosselklappen-/Fahrpedalgeber A - oberer Anschlag unplausibel",
     "Throttle/pedal position sensor A maximum stop performance", "high"),
]:
    put(code, de, en, sev, "throttle", "throttle", True)
for base, letter in [(2120, "D"), (2125, "E"), (2130, "F")]:
    p(base, f"Fahrpedal-/Drosselklappengeber {letter}", f"Throttle/pedal position sensor {letter}", SENSOR5,
      "high", "throttle", "pedal")
for code, pair, throttle in [("P2138", "D/E", True), ("P2139", "D/F", False), ("P2140", "E/F", False)]:
    a, b = pair.split("/")
    put(code, f"Fahrpedal-/Drosselklappengeber {a} und {b} - Spannungen passen nicht zusammen",
        f"Throttle/pedal position sensor {pair} voltage correlation", "high", "throttle", "pedal", throttle)
put("P2141", "AGR-Drosselklappe - Steuerkreis Spannung zu niedrig",
    "Exhaust gas recirculation throttle control circuit low", "medium", "egr", "egr")
put("P2142", "AGR-Drosselklappe - Steuerkreis Spannung zu hoch",
    "Exhaust gas recirculation throttle control circuit high", "medium", "egr", "egr")
for i, group in enumerate("ABCD"):
    p(2146 + 3 * i, f"Einspritzventile Gruppe {group} - Versorgungsspannung",
      f"Fuel injector group {group} supply voltage",
      [("Stromkreis oder Unterbrechung", "circuit/open"), ("Spannung zu niedrig", "circuit low"),
       ("Spannung zu hoch", "circuit high")], "high", "injection", "injector_circuit")
p(2158, "Geschwindigkeitssensor B", "Vehicle speed sensor B",
  [("Fehlfunktion", "malfunction"), ("Signal unplausibel", "range/performance"),
   ("Signal zu niedrig", "circuit low"), ("Signal setzt zeitweise aus", "intermittent/erratic")],
  "medium", "engine", "speed_sensor")
put("P2162", "Geschwindigkeitssensor A und B passen nicht zusammen", "Vehicle speed sensor A/B correlation",
    "medium", "engine", "speed_sensor")
for start, where_de, where_en in [(2177, "außerhalb des Leerlaufs", "off idle"),
                                  (2187, "im Leerlauf", "at idle"),
                                  (2191, "bei höherer Last", "at higher load")]:
    for i, (bank, mix_de, mix_en, detail) in enumerate([(1, "mager", "lean", "lean"), (1, "fett", "rich", "rich"),
                                                        (2, "mager", "lean", "lean"), (2, "fett", "rich", "rich")]):
        put(num("P", start + i), f"Gemisch {where_de} zu {mix_de}, Bank {bank}",
            f"System too {mix_en} {where_en} - bank {bank}", "medium", "fuel", detail)
put("P2181", "Kühlsystem - Funktion", "Cooling system performance", "medium", "cooling", "thermostat")
p(2182, "Kühlmitteltemperatursensor 2", "Engine coolant temperature sensor 2", SENSOR5, "medium", "cooling",
  "ect_sensor")
for code, bank, sensor, mix_de, mix_en in [("P2195", 1, 1, "mager", "lean"), ("P2196", 1, 1, "fett", "rich"),
                                           ("P2197", 2, 1, "mager", "lean"), ("P2198", 2, 1, "fett", "rich"),
                                           ("P2270", 1, 2, "mager", "lean"), ("P2271", 1, 2, "fett", "rich"),
                                           ("P2272", 2, 2, "mager", "lean"), ("P2273", 2, 2, "fett", "rich")]:
    wd, we = bs(bank, sensor)
    sev, detail = ("medium", "o2_up") if sensor == 1 else ("low", "o2_down")
    put(code, f"Lambdasonde {wd} - Signal hängt auf {mix_de}", f"O2 sensor signal biased/stuck {mix_en} - {we}",
        sev, "exhaust", detail)
put("P2199", "Ansauglufttemperatursensor 1 und 2 passen nicht zusammen",
    "Intake air temperature sensor 1/2 correlation", "low", "intake", "iat")
p(2200, "NOx-Sensor Bank 1", "NOx sensor bank 1", SENSOR5, "medium", "diesel", "nox_sensor")
p(2205, "NOx-Sensor Bank 1 Heizung", "NOx sensor heater bank 1", ACTUATOR3, "medium", "diesel", "nox_sensor")
p(2226, "Luftdrucksensor", "Barometric pressure", SENSOR5, "low", "intake", "map")
put("P2231", "Lambdasonde Bank 1, Sonde 1 - Signalkreis hat Kurzschluss zum Heizkreis",
    "O2 sensor signal circuit shorted to heater circuit - bank 1 sensor 1", "medium", "exhaust", "o2_up")
for base, kind_de, kind_en in [(2237, "Pumpstrom-Steuerkreis", "positive current control circuit"),
                               (2251, "Minus-Steuerkreis", "negative current control circuit")]:
    for k, bank in enumerate([1, 2]):
        wd, we = bs(bank, 1)
        p(base + 3 * k, f"Lambdasonde {wd} - {kind_de}", f"O2 sensor {kind_en} - {we}",
          [("Stromkreis oder Unterbrechung", "open"), ("Spannung zu niedrig", "low"),
           ("Spannung zu hoch", "high")], "medium", "exhaust", "o2_up")
for k, bank in enumerate([1, 2]):
    wd, we = bs(bank, 1)
    p(2243 + 4 * k, f"Lambdasonde {wd} - Referenzspannung", f"O2 sensor reference voltage - {we}",
      [("Stromkreis oder Unterbrechung", "circuit/open"), ("Funktion", "performance"),
       ("Spannung zu niedrig", "circuit low"), ("Spannung zu hoch", "circuit high")],
      "medium", "exhaust", "o2_up")
put("P2257", "Sekundärluftsystem Steuerung A - Spannung zu niedrig",
    "Secondary air injection system control A circuit low", "low", "egr", "secondary_air")
put("P2258", "Sekundärluftsystem Steuerung A - Spannung zu hoch",
    "Secondary air injection system control A circuit high", "low", "egr", "secondary_air")
put("P2261", "Schubumluftventil (Turbo-Bypass) - mechanischer Fehler",
    "Turbo/super charger bypass valve - mechanical", "medium", "turbo", "underboost")
put("P2263", "Turbolader - Ladedrucksystem Funktion", "Turbo/super charger boost system performance",
    "medium", "turbo", "underboost")
p(2264, "Wasser-im-Kraftstoff-Sensor", "Water in fuel sensor", SENSOR5, "low", "diesel")
put("P2269", "Wasser im Kraftstoff", "Water in fuel condition", "high", "diesel", "water_in_fuel")
put("P2279", "Ansaugsystem undicht (Falschluft)", "Intake air system leak", "medium", "intake", "lean")
put("P2280", "Ansaugluft gedrosselt oder Falschluft zwischen Luftfilter und Luftmassenmesser",
    "Air flow restriction / air leak between air filter and MAF", "medium", "intake", "lean")
put("P2281", "Falschluft zwischen Luftmassenmesser und Drosselklappe", "Air leak between MAF and throttle body",
    "medium", "intake", "lean")
put("P2282", "Falschluft zwischen Drosselklappe und Einlassventilen",
    "Air leak between throttle body and intake valves", "medium", "intake", "lean")
put("P2293", "Kraftstoffdruckregler 2 - Funktion", "Fuel pressure regulator 2 performance", "high", "fuel",
    "fuel_pressure")
p(2294, "Kraftstoffdruckregler 2", "Fuel pressure regulator 2",
  [("Steuerkreis", "control circuit"), ("Steuerkreis Spannung zu niedrig", "control circuit low"),
   ("Steuerkreis Spannung zu hoch", "control circuit high")], "high", "fuel", "fuel_pressure")
put("P2299", "Bremspedal und Gaspedal gleichzeitig betätigt (unplausibel)",
    "Brake pedal position / accelerator pedal position incompatible", "medium", "throttle", "pedal")
for i in range(12):
    letter = "ABCDEFGHIJKL"[i]
    coil_de = f"Zündspule {letter} (meist Zylinder {i + 1})"
    coil_en = f"Ignition coil {letter} (usually cylinder {i + 1})"
    put(num("P", 2300 + 3 * i), f"{coil_de} - Primärkreis Spannung zu niedrig",
        f"{coil_en} primary control circuit low", "high", "ignition", "coil")
    put(num("P", 2301 + 3 * i), f"{coil_de} - Primärkreis Spannung zu hoch",
        f"{coil_en} primary control circuit high", "high", "ignition", "coil")
    put(num("P", 2302 + 3 * i), f"{coil_de} - Sekundärkreis", f"{coil_en} secondary circuit",
        "high", "ignition", "coil")
for cyl in range(1, 13):
    put(num("P", 2335 + cyl), f"Zylinder {cyl} - Klopfgrenze überschritten",
        f"Cylinder {cyl} above knock threshold", "medium", "ignition", "knock")
p(2400, "Leckdiagnosepumpe Tankentlüftung", "EVAP system leak detection pump",
  [("Steuerkreis oder Unterbrechung", "control circuit/open"),
   ("Steuerkreis Spannung zu niedrig", "control circuit low"),
   ("Steuerkreis Spannung zu hoch", "control circuit high"),
   ("Rückmeldung, Stromkreis oder Unterbrechung", "sense circuit/open"),
   ("Rückmeldung unplausibel", "sense circuit range/performance"),
   ("Rückmeldung Spannung zu niedrig", "sense circuit low"),
   ("Rückmeldung Spannung zu hoch", "sense circuit high"),
   ("Rückmeldung setzt zeitweise aus", "sense circuit intermittent/erratic")], "low", "evap", "evap_valve")
put("P2413", "Abgasrückführung - Systemfunktion", "Exhaust gas recirculation system performance",
    "medium", "egr", "egr")
put("P2414", "Lambdasonde Bank 1, Sonde 1 - Abgasprobe fehlerhaft (Undichtigkeit?)",
    "O2 sensor exhaust sample error - bank 1 sensor 1", "medium", "exhaust", "o2_up")
put("P2415", "Lambdasonde Bank 2, Sonde 1 - Abgasprobe fehlerhaft (Undichtigkeit?)",
    "O2 sensor exhaust sample error - bank 2 sensor 1", "medium", "exhaust", "o2_up")
put("P2422", "Tankentlüftung Belüftungsventil - hängt geschlossen", "Evaporative emission system vent valve stuck closed",
    "low", "evap", "evap_valve")
p(2425, "AGR-Kühlung Umschaltventil", "Exhaust gas recirculation cooling valve", ACTUATOR3, "medium", "egr",
  "egr")
put("P2428", "Abgastemperatur zu hoch, Bank 1", "Exhaust gas temperature too high - bank 1", "high", "exhaust")
put("P2429", "Abgastemperatur zu hoch, Bank 2", "Exhaust gas temperature too high - bank 2", "high", "exhaust")
for base, bank in [(2430, 1), (2435, 2)]:
    p(base, f"Sekundärluft-Durchfluss-/Drucksensor Bank {bank}",
      f"Secondary air injection system air flow/pressure sensor bank {bank}", SENSOR5, "low", "egr",
      "secondary_air")
for code, what_de, what_en, state_de, state_en, bank, sev in [
    ("P2440", "Umschaltventil", "switching valve", "hängt offen", "stuck open", 1, "low"),
    ("P2441", "Umschaltventil", "switching valve", "hängt geschlossen", "stuck closed", 1, "low"),
    ("P2442", "Umschaltventil", "switching valve", "hängt offen", "stuck open", 2, "low"),
    ("P2443", "Umschaltventil", "switching valve", "hängt geschlossen", "stuck closed", 2, "low"),
    ("P2444", "Pumpe", "pump", "läuft dauernd", "stuck on", 1, "medium"),
    ("P2445", "Pumpe", "pump", "läuft nicht", "stuck off", 1, "low"),
    ("P2446", "Pumpe", "pump", "läuft dauernd", "stuck on", 2, "medium"),
    ("P2447", "Pumpe", "pump", "läuft nicht", "stuck off", 2, "low"),
]:
    put(code, f"Sekundärluft-{what_de} Bank {bank} - {state_de}",
        f"Secondary air injection system {what_en} {state_en} - bank {bank}", sev, "egr", "secondary_air")
put("P244A", "Partikelfilter Bank 1 - Differenzdruck zu niedrig",
    "Diesel particulate filter differential pressure too low - bank 1", "medium", "diesel", "dpf_sensor")
put("P244B", "Partikelfilter Bank 1 - Differenzdruck zu hoch",
    "Diesel particulate filter differential pressure too high - bank 1", "high", "diesel", "dpf_soot")
put("P244C", "Abgastemperatur für die Partikelfilter-Regeneration zu niedrig, Bank 1",
    "Exhaust temperature too low for particulate filter regeneration - bank 1", "medium", "diesel", "dpf_soot")
put("P244D", "Abgastemperatur für die Partikelfilter-Regeneration zu hoch, Bank 1",
    "Exhaust temperature too high for particulate filter regeneration - bank 1", "high", "diesel")
p(2452, "Differenzdrucksensor Partikelfilter A", "Diesel particulate filter pressure sensor A", SENSOR5,
  "medium", "diesel", "dpf_sensor")
put("P2457", "AGR-Kühlung - Funktion", "Exhaust gas recirculation cooling system performance", "medium", "egr",
    "egr")
put("P2458", "Partikelfilter - Regeneration dauert zu lange", "Diesel particulate filter regeneration duration",
    "high", "diesel", "dpf_soot")
put("P2459", "Partikelfilter - Regeneration zu häufig", "Diesel particulate filter regeneration frequency",
    "high", "diesel", "dpf_soot")
put("P2463", "Partikelfilter - Rußbeladung zu hoch (verstopft)",
    "Diesel particulate filter restriction - soot accumulation", "high", "diesel", "dpf_soot")
put("P2500", "Lichtmaschine Ladekontrolle (Klemme L) - Spannung zu niedrig", "Generator lamp/L terminal circuit low",
    "high", "charging", "alternator")
put("P2501", "Lichtmaschine Ladekontrolle (Klemme L) - Spannung zu hoch", "Generator lamp/L terminal circuit high",
    "high", "charging", "alternator")
put("P2502", "Ladespannung - Fehlfunktion", "Charging system voltage", "high", "charging", "alternator")
put("P2503", "Ladespannung zu niedrig", "Charging system voltage low", "high", "charging", "alternator")
put("P2504", "Ladespannung zu hoch", "Charging system voltage high", "high", "charging", "voltage_high")
p(2505, "Motorsteuergerät - Spannungseingang", "ECM/PCM power input signal",
  [("Fehlfunktion", "malfunction"), ("Signal unplausibel", "range/performance"),
   ("Spannung zu niedrig", "low"), ("Spannung zu hoch", "high"),
   ("Signal setzt zeitweise aus", "intermittent")], "high", "engine", "ecu_power")
put("P2510", "Hauptrelais Motorsteuergerät - Rückmeldung unplausibel",
    "ECM/PCM power relay sense circuit range/performance", "high", "engine", "ecu_power")
put("P2560", "Kühlmittelstand zu niedrig", "Engine coolant level low", "high", "cooling", "coolant_level")
p(2562, "Ladedrucksteller Positionssensor A", "Turbocharger boost control position sensor A", SENSOR5[:4],
  "medium", "turbo", "boost_actuator")
p(2600, "Kühlmittelpumpe A", "Coolant pump A", ACTUATOR4, "high", "cooling")
put("P2610", "Steuergerät - Zeitmesser bei abgestelltem Motor fehlerhaft",
    "ECM/PCM internal engine off timer performance", "low", "engine")
put("P2614", "Nockenwellensignal-Ausgang - Stromkreis oder Unterbrechung",
    "Camshaft position signal output circuit/open", "medium", "engine")
put("P2617", "Kurbelwellensignal-Ausgang - Stromkreis oder Unterbrechung",
    "Crankshaft position signal output circuit/open", "medium", "engine")
put("P2BA7", "NOx-Grenzwert überschritten - AdBlue-Tank leer", "NOx exceedance - empty reagent tank",
    "high", "diesel", "adblue_level")
put("P2BA9", "NOx-Grenzwert überschritten - AdBlue-Qualität unzureichend",
    "NOx exceedance - insufficient reagent quality", "high", "diesel", "scr")
put("P2BAC", "NOx-Grenzwert überschritten - Abgasrückführung abgeschaltet",
    "NOx exceedance - deactivation of EGR", "medium", "diesel", "egr")

# ==========================================================================
# P3400 to P34FF: cylinder deactivation (SAE controlled part of P3)
# ==========================================================================
put("P3400", "Zylinderabschaltung Bank 1 - Fehlfunktion", "Cylinder deactivation system - bank 1",
    "medium", "timing", "cyl_deactivation")
for cyl in range(1, 9):
    base = 3401 + 8 * (cyl - 1)
    for i, (valve_de, valve_en) in enumerate([("Abschaltung/Einlassventilsteuerung", "deactivation/intake valve control"),
                                              ("Auslassventilsteuerung", "exhaust valve control")]):
        p(base + 4 * i, f"Zylinder {cyl} {valve_de}", f"Cylinder {cyl} {valve_en}",
          [("Stromkreis oder Unterbrechung", "circuit/open"), ("Funktion", "performance"),
           ("Spannung zu niedrig", "circuit low"), ("Spannung zu hoch", "circuit high")],
          "medium", "timing", "cyl_deactivation")
put("P3497", "Zylinderabschaltung Bank 2 - Fehlfunktion", "Cylinder deactivation system - bank 2",
    "medium", "timing", "cyl_deactivation")

# ==========================================================================
# U0xxx: network
# ==========================================================================
_BUS_WIRES = [
    ("Fehlfunktion", "communication bus"), ("Funktion", "communication bus performance"),
    ("Leitung High - Unterbrechung", "bus (+) open"), ("Leitung High - Spannung zu niedrig", "bus (+) low"),
    ("Leitung High - Spannung zu hoch", "bus (+) high"), ("Leitung Low - Unterbrechung", "bus (-) open"),
    ("Leitung Low - Spannung zu niedrig", "bus (-) low"), ("Leitung Low - Spannung zu hoch", "bus (-) high"),
    ("Leitung Low hat Kurzschluss zu Leitung High", "bus (-) shorted to bus (+)"),
]
for start, de, en, sev in [(1, "High-Speed-CAN-Bus", "High speed CAN", "high"),
                           (10, "Medium-Speed-CAN-Bus", "Medium speed CAN", "high"),
                           (19, "Low-Speed-CAN-Bus", "Low speed CAN", "medium")]:
    family(T, "U", start, de, en, _BUS_WIRES, sev, "network", "can_bus")
for code, letter in [("U0073", "A"), ("U0074", "B"), ("U0075", "C")]:
    put(code, f"Steuergeräte-Datenbus {letter} abgeschaltet", f"Control module communication bus {letter} off",
        "high", "network", "can_bus")

# Lost communication: module, German name in the dative, English name, severity.
_MODULES = [
    (0x100, "Motorsteuergerät", "ECM/PCM A", "high"),
    (0x101, "Getriebesteuergerät", "transmission control module", "high"),
    (0x102, "Verteilergetriebe-Steuergerät", "transfer case control module", "medium"),
    (0x103, "Schaltmodul A", "gear shift control module A", "medium"),
    (0x104, "Tempomat-Steuergerät", "cruise control module", "low"),
    (0x105, "Einspritz-Steuergerät", "fuel injector control module", "high"),
    (0x106, "Glühzeitsteuergerät", "glow plug control module", "medium"),
    (0x107, "Drosselklappen-Steuergerät", "throttle actuator control module", "high"),
    (0x108, "Steuergerät für Alternativkraftstoff", "alternative fuel control module", "medium"),
    (0x109, "Kraftstoffpumpen-Steuergerät", "fuel pump control module", "high"),
    (0x110, "Fahrmotor-Steuergerät A", "drive motor control module A", "high"),
    (0x111, "Batteriesteuergerät A", "battery energy control module A", "high"),
    (0x112, "Batteriesteuergerät B", "battery energy control module B", "high"),
    (0x114, "Allradkupplungs-Steuergerät", "four-wheel drive clutch control module", "medium"),
    (0x115, "Motorsteuergerät B", "ECM/PCM B", "high"),
    (0x121, "ABS-Steuergerät", "anti-lock brake system (ABS) control module", "high"),
    (0x122, "Fahrdynamik-Steuergerät (ESP)", "vehicle dynamics control module", "high"),
    (0x123, "Gierratensensor", "yaw rate sensor module", "medium"),
    (0x124, "Querbeschleunigungssensor", "lateral acceleration sensor module", "medium"),
    (0x125, "Beschleunigungssensor (mehrachsig)", "multi-axis acceleration sensor module", "medium"),
    (0x126, "Lenkwinkelsensor", "steering angle sensor module", "medium"),
    (0x127, "Reifendruck-Steuergerät", "tire pressure monitor module", "low"),
    (0x128, "Feststellbremsen-Steuergerät", "park brake control module", "medium"),
    (0x129, "Bremssystem-Steuergerät", "brake system control module", "high"),
    (0x130, "Lenkkraft-Steuergerät", "steering effort control module", "medium"),
    (0x131, "Servolenkungs-Steuergerät", "power steering control module", "high"),
    (0x132, "Niveauregelungs-Steuergerät", "ride level control module", "medium"),
    (0x140, "Bordnetzsteuergerät", "body control module", "medium"),
    (0x141, "Bordnetzsteuergerät A", "body control module A", "medium"),
    (0x142, "Bordnetzsteuergerät B", "body control module B", "medium"),
    (0x151, "Airbag-Steuergerät", "restraints control module", "high"),
    (0x155, "Kombiinstrument", "instrument panel cluster (IPC) control module", "medium"),
    (0x156, "Informationsdisplay A", "information center A", "low"),
    (0x159, "Einparkhilfe-Steuergerät A", "parking assist control module A", "low"),
    (0x164, "Klimasteuergerät", "HVAC control module", "low"),
    (0x167, "Wegfahrsperren-Steuergerät", "vehicle immobilizer control module", "medium"),
    (0x168, "Diebstahlwarnanlage", "vehicle security control module", "low"),
    (0x184, "Radio", "radio", "low"),
    (0x198, "Telematik-Steuergerät A", "telematic control module A", "low"),
    (0x199, "Türsteuergerät A", "door control module A", "low"),
    (0x200, "Türsteuergerät B", "door control module B", "low"),
    (0x208, "Sitzsteuergerät A", "seat control module A", "low"),
    (0x214, "Funkfernbedienung", "remote function actuation", "low"),
    (0x235, "Abstandssensor des Tempomaten", "cruise control front distance range sensor", "low"),
    (0x293, "Hybrid-Steuergerät", "hybrid powertrain control module", "high"),
]
for number, de, en, sev in _MODULES:
    put(f"U{number:04X}", f"Keine Verbindung zum Steuergerät: {de}", f"Lost communication with {en}",
        sev, "network", "lost_comm")
put("U0300", "Software im Steuergerät nicht kompatibel", "Internal control module software incompatibility",
    "medium", "network")
put("U0301", "Software nicht kompatibel mit dem Motorsteuergerät", "Software incompatibility with ECM/PCM",
    "medium", "network")
put("U0302", "Software nicht kompatibel mit dem Getriebesteuergerät",
    "Software incompatibility with transmission control module", "medium", "network")
for code, de, en in [
    ("U0401", "Motorsteuergerät", "ECM/PCM A"),
    ("U0402", "Getriebesteuergerät", "transmission control module"),
    ("U0415", "ABS-Steuergerät", "anti-lock brake system (ABS) control module"),
    ("U0416", "Fahrdynamik-Steuergerät (ESP)", "vehicle dynamics control module"),
    ("U0418", "Bremssystem-Steuergerät", "brake system control module"),
    ("U0422", "Bordnetzsteuergerät", "body control module"),
    ("U0423", "Kombiinstrument", "instrument panel control module"),
    ("U0428", "Lenkwinkelsensor", "steering angle sensor module"),
]:
    put(code, f"Unplausible Daten vom Steuergerät: {de}", f"Invalid data received from {en}",
        "medium", "network", "invalid_data")

# ==========================================================================
# C0xxx and B0xxx: only the codes whose SAE meaning is settled
# ==========================================================================
for code, where_de, where_en in [("C0035", "vorne links", "left front"), ("C0040", "vorne rechts", "right front"),
                                 ("C0045", "hinten links", "left rear"), ("C0050", "hinten rechts", "right rear")]:
    put(code, f"Raddrehzahlsensor {where_de} - Stromkreis oder Signal",
        f"{where_en.capitalize()} wheel speed sensor - circuit or signal", "high", "brakes", "wheel_speed")
put("C0110", "ABS-Pumpenmotor - Stromkreis", "ABS pump motor circuit", "high", "brakes", "abs_unit")
put("C0121", "ABS-Ventilrelais - Stromkreis", "ABS valve relay circuit", "high", "brakes", "abs_unit")
put("C0161", "ABS/ASR - Bremslichtschalter Stromkreis", "ABS/TCS brake switch circuit", "medium", "brakes",
    "brake_switch")
for code, de, en in [
    ("B0001", "Fahrer-Frontairbag Stufe 1 - Zündkreis", "Driver frontal stage 1 deployment control"),
    ("B0002", "Fahrer-Frontairbag Stufe 2 - Zündkreis", "Driver frontal stage 2 deployment control"),
    ("B0010", "Beifahrer-Frontairbag Stufe 1 - Zündkreis", "Passenger frontal stage 1 deployment control"),
    ("B0011", "Beifahrer-Frontairbag Stufe 2 - Zündkreis", "Passenger frontal stage 2 deployment control"),
    ("B0020", "Seitenairbag links - Zündkreis", "Left side airbag deployment control"),
    ("B0028", "Seitenairbag rechts - Zündkreis", "Right side airbag deployment control"),
]:
    put(code, de, en, "high", "airbag", "airbag")
