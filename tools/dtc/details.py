"""Plain-language help for the common faults.

Each block answers the questions a driver without workshop experience has:
may I keep driving, what is the usual cause (most likely first), what can I
check myself, and roughly how expensive the repair tends to be. One block is
shared by every code that describes the same problem, for example all
misfire codes or all oxygen sensor heater codes.

The cost class is a rough workshop price including parts:
cheap below about 200 Euro, medium about 200 to 800, expensive above. It is
left empty where the spread between the likely causes is too wide to say.
"""

from __future__ import annotations

from .model import COSTS, check_text

# key -> (cost, advice, causes, checks); texts are (German, English).
DETAILS: dict[str, tuple] = {}


def detail(key: str, cost: str, advice: tuple[str, str],
           causes: list[tuple[str, str]], checks: list[tuple[str, str]]) -> None:
    assert key not in DETAILS, key
    assert cost in COSTS, cost
    assert causes and checks, key
    for de, en in [advice, *causes, *checks]:
        check_text(key, de)
        check_text(key, en)
    DETAILS[key] = (cost, advice, causes, checks)


# --------------------------------------------------------------------------
# Combustion and ignition
# --------------------------------------------------------------------------
detail(
    "misfire", "medium",
    ("Blinkt die Motorkontrollleuchte, sofort Gas wegnehmen und bald anhalten: "
     "unverbrannter Kraftstoff kann den Katalysator in kurzer Zeit zerstören. Leuchtet "
     "sie nur dauerhaft, behutsam und ohne Volllast zur Werkstatt fahren.",
     "If the check engine light is flashing, ease off at once and stop soon: unburnt "
     "fuel can destroy the catalytic converter within a short time. If it is only lit "
     "steadily, drive gently and without full load to a workshop."),
    [("Zündkerzen verschlissen", "Worn spark plugs"),
     ("Zündspule defekt", "Faulty ignition coil"),
     ("Einspritzventil undicht oder verstopft", "Leaking or clogged injector"),
     ("Falschluft im Ansaugtrakt", "Vacuum leak in the intake"),
     ("Zu wenig Kompression (Ventile, Kolbenringe, Zylinderkopfdichtung)",
      "Low compression (valves, piston rings, head gasket)")],
    [("Bei einem einzelnen Zylinder: Zündspule mit dem Nachbarzylinder tauschen, Fehler "
      "löschen und schauen, ob der Aussetzer mitwandert",
      "For a single cylinder: swap the ignition coil with the neighbouring cylinder, "
      "clear the code and see whether the misfire follows the coil"),
     ("Im Serviceheft nachsehen, wann die Zündkerzen zuletzt gewechselt wurden",
      "Check in the service book when the spark plugs were last replaced"),
     ("Bei laufendem Motor auf Zischen am Ansaugtrakt hören",
      "Listen for hissing at the intake with the engine running")])

detail(
    "coil", "cheap",
    ("Mit einer defekten Zündspule fehlt meist ein Zylinder, der Motor ruckelt. Behutsam "
     "zur Werkstatt fahren, bei blinkender Motorkontrollleuchte anhalten.",
     "With a faulty ignition coil one cylinder usually drops out and the engine "
     "stumbles. Drive gently to a workshop and stop if the check engine light flashes."),
    [("Zündspule defekt", "Faulty ignition coil"),
     ("Stecker an der Spule locker oder korrodiert", "Loose or corroded coil connector"),
     ("Verschlissene Zündkerze überlastet die Spule", "A worn spark plug overloads the coil"),
     ("Ansteuerung im Steuergerät (selten)", "Driver stage in the control unit (rare)")],
    [("Stecker an den Zündspulen auf festen Sitz und Korrosion prüfen",
      "Check the coil connectors for a firm fit and corrosion"),
     ("Spule mit einem Nachbarzylinder tauschen und schauen, ob der Fehler mitwandert",
      "Swap the coil with a neighbouring cylinder and see whether the fault follows it")])

detail(
    "knock", "medium",
    ("Weiterfahren ist möglich. Das Steuergerät nimmt vorsichtshalber Zündung zurück, der "
     "Motor hat dann etwas weniger Leistung und braucht mehr. Bis zur Klärung keine "
     "Volllast.",
     "You can keep driving. The control unit retards the ignition as a precaution, so "
     "the engine has a little less power and uses more fuel. Avoid full load until it "
     "is sorted out."),
    [("Klopfsensor defekt oder Stecker korrodiert", "Faulty knock sensor or corroded connector"),
     ("Sensor mit falschem Drehmoment angezogen", "Sensor tightened to the wrong torque"),
     ("Kabel beschädigt", "Damaged wiring"),
     ("Echtes Klopfen: falscher Kraftstoff, Ablagerungen, zu heißer Motor",
      "Real knocking: wrong fuel, carbon deposits, engine running too hot")],
    [("Den richtigen Kraftstoff getankt? Die Oktanzahl steht meist an der Tankklappe",
      "Did you fill up with the right fuel? The octane rating is usually on the fuel flap"),
     ("Stecker am Klopfsensor prüfen", "Check the knock sensor connector")])

detail(
    "crank", "cheap",
    ("Der Motor kann jederzeit ausgehen oder nicht mehr anspringen. Nur noch kurze Strecken "
     "fahren, am besten direkt zur Werkstatt.",
     "The engine may cut out at any time or fail to restart. Only drive short distances, "
     "ideally straight to a workshop."),
    [("Kurbelwellensensor defekt, oft nur bei warmem Motor",
      "Faulty crankshaft sensor, often only when the engine is hot"),
     ("Stecker oder Kabel beschädigt", "Damaged connector or wiring"),
     ("Geberrad beschädigt oder Metallabrieb am Sensor",
      "Damaged reluctor wheel or metal debris on the sensor")],
    [("Geht der Motor vor allem warm aus und springt nach dem Abkühlen wieder an? Das ist "
      "typisch für diesen Sensor",
      "Does the engine cut out mainly when hot and start again after cooling down? That "
      "is typical for this sensor"),
     ("Stecker am Sensor prüfen", "Check the sensor connector")])

detail(
    "cam_sensor", "cheap",
    ("Weiterfahren ist meist möglich, der Motor kann aber schlechter anspringen oder ins "
     "Notlaufprogramm gehen. Bald prüfen lassen.",
     "You can usually keep driving, but the engine may start poorly or go into limp mode. "
     "Have it checked soon."),
    [("Nockenwellensensor defekt", "Faulty camshaft sensor"),
     ("Stecker oder Kabel, manchmal Öl im Stecker", "Connector or wiring, sometimes oil in the connector"),
     ("Steuerzeiten verstellt (Kette gelängt, Riemen übergesprungen)",
      "Valve timing off (stretched chain, belt jumped a tooth)")],
    [("Stecker am Sensor auf Öl und Korrosion prüfen",
      "Check the sensor connector for oil and corrosion"),
     ("Auf längere Startzeiten achten", "Watch out for longer cranking before it starts")])

detail(
    "cam_crank_corr", "expensive",
    ("Bald in die Werkstatt und bis dahin möglichst wenig fahren. Ist der Steuertrieb "
     "gelängt oder übergesprungen, können Ventile und Kolben zusammenstoßen, das wäre ein "
     "teurer Motorschaden.",
     "Go to a workshop soon and drive as little as possible until then. If the timing "
     "drive has stretched or jumped, valves and pistons can collide, which would be an "
     "expensive engine failure."),
    [("Steuerkette gelängt (bei manchen Motoren häufig)",
      "Stretched timing chain (common on some engines)"),
     ("Nockenwellenversteller hängt oder bekommt zu wenig Öl",
      "Camshaft adjuster sticking or starved of oil"),
     ("Zahnriemen um einen Zahn übergesprungen", "Timing belt jumped a tooth"),
     ("Sensor oder Geberrad defekt", "Faulty sensor or reluctor wheel")],
    [("Ölstand und Ölwechselintervall prüfen: altes oder falsches Öl stört die Verstellung",
      "Check the oil level and service interval: old or wrong oil upsets the adjusters"),
     ("Beim Kaltstart auf Rasseln aus dem Steuertrieb hören",
      "Listen for rattling from the timing drive on a cold start"),
     ("Wechselintervall von Zahnriemen oder Kette im Serviceheft nachsehen",
      "Look up the belt or chain replacement interval in the service book")])

detail(
    "vvt", "medium",
    ("Weiterfahren ist meist möglich, der Motor kann aber Leistung verlieren und unrund "
     "laufen. Zuerst den Ölstand prüfen, denn die Verstellung arbeitet mit Öldruck.",
     "You can usually keep driving, but the engine may lose power and run rough. Check "
     "the oil level first, because the adjusters work with oil pressure."),
    [("Zu wenig, altes oder falsches Motoröl", "Low, old or wrong engine oil"),
     ("Magnetventil der Nockenwellenverstellung verschmutzt oder defekt",
      "Camshaft adjuster solenoid dirty or faulty"),
     ("Sieb im Ölkanal verstopft", "Clogged oil screen in the oil gallery"),
     ("Versteller oder Steuerkette verschlissen", "Worn adjuster or timing chain")],
    [("Ölstand mit dem Messstab prüfen", "Check the oil level with the dipstick"),
     ("Ist der letzte Ölwechsel überfällig?", "Is the last oil change overdue?"),
     ("Stecker am Magnetventil prüfen", "Check the solenoid connector")])

detail(
    "cyl_deactivation", "",
    ("Weiterfahren ist meist möglich. Die Zylinderabschaltung zum Spritsparen arbeitet "
     "nicht richtig, der Motor kann unrund laufen oder mehr verbrauchen.",
     "You can usually keep driving. Cylinder deactivation for saving fuel is not working "
     "properly, so the engine may run rough or use more fuel."),
    [("Zu wenig oder falsches Motoröl", "Low or wrong engine oil"),
     ("Magnetventil der Zylinderabschaltung defekt", "Faulty deactivation solenoid"),
     ("Stecker oder Kabel", "Connector or wiring"),
     ("Ventiltrieb verschlissen (Stößel)", "Worn valve train (lifters)")],
    [("Ölstand und vorgeschriebene Ölsorte prüfen", "Check the oil level and the required oil grade")])

