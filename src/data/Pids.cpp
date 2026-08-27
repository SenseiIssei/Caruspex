#include "data/Pids.h"

#include <cmath>

namespace caruspex {
namespace {

double a(const std::vector<int>& d) { return static_cast<double>(d[0]); }
double b(const std::vector<int>& d) { return static_cast<double>(d[1]); }
double ab(const std::vector<int>& d) { return static_cast<double>((d[0] << 8) | d[1]); }
double cd(const std::vector<int>& d) { return static_cast<double>((d[2] << 8) | d[3]); }

double percent(const std::vector<int>& d) { return a(d) * 100.0 / 255.0; }
double trim(const std::vector<int>& d) { return (a(d) - 128.0) * 100.0 / 128.0; }
double celsius(const std::vector<int>& d) { return a(d) - 40.0; }

std::string fuelStatusText(double value) {
    switch (static_cast<int>(value)) {
        case 0: return "Motor aus";
        case 1: return "offener Regelkreis (Motor noch kalt)";
        case 2: return "geschlossener Regelkreis (normal)";
        case 4: return "offener Regelkreis (Volllast oder Schub)";
        case 8: return "offener Regelkreis (Systemfehler)";
        case 16: return "geschlossen, aber eine Lambdasonde faellt aus";
        default: return "Status " + std::to_string(static_cast<int>(value));
    }
}

std::string secondaryAirText(double value) {
    switch (static_cast<int>(value)) {
        case 1: return "vor dem Katalysator";
        case 2: return "hinter dem Katalysator";
        case 4: return "in die Aussenluft";
        case 8: return "abgeschaltet";
        default: return "Status " + std::to_string(static_cast<int>(value));
    }
}

std::string obdStandardText(double value) {
    switch (static_cast<int>(value)) {
        case 1: return "OBD-II (Kalifornien)";
        case 2: return "OBD (Bundesstaaten USA)";
        case 3: return "OBD und OBD-II";
        case 4: return "OBD-I";
        case 5: return "nicht OBD-pflichtig";
        case 6: return "EOBD (Europa)";
        case 7: return "EOBD und OBD-II";
        case 8: return "EOBD und OBD";
        case 9: return "EOBD, OBD und OBD-II";
        case 10: return "JOBD (Japan)";
        case 11: return "JOBD und OBD-II";
        case 12: return "JOBD und EOBD";
        case 13: return "JOBD, EOBD und OBD-II";
        default: return "Norm " + std::to_string(static_cast<int>(value));
    }
}

std::string fuelTypeText(double value) {
    switch (static_cast<int>(value)) {
        case 1: return "Benzin";
        case 2: return "Methanol";
        case 3: return "Ethanol";
        case 4: return "Diesel";
        case 5: return "LPG (Autogas)";
        case 6: return "CNG (Erdgas)";
        case 7: return "Propan";
        case 8: return "Elektrisch";
        case 9: return "Bifuel Benzin";
        case 10: return "Bifuel Methanol";
        case 11: return "Bifuel Ethanol";
        case 12: return "Bifuel LPG";
        case 13: return "Bifuel CNG";
        case 17: return "Hybrid Benzin";
        case 18: return "Hybrid Ethanol";
        case 19: return "Hybrid Diesel";
        default: return "Kraftstoffart " + std::to_string(static_cast<int>(value));
    }
}

std::vector<PidSpec> buildTable() {
    std::vector<PidSpec> t;

    auto add = [&t](int pid, const char* name, const char* unit, int bytes,
                    std::function<double(const std::vector<int>&)> calc, double lo = 0.0,
                    double hi = 0.0) {
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
    auto addText = [&t](int pid, const char* name, int bytes,
                        std::function<double(const std::vector<int>&)> calc,
                        std::function<std::string(double)> text) {
        PidSpec s;
        s.pid = pid;
        s.name = name;
        s.unit = "";
        s.bytes = bytes;
        s.calc = std::move(calc);
        s.text = std::move(text);
        t.push_back(std::move(s));
    };

    // --- Grundwerte ---------------------------------------------------------
    addText(0x03, "Gemischregelung", 2, a, fuelStatusText);
    add(0x04, "Motorlast berechnet", "%", 1, percent);
    add(0x05, "Kuehlmitteltemperatur", "C", 1, celsius, -10, 110);
    add(0x06, "Kurzzeit-Gemischkorrektur Bank 1", "%", 1, trim, -20, 20);
    add(0x07, "Langzeit-Gemischkorrektur Bank 1", "%", 1, trim, -20, 20);
    add(0x08, "Kurzzeit-Gemischkorrektur Bank 2", "%", 1, trim, -20, 20);
    add(0x09, "Langzeit-Gemischkorrektur Bank 2", "%", 1, trim, -20, 20);
    add(0x0A, "Kraftstoffdruck", "kPa", 1, [](const std::vector<int>& d) { return a(d) * 3.0; });
    add(0x0B, "Saugrohrdruck", "kPa", 1, a);
    add(0x0C, "Motordrehzahl", "1/min", 2, [](const std::vector<int>& d) { return ab(d) / 4.0; },
        0, 6500);
    add(0x0D, "Geschwindigkeit", "km/h", 1, a);
    add(0x0E, "Zuendzeitpunkt", "Grad v.OT", 1,
        [](const std::vector<int>& d) { return a(d) / 2.0 - 64.0; });
    add(0x0F, "Ansauglufttemperatur", "C", 1, celsius);
    add(0x10, "Luftmasse (LMM)", "g/s", 2, [](const std::vector<int>& d) { return ab(d) / 100.0; });
    add(0x11, "Drosselklappenstellung", "%", 1, percent);
    addText(0x12, "Sekundaerluft", 1, a, secondaryAirText);

    // --- Schmalband-Lambdasonden (Spannung) ---------------------------------
    struct Narrow {
        int pid;
        const char* name;
    };
    const Narrow narrow[] = {
        {0x14, "Lambdasonde Bank 1 Sonde 1, Spannung"},
        {0x15, "Lambdasonde Bank 1 Sonde 2, Spannung"},
        {0x16, "Lambdasonde Bank 1 Sonde 3, Spannung"},
        {0x17, "Lambdasonde Bank 1 Sonde 4, Spannung"},
        {0x18, "Lambdasonde Bank 2 Sonde 1, Spannung"},
        {0x19, "Lambdasonde Bank 2 Sonde 2, Spannung"},
        {0x1A, "Lambdasonde Bank 2 Sonde 3, Spannung"},
        {0x1B, "Lambdasonde Bank 2 Sonde 4, Spannung"},
    };
    for (const Narrow& n : narrow) {
        add(n.pid, n.name, "V", 2, [](const std::vector<int>& d) { return a(d) / 200.0; });
    }

    addText(0x1C, "Abgasnorm des Steuergeraets", 1, a, obdStandardText);
    add(0x1F, "Laufzeit seit Motorstart", "s", 2, ab);

    // --- Zaehler und Strecken -----------------------------------------------
    add(0x21, "Strecke mit Fehlerlampe an", "km", 2, ab);
    add(0x22, "Raildruck relativ zum Saugrohr", "kPa", 2,
        [](const std::vector<int>& d) { return ab(d) * 0.079; });
    add(0x23, "Raildruck", "kPa", 2, [](const std::vector<int>& d) { return ab(d) * 10.0; });

    // --- Breitband-Lambdasonden ---------------------------------------------
    // Die Namen leben so lange wie das Programm, PidSpec haelt nur Zeiger.
    static const char* const wideBandNames[8] = {
        "Breitbandsonde Bank 1 Sonde 1, Lambda", "Breitbandsonde Bank 1 Sonde 2, Lambda",
        "Breitbandsonde Bank 1 Sonde 3, Lambda", "Breitbandsonde Bank 1 Sonde 4, Lambda",
        "Breitbandsonde Bank 2 Sonde 1, Lambda", "Breitbandsonde Bank 2 Sonde 2, Lambda",
        "Breitbandsonde Bank 2 Sonde 3, Lambda", "Breitbandsonde Bank 2 Sonde 4, Lambda"};
    for (int i = 0; i < 8; ++i) {
        add(0x24 + i, wideBandNames[i], "", 4,
            [](const std::vector<int>& d) { return ab(d) * 2.0 / 65536.0; });
    }

    add(0x2C, "Abgasrueckfuehrung angefordert", "%", 1, percent);
    add(0x2D, "Abgasrueckfuehrung Regelabweichung", "%", 1, trim, -20, 20);
    add(0x2E, "Tankentlueftung angefordert", "%", 1, percent);
    add(0x2F, "Tankfuellstand", "%", 1, percent);
    add(0x30, "Warmlaeufe seit Loeschen", "", 1, a);
    add(0x31, "Strecke seit Loeschen", "km", 2, ab);
    add(0x32, "Tankdruck", "Pa", 2,
        [](const std::vector<int>& d) { return (ab(d) - 32768.0) / 4.0; });
    add(0x33, "Luftdruck (barometrisch)", "kPa", 1, a);

    // --- Katalysatortemperaturen --------------------------------------------
    const char* catNames[] = {
        "Katalysatortemperatur Bank 1 Sonde 1", "Katalysatortemperatur Bank 2 Sonde 1",
        "Katalysatortemperatur Bank 1 Sonde 2", "Katalysatortemperatur Bank 2 Sonde 2"};
    for (int i = 0; i < 4; ++i) {
        add(0x3C + i, catNames[i], "C", 2,
            [](const std::vector<int>& d) { return ab(d) / 10.0 - 40.0; });
    }

    // --- Bordnetz und Last ---------------------------------------------------
    add(0x42, "Spannung Steuergeraet", "V", 2,
        [](const std::vector<int>& d) { return ab(d) / 1000.0; }, 11.5, 15.0);
    add(0x43, "Absolute Last", "%", 2,
        [](const std::vector<int>& d) { return ab(d) * 100.0 / 255.0; });
    add(0x44, "Lambda angefordert", "", 2,
        [](const std::vector<int>& d) { return ab(d) * 2.0 / 65536.0; });
    add(0x45, "Relative Drosselklappenstellung", "%", 1, percent);
    add(0x46, "Umgebungstemperatur", "C", 1, celsius);
    add(0x47, "Drosselklappe absolut B", "%", 1, percent);
    add(0x48, "Drosselklappe absolut C", "%", 1, percent);
    add(0x49, "Fahrpedalstellung D", "%", 1, percent);
    add(0x4A, "Fahrpedalstellung E", "%", 1, percent);
    add(0x4B, "Fahrpedalstellung F", "%", 1, percent);
    add(0x4C, "Drosselklappen-Ansteuerung", "%", 1, percent);
    add(0x4D, "Laufzeit mit Fehlerlampe an", "min", 2, ab);
    add(0x4E, "Zeit seit dem Loeschen", "min", 2, ab);

    // --- Kraftstoff und Sonstiges -------------------------------------------
    addText(0x51, "Kraftstoffart", 1, a, fuelTypeText);
    add(0x52, "Ethanolanteil", "%", 1, percent);
    add(0x53, "Tankdruck absolut", "kPa", 2,
        [](const std::vector<int>& d) { return ab(d) / 200.0; });
    add(0x54, "Tankdruck", "Pa", 2, [](const std::vector<int>& d) { return ab(d) - 32767.0; });
    add(0x59, "Raildruck absolut", "kPa", 2,
        [](const std::vector<int>& d) { return ab(d) * 10.0; });
    add(0x5A, "Fahrpedalstellung relativ", "%", 1, percent);
    add(0x5B, "Restkapazitaet Hybridbatterie", "%", 1, percent);
    add(0x5C, "Oeltemperatur", "C", 1, celsius, -10, 130);
    add(0x5D, "Einspritzbeginn", "Grad v.OT", 2,
        [](const std::vector<int>& d) { return (ab(d) - 26880.0) / 128.0; });
    add(0x5E, "Kraftstoffdurchsatz", "l/h", 2,
        [](const std::vector<int>& d) { return ab(d) / 20.0; });

    // --- Drehmoment ----------------------------------------------------------
    add(0x61, "Drehmoment angefordert", "%", 1,
        [](const std::vector<int>& d) { return a(d) - 125.0; });
    add(0x62, "Drehmoment tatsaechlich", "%", 1,
        [](const std::vector<int>& d) { return a(d) - 125.0; });
    add(0x63, "Bezugsdrehmoment des Motors", "Nm", 2, ab);

    // --- Abgasnachbehandlung bei Dieseln -------------------------------------
    add(0x78, "Abgastemperatur Bank 1", "C", 4,
        [](const std::vector<int>& d) { return cd(d) / 10.0 - 40.0; });
    add(0x7C, "Differenzdruck Partikelfilter", "Pa", 4,
        [](const std::vector<int>& d) { return (ab(d) - 32767.0) / 128.0; });
    add(0x86, "Feinstaubmasse", "mg/m3", 2,
        [](const std::vector<int>& d) { return ab(d) / 100.0; });
    add(0x9E, "Kraftstoffverbrauch des Motors", "l/h", 2,
        [](const std::vector<int>& d) { return ab(d) / 20.0; });

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
        {0x01, "Katalysator"},         {0x02, "Katalysatorheizung"},
        {0x04, "Tankentlueftung"},     {0x08, "Sekundaerluft"},
        {0x10, "Klimaanlage"},         {0x20, "Lambdasonde"},
        {0x40, "Lambdasondenheizung"}, {0x80, "Abgasrueckfuehrung"}};
    return list;
}

}  // namespace caruspex
