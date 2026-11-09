#include "core/analysis/DriveCycleGuide.h"

#include <algorithm>

namespace caruspex::analysis {

const std::vector<MonitorGuide>& allDriveCycleGuides() {
    static const std::vector<MonitorGuide> guides = {
        {Monitor::Misfire,
         monitorName(Monitor::Misfire),
         {"Prüft, ob in jedem Zylinder das Gemisch sauber zündet. Ein Aussetzer schüttelt den "
          "Motor und schickt unverbranntes Benzin in den Katalysator, der davon überhitzen kann.",
          "Checks that the mixture fires cleanly in every cylinder. A misfire shakes the engine "
          "and sends unburnt fuel into the catalytic converter, which can overheat from it."},
         {"Läuft ständig mit. Ein paar Minuten normale Fahrt mit wechselnder Drehzahl reichen "
          "meistens.",
          "Runs all the time. A few minutes of normal driving at varying engine speeds is "
          "usually enough."},
         {"wenige Minuten", "a few minutes"},
         false},
        {Monitor::FuelSystem,
         monitorName(Monitor::FuelSystem),
         {"Prüft, ob die Gemischregelung das Verhältnis von Luft und Kraftstoff im erlaubten "
          "Bereich halten kann. Muss sie zu stark nachregeln, stimmt etwas nicht (Falschluft, "
          "Einspritzung, Kraftstoffdruck).",
          "Checks that the mixture control can keep the ratio of air and fuel within its "
          "allowed range. If it has to correct too much, something is off (air leak, "
          "injection, fuel pressure)."},
         {"Läuft ständig mit, sobald der Motor warm ist. Einige Minuten Leerlauf und "
          "gleichmäßige Fahrt.",
          "Runs all the time once the engine is warm. A few minutes of idling and steady "
          "driving."},
         {"5 bis 10 Minuten", "5 to 10 minutes"},
         false},
        {Monitor::Components,
         monitorName(Monitor::Components),
         {"Prüft die Sensoren und Stellglieder des Motors auf Kurzschluss, Unterbrechung und "
          "unplausible Werte, zum Beispiel Temperaturfühler und Drosselklappe.",
          "Checks the engine's sensors and actuators for short circuits, open circuits and "
          "implausible values, for example temperature sensors and the throttle."},
         {"Läuft ständig mit und ist meist schon kurz nach dem Start fertig.",
          "Runs all the time and is usually complete shortly after starting."},
         {"unter einer Minute bis wenige Minuten", "under a minute to a few minutes"},
         false},
        {Monitor::Catalyst,
         monitorName(Monitor::Catalyst),
         {"Prüft, ob der Katalysator die Abgase noch richtig reinigt. Dafür vergleicht das "
          "Steuergerät die Lambdasonde vor dem Katalysator mit der dahinter.",
          "Checks that the catalytic converter still cleans the exhaust properly. To do so, "
          "the control unit compares the oxygen sensor in front of the converter with the "
          "one behind it."},
         {"Motor ganz warm fahren, dann 5 bis 10 Minuten gleichmäßig zwischen 70 und 100 km/h "
          "auf ebener Strecke, mit ruhigem Gasfuß. Danach ein paar Mal vom Gas gehen und "
          "ausrollen lassen. Manche Autos brauchen dafür zwei oder drei Fahrten.",
          "Drive until the engine is fully warm, then 5 to 10 minutes at a steady 70 to "
          "100 km/h (45 to 60 mph) on a level road with a light foot. Afterwards lift off the "
          "throttle a few times and let the car coast. Some cars need two or three trips."},
         {"15 bis 30 Minuten, oft über mehrere Fahrten", "15 to 30 minutes, often over several trips"},
         false},
        {Monitor::HeatedCatalyst,
         monitorName(Monitor::HeatedCatalyst),
         {"Prüft die Heizung, die den Katalysator nach dem Kaltstart schneller auf "
          "Betriebstemperatur bringt. Gibt es nur bei wenigen Autos.",
          "Checks the heater that brings the catalytic converter up to temperature faster "
          "after a cold start. Only a few cars have it."},
         {"Kaltstart, danach ein bis zwei Minuten Leerlauf und dann normal losfahren.",
          "Cold start, then idle for one to two minutes and drive off normally."},
         {"10 bis 20 Minuten nach einem Kaltstart", "10 to 20 minutes after a cold start"},
         true},
        {Monitor::Evap,
         monitorName(Monitor::Evap),
         {"Prüft, ob das Tanksystem dicht ist, damit keine Benzindämpfe entweichen. Schon ein "
          "schlecht geschlossener Tankdeckel kann den Test scheitern lassen.",
          "Checks that the fuel tank system is sealed so no fuel vapour escapes. Even a fuel "
          "cap that is not closed properly can make the test fail."},
         {"Der schwierigste Test. Das Auto mindestens 8 Stunden stehen lassen (am besten über "
          "Nacht), Tank zwischen einem Viertel und drei Vierteln voll, Tankdeckel fest zu. "
          "Kaltstart, dann ruhig fahren mit Stadt- und Landstraßenanteil und einer Minute "
          "Leerlauf am Ende. Viele Autos testen erst nach dem Abstellen, also danach wieder "
          "einige Stunden stehen lassen. Bei sehr kaltem oder sehr heißem Wetter läuft der "
          "Test oft gar nicht.",
          "The hardest one. Let the car sit for at least 8 hours (overnight is best), tank "
          "between a quarter and three quarters full, fuel cap tightly closed. Cold start, "
          "then drive calmly with some town and some country road and a minute of idling at "
          "the end. Many cars run the test only after being switched off, so let it sit for "
          "a few hours again afterwards. In very cold or very hot weather the test often "
          "does not run at all."},
         {"oft ein bis mehrere Tage normaler Nutzung", "often one to several days of normal use"},
         true},
        {Monitor::SecondaryAir,
         monitorName(Monitor::SecondaryAir),
         {"Prüft die Pumpe, die nach dem Kaltstart Frischluft in den Auspuff bläst, damit der "
          "Katalysator schneller warm wird.",
          "Checks the pump that blows fresh air into the exhaust after a cold start so the "
          "catalytic converter warms up faster."},
         {"Kaltstart nach mehreren Stunden Standzeit, dann ein bis zwei Minuten im Leerlauf "
          "laufen lassen, ohne Gas zu geben. Danach normal losfahren.",
          "Cold start after several hours parked, then let it idle for one to two minutes "
          "without touching the throttle. Then drive off normally."},
         {"wenige Minuten nach einem Kaltstart", "a few minutes after a cold start"},
         true},
        {Monitor::AcRefrigerant,
         monitorName(Monitor::AcRefrigerant),
         {"Ein alter, kaum genutzter Test für das Kältemittel der Klimaanlage. Heute meldet ihn "
          "fast kein Auto mehr.",
          "An old, rarely used test for the air conditioning refrigerant. Almost no car "
          "reports it any more."},
         {"Klimaanlage bei warmem Motor einige Minuten laufen lassen.",
          "Run the air conditioning for a few minutes with the engine warm."},
         {"wenige Minuten", "a few minutes"},
         false},
        {Monitor::OxygenSensor,
         monitorName(Monitor::OxygenSensor),
         {"Prüft, ob die Lambdasonden schnell und richtig auf den Sauerstoff im Abgas "
          "reagieren. Sie sind die Augen der Gemischregelung.",
          "Checks that the oxygen sensors react quickly and correctly to the oxygen in the "
          "exhaust. They are the eyes of the mixture control."},
         {"Motor warm fahren, dann einige Minuten gleichmäßig zwischen 50 und 90 km/h. Danach "
          "mehrmals vom Gas gehen und von etwa 90 auf 50 km/h ausrollen lassen, ohne zu "
          "bremsen. Zum Schluss eine Minute Leerlauf.",
          "Drive until warm, then a few minutes at a steady 50 to 90 km/h (30 to 55 mph). "
          "Afterwards lift off the throttle several times and coast from about 90 down to "
          "50 km/h without braking. Finish with a minute of idling."},
         {"10 bis 20 Minuten", "10 to 20 minutes"},
         false},
        {Monitor::OxygenSensorHeater,
         monitorName(Monitor::OxygenSensorHeater),
         {"Prüft die Heizung in den Lambdasonden. Die Sonden messen erst ab etwa 300 Grad "
          "richtig, die Heizung bringt sie nach dem Start schnell dahin.",
          "Checks the heaters inside the oxygen sensors. The sensors only measure properly "
          "from about 300 degrees Celsius, and the heater gets them there quickly after "
          "starting."},
         {"Kaltstart, danach ein paar Minuten Leerlauf oder ruhige Fahrt.",
          "Cold start, then a few minutes of idling or gentle driving."},
         {"wenige Minuten nach einem Kaltstart", "a few minutes after a cold start"},
         true},
        {Monitor::Egr,
         monitorName(Monitor::Egr),
         {"Prüft die Abgasrückführung, die einen Teil des Abgases zurück in den Motor leitet "
          "und so Stickoxide senkt. Bei manchen Autos steckt hier auch die Nockenwellen"
          "verstellung mit drin.",
          "Checks the exhaust gas recirculation, which leads part of the exhaust back into the "
          "engine to lower nitrogen oxides. On some cars the variable valve timing is checked "
          "here as well."},
         {"Motor warm fahren, dann gleichmäßig um 70 bis 90 km/h fahren und mehrmals vom Gas "
          "gehen und lange ausrollen lassen, ohne zu bremsen. Beim Diesel helfen zusätzlich "
          "längere Abschnitte mit gleichmäßiger Teillast.",
          "Drive until warm, then cruise at about 70 to 90 km/h (45 to 55 mph) and lift off "
          "the throttle several times, letting the car coast for a long stretch without "
          "braking. On diesels, longer stretches at steady part load help as well."},
         {"10 bis 20 Minuten", "10 to 20 minutes"},
         false},
        {Monitor::NmhcCatalyst,
         monitorName(Monitor::NmhcCatalyst),
         {"Prüft den Oxidationskatalysator des Diesels, der Kohlenwasserstoffe und "
          "Kohlenmonoxid verbrennt.",
          "Checks the diesel oxidation catalyst, which burns off hydrocarbons and carbon "
          "monoxide."},
         {"Motor warm fahren, dann etwa 20 Minuten gleichmäßig über Land oder Autobahn, "
          "zum Beispiel mit 80 bis 110 km/h.",
          "Drive until warm, then about 20 minutes at a steady pace on country roads or the "
          "motorway, for example 80 to 110 km/h (50 to 70 mph)."},
         {"20 bis 30 Minuten", "20 to 30 minutes"},
         false},
        {Monitor::NoxScr,
         monitorName(Monitor::NoxScr),
         {"Prüft die Stickoxid-Reinigung, bei den meisten neueren Dieseln mit AdBlue (SCR) oder "
          "einem Speicherkatalysator.",
          "Checks the nitrogen oxide cleaning, on most newer diesels with AdBlue (SCR) or a "
          "storage catalyst."},
         {"AdBlue-Tank nicht leer, Motor warm fahren, dann 20 bis 30 Minuten gleichmäßig über "
          "Land oder Autobahn. Bei Frost dauert es oft länger.",
          "AdBlue tank not empty, drive until warm, then 20 to 30 minutes at a steady pace on "
          "country roads or the motorway. In freezing weather it often takes longer."},
         {"30 Minuten bis mehrere Fahrten", "30 minutes to several trips"},
         false},
        {Monitor::BoostPressure,
         monitorName(Monitor::BoostPressure),
         {"Prüft, ob der Turbolader den Ladedruck aufbaut, den das Steuergerät verlangt, und "
          "nicht zu viel davon.",
          "Checks that the turbocharger builds the boost pressure the control unit asks for, "
          "and not too much of it."},
         {"Bei warmem Motor ein paar Mal in einem mittleren Gang (3. oder 4.) zügig von etwa "
          "50 auf 100 km/h beschleunigen, wo es sicher und erlaubt ist.",
          "With the engine warm, accelerate briskly a few times in a middle gear (3rd or 4th) "
          "from about 50 to 100 km/h (30 to 60 mph), where it is safe and legal."},
         {"10 bis 15 Minuten", "10 to 15 minutes"},
         false},
        {Monitor::ExhaustGasSensor,
         monitorName(Monitor::ExhaustGasSensor),
         {"Prüft die Sensoren im Abgas des Diesels, etwa Lambda- und NOx-Sensoren.",
          "Checks the sensors in the diesel exhaust, such as oxygen and NOx sensors."},
         {"Motor warm fahren, gleichmäßig fahren und dazwischen einige Male vom Gas gehen und "
          "ausrollen lassen.",
          "Drive until warm, drive steadily and in between lift off the throttle a few times "
          "and let the car coast."},
         {"15 bis 20 Minuten", "15 to 20 minutes"},
         false},
        {Monitor::ParticulateFilter,
         monitorName(Monitor::ParticulateFilter),
         {"Prüft den Rußpartikelfilter: ob er dicht ist und ob er sich beim Freibrennen "
          "(Regeneration) richtig leert.",
          "Checks the diesel particulate filter: that it is intact and that it empties "
          "properly when it burns off the soot (regeneration)."},
         {"Motor warm fahren, dann 15 bis 20 Minuten über 60 km/h, besser auf der Autobahn. "
          "Läuft gerade eine Regeneration (erhöhte Leerlaufdrehzahl, Lüfter läuft nach), "
          "die Fahrt nicht unterbrechen.",
          "Drive until warm, then 15 to 20 minutes above 60 km/h (40 mph), preferably on the "
          "motorway. If a regeneration is running (higher idle speed, fan keeps running), do "
          "not cut the trip short."},
         {"20 Minuten bis mehrere Fahrten", "20 minutes to several trips"},
         false},
    };
    return guides;
}

const MonitorGuide& driveCycleGuide(Monitor monitor) {
    for (const MonitorGuide& g : allDriveCycleGuides()) {
        if (g.monitor == monitor) return g;
    }
    return allDriveCycleGuides().front();  // unreachable: every monitor has a guide
}

std::vector<const MonitorGuide*> guidesForIncomplete(const std::vector<MonitorState>& monitors) {
    std::vector<const MonitorGuide*> out;
    for (const MonitorState& m : monitors) {
        if (!m.supported || m.complete) continue;
        const MonitorGuide* g = &driveCycleGuide(m.monitor);
        if (std::find(out.begin(), out.end(), g) == out.end()) out.push_back(g);
    }
    std::stable_sort(out.begin(), out.end(), [](const MonitorGuide* a, const MonitorGuide* b) {
        return a->needsColdStart && !b->needsColdStart;
    });
    return out;
}

Text driveCycleBasics() {
    return {"So klappt es am ehesten: Auto über Nacht stehen lassen, damit der Motor ganz kalt "
            "ist (Kühlmittel ungefähr so warm wie die Außenluft). Tank zwischen einem Viertel "
            "und drei Vierteln voll. Vorher keine Fehler mehr im Speicher und zwischendurch "
            "nichts löschen, sonst beginnt alles von vorn. Während der Fahrt Zündung nicht "
            "ausschalten und Klimaanlage, Heckscheibenheizung und andere große Verbraucher "
            "möglichst aus lassen.",
            "This works best: let the car sit overnight so the engine is completely cold "
            "(coolant about as warm as the outside air). Tank between a quarter and three "
            "quarters full. No faults left in memory beforehand, and do not clear anything in "
            "between, or everything starts over. Do not switch the ignition off during the "
            "drive, and keep the air conditioning, rear window heating and other big "
            "consumers off if you can."};
}

Text manufacturerNote() {
    return {"Wichtig: Die genauen Bedingungen legt jeder Hersteller selbst fest, oft sogar je "
            "Motor. Die Fahrmuster hier funktionieren bei den meisten Autos, sind aber keine "
            "Werksvorgabe. Wenn ein Test nach mehreren Tagen normaler Fahrt nicht fertig wird, "
            "lohnt ein Blick in die Werkstattunterlagen oder eine Frage an die Werkstatt.",
            "Important: the exact conditions are set by each manufacturer, often even per "
            "engine. The patterns here work on most cars but are not a factory specification. "
            "If a test is still not complete after several days of normal driving, check the "
            "workshop documentation or ask a garage."};
}

}  // namespace caruspex::analysis