# --------------------------------------------------------------------------
# Fuel, mixture and air metering
# --------------------------------------------------------------------------
detail(
    "injector_circuit", "medium",
    ("Der betroffene Zylinder bekommt wahrscheinlich keinen Kraftstoff, der Motor läuft "
     "unrund. Behutsam zur Werkstatt fahren.",
     "The affected cylinder probably gets no fuel and the engine runs rough. Drive gently "
     "to a workshop."),
    [("Stecker am Einspritzventil locker oder korrodiert", "Loose or corroded injector connector"),
     ("Kabelbruch im Kabelbaum", "Broken wire in the harness"),
     ("Spule im Einspritzventil defekt", "Faulty injector coil"),
     ("Endstufe im Steuergerät (selten)", "Driver stage in the control unit (rare)")],
    [("Stecker am Einspritzventil abziehen, auf grüne Korrosion prüfen, wieder fest aufstecken",
      "Unplug the injector connector, look for green corrosion and push it back on firmly"),
     ("Kabelbaum auf Scheuerstellen und Marderbiss ansehen",
      "Inspect the harness for chafing and marten bites")])

detail(
    "injector_balance", "medium",
    ("Ein Zylinder arbeitet nicht so gleichmäßig wie die anderen. Weiterfahren ist meist "
     "möglich, das Problem kann sich aber zu Aussetzern auswachsen.",
     "One cylinder does not work as evenly as the others. You can usually keep driving, "
     "but it can develop into misfires."),
    [("Einspritzventil verschlissen oder verkokt", "Worn or coked injector"),
     ("Zu wenig Kompression an diesem Zylinder", "Low compression on this cylinder"),
     ("Schwache Zündung an diesem Zylinder (Benziner)", "Weak ignition on this cylinder (petrol)")],
    [("Auf Ruckeln, Aussetzer und Rauch achten", "Watch for stumbling, misfires and smoke"),
     ("Beim Diesel die Mengenabweichung der Injektoren in der Werkstatt messen lassen",
      "On a diesel, have the injector quantity deviation measured at a workshop")])

detail(
    "lean", "cheap",
    ("Weiterfahren ist möglich. Bald beheben lassen: ein zu mageres Gemisch kann zu "
     "Aussetzern führen und auf Dauer Ventile und Katalysator belasten.",
     "You can keep driving. Have it fixed soon: a lean mixture can cause misfires and in "
     "the long run strains the valves and the catalytic converter."),
    [("Falschluft: rissige Schläuche, Ansaugkrümmerdichtung, Kurbelgehäuseentlüftung",
      "Vacuum leak: cracked hoses, intake manifold gasket, crankcase ventilation"),
     ("Luftmassenmesser verschmutzt oder defekt", "Dirty or faulty mass air flow sensor"),
     ("Kraftstoffdruck zu niedrig (Pumpe, Filter)", "Fuel pressure too low (pump, filter)"),
     ("Undichtigkeit im Auspuff vor der Lambdasonde", "Exhaust leak upstream of the oxygen sensor")],
    [("Bei laufendem Motor auf Zischen am Ansaugtrakt hören",
      "Listen for hissing at the intake with the engine running"),
     ("Unterdruckschläuche auf Risse prüfen, besonders an der Kurbelgehäuseentlüftung",
      "Check the vacuum hoses for cracks, especially at the crankcase ventilation"),
     ("Messwerte der Kurz- und Langzeitkorrektur ansehen: hoch im Leerlauf und kleiner bei "
      "höherer Drehzahl spricht für Falschluft",
      "Look at the short and long term fuel trims: high at idle and lower at higher revs "
      "points to a vacuum leak")])

detail(
    "rich", "medium",
    ("Weiterfahren ist möglich, aber nicht auf Dauer: ein zu fettes Gemisch erhöht den "
     "Verbrauch, verrußt die Zündkerzen und kann den Katalysator überhitzen.",
     "You can keep driving, but not for long: a rich mixture raises fuel consumption, "
     "fouls the spark plugs and can overheat the catalytic converter."),
    [("Einspritzventil undicht", "Leaking injector"),
     ("Kraftstoffdruck zu hoch (Druckregler)", "Fuel pressure too high (pressure regulator)"),
     ("Luftmassenmesser misst zu wenig Luft", "Mass air flow sensor reads too little air"),
     ("Luftfilter stark verschmutzt", "Badly clogged air filter"),
     ("Tankentlüftungsventil hängt offen", "EVAP purge valve stuck open")],
    [("Luftfilter ansehen", "Look at the air filter"),
     ("Riecht das Motoröl nach Benzin? Das deutet auf ein undichtes Einspritzventil",
      "Does the engine oil smell of petrol? That points to a leaking injector"),
     ("Auf schwarzen Rauch und Benzingeruch am Auspuff achten",
      "Watch for black smoke and a fuel smell at the tailpipe")])

detail(
    "fuel_trim", "",
    ("Weiterfahren ist möglich. Die Gemischregelung stößt an ihre Grenzen, Verbrauch und "
     "Abgaswerte können leiden. Bald prüfen lassen.",
     "You can keep driving. The mixture control is reaching its limits, so consumption "
     "and emissions may suffer. Have it checked soon."),
    [("Falschluft im Ansaugtrakt", "Vacuum leak in the intake"),
     ("Luftmassenmesser verschmutzt", "Dirty mass air flow sensor"),
     ("Kraftstoffdruck nicht in Ordnung", "Fuel pressure out of range"),
     ("Lambdasonde gealtert", "Aged oxygen sensor")],
    [("Messwerte der Gemischkorrektur ansehen: positive Werte heißen zu mager, negative zu fett",
      "Look at the fuel trim values: positive means too lean, negative too rich"),
     ("Unterdruckschläuche auf Risse prüfen", "Check the vacuum hoses for cracks")])

detail(
    "fuel_pressure", "",
    ("Der Motor kann Leistung verlieren, ruckeln oder sogar im Fahren ausgehen. Vorsichtig "
     "fahren, nicht mit fast leerem Tank, und bald prüfen lassen.",
     "The engine may lose power, stumble or even cut out while driving. Drive carefully, "
     "not with a nearly empty tank, and have it checked soon."),
    [("Kraftstofffilter verstopft", "Clogged fuel filter"),
     ("Kraftstoffpumpe im Tank oder Hochdruckpumpe schwach", "Weak in-tank pump or high-pressure pump"),
     ("Druckregler oder Mengenregelventil defekt", "Faulty pressure regulator or metering valve"),
     ("Kraftstoffdrucksensor misst falsch", "Fuel pressure sensor reads wrong")],
    [("Wann wurde der Kraftstofffilter zuletzt gewechselt? Beim Diesel oft alle 2 bis 4 Jahre",
      "When was the fuel filter last replaced? On diesels often every 2 to 4 years"),
     ("Soll- und Istdruck im Rail als Messwert vergleichen, falls angeboten",
      "Compare desired and actual rail pressure as live values, if available")])

detail(
    "fuel_leak", "",
    ("Riecht es nach Kraftstoff oder tropft etwas: nicht weiterfahren, Brandgefahr. Motor "
     "abstellen, nicht rauchen und den Pannendienst rufen.",
     "If you smell fuel or see drips, do not drive on: risk of fire. Switch off the "
     "engine, do not smoke and call roadside assistance."),
    [("Undichte Kraftstoffleitung oder Verbindung", "Leaking fuel line or fitting"),
     ("Undichtes Einspritzventil oder Rail", "Leaking injector or fuel rail"),
     ("Fehlmessung des Drucksensors", "False reading from the pressure sensor")],
    [("Bei abgestelltem Motor nach Kraftstoffgeruch und Tropfspuren schauen",
      "With the engine off, look and smell for fuel leaks")])

detail(
    "fuel_pump", "medium",
    ("Der Motor kann ausgehen oder nicht mehr anspringen. Möglichst bald prüfen lassen.",
     "The engine may cut out or fail to start. Have it checked as soon as possible."),
    [("Kraftstoffpumpe verschlissen", "Worn fuel pump"),
     ("Relais oder Sicherung der Pumpe", "Fuel pump relay or fuse"),
     ("Stecker an der Pumpe korrodiert, oft unter der Rückbank",
      "Corroded pump connector, often under the rear seat")],
    [("Beim Einschalten der Zündung hört man die Pumpe meist ein bis zwei Sekunden summen",
      "When you switch on the ignition you can usually hear the pump hum for a second or two"),
     ("Sicherung der Kraftstoffpumpe prüfen", "Check the fuel pump fuse")])

detail(
    "fuel_level", "medium",
    ("Unbedenklich, aber die Tankanzeige kann falsch sein. Lieber früher tanken und den "
     "Tageskilometerzähler im Blick behalten.",
     "Harmless, but the fuel gauge may be wrong. Fill up early and keep an eye on the "
     "trip meter."),
    [("Tankgeber verschlissen (Schleifbahn)", "Worn fuel level sender (track)"),
     ("Stecker am Tank korrodiert", "Corroded connector at the tank"),
     ("Kombiinstrument (selten)", "Instrument cluster (rare)")],
    [("Springt die Tankanzeige oder zeigt sie Unsinn?", "Does the fuel gauge jump or show nonsense?")])

detail(
    "water_in_fuel", "cheap",
    ("Wasser im Diesel schadet Hochdruckpumpe und Einspritzventilen. Bald den "
     "Kraftstofffilter entwässern oder tauschen lassen und bis dahin wenig fahren.",
     "Water in diesel harms the high-pressure pump and the injectors. Have the fuel "
     "filter drained or replaced soon and drive little until then."),
    [("Wasser hat sich im Kraftstofffilter gesammelt", "Water has collected in the fuel filter"),
     ("Schlechter Kraftstoff getankt", "Poor quality fuel"),
     ("Wassersensor defekt", "Faulty water sensor")],
    [("Wasserablass am Kraftstofffilter nutzen, falls vorhanden",
      "Use the water drain on the fuel filter, if fitted"),
     ("Wann wurde der Filter zuletzt gewechselt?", "When was the filter last replaced?")])

detail(
    "maf", "medium",
    ("Weiterfahren ist meist möglich, das Steuergerät rechnet mit Ersatzwerten. Der Motor "
     "kann schwächer ziehen oder ruckeln. Bald prüfen lassen.",
     "You can usually keep driving, the control unit uses substitute values. The engine "
     "may pull less or stumble. Have it checked soon."),
    [("Luftmassenmesser verschmutzt oder gealtert", "Dirty or aged mass air flow sensor"),
     ("Falschluft zwischen Luftmassenmesser und Motor", "Air leak between the MAF sensor and the engine"),
     ("Stecker oder Kabel", "Connector or wiring"),
     ("Luftfilter verschmutzt oder schief eingesetzt", "Air filter dirty or not seated properly")],
    [("Luftfilterkasten öffnen: sitzt der Filter richtig, ist er sauber?",
      "Open the air box: is the filter seated properly and clean?"),
     ("Ansaugschlauch hinter dem Luftmassenmesser auf Risse prüfen",
      "Check the intake hose after the MAF sensor for cracks"),
     ("Messwert ansehen: im Leerlauf grob 2 bis 7 g/s je nach Hubraum, beim Gasgeben muss er "
      "gleichmäßig steigen",
      "Look at the live value: roughly 2 to 7 g/s at idle depending on engine size, and it "
      "must rise smoothly when you rev")])

