#include "data/Pids.h"

#include <cmath>

#include "core/Lang.h"

namespace caruspex {
namespace {

double a(const std::vector<int>& d) { return static_cast<double>(d[0]); }
double b(const std::vector<int>& d) { return static_cast<double>(d[1]); }
double ab(const std::vector<int>& d) { return static_cast<double>((d[0] << 8) | d[1]); }
double cd(const std::vector<int>& d) { return static_cast<double>((d[2] << 8) | d[3]); }

double percent(const std::vector<int>& d) { return a(d) * 100.0 / 255.0; }
double trim(const std::vector<int>& d) { return (a(d) - 128.0) * 100.0 / 128.0; }
double celsius(const std::vector<int>& d) { return a(d) - 40.0; }

// The status texts are resolved when they are called, so they always follow
// the active language.
std::string fuelStatusText(double value) {
    switch (static_cast<int>(value)) {
        case 0: return L("Motor aus", "engine off");
        case 1: return L("offener Regelkreis (Motor noch kalt)", "open loop (engine still cold)");
        case 2: return L("geschlossener Regelkreis (normal)", "closed loop (normal)");
        case 4: return L("offener Regelkreis (Volllast oder Schub)",
                         "open loop (full load or overrun)");
        case 8: return L("offener Regelkreis (Systemfehler)", "open loop (system fault)");
        case 16: return L("geschlossen, aber eine Lambdasonde fällt aus",
                          "closed loop, but an oxygen sensor has failed");
        default: return L("Status ", "status ") + std::to_string(static_cast<int>(value));
    }
}

std::string secondaryAirText(double value) {
    switch (static_cast<int>(value)) {
        case 1: return L("vor dem Katalysator", "upstream of the catalytic converter");
        case 2: return L("hinter dem Katalysator", "downstream of the catalytic converter");
        case 4: return L("in die Außenluft", "to the outside air");
        case 8: return L("abgeschaltet", "off");
        default: return L("Status ", "status ") + std::to_string(static_cast<int>(value));
    }
}

std::string obdStandardText(double value) {
    switch (static_cast<int>(value)) {
        case 1: return L("OBD-II (Kalifornien)", "OBD-II (California)");
        case 2: return L("OBD (Bundesstaaten USA)", "OBD (federal, USA)");
        case 3: return L("OBD und OBD-II", "OBD and OBD-II");
        case 4: return "OBD-I";
        case 5: return L("nicht OBD-pflichtig", "not OBD compliant");
        case 6: return L("EOBD (Europa)", "EOBD (Europe)");
        case 7: return L("EOBD und OBD-II", "EOBD and OBD-II");
        case 8: return L("EOBD und OBD", "EOBD and OBD");
        case 9: return L("EOBD, OBD und OBD-II", "EOBD, OBD and OBD-II");
        case 10: return "JOBD (Japan)";
        case 11: return L("JOBD und OBD-II", "JOBD and OBD-II");
        case 12: return L("JOBD und EOBD", "JOBD and EOBD");
        case 13: return L("JOBD, EOBD und OBD-II", "JOBD, EOBD and OBD-II");
        default: return L("Norm ", "standard ") + std::to_string(static_cast<int>(value));
    }
}

std::string fuelTypeText(double value) {
    switch (static_cast<int>(value)) {
        case 1: return L("Benzin", "Gasoline");
        case 2: return "Methanol";
        case 3: return "Ethanol";
        case 4: return "Diesel";
        case 5: return L("LPG (Autogas)", "LPG");
        case 6: return L("CNG (Erdgas)", "CNG (natural gas)");
        case 7: return L("Propan", "Propane");
        case 8: return L("Elektrisch", "Electric");
        case 9: return L("Bifuel Benzin", "Bifuel gasoline");
        case 10: return "Bifuel Methanol";
        case 11: return "Bifuel Ethanol";
        case 12: return "Bifuel LPG";
        case 13: return "Bifuel CNG";
        case 17: return L("Hybrid Benzin", "Hybrid gasoline");
        case 18: return "Hybrid Ethanol";
        case 19: return "Hybrid Diesel";
        default: return L("Kraftstoffart ", "fuel type ") + std::to_string(static_cast<int>(value));
    }
}

// Builds the table for one language. Names and units are picked here, once
// per language, so PidSpec can keep plain pointers to string literals.
std::vector<PidSpec> buildTable(Language lang) {
    const bool en = lang == Language::English;
    auto T = [en](const char* de, const char* english) { return en ? english : de; };

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

    // Ignition angle before top dead center.
    const char* degBtdc = T("Grad v.OT", "deg BTDC");

    // --- Basic values -------------------------------------------------------
    addText(0x03, T("Gemischregelung", "Fuel system status"), 2, a, fuelStatusText);
    add(0x04, T("Motorlast berechnet", "Calculated engine load"), "%", 1, percent);
    add(0x05, T("Kühlmitteltemperatur", "Coolant temperature"), "C", 1, celsius, -10, 110);
    add(0x06, T("Kurzzeit-Gemischkorrektur Bank 1", "Short-term fuel trim bank 1"), "%", 1, trim,
        -20, 20);
    add(0x07, T("Langzeit-Gemischkorrektur Bank 1", "Long-term fuel trim bank 1"), "%", 1, trim,
        -20, 20);
    add(0x08, T("Kurzzeit-Gemischkorrektur Bank 2", "Short-term fuel trim bank 2"), "%", 1, trim,
        -20, 20);
    add(0x09, T("Langzeit-Gemischkorrektur Bank 2", "Long-term fuel trim bank 2"), "%", 1, trim,
        -20, 20);
    add(0x0A, T("Kraftstoffdruck", "Fuel pressure"), "kPa", 1,
        [](const std::vector<int>& d) { return a(d) * 3.0; });
    add(0x0B, T("Saugrohrdruck", "Intake manifold pressure"), "kPa", 1, a);
    add(0x0C, T("Motordrehzahl", "Engine speed"), "1/min", 2,
        [](const std::vector<int>& d) { return ab(d) / 4.0; }, 0, 6500);
    add(0x0D, T("Geschwindigkeit", "Vehicle speed"), "km/h", 1, a);
    add(0x0E, T("Zündzeitpunkt", "Ignition timing advance"), degBtdc, 1,
        [](const std::vector<int>& d) { return a(d) / 2.0 - 64.0; });
    add(0x0F, T("Ansauglufttemperatur", "Intake air temperature"), "C", 1, celsius);
    add(0x10, T("Luftmasse (LMM)", "Mass air flow (MAF)"), "g/s", 2,
        [](const std::vector<int>& d) { return ab(d) / 100.0; });
    add(0x11, T("Drosselklappenstellung", "Throttle position"), "%", 1, percent);
    addText(0x12, T("Sekundärluft", "Secondary air status"), 1, a, secondaryAirText);

    // --- Narrow-band oxygen sensors (voltage) -------------------------------
    struct Narrow {
        int pid;
        const char* de;
        const char* en;
    };
    const Narrow narrow[] = {
        {0x14, "Lambdasonde Bank 1 Sonde 1, Spannung", "O2 sensor bank 1 sensor 1, voltage"},
        {0x15, "Lambdasonde Bank 1 Sonde 2, Spannung", "O2 sensor bank 1 sensor 2, voltage"},
        {0x16, "Lambdasonde Bank 1 Sonde 3, Spannung", "O2 sensor bank 1 sensor 3, voltage"},
        {0x17, "Lambdasonde Bank 1 Sonde 4, Spannung", "O2 sensor bank 1 sensor 4, voltage"},
        {0x18, "Lambdasonde Bank 2 Sonde 1, Spannung", "O2 sensor bank 2 sensor 1, voltage"},
        {0x19, "Lambdasonde Bank 2 Sonde 2, Spannung", "O2 sensor bank 2 sensor 2, voltage"},
        {0x1A, "Lambdasonde Bank 2 Sonde 3, Spannung", "O2 sensor bank 2 sensor 3, voltage"},
        {0x1B, "Lambdasonde Bank 2 Sonde 4, Spannung", "O2 sensor bank 2 sensor 4, voltage"},
    };
    for (const Narrow& n : narrow) {
        add(n.pid, T(n.de, n.en), "V", 2, [](const std::vector<int>& d) { return a(d) / 200.0; });
    }

    addText(0x1C, T("Abgasnorm des Steuergeräts", "OBD standard of the ECU"), 1, a,
            obdStandardText);
    add(0x1F, T("Laufzeit seit Motorstart", "Run time since engine start"), "s", 2, ab);

    // --- Counters and distances ---------------------------------------------
    add(0x21, T("Strecke mit Fehlerlampe an", "Distance with MIL on"), "km", 2, ab);
    add(0x22, T("Raildruck relativ zum Saugrohr", "Fuel rail pressure (relative to manifold)"),
        "kPa", 2, [](const std::vector<int>& d) { return ab(d) * 0.079; });
    add(0x23, T("Raildruck", "Fuel rail pressure"), "kPa", 2,
        [](const std::vector<int>& d) { return ab(d) * 10.0; });

    // --- Wide-band oxygen sensors -------------------------------------------
    // The names live as long as the program, PidSpec only holds pointers.
    static const char* const wideBandNames[2][8] = {
        {"Breitbandsonde Bank 1 Sonde 1, Lambda", "Breitbandsonde Bank 1 Sonde 2, Lambda",
         "Breitbandsonde Bank 1 Sonde 3, Lambda", "Breitbandsonde Bank 1 Sonde 4, Lambda",
         "Breitbandsonde Bank 2 Sonde 1, Lambda", "Breitbandsonde Bank 2 Sonde 2, Lambda",
         "Breitbandsonde Bank 2 Sonde 3, Lambda", "Breitbandsonde Bank 2 Sonde 4, Lambda"},
        {"Wide-band O2 sensor bank 1 sensor 1, lambda", "Wide-band O2 sensor bank 1 sensor 2, lambda",
         "Wide-band O2 sensor bank 1 sensor 3, lambda", "Wide-band O2 sensor bank 1 sensor 4, lambda",
         "Wide-band O2 sensor bank 2 sensor 1, lambda", "Wide-band O2 sensor bank 2 sensor 2, lambda",
         "Wide-band O2 sensor bank 2 sensor 3, lambda", "Wide-band O2 sensor bank 2 sensor 4, lambda"}};
    for (int i = 0; i < 8; ++i) {
        add(0x24 + i, wideBandNames[en ? 1 : 0][i], "", 4,
            [](const std::vector<int>& d) { return ab(d) * 2.0 / 65536.0; });
    }

    add(0x2C, T("Abgasrückführung angefordert", "Commanded EGR"), "%", 1, percent);
    add(0x2D, T("Abgasrückführung Regelabweichung", "EGR error"), "%", 1, trim, -20, 20);
    add(0x2E, T("Tankentlüftung angefordert", "Commanded evaporative purge"), "%", 1, percent);
    add(0x2F, T("Tankfüllstand", "Fuel tank level"), "%", 1, percent);
    add(0x30, T("Warmläufe seit Löschen", "Warm-ups since codes cleared"), "", 1, a);
    add(0x31, T("Strecke seit Löschen", "Distance since codes cleared"), "km", 2, ab);
    add(0x32, T("Tankdruck", "Fuel tank vapor pressure"), "Pa", 2,
        [](const std::vector<int>& d) { return (ab(d) - 32768.0) / 4.0; });
    add(0x33, T("Luftdruck (barometrisch)", "Barometric pressure"), "kPa", 1, a);

    // --- Catalytic converter temperatures -----------------------------------
    const char* catNames[2][4] = {
        {"Katalysatortemperatur Bank 1 Sonde 1", "Katalysatortemperatur Bank 2 Sonde 1",
         "Katalysatortemperatur Bank 1 Sonde 2", "Katalysatortemperatur Bank 2 Sonde 2"},
        {"Catalyst temperature bank 1 sensor 1", "Catalyst temperature bank 2 sensor 1",
         "Catalyst temperature bank 1 sensor 2", "Catalyst temperature bank 2 sensor 2"}};
    for (int i = 0; i < 4; ++i) {
        add(0x3C + i, catNames[en ? 1 : 0][i], "C", 2,
            [](const std::vector<int>& d) { return ab(d) / 10.0 - 40.0; });
    }

    // --- Electrical system and load ------------------------------------------
    add(0x42, T("Spannung Steuergerät", "Control unit voltage"), "V", 2,
        [](const std::vector<int>& d) { return ab(d) / 1000.0; }, 11.5, 15.0);
    add(0x43, T("Absolute Last", "Absolute load"), "%", 2,
        [](const std::vector<int>& d) { return ab(d) * 100.0 / 255.0; });
    add(0x44, T("Lambda angefordert", "Commanded lambda"), "", 2,
        [](const std::vector<int>& d) { return ab(d) * 2.0 / 65536.0; });
    add(0x45, T("Relative Drosselklappenstellung", "Relative throttle position"), "%", 1, percent);
    add(0x46, T("Umgebungstemperatur", "Ambient air temperature"), "C", 1, celsius);
    add(0x47, T("Drosselklappe absolut B", "Absolute throttle position B"), "%", 1, percent);
    add(0x48, T("Drosselklappe absolut C", "Absolute throttle position C"), "%", 1, percent);
    add(0x49, T("Fahrpedalstellung D", "Accelerator pedal position D"), "%", 1, percent);
    add(0x4A, T("Fahrpedalstellung E", "Accelerator pedal position E"), "%", 1, percent);
    add(0x4B, T("Fahrpedalstellung F", "Accelerator pedal position F"), "%", 1, percent);
    add(0x4C, T("Drosselklappen-Ansteuerung", "Commanded throttle actuator"), "%", 1, percent);
    add(0x4D, T("Laufzeit mit Fehlerlampe an", "Run time with MIL on"), "min", 2, ab);
    add(0x4E, T("Zeit seit dem Löschen", "Time since codes cleared"), "min", 2, ab);

    // --- Fuel and miscellaneous ---------------------------------------------
    addText(0x51, T("Kraftstoffart", "Fuel type"), 1, a, fuelTypeText);
    add(0x52, T("Ethanolanteil", "Ethanol content"), "%", 1, percent);
    add(0x53, T("Tankdruck absolut", "Absolute fuel tank vapor pressure"), "kPa", 2,
        [](const std::vector<int>& d) { return ab(d) / 200.0; });
    add(0x54, T("Tankdruck", "Fuel tank vapor pressure"), "Pa", 2,
        [](const std::vector<int>& d) { return ab(d) - 32767.0; });
    add(0x59, T("Raildruck absolut", "Fuel rail pressure (absolute)"), "kPa", 2,
        [](const std::vector<int>& d) { return ab(d) * 10.0; });
    add(0x5A, T("Fahrpedalstellung relativ", "Relative accelerator pedal position"), "%", 1,
        percent);
    add(0x5B, T("Restkapazität Hybridbatterie", "Hybrid battery remaining life"), "%", 1, percent);
    add(0x5C, T("Öltemperatur", "Engine oil temperature"), "C", 1, celsius, -10, 130);
    add(0x5D, T("Einspritzbeginn", "Fuel injection timing"), degBtdc, 2,
        [](const std::vector<int>& d) { return (ab(d) - 26880.0) / 128.0; });
    add(0x5E, T("Kraftstoffdurchsatz", "Engine fuel rate"), "l/h", 2,
        [](const std::vector<int>& d) { return ab(d) / 20.0; });

    // --- Torque --------------------------------------------------------------
    add(0x61, T("Drehmoment angefordert", "Driver's demand engine torque"), "%", 1,
        [](const std::vector<int>& d) { return a(d) - 125.0; });
    add(0x62, T("Drehmoment tatsächlich", "Actual engine torque"), "%", 1,
        [](const std::vector<int>& d) { return a(d) - 125.0; });
    add(0x63, T("Bezugsdrehmoment des Motors", "Engine reference torque"), "Nm", 2, ab);

    // --- Exhaust aftertreatment on diesels -----------------------------------
    add(0x78, T("Abgastemperatur Bank 1", "Exhaust gas temperature bank 1"), "C", 4,
        [](const std::vector<int>& d) { return cd(d) / 10.0 - 40.0; });
    add(0x7C, T("Differenzdruck Partikelfilter", "Diesel particulate filter differential pressure"),
        "Pa", 4, [](const std::vector<int>& d) { return (ab(d) - 32767.0) / 128.0; });
    add(0x86, T("Feinstaubmasse", "Particulate matter"), "mg/m3", 2,
        [](const std::vector<int>& d) { return ab(d) / 100.0; });
    add(0x9E, T("Kraftstoffverbrauch des Motors", "Engine fuel consumption"), "l/h", 2,
        [](const std::vector<int>& d) { return ab(d) / 20.0; });

    return t;
}

}  // namespace

// One table per language, both built on first use. The active language
// decides which one is handed out, so names and units follow it.
const std::vector<PidSpec>& pidTable() {
    static const std::vector<PidSpec> german = buildTable(Language::German);
    static const std::vector<PidSpec> english = buildTable(Language::English);
    return language() == Language::English ? english : german;
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
    static const std::vector<MonitorBit> german = {{0x01, "Verbrennungsaussetzer"},
                                                   {0x02, "Kraftstoffsystem"},
                                                   {0x04, "Bauteilüberwachung"}};
    static const std::vector<MonitorBit> english = {{0x01, "Misfire"},
                                                    {0x02, "Fuel system"},
                                                    {0x04, "Comprehensive components"}};
    return language() == Language::English ? english : german;
}

const std::vector<MonitorBit>& monitorsC() {
    static const std::vector<MonitorBit> german = {
        {0x01, "Katalysator"},         {0x02, "Katalysatorheizung"},
        {0x04, "Tankentlüftung"},     {0x08, "Sekundärluft"},
        {0x10, "Klimaanlage"},         {0x20, "Lambdasonde"},
        {0x40, "Lambdasondenheizung"}, {0x80, "Abgasrückführung"}};
    static const std::vector<MonitorBit> english = {
        {0x01, "Catalyst"},              {0x02, "Heated catalyst"},
        {0x04, "Evaporative system"},    {0x08, "Secondary air system"},
        {0x10, "A/C refrigerant"},       {0x20, "Oxygen sensor"},
        {0x40, "Oxygen sensor heater"},  {0x80, "EGR system"}};
    return language() == Language::English ? english : german;
}

}  // namespace caruspex
