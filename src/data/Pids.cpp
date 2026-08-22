#include "data/Pids.h"

namespace caruspex {
namespace {

double a(const std::vector<int>& d) { return static_cast<double>(d[0]); }
double ab(const std::vector<int>& d) { return static_cast<double>((d[0] << 8) | d[1]); }

std::string fuelStatusText(double value) {
    switch (static_cast<int>(value)) {
        case 1: return "offener Regelkreis (Motor kalt)";
        case 2: return "geschlossener Regelkreis (normal)";
        case 4: return "offener Regelkreis (Volllast oder Schub)";
        case 8: return "offener Regelkreis (Systemfehler)";
        case 16: return "geschlossen, aber Lambdasonde fehlerhaft";
        default: return "Status " + std::to_string(static_cast<int>(value));
    }
}

std::vector<PidSpec> buildTable() {
    std::vector<PidSpec> t;
    auto add = [&t](int pid, const char* name, const char* unit, int bytes,
                    std::function<double(const std::vector<int>&)> calc,
                    double lo = 0.0, double hi = 0.0) {
        PidSpec s;
        s.pid = pid;
        s.name = name;
        s.unit = unit;
        s.bytes = bytes;
        s.calc = std::move(calc);
        s.warnLow = lo;
        s.warnHigh = hi;
        t.push_back(std::move(s));
    };

    PidSpec fuel;
    fuel.pid = 0x03;
    fuel.name = "Gemischregelung";
    fuel.unit = "";
    fuel.bytes = 2;
    fuel.calc = a;
    fuel.text = fuelStatusText;
    t.push_back(std::move(fuel));

    add(0x04, "Motorlast berechnet", "%", 1, [](const std::vector<int>& d) { return a(d) * 100.0 / 255.0; });
    add(0x05, "Kuehlmitteltemperatur", "C", 1, [](const std::vector<int>& d) { return a(d) - 40.0; }, -10, 110);
    add(0x06, "Kurzzeit-Gemischkorrektur", "%", 1,
        [](const std::vector<int>& d) { return (a(d) - 128.0) * 100.0 / 128.0; }, -20, 20);
    add(0x07, "Langzeit-Gemischkorrektur", "%", 1,
        [](const std::vector<int>& d) { return (a(d) - 128.0) * 100.0 / 128.0; }, -20, 20);
    add(0x0A, "Kraftstoffdruck", "kPa", 1, [](const std::vector<int>& d) { return a(d) * 3.0; });
    add(0x0B, "Saugrohrdruck", "kPa", 1, a);
    add(0x0C, "Motordrehzahl", "1/min", 2, [](const std::vector<int>& d) { return ab(d) / 4.0; }, 0, 6500);
    add(0x0D, "Geschwindigkeit", "km/h", 1, a);
    add(0x0E, "Zuendzeitpunkt", "Grad v.OT", 1, [](const std::vector<int>& d) { return a(d) / 2.0 - 64.0; });
    add(0x0F, "Ansauglufttemperatur", "C", 1, [](const std::vector<int>& d) { return a(d) - 40.0; });
    add(0x10, "Luftmasse (LMM)", "g/s", 2, [](const std::vector<int>& d) { return ab(d) / 100.0; });
    add(0x11, "Drosselklappenstellung", "%", 1, [](const std::vector<int>& d) { return a(d) * 100.0 / 255.0; });
    add(0x14, "Lambdasonde 1 Spannung", "V", 2, [](const std::vector<int>& d) { return d[0] / 200.0; });
    add(0x15, "Lambdasonde 2 Spannung", "V", 2, [](const std::vector<int>& d) { return d[0] / 200.0; });
    add(0x1F, "Laufzeit seit Motorstart", "s", 2, ab);
    add(0x21, "Strecke mit Fehlerlampe an", "km", 2, ab);
    add(0x2F, "Tankfuellstand", "%", 1, [](const std::vector<int>& d) { return a(d) * 100.0 / 255.0; });
    add(0x30, "Warmlaeufe seit Loeschen", "", 1, a);
    add(0x31, "Strecke seit Loeschen", "km", 2, ab);
    add(0x33, "Luftdruck (barometrisch)", "kPa", 1, a);
    add(0x42, "Spannung Steuergeraet", "V", 2, [](const std::vector<int>& d) { return ab(d) / 1000.0; }, 11.5, 15.0);
    add(0x43, "Absolute Last", "%", 2, [](const std::vector<int>& d) { return ab(d) * 100.0 / 255.0; });
    add(0x45, "Relative Drosselklappenstellung", "%", 1, [](const std::vector<int>& d) { return a(d) * 100.0 / 255.0; });
    add(0x46, "Umgebungstemperatur", "C", 1, [](const std::vector<int>& d) { return a(d) - 40.0; });
    add(0x47, "Drosselklappe absolut B", "%", 1, [](const std::vector<int>& d) { return a(d) * 100.0 / 255.0; });
    add(0x49, "Fahrpedalstellung D", "%", 1, [](const std::vector<int>& d) { return a(d) * 100.0 / 255.0; });
    add(0x4A, "Fahrpedalstellung E", "%", 1, [](const std::vector<int>& d) { return a(d) * 100.0 / 255.0; });
    add(0x4C, "Drosselklappen-Ansteuerung", "%", 1, [](const std::vector<int>& d) { return a(d) * 100.0 / 255.0; });
    add(0x5C, "Oeltemperatur", "C", 1, [](const std::vector<int>& d) { return a(d) - 40.0; });
    return t;
}

}  // namespace

const std::vector<PidSpec>& pidTable() {
    static const std::vector<PidSpec> table = buildTable();
    return table;
}

const PidSpec* findPid(int pid) {
    for (const PidSpec& s : pidTable()) {
        if (s.pid == pid) return &s;
    }
    return nullptr;
}

const std::vector<int>& defaultLivePids() {
    static const std::vector<int> list = {0x0C, 0x0D, 0x05, 0x0F, 0x04, 0x11,
                                          0x10, 0x0B, 0x06, 0x07, 0x42, 0x03};
    return list;
}

const std::vector<MonitorBit>& monitorsB() {
    static const std::vector<MonitorBit> list = {{0x01, "Verbrennungsaussetzer"},
                                                 {0x02, "Kraftstoffsystem"},
                                                 {0x04, "Bauteilueberwachung"}};
    return list;
}

const std::vector<MonitorBit>& monitorsC() {
    static const std::vector<MonitorBit> list = {
        {0x01, "Katalysator"},      {0x02, "Katalysatorheizung"}, {0x04, "Tankentlueftung"},
        {0x08, "Sekundaerluft"},    {0x10, "Klimaanlage"},        {0x20, "Lambdasonde"},
        {0x40, "Lambdasondenheizung"}, {0x80, "Abgasrueckfuehrung"}};
    return list;
}

}  // namespace caruspex