detail(
    "map", "cheap",
    ("Weiterfahren ist meist möglich, der Motor kann aber unrund laufen oder schwächer "
     "ziehen. Bald prüfen lassen.",
     "You can usually keep driving, but the engine may run rough or pull less. Have it "
     "checked soon."),
    [("Sensor verschmutzt (Ölnebel, beim Diesel Ruß)", "Dirty sensor (oil mist, soot on diesels)"),
     ("Unterdruckschlauch zum Sensor rissig oder ab", "Cracked or detached vacuum hose to the sensor"),
     ("Stecker oder Kabel", "Connector or wiring"),
     ("Falschluft", "Vacuum leak")],
    [("Bei Zündung an und stehendem Motor muss der Sensor etwa den Luftdruck zeigen, rund "
      "100 kPa auf Meereshöhe",
      "With the ignition on and the engine stopped the sensor should read about "
      "atmospheric pressure, around 100 kPa at sea level"),
     ("Schlauch und Stecker am Sensor prüfen", "Check the hose and connector at the sensor")])

detail(
    "iat", "cheap",
    ("Weiterfahren ist in Ordnung. Das Steuergerät nimmt einen Ersatzwert, Kaltstart und "
     "Verbrauch können etwas leiden.",
     "It is fine to keep driving. The control unit uses a substitute value, so cold starts "
     "and consumption may suffer slightly."),
    [("Sensor defekt, oft im Luftmassenmesser oder Saugrohrdrucksensor eingebaut",
      "Faulty sensor, often built into the MAF or MAP sensor"),
     ("Stecker oder Kabel", "Connector or wiring")],
    [("Nach langer Standzeit den Messwert mit der Außentemperatur vergleichen",
      "After the car has stood for a long time, compare the reading with the outside temperature")])

detail(
    "throttle", "medium",
    ("Das Auto kann ins Notlaufprogramm gehen: wenig Leistung, begrenzte Drehzahl. "
     "Vorsichtig fahren, bei plötzlichem Leistungsverlust nicht überholen, bald prüfen lassen.",
     "The car may go into limp mode with little power and limited revs. Drive carefully, "
     "do not overtake in case power drops suddenly, and have it checked soon."),
    [("Drosselklappe verschmutzt (Ölkohle, Ruß)", "Dirty throttle body (carbon, soot)"),
     ("Drosselklappeneinheit verschlissen (Potentiometer)", "Worn throttle body (position sensor)"),
     ("Stecker oder Kabel", "Connector or wiring"),
     ("Drosselklappe nach Reinigung oder Batteriewechsel nicht angelernt",
      "Throttle body not adapted after cleaning or a battery change")],
    [("Stecker an der Drosselklappe prüfen", "Check the throttle body connector"),
     ("Bei ausgeschalteter Zündung den Ansaugschlauch abnehmen und die Klappe auf "
      "Ablagerungen ansehen",
      "With the ignition off, remove the intake hose and look for deposits on the throttle plate"),
     ("Nach Reinigung oder Batteriewechsel die Grundeinstellung (Anlernen) durchführen lassen",
      "After cleaning or a battery change, have the basic setting (adaptation) carried out")])

detail(
    "pedal", "medium",
    ("Das Auto kann ins Notlaufprogramm gehen und reagiert dann nur noch träge aufs "
     "Gaspedal. Vorsichtig zur Werkstatt fahren.",
     "The car may go into limp mode and then responds only sluggishly to the accelerator. "
     "Drive carefully to a workshop."),
    [("Fahrpedalgeber verschlissen", "Worn accelerator pedal sensor"),
     ("Stecker am Pedal locker oder Kabel gebrochen", "Loose pedal connector or broken wire"),
     ("Fußmatte drückt auf das Pedal", "Floor mat pressing on the pedal")],
    [("Fußmatte richtig befestigen", "Make sure the floor mat is fastened properly"),
     ("Messwerte beider Pedalgeber ansehen: sie müssen beim Treten gleichmäßig zusammen steigen",
      "Look at both pedal sensor values: they must rise smoothly together as you press")])

detail(
    "idle", "cheap",
    ("Weiterfahren ist in Ordnung. Im Leerlauf kann die Drehzahl schwanken oder der Motor "
     "ausgehen, besonders kalt oder mit eingeschalteter Klimaanlage.",
     "It is fine to keep driving. The idle speed may hunt or the engine may stall, "
     "especially when cold or with the air conditioning on."),
    [("Drosselklappe oder Leerlaufsteller verschmutzt", "Dirty throttle body or idle air control valve"),
     ("Falschluft", "Vacuum leak"),
     ("Drosselklappe nicht angelernt", "Throttle body not adapted")],
    [("Drosselklappe reinigen und anlernen lassen", "Have the throttle body cleaned and adapted"),
     ("Unterdruckschläuche prüfen", "Check the vacuum hoses")])

detail(
    "imrc", "medium",
    ("Weiterfahren ist in Ordnung, der Motor kann etwas Leistung oder Drehmoment verlieren.",
     "It is fine to keep driving; the engine may lose a little power or torque."),
    [("Saugrohrklappen verkokt oder Gestänge gebrochen", "Coked intake runner flaps or broken linkage"),
     ("Stellmotor oder Unterdruckdose defekt", "Faulty actuator motor or vacuum capsule"),
     ("Stecker oder Unterdruckschlauch", "Connector or vacuum hose")],
    [("Gestänge der Saugrohrklappen ansehen", "Look at the intake runner flap linkage"),
     ("Stecker und Unterdruckschläuche prüfen", "Check the connector and the vacuum hoses")])

# --------------------------------------------------------------------------
# Turbo and boost
# --------------------------------------------------------------------------
detail(
    "underboost", "medium",
    ("Weiterfahren ist möglich, mit weniger Leistung. Nicht hoch drehen und keinen Anhänger "
     "ziehen, bis es geklärt ist.",
     "You can keep driving with less power. Do not rev hard or tow until it is sorted out."),
    [("Ladeluftschlauch gerissen oder abgerutscht", "Split or detached charge air hose"),
     ("Unterdruckschläuche oder Ladedruckdose undicht", "Leaking vacuum hoses or wastegate actuator"),
     ("Verstellung des Turboladers (VTG) verrußt und schwergängig",
      "Turbo vane mechanism (VTG) sooted and sticking"),
     ("Turbolader verschlissen", "Worn turbocharger"),
     ("AGR oder Partikelfilter zugesetzt", "Clogged EGR or particulate filter")],
    [("Ladeluftschläuche und Ladeluftkühler auf Risse und Ölspuren prüfen",
      "Check the charge air hoses and intercooler for cracks and oil traces"),
     ("Beim Beschleunigen auf Zischen oder Pfeifen achten",
      "Listen for hissing or whistling when accelerating"),
     ("Soll- und Istladedruck als Messwert vergleichen", "Compare desired and actual boost as live values")])

detail(
    "overboost", "medium",
    ("Nur noch behutsam fahren, keine Volllast. Zu hoher Ladedruck kann den Motor schädigen, "
     "meist schaltet das Auto deshalb in den Notlauf.",
     "Drive gently and avoid full load. Excessive boost can damage the engine, which is "
     "why the car usually switches to limp mode."),
    [("Verstellung des Turboladers (VTG) oder Wastegate klemmt",
      "Turbo vane mechanism (VTG) or wastegate sticking"),
     ("Magnetventil der Ladedruckregelung defekt", "Faulty boost control solenoid"),
     ("Unterdruckschlauch undicht oder vertauscht", "Leaking or swapped vacuum hose"),
     ("Ladedrucksensor misst falsch", "Boost pressure sensor reads wrong")],
    [("Unterdruckschläuche zur Ladedruckdose prüfen", "Check the vacuum hoses to the boost actuator"),
     ("Bei kaltem, stehendem Motor prüfen, ob sich das Gestänge am Turbo leicht bewegt",
      "With the engine cold and off, check that the turbo linkage moves freely")])

detail(
    "boost_sensor", "cheap",
    ("Weiterfahren ist möglich, oft mit weniger Leistung im Notlauf. Bald prüfen lassen.",
     "You can keep driving, often with reduced power in limp mode. Have it checked soon."),
    [("Ladedrucksensor verschmutzt (Öl, Ruß) oder defekt", "Dirty (oil, soot) or faulty boost sensor"),
     ("Stecker oder Kabel", "Connector or wiring"),
     ("Ladeluftschlauch am Sensor undicht", "Charge air hose leaking at the sensor")],
    [("Bei Zündung an und stehendem Motor muss der Sensor etwa den Luftdruck zeigen",
      "With the ignition on and the engine stopped the sensor should read about atmospheric pressure"),
     ("Stecker am Sensor prüfen", "Check the sensor connector")])

detail(
    "boost_actuator", "medium",
    ("Weiterfahren ist möglich, oft mit weniger Leistung im Notlauf. Bald prüfen lassen.",
     "You can keep driving, often with reduced power in limp mode. Have it checked soon."),
    [("Magnetventil der Ladedruckregelung defekt", "Faulty boost control solenoid"),
     ("Elektrischer Steller am Turbolader defekt oder schwergängig",
      "Electric turbo actuator faulty or sticking"),
     ("Stecker oder Kabel", "Connector or wiring"),
     ("Unterdruckschläuche undicht", "Leaking vacuum hoses")],
    [("Stecker am Ventil oder Steller prüfen", "Check the connector at the valve or actuator"),
     ("Unterdruckschläuche auf Risse prüfen", "Check the vacuum hoses for cracks")])

# --------------------------------------------------------------------------
# Cooling and oil
# --------------------------------------------------------------------------
detail(
    "ect_sensor", "cheap",
    ("Vorsichtig weiterfahren und die Temperaturanzeige im Blick behalten. Ein falscher Wert "
     "kann den Lüfter falsch steuern sowie Kaltstart und Verbrauch verschlechtern. Steigt "
     "die Temperatur ungewöhnlich, sofort anhalten.",
     "Drive on carefully and keep an eye on the temperature gauge. A wrong reading can "
     "upset the fan control, cold starts and consumption. If the temperature climbs "
     "unusually, stop at once."),
    [("Temperaturgeber defekt (häufig, günstiges Teil)", "Faulty temperature sensor (common, cheap part)"),
     ("Stecker korrodiert oder Kabel beschädigt", "Corroded connector or damaged wire"),
     ("Zu wenig Kühlmittel, der Geber sitzt in Luft", "Low coolant, the sensor sits in air")],
    [("Kühlmittelstand bei kaltem Motor prüfen", "Check the coolant level with the engine cold"),
     ("Nach langer Standzeit den Messwert mit der Außentemperatur vergleichen",
      "After the car has stood for a long time, compare the reading with the outside temperature")])

