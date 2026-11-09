#include "core/analysis/BatteryTest.h"

#include <algorithm>
#include <cmath>

namespace caruspex::analysis {

namespace {

bool smartCharging(const BatteryInput& in) {
    return in.startStop || in.type == BatteryType::Agm || in.type == BatteryType::Efb;
}

// AGM batteries rest about 0.1 V higher than flooded ones.
double restingOffset(BatteryType type) { return type == BatteryType::Agm ? 0.1 : 0.0; }

Phrase volts(double v) { return format("{} V", "{} V", {number(v, 2)}); }

void addStep(BatteryReport& r, const char* id, Level level, double v, Phrase title,
             Phrase explanation, Phrase advice = {}) {
    r.steps.push_back({id, level, v, std::move(title), std::move(explanation), std::move(advice)});
}

void evaluateResting(const BatteryInput& in, BatteryReport& r) {
    if (!in.restingVolts) {
        r.missingSteps.push_back("resting");
        return;
    }
    const double v = *in.restingVolts;
    const double off = restingOffset(in.type);
    const int soc = estimateStateOfCharge(v, in.type);
    r.stateOfChargePercent = soc;

    Phrase notes;
    const bool recentlyDriven = in.minutesSinceEngineOff && *in.minutesSinceEngineOff < 30.0;
    if (recentlyDriven) {
        notes = format("Der Motor ist erst {} Minuten aus. Kurz nach der Fahrt zeigt die Batterie "
                       "eine Oberflächenladung und liest sich zu gut. Am aussagekräftigsten ist "
                       "der Wert nach einigen Stunden Standzeit.",
                       "The engine has only been off for {} minutes. Shortly after driving the "
                       "battery carries a surface charge and reads too good. The value means "
                       "most after a few hours parked.",
                       {number(*in.minutesSinceEngineOff, 0)});
    } else if (v > 12.9 + off) {
        notes = format("Über 12,9 V deutet auf eine Oberflächenladung kurz nach der Fahrt oder dem "
                       "Laden hin. Nach einigen Stunden Standzeit noch einmal messen.",
                       "Above 12.9 V points to a surface charge shortly after driving or "
                       "charging. Measure again after a few hours parked.");
    }
    if (in.ambientCelsius && *in.ambientCelsius < 5.0) {
        notes = notes + format("Bei Kälte liegt die Ruhespannung etwas niedriger, ohne dass die "
                               "Batterie schlechter ist.",
                               "In the cold the resting voltage reads a little lower without "
                               "the battery being worse.");
    }
    const Phrase socText = format("Das entspricht grob {} % Ladung.", "That is roughly {}% charge.",
                                  {number(soc, 0)});

    if (v >= battery::kFull + off) {
        addStep(r, "resting", Level::Good, v, format("Batterie voll", "Battery full"),
                format("Ruhespannung {}. Eine volle Batterie hat etwa 12,6 V (AGM etwa 12,7 V).",
                       "Resting voltage {}. A full battery has about 12.6 V (AGM about 12.7 V).",
                       {volts(v)}) + socText + notes);
    } else if (v >= battery::kThreeQuarters + off) {
        addStep(r, "resting", Level::Good, v, format("Batterie gut geladen", "Battery well charged"),
                format("Ruhespannung {}. Nicht ganz voll, aber im grünen Bereich.",
                       "Resting voltage {}. Not quite full, but fine.", {volts(v)}) + socText + notes);
    } else if (v >= battery::kWeak + off) {
        addStep(r, "resting", Level::Info, v,
                format("Batterie halb geladen", "Battery half charged"),
                format("Ruhespannung {}. Die Batterie ist nur etwa halb voll. Auf Dauer schadet "
                       "das, weil sich Sulfat an den Platten festsetzt.",
                       "Resting voltage {}. The battery is only about half full. In the long "
                       "run that harms it, because sulphate builds up on the plates.",
                       {volts(v)}) + socText + notes,
                format("Eine längere Fahrt machen oder mit einem Ladegerät nachladen.",
                       "Take a longer drive or top it up with a charger."));
    } else if (v >= battery::kFlat + off) {
        addStep(r, "resting", Level::Warning, v, format("Batterie schwach", "Battery weak"),
                format("Ruhespannung {}. Unter 12,2 V ist die Batterie stark entladen oder "
                       "gealtert. Bei Kälte kann das Auto damit schon schlecht anspringen.",
                       "Resting voltage {}. Below 12.2 V the battery is heavily discharged or "
                       "aged. In the cold the car may already struggle to start.",
                       {volts(v)}) + socText + notes,
                format("Mit einem Ladegerät voll laden und nach einem Tag erneut messen. Fällt "
                       "die Spannung wieder ab, ist die Batterie verbraucht oder etwas zieht im "
                       "Stand Strom.",
                       "Charge it fully with a charger and measure again after a day. If the "
                       "voltage drops again, the battery is worn out or something drains it "
                       "while parked."));
    } else {
        addStep(r, "resting", Level::Critical, v, format("Batterie tiefentladen", "Battery deeply discharged"),
                format("Ruhespannung {}. Die Batterie ist so gut wie leer. Tiefentladung schadet "
                       "Bleibatterien dauerhaft.",
                       "Resting voltage {}. The battery is practically empty. Deep discharge "
                       "permanently harms lead-acid batteries.",
                       {volts(v)}) + notes,
                format("Sofort laden. Danach prüfen lassen, ob sie die Ladung noch hält, und "
                       "nach einem Stromverbraucher im Stand suchen.",
                       "Charge it right away. Afterwards have it checked whether it still holds "
                       "a charge, and look for something draining it while parked."));
    }
}

void evaluateCranking(const BatteryInput& in, BatteryReport& r) {
    std::optional<double> minimum = in.crankingMinimumVolts;
    if (!in.cranking.empty()) {
        double lowest = in.cranking.front().volts;
        double first = in.cranking.front().t, last = first;
        for (const VoltageSample& s : in.cranking) {
            lowest = std::min(lowest, s.volts);
            first = std::min(first, s.t);
            last = std::max(last, s.t);
        }
        minimum = minimum ? std::min(*minimum, lowest) : lowest;
        if (in.cranking.size() >= 2 && last > first) {
            r.crankingSampleRateHz = (in.cranking.size() - 1) / (last - first);
        }
    }
    if (!minimum) {
        r.missingSteps.push_back("cranking");
        return;
    }
    const double v = *minimum;

    Phrase notes;
    const bool slow = r.crankingSampleRateHz > 0 && r.crankingSampleRateHz < 10.0;
    if (slow) {
        notes = format("Gemessen wurde mit {} Werten pro Sekunde. Der Einbruch beim Anlassen "
                       "dauert oft nur Sekundenbruchteile, der echte Tiefstwert kann also noch "
                       "etwas niedriger liegen.",
                       "Measured at {} readings per second. The dip while cranking often lasts "
                       "only a fraction of a second, so the real minimum may be a little lower.",
                       {number(r.crankingSampleRateHz, 0)});
    }
    if (in.ambientCelsius && *in.ambientCelsius < 0.0) {
        notes = notes + format("Bei Frost braucht der Anlasser mehr Strom, ein etwas tieferer "
                               "Einbruch ist dann normal.",
                               "In freezing weather the starter needs more current, so a "
                               "slightly deeper dip is normal then.");
    }

    // A "dip" that is hardly below the resting voltage means the start was
    // not caught at all.
    if (in.restingVolts && v > *in.restingVolts - 0.3) {
        addStep(r, "cranking", Level::Info, v,
                format("Kein Einbruch beim Start gemessen", "No dip measured while cranking"),
                format("Der tiefste Wert ({}) liegt kaum unter der Ruhespannung. Vermutlich "
                       "wurde der Startvorgang nicht erfasst.",
                       "The lowest value ({}) is hardly below the resting voltage. The start was "
                       "probably not captured.",
                       {volts(v)}) + notes,
                format("Den Schritt wiederholen und erst nach dem Start der Messung den "
                       "Schlüssel drehen.",
                       "Repeat the step and only turn the key after the measurement has "
                       "started."));
        return;
    }

    if (v >= battery::kCrankGood) {
        addStep(r, "cranking", Level::Good, v, format("Startspannung gut", "Cranking voltage good"),
                format("Beim Anlassen fiel die Spannung nur auf {}. Über 10 V hat die Batterie "
                       "genug Kraft für den Anlasser.",
                       "While cranking the voltage only dropped to {}. Above 10 V the battery "
                       "has enough strength for the starter.",
                       {volts(v)}) + notes);
    } else if (v >= battery::kCrankWeak) {
        addStep(r, "cranking", Level::Info, v, format("Startspannung knapp", "Cranking voltage borderline"),
                format("Beim Anlassen fiel die Spannung auf {}. Das reicht noch, aber unter 9,6 V "
                       "wird es kritisch, vor allem im Winter.",
                       "While cranking the voltage dropped to {}. That is still enough, but "
                       "below 9.6 V it gets critical, especially in winter.",
                       {volts(v)}) + notes,
                format("Vor dem Winter die Batterie prüfen lassen.",
                       "Have the battery tested before winter."));
    } else if (v >= battery::kCrankBad) {
        addStep(r, "cranking", Level::Warning, v, format("Startspannung zu niedrig", "Cranking voltage too low"),
                format("Beim Anlassen fiel die Spannung auf {}. Unter 9,6 V ist meist die "
                       "Batterie schwach. Ist sie voll geladen und trotzdem so tief, kann auch "
                       "der Anlasser zu viel Strom ziehen oder ein Kabel schlecht Kontakt haben.",
                       "While cranking the voltage dropped to {}. Below 9.6 V the battery is "
                       "usually weak. If it is fully charged and still drops this low, the "
                       "starter may draw too much current or a cable may have a poor contact.",
                       {volts(v)}) + notes,
                format("Batterie laden und einen Belastungstest machen lassen. Polklemmen und "
                       "Massekabel auf festen Sitz und Korrosion prüfen.",
                       "Charge the battery and have a load test done. Check the terminals and "
                       "ground cable for tightness and corrosion."));
    } else {
        addStep(r, "cranking", Level::Critical, v, format("Batterie bricht beim Start ein", "Battery collapses while cranking"),
                format("Beim Anlassen fiel die Spannung auf {}. So tief schalten sich manche "
                       "Steuergeräte schon ab, und der nächste Start kann scheitern.",
                       "While cranking the voltage dropped to {}. That low, some control units "
                       "already switch off, and the next start may fail.",
                       {volts(v)}) + notes,
                format("Batterie bald ersetzen oder prüfen lassen, Starthilfekabel mitnehmen.",
                       "Replace or test the battery soon, and carry jump leads."));
    }
}

const Phrase& smartNote() {
    static const Phrase note = format(
        "Dieses Auto hat ein intelligentes Lademanagement (Start-Stopp, AGM- oder EFB-Batterie). "
        "Es lädt nur, wenn es nötig ist, und nutzt gern die Bremsphasen. Werte von etwa 12,5 bis "
        "15 V sind dabei normal.",
        "This car has smart charging (start-stop, AGM or EFB battery). It only charges when "
        "needed and likes to use braking phases. Values of about 12.5 to 15 V are normal then.");
    return note;
}

void evaluateCharging(const BatteryInput& in, BatteryReport& r) {
    const bool smart = smartCharging(in);
    if (!in.idleVolts) {
        r.missingSteps.push_back("charging_idle");
    } else {
        const double v = *in.idleVolts;
        const Phrase base = format("Im Leerlauf ohne zusätzliche Verbraucher liegen {} an. Normal "
                                   "sind 13,8 bis 14,8 V.",
                                   "At idle without extra consumers there are {}. Normal is 13.8 "
                                   "to 14.8 V.",
                                   {volts(v)});
        if (v > battery::kChargeDangerous) {
            addStep(r, "charging_idle", Level::Critical, v, format("Ladespannung viel zu hoch", "Charging voltage far too high"),
                    base + format("Über 15,5 V kocht die Batterie und Elektronik kann Schaden "
                                  "nehmen. Meist ist der Laderegler defekt.",
                                  "Above 15.5 V the battery boils and electronics can be "
                                  "damaged. Usually the voltage regulator has failed."),
                    format("Bald in die Werkstatt.", "Have it seen by a garage soon."));
        } else if (v > battery::kChargeTooHigh) {
            addStep(r, "charging_idle", smart ? Level::Info : Level::Warning, v,
                    format("Ladespannung hoch", "Charging voltage high"),
                    base + (smart ? smartNote()
                                  : format("Dauerhaft über 15 V wird die Batterie überladen.",
                                           "Permanently above 15 V the battery gets overcharged.")),
                    format("Nach ein paar Minuten noch einmal messen. Bleibt es so, Regler prüfen "
                           "lassen.",
                           "Measure again after a few minutes. If it stays like this, have the "
                           "regulator checked."));
        } else if (v > battery::kChargeHigh) {
            addStep(r, "charging_idle", smart ? Level::Good : Level::Info, v,
                    format("Ladespannung am oberen Rand", "Charging voltage at the upper edge"),
                    base + (smart ? smartNote()
                                  : format("Kurz nach dem Start oder bei Kälte ist das normal.",
                                           "Shortly after starting or in the cold that is normal.")));
        } else if (v >= battery::kChargeLow) {
            addStep(r, "charging_idle", Level::Good, v, format("Lichtmaschine lädt richtig", "Alternator charges properly"),
                    base);
        } else if (v >= battery::kChargeNone) {
            addStep(r, "charging_idle", smart ? Level::Good : Level::Info, v,
                    format("Ladespannung eher niedrig", "Charging voltage rather low"),
                    base + (smart ? smartNote()
                                  : format("Etwas wenig; der Test mit eingeschalteten "
                                           "Verbrauchern zeigt, ob die Lichtmaschine mithält.",
                                           "A little low; the test with consumers switched on "
                                           "shows whether the alternator keeps up.")));
        } else if (smart && v >= 12.4) {
            addStep(r, "charging_idle", Level::Info, v,
                    format("Lademanagement pausiert vermutlich", "Smart charging probably paused"),
                    base + smartNote(),
                    format("Der nächste Schritt mit eingeschalteten Verbrauchern zeigt, ob die "
                           "Lichtmaschine liefert.",
                           "The next step with consumers switched on shows whether the "
                           "alternator delivers."));
        } else if (v >= 12.7) {
            addStep(r, "charging_idle", Level::Warning, v, format("Ladespannung zu niedrig", "Charging voltage too low"),
                    base + format("Unter 13,2 V lädt die Batterie kaum.",
                                  "Below 13.2 V the battery barely charges."),
                    format("Keilrippenriemen, Kabel und Lichtmaschine prüfen lassen.",
                           "Have the drive belt, cables and alternator checked."));
        } else {
            addStep(r, "charging_idle", Level::Critical, v, format("Lichtmaschine lädt nicht", "Alternator is not charging"),
                    base + format("Die Spannung liegt auf Batterieniveau, die Batterie wird also "
                                  "leer gefahren.",
                                  "The voltage is at battery level, so the battery is being "
                                  "drained."),
                    format("Bald prüfen lassen, sonst bleibt das Auto liegen.",
                           "Have it checked soon, or the car will stop on the road."));
        }
    }

    if (!in.loadedVolts) {
        r.missingSteps.push_back("charging_load");
        return;
    }
    const double v = *in.loadedVolts;
    const Phrase base = format("Mit Licht, Gebläse und Heckscheibenheizung liegen {} an.",
                               "With lights, blower and rear window heating on there are {}.",
                               {volts(v)});
    Phrase drop;
    if (in.idleVolts) {
        drop = format("Das sind {} V weniger als ohne Verbraucher.",
                      "That is {} V less than without consumers.",
                      {number(*in.idleVolts - v, 2)});
        if (*in.idleVolts - v <= 0.05) drop = {};
    }
    const double floorBad = smart ? 12.4 : 12.7;
    const double floorWeak = smart ? 12.8 : battery::kChargeNone;
    if (v < floorBad) {
        addStep(r, "charging_load", Level::Critical, v,
                format("Lichtmaschine schafft die Last nicht", "Alternator cannot handle the load"),
                base + drop + format("Die Batterie muss mitliefern und wird dabei leer.",
                                     "The battery has to help out and drains in the process."),
                format("Lichtmaschine und Keilrippenriemen bald prüfen lassen.",
                       "Have the alternator and drive belt checked soon."));
    } else if (v < floorWeak) {
        addStep(r, "charging_load", Level::Warning, v,
                format("Ladespannung bricht unter Last ein", "Charging voltage sags under load"),
                base + drop + format("Eine gesunde Lichtmaschine hält auch unter Last über "
                                     "13,2 V.",
                                     "A healthy alternator stays above 13.2 V even under load.") +
                    (smart ? smartNote() : Phrase{}),
                format("Riemenspannung prüfen und die Lichtmaschine testen lassen.",
                       "Check the belt tension and have the alternator tested."));
    } else if (in.idleVolts && *in.idleVolts - v > 0.5 && !smart) {
        addStep(r, "charging_load", Level::Info, v,
                format("Ladespannung sinkt unter Last spürbar", "Charging voltage drops noticeably under load"),
                base + drop + format("Noch ausreichend, aber mehr als die üblichen 0,5 V Abfall.",
                                     "Still enough, but more than the usual 0.5 V drop."),
                format("Bei Gelegenheit Riemen und Lichtmaschine ansehen lassen.",
                       "Have the belt and alternator looked at when convenient."));
    } else {
        addStep(r, "charging_load", Level::Good, v,
                format("Lichtmaschine hält unter Last", "Alternator copes with the load"),
                base + drop);
    }
}

}  // namespace

int estimateStateOfCharge(double restingVolts, BatteryType type) {
    // Open-circuit voltage of a lead-acid battery against charge, roughly
    // linear between these points.
    static const double table[][2] = {
        {11.8, 0}, {12.0, 25}, {12.2, 50}, {12.4, 75}, {12.6, 100}};
    const double v = restingVolts - restingOffset(type);
    if (v <= table[0][0]) return 0;
    if (v >= table[4][0]) return 100;
    for (int i = 1; i < 5; ++i) {
        if (v <= table[i][0]) {
            const double f = (v - table[i - 1][0]) / (table[i][0] - table[i - 1][0]);
            const double soc = table[i - 1][1] + f * (table[i][1] - table[i - 1][1]);
            return static_cast<int>(std::lround(soc / 5.0) * 5);   // steps of 5 %
        }
    }
    return 100;
}

BatteryReport evaluateBattery(const BatteryInput& input) {
    BatteryReport r;
    evaluateResting(input, r);
    evaluateCranking(input, r);
    evaluateCharging(input, r);

    r.overall = Level::Good;
    for (const BatteryStepResult& s : r.steps) r.overall = worse(r.overall, s.level);

    if (r.steps.empty()) {
        r.summary = format("Noch keine Messung. Bitte die Schritte des Tests durchgehen.",
                           "No measurement yet. Please go through the steps of the test.");
        return r;
    }
    switch (r.overall) {
        case Level::Good:
        case Level::Info:
            r.summary = format("Batterie und Lichtmaschine sind in Ordnung.",
                               "Battery and alternator are fine.");
            break;
        case Level::Warning:
            r.summary = format("Ein Teil des Bordnetzes schwächelt. Details stehen bei den "
                               "einzelnen Schritten.",
                               "Part of the electrical system is weak. The details are with the "
                               "individual steps.");
            break;
        case Level::Critical:
            r.summary = format("Das Bordnetz hat ein ernstes Problem. Bitte bald darum kümmern, "
                               "sonst springt das Auto womöglich nicht mehr an.",
                               "The electrical system has a serious problem. Please deal with it "
                               "soon, or the car may not start any more.");
            break;
    }
    if (!r.missingSteps.empty()) {
        r.summary = r.summary + format("{} von 4 Schritten fehlen noch.", "{} of 4 steps are still missing.",
                                       {number(static_cast<double>(r.missingSteps.size()), 0)});
    }
    return r;
}

const std::vector<BatteryStep>& batteryTestSteps() {
    static const std::vector<BatteryStep> steps = {
        {"resting",
         {"Schritt 1, Ruhespannung: Das Auto sollte mindestens 30 Minuten stehen, besser einige "
          "Stunden. Alle Verbraucher aus, Türen zu. Zündung nur kurz einschalten (nicht "
          "starten), damit der Adapter messen kann.",
          "Step 1, resting voltage: the car should have been parked for at least 30 minutes, "
          "better a few hours. All consumers off, doors closed. Switch the ignition on only "
          "briefly (do not start) so the adapter can measure."}},
        {"cranking",
         {"Schritt 2, Startspannung: Messung starten und dann den Motor anlassen. Caruspex "
          "fragt währenddessen nur die Spannung ab, so schnell es geht, und merkt sich den "
          "tiefsten Wert.",
          "Step 2, cranking voltage: start the measurement and then start the engine. During "
          "this Caruspex polls only the voltage, as fast as it can, and keeps the lowest "
          "value."}},
        {"charging_idle",
         {"Schritt 3, Ladespannung im Leerlauf: Motor im Leerlauf laufen lassen, alle "
          "Verbraucher aus. Etwa eine Minute warten.",
          "Step 3, charging voltage at idle: let the engine idle with all consumers off. Wait "
          "about a minute."}},
        {"charging_load",
         {"Schritt 4, Ladespannung unter Last: Abblendlicht, Gebläse auf höchster Stufe und "
          "Heckscheibenheizung einschalten. Etwa eine Minute warten.",
          "Step 4, charging voltage under load: switch on the low beams, the blower at full "
          "speed and the rear window heating. Wait about a minute."}},
    };
    return steps;
}

}  // namespace caruspex::analysis