detail(
    "thermostat", "medium",
    ("Weiterfahren ist möglich. Der Motor wird nicht richtig warm: höherer Verbrauch, "
     "schwache Heizung, mehr Verschleiß. In den nächsten Wochen beheben.",
     "You can keep driving. The engine does not warm up properly: higher consumption, "
     "weak heating, more wear. Fix it within the next few weeks."),
    [("Thermostat hängt offen", "Thermostat stuck open"),
     ("Kühlmitteltemperaturgeber misst falsch", "Coolant temperature sensor reads wrong"),
     ("Sehr kalte Umgebung und viel Kurzstrecke", "Very cold weather and many short trips")],
    [("Braucht der Motor sehr lange, bis er warm ist, oder erreicht er auf der Autobahn "
      "keine 90 Grad?",
      "Does the engine take very long to warm up or stay below 90 degrees on the motorway?"),
     ("Heizt die Heizung im Winter schwach?", "Is the heater weak in winter?"),
     ("Kühlmitteltemperatur während der Fahrt als Messwert verfolgen",
      "Follow the coolant temperature as a live value while driving")])

detail(
    "overheat", "",
    ("Nicht weiterfahren. Motor abstellen, abkühlen lassen und den Kühlmittelstand prüfen. "
     "Den Deckel am Kühlsystem nie bei heißem Motor öffnen: Verbrühungsgefahr.",
     "Do not drive on. Switch off the engine, let it cool down and check the coolant level. "
     "Never open the cooling system cap while the engine is hot: risk of scalding."),
    [("Kühlmittelverlust durch Leck, Schlauch oder Wasserpumpe",
      "Coolant loss through a leak, hose or water pump"),
     ("Kühlerlüfter läuft nicht", "Radiator fan not running"),
     ("Thermostat hängt geschlossen", "Thermostat stuck closed"),
     ("Zylinderkopfdichtung defekt", "Failed head gasket")],
    [("Bei kaltem Motor den Kühlmittelstand im Ausgleichsbehälter prüfen",
      "With the engine cold, check the coolant level in the expansion tank"),
     ("Nach Pfützen oder Spuren unter dem Auto schauen", "Look for puddles or traces under the car"),
     ("Läuft der Lüfter, wenn der Motor heiß ist?", "Does the fan run when the engine is hot?")])

detail(
    "coolant_level", "cheap",
    ("Bald anhalten und bei kaltem Motor den Kühlmittelstand prüfen. Mit zu wenig "
     "Kühlmittel droht Überhitzung.",
     "Stop soon and check the coolant level with the engine cold. With too little coolant "
     "the engine can overheat."),
    [("Kühlmittelverlust durch ein Leck", "Coolant loss through a leak"),
     ("Niveaugeber im Ausgleichsbehälter defekt", "Faulty level sensor in the expansion tank")],
    [("Kühlmittelstand prüfen und mit dem passenden Kühlmittel auffüllen",
      "Check the coolant level and top up with the correct coolant"),
     ("Nach Spuren unter dem Auto und an den Schläuchen schauen",
      "Look for traces under the car and at the hoses")])

detail(
    "fan", "medium",
    ("Vorsichtig weiterfahren und die Temperaturanzeige beobachten, besonders im Stau und "
     "im Sommer. Läuft der Lüfter nicht, kann der Motor im Stand überhitzen.",
     "Drive on carefully and watch the temperature gauge, especially in traffic jams and in "
     "summer. If the fan does not run, the engine can overheat when stationary."),
    [("Lüftermotor defekt", "Faulty fan motor"),
     ("Lüftersteuergerät oder Relais", "Fan control module or relay"),
     ("Sicherung", "Fuse"),
     ("Stecker oder Kabel", "Connector or wiring")],
    [("Bei warmem Motor im Stand: springt der Lüfter an?",
      "With the engine warm and the car stationary: does the fan come on?"),
     ("Bei vielen Autos läuft der Lüfter sofort mit, wenn die Klimaanlage an ist",
      "On many cars the fan runs as soon as the air conditioning is switched on"),
     ("Sicherungen prüfen", "Check the fuses")])

detail(
    "oil_pressure", "",
    ("Leuchtet die Öldrucklampe oder ist der Ölstand niedrig: sofort anhalten und den Motor "
     "abstellen. Ohne Öldruck ist der Motor in kurzer Zeit zerstört.",
     "If the oil pressure light is on or the oil level is low, stop at once and switch off "
     "the engine. Without oil pressure the engine is destroyed within a short time."),
    [("Zu wenig Motoröl", "Low engine oil"),
     ("Öldruckschalter oder Sensor defekt oder undicht", "Faulty or leaking oil pressure switch or sensor"),
     ("Ölpumpe oder Ansaugsieb verstopft", "Clogged oil pump or pickup screen"),
     ("Verschlissene Lager", "Worn bearings")],
    [("Ölstand auf ebenem Boden mit dem Messstab prüfen", "Check the oil level on level ground with the dipstick"),
     ("Nach Ölflecken unter dem Auto schauen", "Look for oil stains under the car"),
     ("Ist der Ölwechsel überfällig?", "Is the oil change overdue?")])

# --------------------------------------------------------------------------
# Exhaust, oxygen sensors and emissions
# --------------------------------------------------------------------------
detail(
    "o2_up", "medium",
    ("Weiterfahren ist möglich. Die Gemischregelung arbeitet ungenauer, der Verbrauch kann "
     "steigen. Bald prüfen lassen, bevor der Katalysator leidet.",
     "You can keep driving. Mixture control is less precise and consumption may rise. Have "
     "it checked soon, before the catalytic converter suffers."),
    [("Lambdasonde gealtert oder verschmutzt", "Aged or contaminated oxygen sensor"),
     ("Stecker oder Kabel der Sonde beschädigt (Hitze, Steinschlag)",
      "Damaged sensor connector or wiring (heat, stone chips)"),
     ("Falschluft oder Undichtigkeit im Auspuff vor der Sonde",
      "Vacuum leak or exhaust leak upstream of the sensor"),
     ("Das Gemisch ist wirklich falsch, die Ursache liegt woanders",
      "The mixture really is off and the cause lies elsewhere")],
    [("Kabel und Stecker der Sonde auf Schmelzstellen prüfen",
      "Check the sensor wiring and connector for melted spots"),
     ("Messwert ansehen: eine klassische Sprungsonde pendelt bei warmem Motor schnell "
      "zwischen etwa 0,1 und 0,9 Volt (bei Breitbandsonden gilt das nicht)",
      "Look at the live value: a classic switching sensor swings quickly between about "
      "0.1 and 0.9 volts with the engine warm (this does not apply to wideband sensors)"),
     ("Auspuff vor der Sonde auf Undichtigkeiten prüfen", "Check the exhaust upstream of the sensor for leaks")])

detail(
    "o2_down", "medium",
    ("Weiterfahren ist in Ordnung. Die Sonde nach dem Katalysator überwacht vor allem den "
     "Kat, das Gemisch regelt sie kaum mit.",
     "It is fine to keep driving. The sensor after the catalytic converter mainly monitors "
     "the converter and has little say in the mixture."),
    [("Lambdasonde gealtert", "Aged oxygen sensor"),
     ("Stecker oder Kabel beschädigt", "Damaged connector or wiring"),
     ("Undichtigkeit im Auspuff", "Exhaust leak"),
     ("Katalysator gealtert", "Aged catalytic converter")],
    [("Kabel und Stecker der Sonde prüfen", "Check the sensor wiring and connector"),
     ("Auf Blubbern oder Rußspuren am Auspuff achten", "Look for puffing or soot marks along the exhaust")])

detail(
    "o2_heater", "cheap",
    ("Weiterfahren ist in Ordnung. Ohne Heizung braucht die Sonde nach dem Kaltstart länger, "
     "bis sie misst, die Abgaswerte sind dann kurz schlechter.",
     "It is fine to keep driving. Without its heater the sensor takes longer to work after "
     "a cold start, so emissions are briefly worse."),
    [("Heizelement der Sonde durchgebrannt, die Sonde muss getauscht werden",
      "Burnt-out sensor heater, the sensor has to be replaced"),
     ("Sicherung der Sondenheizung", "Oxygen sensor heater fuse"),
     ("Stecker korrodiert oder Kabel durchgescheuert", "Corroded connector or chafed wire")],
    [("Sicherung der Lambdasondenheizung prüfen (Belegung im Handbuch)",
      "Check the oxygen sensor heater fuse (see the owner's manual for its position)"),
     ("Stecker der Sonde trennen und auf Korrosion prüfen",
      "Unplug the sensor connector and look for corrosion")])

detail(
    "catalyst", "expensive",
    ("Weiterfahren ist möglich. Der Katalysator reinigt das Abgas nicht mehr gut genug, so "
     "wird die Abgasuntersuchung nicht bestanden. Erst die Ursache klären, bevor ein neuer "
     "Kat eingebaut wird.",
     "You can keep driving. The catalytic converter no longer cleans the exhaust well "
     "enough, so the car would fail an emissions test. Find the cause before fitting a "
     "new converter."),
    [("Katalysator gealtert oder beschädigt", "Aged or damaged catalytic converter"),
     ("Lambdasonde nach dem Kat defekt oder träge", "Faulty or slow oxygen sensor after the converter"),
     ("Undichtigkeit im Auspuff vor oder an der Sonde", "Exhaust leak before or at the sensor"),
     ("Frühere Zündaussetzer oder hoher Ölverbrauch haben den Kat geschädigt",
      "Earlier misfires or high oil consumption have damaged the converter")],
    [("Bei kaltem Auspuff vorsichtig gegen den Kat klopfen: Rasseln deutet auf gebrochene Keramik",
      "With the exhaust cold, tap the converter gently: rattling points to broken ceramic"),
     ("Am Krümmer nach Rußspuren suchen", "Look for soot marks at the exhaust manifold"),
     ("Messwert der hinteren Lambdasonde ansehen: springt sie wie die vordere, speichert der "
      "Kat keinen Sauerstoff mehr",
      "Look at the rear oxygen sensor: if it switches like the front one, the converter no "
      "longer stores oxygen")])

detail(
    "egr", "medium",
    ("Weiterfahren ist möglich. Bei verrußtem AGR-Ventil kann der Motor Leistung verlieren "
     "oder ruckeln, beim Diesel steigt die Rußmenge. Bald prüfen lassen.",
     "You can keep driving. With a sooted EGR valve the engine may lose power or stumble, "
     "and a diesel produces more soot. Have it checked soon."),
    [("AGR-Ventil verrußt oder klemmt, häufig bei Diesel und viel Kurzstrecke",
      "EGR valve sooted or sticking, common on diesels and with many short trips"),
     ("Unterdruckschlauch oder Steuerventil bei pneumatischen Ventilen",
      "Vacuum hose or control valve on vacuum-operated valves"),
     ("Stecker oder Positionssensor des Ventils", "Connector or position sensor of the valve"),
     ("AGR-Kühler oder Kanäle zugesetzt", "Clogged EGR cooler or passages")],
    [("Stecker am AGR-Ventil prüfen", "Check the EGR valve connector"),
     ("Unterdruckschläuche auf Risse prüfen", "Check the vacuum hoses for cracks")])

detail(
    "secondary_air", "medium",
    ("Weiterfahren ist in Ordnung. Das Sekundärluftsystem arbeitet nur kurz nach dem "
     "Kaltstart, damit der Katalysator schneller warm wird.",
     "It is fine to keep driving. The secondary air system only runs briefly after a cold "
     "start to warm up the catalytic converter faster."),
    [("Sekundärluftpumpe defekt, oft durch eingedrungenes Wasser",
      "Faulty secondary air pump, often from water ingress"),
     ("Kombiventil hängt oder ist verkokt", "Combination valve stuck or coked"),
     ("Relais oder Sicherung der Pumpe", "Pump relay or fuse"),
     ("Schläuche gerissen", "Split hoses")],
    [("Nach einem Kaltstart hinhören: die Pumpe läuft hörbar etwa eine Minute",
      "Listen after a cold start: the pump can be heard running for about a minute"),
     ("Sicherung und Schläuche prüfen", "Check the fuse and the hoses")])

detail(
    "egt_sensor", "cheap",
    ("Weiterfahren ist möglich. Ohne Temperaturfühler kann der Partikelfilter oft nicht "
     "regenerieren, und Turbo und Katalysator werden schlechter geschützt.",
     "You can keep driving. Without the temperature sensor the particulate filter often "
     "cannot regenerate, and the turbo and converter are less well protected."),
    [("Abgastemperaturfühler defekt (Hitze, Alterung)", "Faulty exhaust temperature sensor (heat, age)"),
     ("Stecker oder Kabel", "Connector or wiring")],
    [("Kabel am Auspuff auf Schmelzstellen prüfen", "Check the wiring along the exhaust for melted spots"),
     ("Bei kaltem Motor den Messwert mit der Außentemperatur vergleichen",
      "With the engine cold, compare the reading with the outside temperature")])

# --------------------------------------------------------------------------
# Evaporative emissions
# --------------------------------------------------------------------------
detail(
    "evap_small", "cheap",
    ("Unbedenklich für den Motor, Weiterfahren ist in Ordnung. Es entweicht nur etwas "
     "Benzindampf.",
     "Harmless for the engine, it is fine to keep driving. Only a little fuel vapour escapes."),
    [("Tankdeckel nicht richtig geschlossen oder Dichtung rissig",
      "Fuel cap not closed properly or seal cracked"),
     ("Schlauch am Aktivkohlefilter oder am Tankentlüftungsventil undicht",
      "Leaking hose at the charcoal canister or purge valve"),
     ("Tankentlüftungsventil schließt nicht dicht", "Purge valve does not seal"),
     ("Leckdiagnosepumpe defekt", "Faulty leak detection pump")],
    [("Tankdeckel abnehmen, Dichtung ansehen und wieder fest schließen, bis er klickt",
      "Remove the fuel cap, look at the seal and close it firmly until it clicks"),
     ("Fehler löschen und einige Fahrten abwarten: der Test läuft nur unter bestimmten Bedingungen",
      "Clear the code and wait a few trips: the test only runs under certain conditions"),
     ("Schläuche am Aktivkohlefilter ansehen, oft nahe dem Tank oder im Radkasten",
      "Look at the hoses at the charcoal canister, often near the tank or in a wheel arch")])

detail(
    "evap_large", "cheap",
    ("Für den Motor unbedenklich. Riecht es deutlich nach Benzin, das Auto nicht in "
     "geschlossenen Räumen abstellen und bald prüfen lassen.",
     "Harmless for the engine. If there is a clear smell of petrol, do not park the car "
     "indoors and have it checked soon."),
    [("Tankdeckel offen oder fehlt", "Fuel cap open or missing"),
     ("Schlauch abgerutscht oder gerissen", "Hose detached or split"),
     ("Einfüllstutzen undicht oder durchgerostet", "Leaking or rusted filler neck"),
     ("Tankentlüftungsventil hängt", "Purge valve sticking")],
    [("Tankdeckel prüfen und fest schließen", "Check the fuel cap and close it firmly"),
     ("Auf Benzingeruch am Tank und im Motorraum achten",
      "Smell for petrol at the tank and in the engine bay")])

detail(
    "evap_valve", "cheap",
    ("Unbedenklich für den Motor, Weiterfahren ist in Ordnung. Hängt ein Ventil offen, kann "
     "der Motor nach dem Tanken schlecht anspringen oder im Leerlauf unruhig laufen.",
     "Harmless for the engine, it is fine to keep driving. If a valve sticks open, the "
     "engine may start poorly after refuelling or idle unevenly."),
    [("Tankentlüftungsventil defekt oder hängt", "Faulty or sticking purge valve"),
     ("Stecker oder Leitung zum Ventil", "Connector or wiring to the valve"),
     ("Schlauch abgerutscht oder verstopft", "Detached or blocked hose"),
     ("Aktivkohlefilter verstopft, oft durch Überfüllen des Tanks",
      "Clogged charcoal canister, often from overfilling the tank")],
    [("Stecker am Ventil prüfen", "Check the valve connector"),
     ("Nach dem Abschalten der Zapfpistole nicht weiter nachtanken",
      "Stop filling once the pump nozzle clicks off"),
     ("Schläuche auf festen Sitz prüfen", "Check that the hoses are firmly attached")])

# --------------------------------------------------------------------------
# Diesel aftertreatment and glow plugs
# --------------------------------------------------------------------------
detail(
    "dpf_soot", "expensive",
    ("Möglichst bald 15 bis 20 Minuten zügig über Land oder Autobahn fahren, damit der "
     "Filter sich freibrennt. Geht die Warnung danach nicht aus oder fährt das Auto im "
     "Notlauf, in die Werkstatt: ein vollgesetzter Filter wird teuer.",
     "As soon as possible, drive briskly for 15 to 20 minutes on a main road or motorway "
     "so the filter can burn itself clean. If the warning stays on or the car is in limp "
     "mode, go to a workshop: a fully clogged filter gets expensive."),
    [("Viel Kurzstrecke, die Regeneration bricht immer wieder ab",
      "Lots of short trips, regeneration keeps being interrupted"),
     ("Regeneration verhindert durch einen anderen Fehler (Drucksensor, Temperaturfühler, AGR)",
      "Regeneration blocked by another fault (pressure sensor, temperature sensor, EGR)"),
     ("Motor erzeugt zu viel Ruß (Einspritzventile, Turbolader, AGR)",
      "Engine produces too much soot (injectors, turbo, EGR)"),
     ("Filter durch Asche am Ende seiner Lebensdauer", "Filter at the end of its life from ash")],
    [("Gibt es weitere Fehler zu Sensoren oder AGR? Die zuerst beheben",
      "Are there other faults for sensors or EGR? Fix those first"),
     ("Ist ein Öl mit Freigabe für Partikelfilter (Low SAPS) eingefüllt?",
      "Is the engine filled with an oil approved for particulate filters (low SAPS)?"),
     ("Ölstand prüfen: viele abgebrochene Regenerationen verdünnen das Öl mit Diesel",
      "Check the oil level: many interrupted regenerations dilute the oil with diesel")])

detail(
    "dpf_sensor", "cheap",
    ("Weiterfahren ist möglich, aber der Filter wird nicht mehr richtig überwacht und "
     "regeneriert eventuell nicht. Bald prüfen lassen.",
     "You can keep driving, but the filter is no longer monitored properly and may not "
     "regenerate. Have it checked soon."),
    [("Schläuche zum Differenzdrucksensor gerissen, abgerutscht oder verstopft",
      "Hoses to the differential pressure sensor split, detached or blocked"),
     ("Differenzdrucksensor defekt", "Faulty differential pressure sensor"),
     ("Stecker oder Kabel", "Connector or wiring"),
     ("Filter beschädigt oder ausgebaut", "Filter damaged or removed")],
    [("Die zwei dünnen Schläuche vom Sensor zum Filter auf Risse und festen Sitz prüfen",
      "Check the two thin hoses from the sensor to the filter for cracks and a firm fit"),
     ("Messwert Differenzdruck bei abgestelltem Motor ansehen: er sollte nahe null liegen",
      "Look at the differential pressure with the engine off: it should be close to zero")])

detail(
    "scr", "expensive",
    ("Weiterfahren ist möglich. Bleibt der Fehler, zählen viele Autos eine Restreichweite "
     "herunter, nach der der Motor nicht mehr startet. Die Anzeige im Kombiinstrument "
     "beachten und zeitnah in die Werkstatt.",
     "You can keep driving. If the fault stays, many cars count down a remaining range "
     "after which the engine will not restart. Watch the message in the instrument cluster "
     "and go to a workshop soon."),
    [("Schlechte oder alte AdBlue-Qualität", "Poor or old AdBlue"),
     ("NOx-Sensor defekt", "Faulty NOx sensor"),
     ("Dosierventil verkristallisiert oder verstopft", "Dosing valve crystallised or blocked"),
     ("SCR-Katalysator gealtert", "Aged SCR catalyst"),
     ("AdBlue-Pumpe oder Heizung defekt", "Faulty AdBlue pump or heater")],
    [("Nur AdBlue nach ISO 22241 tanken, nie Wasser oder andere Flüssigkeiten",
      "Only fill AdBlue to ISO 22241, never water or other fluids"),
     ("Weiße Kristalle am Dosierventil im Auspuff deuten auf ein Problem",
      "White crystals at the dosing valve in the exhaust point to a problem"),
     ("Restreichweite im Kombiinstrument beachten", "Watch the remaining range in the instrument cluster")])

detail(
    "adblue_level", "cheap",
    ("AdBlue nachfüllen. Fährt man weiter, sperrt das Auto nach Ablauf der angezeigten "
     "Restreichweite den nächsten Motorstart.",
     "Top up AdBlue. If you keep driving, the car blocks the next engine start once the "
     "displayed remaining range has run out."),
    [("AdBlue-Tank leer oder fast leer", "AdBlue tank empty or nearly empty"),
     ("Füllstandssensor defekt, oft im Fördermodul verbaut", "Faulty level sensor, often part of the delivery module"),
     ("AdBlue bei Frost eingefroren, taut während der Fahrt auf",
      "AdBlue frozen in frost, it thaws while driving")],
    [("Mehrere Liter AdBlue nachfüllen und danach die Zündung einige Minuten eingeschaltet "
      "lassen oder losfahren",
      "Add several litres of AdBlue, then leave the ignition on for a few minutes or start driving"),
     ("Bleibt die Meldung trotz vollem Tank, ist meist der Sensor schuld",
      "If the message stays with a full tank, the sensor is usually to blame")])

detail(
    "adblue_dosing", "expensive",
    ("Weiterfahren ist möglich, doch viele Autos zählen dann eine Restreichweite herunter, "
     "nach der der Motor nicht mehr startet. Zeitnah in die Werkstatt.",
     "You can keep driving, but many cars then count down a remaining range after which "
     "the engine will not restart. Go to a workshop soon."),
    [("Dosierventil verkristallisiert oder defekt", "Dosing valve crystallised or faulty"),
     ("AdBlue-Pumpe oder Leitungen (Frost, Kristalle)", "AdBlue pump or lines (frost, crystals)"),
     ("Stecker oder Kabel", "Connector or wiring"),
     ("Heizung des AdBlue-Systems", "AdBlue system heater")],
    [("Weiße Kristalle am Dosierventil im Auspuff deuten auf ein Problem",
      "White crystals at the dosing valve in the exhaust point to a problem"),
     ("Restreichweite im Kombiinstrument beachten", "Watch the remaining range in the instrument cluster")])

detail(
    "nox_sensor", "medium",
    ("Weiterfahren ist möglich. Ohne gültigen NOx-Wert kann die AdBlue-Dosierung nicht "
     "geregelt werden, manche Autos starten dann eine Restreichweiten-Warnung.",
     "You can keep driving. Without a valid NOx reading the AdBlue dosing cannot be "
     "controlled, and some cars then start a remaining range warning."),
    [("NOx-Sensor defekt (das Steuergerät sitzt im Kabel des Sensors)",
      "Faulty NOx sensor (its control unit sits in the sensor cable)"),
     ("Stecker oder Kabel", "Connector or wiring"),
     ("Spannungsversorgung des Sensors", "Sensor power supply")],
    [("Stecker und Kabel am Sensor prüfen", "Check the sensor connector and wiring")])

detail(
    "glow_plug", "medium",
    ("Weiterfahren ist in Ordnung. Bei Kälte springt der Motor schlechter an und läuft die "
     "ersten Sekunden unrund. Vor dem Winter beheben.",
     "It is fine to keep driving. In cold weather the engine starts harder and runs rough "
     "for the first few seconds. Fix it before winter."),
    [("Glühkerze durchgebrannt", "Burnt-out glow plug"),
     ("Stecker an der Glühkerze lose", "Loose glow plug connector"),
     ("Kabel beschädigt", "Damaged wiring"),
     ("Glühzeitsteuergerät", "Glow plug control module")],
    [("Springt der Motor bei Frost schlechter an, mit weißem oder blauem Rauch?",
      "Does the engine start harder in frost, with white or blue smoke?"),
     ("Widerstand der Glühkerzen in der Werkstatt messen lassen; festgebackene Kerzen nicht "
      "mit Gewalt lösen",
      "Have the glow plug resistance measured at a workshop; do not force seized plugs out")])

detail(
    "glow_module", "medium",
    ("Weiterfahren ist möglich. Fallen alle Glühkerzen aus, springt der Motor bei Frost "
     "unter Umständen gar nicht an.",
     "You can keep driving. If all glow plugs fail, the engine may not start at all in frost."),
    [("Glühzeitsteuergerät defekt", "Faulty glow plug control module"),
     ("Sicherung", "Fuse"),
     ("Glühkerze mit Kurzschluss", "Glow plug with a short circuit"),
     ("Stecker oder Kabel", "Connector or wiring")],
    [("Leuchtet die Vorglühlampe beim Einschalten der Zündung wie gewohnt?",
      "Does the glow plug light come on as usual when you switch on the ignition?"),
     ("Sicherungen prüfen", "Check the fuses")])

# --------------------------------------------------------------------------
# Electrical system and control units
# --------------------------------------------------------------------------
detail(
    "battery_low", "medium",
    ("Weiterfahren ist möglich, aber das Auto springt beim nächsten Mal vielleicht nicht an. "
     "Batterie und Ladung prüfen lassen. Leuchtet zusätzlich die Batterielampe, lädt die "
     "Lichtmaschine nicht: dann nicht mehr weit fahren.",
     "You can keep driving, but the car may not start next time. Have the battery and "
     "charging checked. If the battery light is also on, the alternator is not charging: "
     "do not drive far."),
    [("Batterie alt oder entladen (viel Kurzstrecke, lange Standzeit)",
      "Old or discharged battery (many short trips, long periods parked)"),
     ("Lichtmaschine lädt nicht genug", "Alternator not charging enough"),
     ("Polklemmen locker oder korrodiert", "Loose or corroded battery terminals"),
     ("Ein Verbraucher entlädt die Batterie im Stand", "Something drains the battery while parked")],
    [("Ruhespannung messen, nachdem das Auto einige Stunden stand: gesund sind etwa 12,4 bis "
      "12,7 Volt",
      "Measure the resting voltage after the car has stood for a few hours: about 12.4 to "
      "12.7 volts is healthy"),
     ("Bei laufendem Motor sollten es etwa 13,8 bis 14,7 Volt sein, manche neueren Autos "
      "laden zeitweise weniger",
      "With the engine running it should be about 13.8 to 14.7 volts; some newer cars charge "
      "less at times"),
     ("Polklemmen auf festen Sitz und Korrosion prüfen", "Check the terminals for a firm fit and corrosion"),
     ("Wie alt ist die Batterie? Nach 5 bis 6 Jahren ist sie oft am Ende",
      "How old is the battery? After 5 to 6 years it is often worn out")])

detail(
    "voltage_high", "medium",
    ("Nicht lange weiterfahren. Zu hohe Spannung kann Batterie, Lampen und Steuergeräte "
     "schädigen. Bald prüfen lassen.",
     "Do not drive for long. Excessive voltage can damage the battery, bulbs and control "
     "units. Have it checked soon."),
    [("Laderegler der Lichtmaschine defekt", "Faulty alternator voltage regulator"),
     ("Schlechte Masseverbindung", "Poor ground connection"),
     ("Ladegerät oder Starthilfe angeschlossen", "Charger or jump starter connected")],
    [("Spannung bei laufendem Motor messen: über etwa 15 Volt ist zu viel",
      "Measure the voltage with the engine running: above about 15 volts is too much"),
     ("Massebänder an Motor und Karosserie prüfen", "Check the ground straps on the engine and body")])

detail(
    "alternator", "medium",
    ("Das Auto fährt nur noch mit der Batterieladung und kann nach einiger Zeit ausgehen. "
     "Unnötige Verbraucher ausschalten und direkt zur Werkstatt fahren.",
     "The car is running on battery charge only and may cut out after a while. Switch off "
     "unnecessary loads and drive straight to a workshop."),
    [("Lichtmaschine oder Regler defekt", "Faulty alternator or regulator"),
     ("Keilrippenriemen rutscht oder ist gerissen", "Drive belt slipping or broken"),
     ("Stecker oder Kabel an der Lichtmaschine", "Connector or wiring at the alternator"),
     ("Freilauf der Riemenscheibe defekt", "Faulty alternator pulley freewheel")],
    [("Leuchtet die Batterielampe?", "Is the battery warning light on?"),
     ("Spannung bei laufendem Motor messen: unter etwa 13 Volt lädt die Lichtmaschine nicht",
      "Measure the voltage with the engine running: below about 13 volts the alternator is not charging"),
     ("Keilrippenriemen ansehen: Risse, quietscht er?", "Look at the drive belt: cracks, does it squeal?")])

detail(
    "ecu_internal", "",
    ("Das Steuergerät meldet einen inneren Fehler. Läuft das Auto normal, vorsichtig zur "
     "Werkstatt. Vorher die Batterie prüfen, denn schwache Spannung löst solche Fehler oft aus.",
     "The control unit reports an internal fault. If the car runs normally, drive carefully "
     "to a workshop. Check the battery first, because low voltage often triggers such faults."),
    [("Spannungseinbruch durch schwache Batterie oder Starthilfe",
      "Voltage drop from a weak battery or a jump start"),
     ("Programmierung oder Software-Update unterbrochen", "Interrupted programming or software update"),
     ("Feuchtigkeit oder Korrosion am Steuergerät", "Moisture or corrosion at the control unit"),
     ("Steuergerät defekt (selten)", "Faulty control unit (rare)")],
    [("Batteriespannung prüfen", "Check the battery voltage"),
     ("Fehler löschen und beobachten, ob er wiederkommt", "Clear the code and watch whether it returns"),
     ("In der Werkstatt nach Software-Updates fragen", "Ask the workshop about software updates")])

detail(
    "ecu_power", "cheap",
    ("Die Stromversorgung des Steuergeräts ist gestört, der Motor kann ausgehen oder nicht "
     "anspringen. Bald prüfen lassen.",
     "The control unit power supply is disturbed, so the engine may cut out or fail to "
     "start. Have it checked soon."),
    [("Hauptrelais defekt", "Faulty main relay"),
     ("Sicherung", "Fuse"),
     ("Masseverbindung oder Stecker korrodiert", "Corroded ground connection or connector"),
     ("Schwache Batterie", "Weak battery")],
    [("Sicherungen für Motorsteuerung prüfen", "Check the engine management fuses"),
     ("Batteriespannung und Polklemmen prüfen", "Check the battery voltage and terminals")])

detail(
    "ref_voltage", "cheap",
    ("Mehrere Sensoren teilen sich diese Versorgung, daher treten oft viele Folgefehler auf "
     "und das Auto kann ins Notlaufprogramm gehen. Bald prüfen lassen.",
     "Several sensors share this supply, so many follow-up faults often appear and the car "
     "may go into limp mode. Have it checked soon."),
    [("Ein Sensor an dieser Versorgung hat einen Kurzschluss", "A sensor on this supply has a short circuit"),
     ("Kabelbaum durchgescheuert oder Marderbiss", "Chafed harness or marten bite"),
     ("Steuergerät (selten)", "Control unit (rare)")],
    [("Kabelbaum auf Scheuerstellen und Bissspuren prüfen",
      "Inspect the harness for chafing and bite marks"),
     ("In der Werkstatt die Sensoren einzeln abstecken lassen: verschwindet der Fehler, ist "
      "der Schuldige gefunden",
      "Have the workshop unplug the sensors one by one: when the fault disappears, the "
      "culprit is found")])

detail(
    "mil_lamp", "cheap",
    ("Die Motorkontrollleuchte kann ausgefallen sein, neue Fehler würdest du dann nicht "
     "sehen. Bald prüfen lassen.",
     "The check engine light may have failed, so you would not see new faults. Have it "
     "checked soon."),
    [("Leuchte im Kombiinstrument", "Lamp in the instrument cluster"),
     ("Leitung zum Kombiinstrument", "Wiring to the instrument cluster")],
    [("Beim Einschalten der Zündung muss die Motorkontrollleuchte kurz aufleuchten",
      "The check engine light must light up briefly when you switch on the ignition")])

detail(
    "immobilizer", "medium",
    ("Solange der Motor läuft, fährt das Auto normal. Nach dem Abstellen springt es aber "
     "möglicherweise nicht wieder an, also nicht an einem abgelegenen Ort parken. Den "
     "Zweitschlüssel ausprobieren.",
     "As long as the engine runs, the car drives normally. After switching off it may not "
     "restart, so do not park somewhere remote. Try the spare key."),
    [("Transponder im Schlüssel defekt oder Schlüsselbatterie leer",
      "Faulty key transponder or flat key battery"),
     ("Lesespule am Zündschloss", "Reader coil at the ignition lock"),
     ("Schlüssel nicht angelernt, etwa nach einem Tausch", "Key not programmed, for example after a replacement"),
     ("Verbindung zwischen Wegfahrsperre und Motorsteuerung", "Link between the immobilizer and the engine control unit")],
    [("Mit dem Zweitschlüssel versuchen", "Try the spare key"),
     ("Batterie im Funkschlüssel tauschen", "Replace the battery in the remote key"),
     ("Andere Transponder-Schlüssel vom Schlüsselbund nehmen",
      "Take other transponder keys off the key ring")])

detail(
    "ksds", "",
    ("Das System hat Geräusche erkannt, die auf verschlissene Pleuellager hindeuten, und "
     "schaltet in den Notlauf. Möglichst nicht weiterfahren und die Marken-Werkstatt "
     "anrufen: für viele betroffene Motoren gibt es eine Rückruf- oder Garantieaktion.",
     "The system has detected noise that points to worn connecting rod bearings and "
     "switches to limp mode. Avoid driving further and call a dealer workshop: many "
     "affected engines are covered by a recall or warranty campaign."),
    [("Verschleiß an den Pleuellagern", "Wear on the connecting rod bearings"),
     ("Zu wenig oder altes Motoröl", "Low or old engine oil")],
    [("Ölstand prüfen", "Check the oil level"),
     ("Beim Hersteller nach offenen Rückrufen für die Fahrgestellnummer fragen",
      "Ask the manufacturer about open recalls for the VIN")])

detail(
    "readiness_info", "",
    ("Kein Fehler. Die Eigendiagnosen sind seit dem letzten Löschen noch nicht alle "
     "gelaufen. Normal weiterfahren, der Code verschwindet von selbst.",
     "Not a fault. The self-tests have not all run since the codes were last cleared. Keep "
     "driving normally and the code will go away by itself."),
    [("Fehlerspeicher gelöscht oder Batterie abgeklemmt", "Codes cleared or battery disconnected"),
     ("Noch nicht genug gefahren", "Not driven enough yet")],
    [("Vor der Abgasuntersuchung die Bereitschaftstests (Readiness) ansehen",
      "Check the readiness monitors before an emissions test")])

# --------------------------------------------------------------------------
# Network
# --------------------------------------------------------------------------
detail(
    "can_bus", "",
    ("Die Steuergeräte können sich nicht mehr richtig austauschen. Funktionieren Bremsen, "
     "Lenkung und Warnleuchten normal, vorsichtig zur Werkstatt fahren.",
     "The control units can no longer communicate properly. If brakes, steering and warning "
     "lights work normally, drive carefully to a workshop."),
    [("Schwache Batterie oder Spannungseinbrüche", "Weak battery or voltage drops"),
     ("Stecker oder Kabel am Datenbus (Feuchtigkeit, Korrosion)",
      "Connector or wiring on the data bus (moisture, corrosion)"),
     ("Ein defektes Steuergerät stört den Bus", "A faulty control unit disturbs the bus"),
     ("Nachgerüstete Geräte am Bordnetz", "Aftermarket devices connected to the vehicle")],
    [("Batteriespannung prüfen", "Check the battery voltage"),
     ("Nachgerüstete Geräte abstecken", "Unplug aftermarket devices"),
     ("Nach Wasser im Fußraum oder Kofferraum schauen, dort liegen oft Steuergeräte und Stecker",
      "Look for water in the footwells or boot, where control units and connectors often sit")])

detail(
    "lost_comm", "",
    ("Ein Steuergerät meldet sich nicht auf dem Datenbus. Wie schlimm das ist, hängt vom "
     "Gerät ab: betrifft es Motor, Bremsen oder Airbag, bald in die Werkstatt. Oft steckt "
     "nur ein kurzer Spannungseinbruch dahinter.",
     "A control unit is silent on the data bus. How serious that is depends on the unit: "
     "if it is the engine, brakes or airbag, go to a workshop soon. Often it was only a "
     "brief voltage drop."),
    [("Spannungseinbruch beim Starten mit schwacher Batterie, dann oft einmalig und harmlos",
      "Voltage drop when cranking with a weak battery, then often one-off and harmless"),
     ("Sicherung des betroffenen Steuergeräts", "Fuse of the affected control unit"),
     ("Stecker oder Kabel (Feuchtigkeit, Korrosion)", "Connector or wiring (moisture, corrosion)"),
     ("Steuergerät defekt", "Faulty control unit")],
    [("Batteriespannung prüfen", "Check the battery voltage"),
     ("Fehler löschen und schauen, ob er wiederkommt", "Clear the code and see whether it returns"),
     ("Sicherung des genannten Steuergeräts prüfen", "Check the fuse of the named control unit")])

detail(
    "invalid_data", "",
    ("Ein Steuergerät schickt Daten, die nicht plausibel sind. Häufig ist das ein "
     "Folgefehler: zuerst den Fehlerspeicher des genannten Steuergeräts auslesen.",
     "A control unit sends data that does not make sense. This is often a follow-up fault: "
     "read the fault memory of the named control unit first."),
    [("Fehler im sendenden Steuergerät oder an seinen Sensoren",
      "Fault in the sending control unit or its sensors"),
     ("Falsche Codierung nach dem Tausch eines Steuergeräts",
      "Wrong coding after a control unit was replaced"),
     ("Spannungseinbruch", "Voltage drop")],
    [("Fehlerspeicher des genannten Steuergeräts auslesen", "Read the fault memory of the named control unit"),
     ("Batteriespannung prüfen", "Check the battery voltage")])

# --------------------------------------------------------------------------
# Brakes, airbags, vehicle speed
# --------------------------------------------------------------------------
detail(
    "wheel_speed", "cheap",
    ("Die normale Bremse funktioniert weiter, ABS und ESP sind aber wahrscheinlich "
     "abgeschaltet. Vorsichtig fahren, besonders bei Nässe und Glätte, und bald prüfen lassen.",
     "The normal brakes still work, but ABS and stability control are probably switched "
     "off. Drive carefully, especially in wet or icy conditions, and have it checked soon."),
    [("Raddrehzahlsensor verschmutzt oder defekt", "Dirty or faulty wheel speed sensor"),
     ("Kabel am Rad beschädigt (Steinschlag, Marder, Bruch beim Einfedern)",
      "Damaged cable at the wheel (stone chips, martens, broken by suspension movement)"),
     ("Geberring oder Magnetring im Radlager beschädigt oder verschmutzt",
      "Damaged or dirty tone ring or magnetic ring in the wheel bearing"),
     ("Radlager mit Spiel", "Worn wheel bearing with play")],
    [("Kabel am betroffenen Rad bis zum Stecker ansehen",
      "Follow the cable at the affected wheel up to its connector and inspect it"),
     ("Nach einem Radlagertausch: ist der Magnetring richtig herum eingebaut?",
      "After a wheel bearing replacement: was the magnetic ring fitted the right way round?")])

detail(
    "abs_unit", "expensive",
    ("Die normale Bremse funktioniert weiter, ABS und ESP sind aber wahrscheinlich "
     "abgeschaltet. Vorsichtig fahren und bald prüfen lassen. Fühlt sich das Bremspedal "
     "anders an als sonst, nicht weiterfahren.",
     "The normal brakes still work, but ABS and stability control are probably switched "
     "off. Drive carefully and have it checked soon. If the brake pedal feels different "
     "from usual, do not drive on."),
    [("ABS-Hydraulikeinheit oder Pumpe defekt", "Faulty ABS hydraulic unit or pump"),
     ("Sicherung oder Relais", "Fuse or relay"),
     ("Stecker am ABS-Steuergerät korrodiert", "Corroded ABS control unit connector"),
     ("Spannungsversorgung, schwache Batterie", "Power supply, weak battery")],
    [("Sicherungen für ABS prüfen", "Check the ABS fuses"),
     ("Batteriespannung prüfen", "Check the battery voltage")])

detail(
    "brake_switch", "cheap",
    ("Weiterfahren ist möglich, aber prüfen, ob die Bremslichter funktionieren: sonst sieht "
     "der Hintermann nicht, dass du bremst. Tempomat und Start-Stopp gehen oft nicht mehr.",
     "You can keep driving, but check that the brake lights work: otherwise the driver "
     "behind cannot see you braking. Cruise control and start-stop often stop working."),
    [("Bremslichtschalter am Pedal defekt oder verstellt", "Brake light switch at the pedal faulty or misadjusted"),
     ("Stecker am Schalter", "Switch connector"),
     ("Bremslichtlampen durchgebrannt", "Burnt-out brake light bulbs")],
    [("Bremslichter mit einer zweiten Person oder vor einer spiegelnden Fläche prüfen",
      "Check the brake lights with a helper or in front of a reflective surface"),
     ("Sitz des Schalters am Bremspedal prüfen", "Check how the switch sits at the brake pedal")])

detail(
    "speed_sensor", "cheap",
    ("Weiterfahren ist möglich. Tacho, Tempomat, Schaltpunkte oder ABS können betroffen sein.",
     "You can keep driving. The speedometer, cruise control, shift points or ABS may be affected."),
    [("Geschwindigkeitssensor oder ABS-Raddrehzahlsensor defekt",
      "Faulty vehicle speed sensor or ABS wheel speed sensor"),
     ("Stecker oder Kabel", "Connector or wiring"),
     ("Fehler im ABS-Steuergerät, das das Signal liefert",
      "Fault in the ABS control unit that provides the signal")],
    [("Funktioniert der Tacho?", "Does the speedometer work?"),
     ("Gibt es zugleich ABS-Fehler?", "Are there ABS faults at the same time?")])

detail(
    "airbag", "medium",
    ("Airbag oder Gurtstraffer lösen bei einem Unfall möglicherweise nicht aus. Technisch "
     "kann man weiterfahren, die Sicherheit ist aber eingeschränkt: bald in die Werkstatt. "
     "Nicht selbst an Airbag-Steckern arbeiten.",
     "The airbag or seat belt pretensioner may not deploy in a crash. The car can still be "
     "driven, but safety is reduced: go to a workshop soon. Do not work on airbag "
     "connectors yourself."),
    [("Wackelkontakt an den Steckern unter den Vordersitzen, häufig nach Sitzverstellen oder Ausbau",
      "Loose contact at the connectors under the front seats, often after moving or removing a seat"),
     ("Wickelfeder (Schleifring) im Lenkrad verschlissen", "Worn clock spring in the steering wheel"),
     ("Zündpille oder Airbag-Modul defekt", "Faulty igniter or airbag module"),
     ("Zu niedrige Batteriespannung beim Start", "Battery voltage too low when starting")],
    [("Leuchtet die Airbag-Leuchte dauerhaft?", "Is the airbag light on permanently?"),
     ("Wurde kürzlich an Sitzen oder Lenkrad gearbeitet?", "Was any work done on the seats or steering wheel recently?"),
     ("Funktionieren Hupe und Lenkradtasten? Ein Ausfall deutet auf die Wickelfeder",
      "Do the horn and steering wheel buttons work? If not, the clock spring is a likely cause")])

# --------------------------------------------------------------------------
# Transmission
# --------------------------------------------------------------------------
detail(
    "trans_general", "",
    ("Das Getriebesteuergerät hat selbst einen Fehler gespeichert. Den Getriebe-"
     "Fehlerspeicher auslesen, dort steht der eigentliche Grund. Fährt das Getriebe im "
     "Notlauf mit festem Gang, nur kurz und behutsam fahren.",
     "The transmission control unit has stored a fault of its own. Read the transmission "
     "fault memory, which holds the actual cause. If the gearbox is in limp mode with a "
     "fixed gear, only drive briefly and gently."),
    [("Fehler im Getriebe: Sensor, Magnetventil oder Ölstand", "Fault in the transmission: sensor, solenoid or fluid level"),
     ("Spannungseinbruch", "Voltage drop")],
    [("Fehlerspeicher des Getriebesteuergeräts auslesen", "Read the transmission control unit fault memory"),
     ("Getriebeölstand in der Werkstatt prüfen lassen", "Have the transmission fluid level checked at a workshop")])

detail(
    "trans_solenoid", "expensive",
    ("Schaltet das Getriebe hart, rutscht es oder bleibt es in einem Gang, nur noch "
     "behutsam fahren: die Kupplungen im Getriebe können Schaden nehmen. Bald in eine "
     "Fachwerkstatt.",
     "If the gearbox shifts harshly, slips or stays in one gear, drive gently only: the "
     "clutches inside can be damaged. Go to a specialist soon."),
    [("Getriebeöl alt, verschmutzt oder zu wenig", "Old, dirty or low transmission fluid"),
     ("Magnetventil oder Ventilkörper defekt", "Faulty solenoid or valve body"),
     ("Stecker am Getriebe undicht, Öl im Kabel", "Leaking transmission connector, oil in the wiring"),
     ("Getriebesteuergerät", "Transmission control unit")],
    [("Getriebeölstand von einer Werkstatt prüfen lassen",
      "Have the transmission fluid level checked by a workshop"),
     ("Ölspuren am Getriebestecker suchen", "Look for oil at the transmission connector"),
     ("Ist schon einmal ein Getriebeölwechsel gemacht worden?",
      "Has the transmission fluid ever been changed?")])

detail(
    "trans_ratio", "expensive",
    ("Das Getriebe rutscht oder schaltet falsch. Nur noch behutsam fahren, nichts ziehen und "
     "nicht stark beschleunigen, bald in die Werkstatt.",
     "The transmission is slipping or shifting wrongly. Drive gently only, do not tow or "
     "accelerate hard, and go to a workshop soon."),
    [("Getriebeölstand zu niedrig oder Öl verbraucht", "Low or worn-out transmission fluid"),
     ("Kupplungen oder Bremsbänder im Getriebe verschlissen", "Worn clutches or bands inside the gearbox"),
     ("Magnetventile oder Ventilkörper", "Solenoids or valve body"),
     ("Drehzahlsensoren", "Speed sensors")],
    [("Steigt beim Beschleunigen die Drehzahl, ohne dass das Auto schneller wird?",
      "Do the revs rise when accelerating without the car going faster?"),
     ("Getriebeöl prüfen lassen", "Have the transmission fluid checked")])

detail(
    "trans_sensor", "medium",
    ("Weiterfahren ist meist möglich, das Getriebe kann aber hart schalten oder in einen "
     "Notlaufgang wechseln. Bald prüfen lassen.",
     "You can usually keep driving, but the gearbox may shift harshly or switch to a limp "
     "mode gear. Have it checked soon."),
    [("Sensor defekt", "Faulty sensor"),
     ("Stecker oder Kabel am Getriebe, oft Öl im Stecker", "Connector or wiring at the gearbox, often oil in the connector"),
     ("Getriebeölstand", "Transmission fluid level")],
    [("Stecker am Getriebe auf Öl und Korrosion prüfen", "Check the transmission connectors for oil and corrosion")])

detail(
    "trans_temp", "",
    ("Das Getriebe ist zu heiß. Tempo verringern, keinen Anhänger ziehen und nicht im "
     "Stop-and-go quälen. Erscheint eine Warnmeldung, anhalten und abkühlen lassen.",
     "The transmission is too hot. Slow down, do not tow and avoid stop-and-go. If a "
     "warning appears, stop and let it cool down."),
    [("Getriebeöl alt oder zu wenig", "Old or low transmission fluid"),
     ("Getriebeölkühler zugesetzt", "Clogged transmission cooler"),
     ("Starke Belastung durch Anhänger oder Bergfahrt", "Heavy load from towing or climbing"),
     ("Temperatursensor misst falsch", "Temperature sensor reads wrong")],
    [("Getriebeölstand prüfen lassen", "Have the transmission fluid level checked")])

# --------------------------------------------------------------------------
# Hybrid and electric
# --------------------------------------------------------------------------
detail(
    "hybrid_battery", "expensive",
    ("Weiterfahren ist meist möglich, oft mit weniger Leistung. Die Hochvoltbatterie "
     "verliert Kapazität oder ihre Zellen driften auseinander. In einer Fachwerkstatt für "
     "Hybride prüfen lassen und nicht selbst an orangefarbenen Hochvoltleitungen arbeiten.",
     "You can usually keep driving, often with less power. The high-voltage battery is "
     "losing capacity or its cells are drifting apart. Have it checked by a hybrid "
     "specialist and never work on the orange high-voltage cables yourself."),
    [("Gealterte Zellen der Hochvoltbatterie", "Aged cells in the high-voltage battery"),
     ("Kühlung der Batterie behindert, etwa verstopfter Lüftereinlass",
      "Battery cooling obstructed, for example a blocked fan intake"),
     ("Spannungsmessung oder Batteriesteuergerät", "Voltage sensing or battery control unit")],
    [("Lüftungsöffnung der Batteriekühlung auf Staub und Tierhaare prüfen, oft an den Rücksitzen",
      "Check the battery cooling intake for dust and pet hair, often near the rear seats"),
     ("Wird die Ladeanzeige auffällig schnell voll oder leer?",
      "Does the charge display fill up or drop unusually fast?")])

detail(
    "hv_isolation", "expensive",
    ("Isolationsfehler im Hochvoltsystem: nicht weiterfahren und das Auto bei sichtbaren "
     "Schäden nicht berühren. Pannendienst oder Hybrid-Fachwerkstatt rufen.",
     "Insulation fault in the high-voltage system: do not drive on and do not touch the car "
     "if there is visible damage. Call roadside assistance or a hybrid specialist."),
    [("Feuchtigkeit oder Schaden an Hochvoltleitungen", "Moisture or damage on high-voltage cables"),
     ("Elektrischer Klimakompressor oder Wechselrichter mit Isolationsschaden",
      "Electric A/C compressor or inverter with insulation damage"),
     ("Hochvoltbatterie", "High-voltage battery")],
    [("Nur ansehen, nicht anfassen: Unfallschaden oder Wasser im Auto?",
      "Look only, do not touch: accident damage or water inside the car?")])

detail(
    "hybrid_system", "expensive",
    ("Das Hybridsystem meldet eine Störung, das Auto kann in einen Notlauf gehen. "
     "Vorsichtig zu einer Fachwerkstatt fahren und nicht selbst an Hochvoltteilen arbeiten.",
     "The hybrid system reports a fault and the car may enter limp mode. Drive carefully "
     "to a specialist and never work on high-voltage parts yourself."),
    [("Wechselrichter oder seine Kühlung (Kühlmittelpumpe)", "Inverter or its cooling (coolant pump)"),
     ("Schwache 12-Volt-Batterie", "Weak 12 volt battery"),
     ("Hochvoltbatterie", "High-voltage battery"),
     ("Verbrennungsmotor springt nicht an (Tank leer, Zündung)", "Combustion engine does not start (empty tank, ignition)")],
    [("12-Volt-Batterie prüfen", "Check the 12 volt battery"),
     ("Kühlmittelstand im eigenen Behälter für den Wechselrichter ansehen",
      "Look at the coolant level in the separate inverter reservoir"),
     ("Ist noch genug Kraftstoff im Tank?", "Is there enough fuel in the tank?")])

# --------------------------------------------------------------------------
# Comfort
# --------------------------------------------------------------------------
detail(
    "ac", "medium",
    ("Unbedenklich für die Fahrt, nur die Klimaanlage arbeitet nicht oder eingeschränkt.",
     "Harmless for driving, only the air conditioning does not work or works poorly."),
    [("Zu wenig Kältemittel durch ein Leck", "Low refrigerant due to a leak"),
     ("Drucksensor defekt", "Faulty pressure sensor"),
     ("Kompressorkupplung oder Relais", "Compressor clutch or relay")],
    [("Beim Klimaservice die Kältemittelmenge prüfen lassen",
      "Have the refrigerant quantity checked at an A/C service")])

detail(
    "cruise", "cheap",
    ("Unbedenklich, nur der Tempomat geht eventuell nicht.",
     "Harmless, only the cruise control may not work."),
    [("Bedienhebel am Lenkrad", "Control stalk at the steering wheel"),
     ("Bremslichtschalter", "Brake light switch"),
     ("Kabel oder Wickelfeder", "Wiring or clock spring")],
    [("Bremslichter prüfen, der Tempomat hängt oft am Bremslichtschalter",
      "Check the brake lights, cruise control often depends on the brake light switch")])
