#include "data/Pids.h"

#include <cstdint>
#include <deque>
#include <string>
#include <utility>

#include "core/Lang.h"

// The table below uses designated initializers and leaves the fields it does
// not need at their defaults on purpose. Newer Clang versions warn about that
// under -Wextra; GCC and MSVC do not.
#if defined(__clang__)
#pragma clang diagnostic ignored "-Wmissing-field-initializers"
#endif

namespace caruspex {
namespace {

// ---------------------------------------------------------------------------
// Reading raw bytes. d[0] is byte A, the first byte after the PID number.
// ---------------------------------------------------------------------------

std::uint32_t rawUnsigned(const std::vector<int>& d, int offset, int length) {
    std::uint32_t v = 0;
    for (int i = 0; i < length; ++i) v = (v << 8) | static_cast<std::uint32_t>(d[offset + i] & 0xFF);
    return v;
}

// Two's complement for one or two bytes, as SAE J1979 uses for signed values.
std::int32_t rawSigned(const std::vector<int>& d, int offset, int length) {
    const std::uint32_t v = rawUnsigned(d, offset, length);
    const std::uint32_t sign = 1u << (length * 8 - 1);
    if (v & sign) return static_cast<std::int32_t>(v) - static_cast<std::int32_t>(sign << 1);
    return static_cast<std::int32_t>(v);
}

// A value inside the response: where it sits and how it is converted.
struct Field {
    PidCalc calc;
    int need = 1;   // bytes the value needs, counted from A
};

// value = raw * mul / div + add. Multiplying first and dividing afterwards
// keeps round numbers exact (14000 mV / 1000 is exactly 14 V).
Field scaled(int offset, int length, double mul, double div, double add = 0.0,
             bool isSigned = false) {
    Field f;
    f.need = offset + length;
    f.calc = [=](const std::vector<int>& d) {
        const double raw = isSigned ? static_cast<double>(rawSigned(d, offset, length))
                                    : static_cast<double>(rawUnsigned(d, offset, length));
        return raw * mul / div + add;
    };
    return f;
}

Field raw8(int offset) { return scaled(offset, 1, 1, 1); }
Field raw16(int offset) { return scaled(offset, 2, 1, 1); }
Field raw32(int offset) { return scaled(offset, 4, 1, 1); }
// 0 to 100 %, 100/255 per bit.
Field percent(int offset) { return scaled(offset, 1, 100, 255); }
// -100 to 99.2 %, 100/128 per bit with offset: fuel trims and EGR error.
Field trim(int offset) { return scaled(offset, 1, 100, 128, -100); }
// -40 to 215 C, 1 C per bit.
Field temp8(int offset) { return scaled(offset, 1, 1, 1, -40); }
// -40 to 6513.5 C, 0.1 C per bit.
Field temp16(int offset) { return scaled(offset, 2, 1, 10, -40); }
// -125 to 130 %, torque values.
Field torque(int offset) { return scaled(offset, 1, 1, 1, -125); }
// Equivalence ratio lambda, 0 to < 2, 2/65536 per bit.
Field lambda(int offset) { return scaled(offset, 2, 2, 65536); }
// A single bit as 0 or 1.
Field bitOf(int offset, int bit) {
    Field f;
    f.need = offset + 1;
    f.calc = [=](const std::vector<int>& d) { return static_cast<double>((d[offset] >> bit) & 1); };
    return f;
}
// Several bits as a small number.
Field bitsOf(int offset, int shift, int mask) {
    Field f;
    f.need = offset + 1;
    f.calc = [=](const std::vector<int>& d) {
        return static_cast<double>((d[offset] >> shift) & mask);
    };
    return f;
}

// Temperature byte with an optional wide range: if the given bit in byte A
// is set, the control unit reports 4 C per bit instead of 1 C (J1979-DA,
// used by EGR and charge air cooler temperatures). Reserved bits are zero,
// so control units without wide range data get the normal scaling.
Field temp8Wide(int offset, int wideBit) {
    Field f;
    f.need = offset + 1;
    f.calc = [=](const std::vector<int>& d) {
        const double raw = static_cast<double>(d[offset] & 0xFF);
        const bool wide = ((d[0] >> wideBit) & 1) != 0;
        return (wide ? raw * 4.0 : raw) - 40.0;
    };
    return f;
}

PidAvailable supportBit(int bit) {
    return [bit](const std::vector<int>& d) { return ((d[0] >> bit) & 1) != 0; };
}

// ---------------------------------------------------------------------------
// Status texts. They are resolved when called, so they follow the language.
// ---------------------------------------------------------------------------

std::string number(int value) { return std::to_string(value); }

std::string fuelStatusText(double value) {
    switch (static_cast<int>(value)) {
        case 0: return L("Motor aus oder nicht vorhanden", "engine off or not present");
        case 1: return L("offener Regelkreis (Motor noch kalt)", "open loop (engine still cold)");
        case 2: return L("geschlossener Regelkreis (normal)", "closed loop (normal)");
        case 4: return L("offener Regelkreis (Volllast oder Schub)",
                         "open loop (full load or overrun)");
        case 8: return L("offener Regelkreis (Systemfehler)", "open loop (system fault)");
        case 16: return L("geschlossen, aber eine Lambdasonde fällt aus",
                          "closed loop, but an oxygen sensor has failed");
        default: return L("Status ", "status ") + number(static_cast<int>(value));
    }
}

std::string secondaryAirText(double value) {
    switch (static_cast<int>(value)) {
        case 1: return L("vor dem Katalysator", "upstream of the catalytic converter");
        case 2: return L("hinter dem Katalysator", "downstream of the catalytic converter");
        case 4: return L("aus der Außenluft oder abgeschaltet", "from the outside air or off");
        case 8: return L("Pumpe für Prüfung eingeschaltet", "pump switched on for a check");
        default: return L("Status ", "status ") + number(static_cast<int>(value));
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
        case 14: return L("OBD, EOBD und KOBD", "OBD, EOBD and KOBD");
        case 15: return L("OBD, OBD-II, EOBD und KOBD", "OBD, OBD-II, EOBD and KOBD");
        case 17: return L("Herstellerdiagnose (EMD)", "Engine manufacturer diagnostics (EMD)");
        case 18: return L("Erweiterte Herstellerdiagnose (EMD+)",
                          "Enhanced engine manufacturer diagnostics (EMD+)");
        case 19: return L("Nutzfahrzeug-OBD, teilweise (HD OBD-C)",
                          "Heavy duty OBD, partial (HD OBD-C)");
        case 20: return L("Nutzfahrzeug-OBD (HD OBD)", "Heavy duty OBD (HD OBD)");
        case 21: return L("Weltweit harmonisierte OBD (WWH-OBD)",
                          "World wide harmonized OBD (WWH-OBD)");
        case 23: return L("Nutzfahrzeug Euro-OBD Stufe I ohne NOx-Regelung",
                          "Heavy duty Euro OBD stage I without NOx control");
        case 24: return L("Nutzfahrzeug Euro-OBD Stufe I mit NOx-Regelung",
                          "Heavy duty Euro OBD stage I with NOx control");
        case 25: return L("Nutzfahrzeug Euro-OBD Stufe II ohne NOx-Regelung",
                          "Heavy duty Euro OBD stage II without NOx control");
        case 26: return L("Nutzfahrzeug Euro-OBD Stufe II mit NOx-Regelung",
                          "Heavy duty Euro OBD stage II with NOx control");
        case 28: return L("Brasilien OBD Phase 1", "Brazil OBD phase 1");
        case 29: return L("Brasilien OBD Phase 2", "Brazil OBD phase 2");
        case 30: return L("Korea (KOBD)", "Korea (KOBD)");
        case 31: return L("Indien OBD I", "India OBD I");
        case 32: return L("Indien OBD II", "India OBD II");
        case 33: return L("Nutzfahrzeug Euro-OBD Stufe VI", "Heavy duty Euro OBD stage VI");
        default: return L("Norm ", "standard ") + number(static_cast<int>(value));
    }
}

std::string emissionDesignText(double value) {
    switch (static_cast<int>(value)) {
        case 0x0E: return L("Nutzfahrzeug Euro IV (B1)", "Heavy duty Euro IV (B1)");
        case 0x0F: return L("Nutzfahrzeug Euro V (B2)", "Heavy duty Euro V (B2)");
        case 0x10: return L("Nutzfahrzeug EEV (C)", "Heavy duty EEV (C)");
        default: return L("Anforderung ", "requirement ") + number(static_cast<int>(value));
    }
}

std::string fuelTypeText(double value) {
    switch (static_cast<int>(value)) {
        case 0: return L("nicht angegeben", "not available");
        case 1: return L("Benzin", "Gasoline");
        case 2: return "Methanol";
        case 3: return "Ethanol";
        case 4: return "Diesel";
        case 5: return L("LPG (Autogas)", "LPG");
        case 6: return L("CNG (Erdgas)", "CNG (natural gas)");
        case 7: return L("Propan", "Propane");
        case 8: return L("Elektrisch", "Electric");
        case 9: return L("Bifuel, fährt mit Benzin", "Bifuel running gasoline");
        case 10: return L("Bifuel, fährt mit Methanol", "Bifuel running methanol");
        case 11: return L("Bifuel, fährt mit Ethanol", "Bifuel running ethanol");
        case 12: return L("Bifuel, fährt mit LPG", "Bifuel running LPG");
        case 13: return L("Bifuel, fährt mit CNG", "Bifuel running CNG");
        case 14: return L("Bifuel, fährt mit Propan", "Bifuel running propane");
        case 15: return L("Bifuel, fährt elektrisch", "Bifuel running electricity");
        case 16: return L("Bifuel, elektrisch und Verbrenner", "Bifuel running electric and combustion");
        case 17: return L("Hybrid Benzin", "Hybrid gasoline");
        case 18: return "Hybrid Ethanol";
        case 19: return "Hybrid Diesel";
        case 20: return L("Hybrid elektrisch", "Hybrid electric");
        case 21: return L("Hybrid, elektrisch und Verbrenner", "Hybrid running electric and combustion");
        case 22: return L("Hybrid, Rückgewinnung", "Hybrid regenerative");
        case 23: return L("Bifuel, fährt mit Diesel", "Bifuel running diesel");
        default: return L("Kraftstoffart ", "fuel type ") + number(static_cast<int>(value));
    }
}

std::string onOffText(double value) {
    return value != 0.0 ? L("an", "on") : L("aus", "off");
}

std::string activeText(double value) {
    return value != 0.0 ? L("aktiv", "active") : L("nicht aktiv", "not active");
}

std::string engineTypeText(double value) {
    return value != 0.0 ? L("Diesel (Selbstzünder)", "Diesel (compression ignition)")
                        : L("Benziner (Fremdzünder)", "Gasoline (spark ignition)");
}

std::string automaticRangeText(double value) {
    return value != 0.0 ? L("Fahrstufe eingelegt", "drive range engaged")
                        : L("Parken oder Neutral", "park or neutral");
}

std::string manualGearText(double value) {
    return value != 0.0 ? L("Gang eingelegt", "in gear")
                        : L("Leerlauf oder Kupplung getreten", "neutral or clutch pressed");
}

// Boost pressure and turbocharger control: two bits per control loop.
std::string controlLoopText(double value) {
    switch (static_cast<int>(value)) {
        case 1: return L("gesteuert (offener Regelkreis)", "open loop");
        case 2: return L("geregelt (geschlossener Regelkreis)", "closed loop");
        case 3: return L("Fehler erkannt", "fault present");
        default: return L("keine Angabe", "not reported");
    }
}

// NOx and particulate "not to exceed" control area (heavy duty diesels).
std::string nteText(double value) {
    const int v = static_cast<int>(value);
    std::string out;
    auto add = [&out](const char* part) {
        if (!out.empty()) out += ", ";
        out += part;
    };
    if (v & 0x01) add(L("im Kontrollbereich", "inside the control area"));
    if (v & 0x02) add(L("außerhalb des Kontrollbereichs", "outside the control area"));
    if (v & 0x04) add(L("im Ausnahmebereich des Herstellers", "inside the manufacturer carve-out area"));
    if (v & 0x08) add(L("Mangel aktiv", "deficiency active"));
    if (out.empty()) out = L("keine Angabe", "not reported");
    return out;
}

std::string regenerationText(double value) {
    return value != 0.0 ? L("läuft gerade", "in progress") : L("läuft nicht", "not running");
}

std::string regenerationTypeText(double value) {
    return value != 0.0 ? L("aktiv (vom Motor ausgelöst)", "active (triggered by the engine)")
                        : L("passiv (von selbst beim Fahren)", "passive (on its own while driving)");
}

std::string gearText(double value) {
    const int gear = static_cast<int>(value);
    if (gear == 0) return L("Leerlauf", "neutral");
    return L("Gang ", "gear ") + number(gear);
}

// Which oxygen sensors are fitted (PID 0x13 and 0x1D).
std::string o2PresentText(int mask, int banks, int perBank) {
    std::string out;
    for (int bank = 0; bank < banks; ++bank) {
        std::string sensors;
        for (int s = 0; s < perBank; ++s) {
            if (mask & (1 << (bank * perBank + s))) {
                if (!sensors.empty()) sensors += ", ";
                sensors += number(s + 1);
            }
        }
        if (sensors.empty()) continue;
        if (!out.empty()) out += "; ";
        out += L("Bank ", "bank ") + number(bank + 1) + L(": Sonde ", ": sensor ") + sensors;
    }
    if (out.empty()) out = L("keine", "none");
    return out;
}

// ---------------------------------------------------------------------------
// Building the table
// ---------------------------------------------------------------------------

// Names that are put together in loops need storage for the whole program
// run, because PidSpec only holds plain pointers. A deque never moves its
// elements, so the pointers stay valid.
const char* keep(std::string text) {
    static std::deque<std::string> pool;
    pool.push_back(std::move(text));
    return pool.back().c_str();
}

Text keepText(const std::string& de, const std::string& en) { return {keep(de), keep(en)}; }

// Description of one value, written with designated initializers below.
struct ValueDef {
    Text name;
    const char* unit = "";
    Field field;
    int bit = -1;               // support bit in byte A, -1 = always present
    double min = 0.0;           // gauge range
    double max = 0.0;
    double warnLow = 0.0;
    double warnHigh = 0.0;
    PidText text = {};
    PidAvailable available = {};  // custom presence check (instead of a bit)
    const char* unitEn = nullptr;  // only if the unit differs in English
};

class TableBuilder {
public:
    explicit TableBuilder(bool english) : en_(english) {}

    void add(int pid, int bytes, PidCategory category, Text title, std::vector<ValueDef> defs) {
        PidSpec s;
        s.pid = pid;
        s.bytes = bytes;
        s.category = category;
        int sub = 0;
        for (ValueDef& def : defs) {
            PidValueSpec v;
            v.sub = sub;
            v.key = liveKey(pid, sub);
            v.name = pick(def.name);
            v.unit = (en_ && def.unitEn) ? def.unitEn : def.unit;
            v.needBytes = def.field.need;
            v.calc = std::move(def.field.calc);
            v.text = std::move(def.text);
            if (def.available) {
                v.available = std::move(def.available);
            } else if (def.bit >= 0) {
                v.available = supportBit(def.bit);
            }
            v.displayMin = def.min;
            v.displayMax = def.max;
            v.warnLow = def.warnLow;
            v.warnHigh = def.warnHigh;
            s.values.push_back(std::move(v));
            ++sub;
        }
        const PidValueSpec& primary = s.values.front();
        s.name = primary.name;
        s.unit = primary.unit;
        s.calc = primary.calc;
        s.text = primary.text;
        s.warnLow = primary.warnLow;
        s.warnHigh = primary.warnHigh;
        s.displayMin = primary.displayMin;
        s.displayMax = primary.displayMax;
        s.title = title.de ? pick(title) : s.name;
        table_.push_back(std::move(s));
    }

    // Shorthand for the common single-value PID.
    void single(int pid, int bytes, PidCategory category, ValueDef def) {
        std::vector<ValueDef> defs;
        defs.push_back(std::move(def));
        add(pid, bytes, category, Text{nullptr, nullptr}, std::move(defs));
    }

    std::vector<PidSpec> take() { return std::move(table_); }

private:
    const char* pick(const Text& t) const { return en_ ? t.en : t.de; }

    bool en_;
    std::vector<PidSpec> table_;
};

constexpr Text kNoTitle{nullptr, nullptr};

// Position names of the oxygen sensors when PID 0x13 describes the layout
// (two banks with up to four sensors each), which almost every car uses.
// Sensor 1 sits before the catalytic converter, sensor 2 behind it.
std::string o2Position(int index, bool english) {
    const int bank = index / 4 + 1;
    const int sensor = index % 4 + 1;
    std::string out = (english ? "bank " : "Bank ") + number(bank) +
                      (english ? " sensor " : " Sonde ") + number(sensor);
    if (sensor == 1) out += english ? " (before catalyst)" : " (vor Kat)";
    if (sensor == 2) out += english ? " (after catalyst)" : " (hinter Kat)";
    return out;
}

// Builds the table for one language. Only names and units are picked here;
// status texts are functions and follow the language when they are called.
std::vector<PidSpec> buildTable(bool english) {
    using C = PidCategory;
    TableBuilder t(english);

    // --- 0x01 to 0x1F --------------------------------------------------------
    t.add(0x01, 4, C::Diagnostics, {"Fehlerlampe und Fehlerzahl", "MIL and number of codes"},
          {{.name = {"Motorkontrollleuchte", "Check engine light (MIL)"}, .field = bitOf(0, 7),
            .min = 0, .max = 1, .text = onOffText},
           // The count lives in the low seven bits, bit 7 is the lamp.
           {.name = {"Anzahl gespeicherter Fehlercodes", "Number of stored trouble codes"},
            .field = bitsOf(0, 0, 0x7F), .min = 0, .max = 20},
           {.name = {"Motorart laut Steuergerät", "Engine type reported by the ECU"},
            .field = bitOf(1, 3), .text = engineTypeText}});
    t.add(0x03, 2, C::Fuel, {"Gemischregelung", "Fuel system status"},
          {{.name = {"Gemischregelung", "Fuel system status"}, .field = raw8(0),
            .text = fuelStatusText},
           {.name = {"Gemischregelung, zweites System", "Fuel system 2 status"}, .field = raw8(1),
            .text = fuelStatusText,
            .available = [](const std::vector<int>& d) { return d.size() >= 2 && d[1] != 0; }}});
    t.single(0x04, 1, C::Engine,
             {.name = {"Motorlast berechnet", "Calculated engine load"}, .unit = "%",
              .field = percent(0), .min = 0, .max = 100});
    t.single(0x05, 1, C::Temperatures,
             {.name = {"Kühlmitteltemperatur", "Coolant temperature"}, .unit = "°C",
              .field = temp8(0), .min = -40, .max = 130, .warnLow = -10, .warnHigh = 110});
    const Text trims[4] = {{"Kurzzeit-Gemischkorrektur Bank 1", "Short-term fuel trim bank 1"},
                           {"Langzeit-Gemischkorrektur Bank 1", "Long-term fuel trim bank 1"},
                           {"Kurzzeit-Gemischkorrektur Bank 2", "Short-term fuel trim bank 2"},
                           {"Langzeit-Gemischkorrektur Bank 2", "Long-term fuel trim bank 2"}};
    for (int i = 0; i < 4; ++i) {
        t.single(0x06 + i, 1, C::Fuel,
                 {.name = trims[i], .unit = "%", .field = trim(0), .min = -30, .max = 30,
                  .warnLow = -20, .warnHigh = 20});
    }
    t.single(0x0A, 1, C::Fuel,
             {.name = {"Kraftstoffdruck", "Fuel pressure"}, .unit = "kPa",
              .field = scaled(0, 1, 3, 1), .min = 0, .max = 765});
    t.single(0x0B, 1, C::Air,
             {.name = {"Saugrohrdruck", "Intake manifold pressure"}, .unit = "kPa",
              .field = raw8(0), .min = 0, .max = 255});
    t.single(0x0C, 2, C::Engine,
             {.name = {"Motordrehzahl", "Engine speed"}, .unit = "1/min",
              .field = scaled(0, 2, 1, 4), .min = 0, .max = 8000, .warnLow = 0,
              .warnHigh = 6500});
    t.single(0x0D, 1, C::Vehicle,
             {.name = {"Geschwindigkeit", "Vehicle speed"}, .unit = "km/h", .field = raw8(0),
              .min = 0, .max = 250});
    t.single(0x0E, 1, C::Engine,
             {.name = {"Zündzeitpunkt", "Ignition timing advance"}, .unit = "Grad v.OT",
              .field = scaled(0, 1, 1, 2, -64), .min = -20, .max = 60, .unitEn = "deg BTDC"});
    t.single(0x0F, 1, C::Temperatures,
             {.name = {"Ansauglufttemperatur", "Intake air temperature"}, .unit = "°C",
              .field = temp8(0), .min = -40, .max = 100});
    t.single(0x10, 2, C::Air,
             {.name = {"Luftmasse (LMM)", "Mass air flow (MAF)"}, .unit = "g/s",
              .field = scaled(0, 2, 1, 100), .min = 0, .max = 300});
    t.single(0x11, 1, C::Air,
             {.name = {"Drosselklappenstellung", "Throttle position"}, .unit = "%",
              .field = percent(0), .min = 0, .max = 100});
    t.single(0x12, 1, C::Emissions,
             {.name = {"Sekundärluft", "Secondary air status"}, .field = raw8(0),
              .text = secondaryAirText});
    t.single(0x13, 1, C::Diagnostics,
             {.name = {"Verbaute Lambdasonden", "Oxygen sensors fitted"}, .field = raw8(0),
              .text = [](double v) { return o2PresentText(static_cast<int>(v), 2, 4); }});

    // Narrow-band oxygen sensors: voltage, and the short-term trim the sensor
    // feeds. B = 0xFF means the sensor is not used for the trim.
    for (int i = 0; i < 8; ++i) {
        const std::string de = "Lambdasonde " + o2Position(i, false);
        const std::string en = "O2 sensor " + o2Position(i, true);
        t.add(0x14 + i, 2, C::Exhaust, keepText(de, en),
              {{.name = keepText(de + ", Spannung", en + ", voltage"), .unit = "V",
                .field = scaled(0, 1, 1, 200), .min = 0, .max = 1.275},
               {.name = keepText(de + ", Gemischkorrektur", en + ", fuel trim"), .unit = "%",
                .field = trim(1), .min = -30, .max = 30,
                .available = [](const std::vector<int>& d) { return d.size() >= 2 && d[1] != 0xFF; }}});
    }

    t.single(0x1C, 1, C::Diagnostics,
             {.name = {"Abgasnorm des Steuergeräts", "OBD standard of the ECU"}, .field = raw8(0),
              .text = obdStandardText});
    t.single(0x1D, 1, C::Diagnostics,
             {.name = {"Verbaute Lambdasonden (4 Bänke)", "Oxygen sensors fitted (4 banks)"},
              .field = raw8(0),
              .text = [](double v) { return o2PresentText(static_cast<int>(v), 4, 2); }});
    t.single(0x1E, 1, C::Vehicle,
             {.name = {"Nebenantrieb (PTO)", "Power take-off (PTO)"}, .field = bitOf(0, 0),
              .text = activeText});
    t.single(0x1F, 2, C::Vehicle,
             {.name = {"Laufzeit seit Motorstart", "Run time since engine start"}, .unit = "s",
              .field = raw16(0), .min = 0, .max = 7200});

    // --- 0x21 to 0x3F --------------------------------------------------------
    t.single(0x21, 2, C::Diagnostics,
             {.name = {"Strecke mit Fehlerlampe an", "Distance with MIL on"}, .unit = "km",
              .field = raw16(0), .min = 0, .max = 65535});
    t.single(0x22, 2, C::Fuel,
             {.name = {"Raildruck relativ zum Saugrohr", "Fuel rail pressure (relative to manifold)"},
              .unit = "kPa", .field = scaled(0, 2, 0.079, 1), .min = 0, .max = 5177.265});
    t.single(0x23, 2, C::Fuel,
             {.name = {"Raildruck", "Fuel rail pressure"}, .unit = "kPa",
              .field = scaled(0, 2, 10, 1), .min = 0, .max = 250000});

    // Wide-band oxygen sensors: lambda together with voltage (0x24 to 0x2B)
    // or with pump current (0x34 to 0x3B).
    for (int i = 0; i < 8; ++i) {
        const std::string de = "Breitbandsonde " + o2Position(i, false);
        const std::string en = "Wide-band O2 sensor " + o2Position(i, true);
        t.add(0x24 + i, 4, C::Exhaust, keepText(de, en),
              {{.name = keepText(de + ", Lambda", en + ", lambda"), .field = lambda(0), .min = 0,
                .max = 2},
               {.name = keepText(de + ", Spannung", en + ", voltage"), .unit = "V",
                .field = scaled(2, 2, 8, 65536), .min = 0, .max = 8}});
    }
    t.single(0x2C, 1, C::Emissions,
             {.name = {"Abgasrückführung angefordert", "Commanded EGR"}, .unit = "%",
              .field = percent(0), .min = 0, .max = 100});
    t.single(0x2D, 1, C::Emissions,
             {.name = {"Abgasrückführung Regelabweichung", "EGR error"}, .unit = "%",
              .field = trim(0), .min = -100, .max = 100, .warnLow = -20, .warnHigh = 20});
    t.single(0x2E, 1, C::Emissions,
             {.name = {"Tankentlüftung angefordert", "Commanded evaporative purge"}, .unit = "%",
              .field = percent(0), .min = 0, .max = 100});
    t.single(0x2F, 1, C::Fuel,
             {.name = {"Tankfüllstand", "Fuel tank level"}, .unit = "%", .field = percent(0),
              .min = 0, .max = 100, .warnLow = 10, .warnHigh = 101});
    t.single(0x30, 1, C::Diagnostics,
             {.name = {"Warmläufe seit Löschen", "Warm-ups since codes cleared"},
              .field = raw8(0), .min = 0, .max = 255});
    t.single(0x31, 2, C::Diagnostics,
             {.name = {"Strecke seit Löschen", "Distance since codes cleared"}, .unit = "km",
              .field = raw16(0), .min = 0, .max = 65535});
    t.single(0x32, 2, C::Emissions,
             {.name = {"Tankdruck", "Fuel tank vapor pressure"}, .unit = "Pa",
              .field = scaled(0, 2, 1, 4, 0, true), .min = -8192, .max = 8192});
    t.single(0x33, 1, C::Air,
             {.name = {"Luftdruck (barometrisch)", "Barometric pressure"}, .unit = "kPa",
              .field = raw8(0), .min = 50, .max = 110});
    for (int i = 0; i < 8; ++i) {
        const std::string de = "Breitbandsonde " + o2Position(i, false);
        const std::string en = "Wide-band O2 sensor " + o2Position(i, true);
        t.add(0x34 + i, 4, C::Exhaust, keepText(de + " (Strom)", en + " (current)"),
              {{.name = keepText(de + ", Lambda", en + ", lambda"), .field = lambda(0), .min = 0,
                .max = 2},
               // Signed with offset 0x8000, 1/256 mA per bit.
               {.name = keepText(de + ", Pumpstrom", en + ", current"), .unit = "mA",
                .field = scaled(2, 2, 1, 256, -128), .min = -5, .max = 5}});
    }
    const Text cats[4] = {{"Katalysatortemperatur Bank 1 Sonde 1", "Catalyst temperature bank 1 sensor 1"},
                          {"Katalysatortemperatur Bank 2 Sonde 1", "Catalyst temperature bank 2 sensor 1"},
                          {"Katalysatortemperatur Bank 1 Sonde 2", "Catalyst temperature bank 1 sensor 2"},
                          {"Katalysatortemperatur Bank 2 Sonde 2", "Catalyst temperature bank 2 sensor 2"}};
    for (int i = 0; i < 4; ++i) {
        t.single(0x3C + i, 2, C::Exhaust,
                 {.name = cats[i], .unit = "°C", .field = temp16(0), .min = 0, .max = 1000,
                  .warnLow = -50, .warnHigh = 900});
    }

    // --- 0x41 to 0x5F --------------------------------------------------------
    t.single(0x42, 2, C::Electrical,
             {.name = {"Spannung Steuergerät", "Control unit voltage"}, .unit = "V",
              .field = scaled(0, 2, 1, 1000), .min = 8, .max = 16, .warnLow = 11.5,
              .warnHigh = 15.0});
    t.single(0x43, 2, C::Engine,
             {.name = {"Absolute Last", "Absolute load"}, .unit = "%",
              .field = scaled(0, 2, 100, 255), .min = 0, .max = 200});
    t.single(0x44, 2, C::Fuel,
             {.name = {"Lambda angefordert", "Commanded lambda"}, .field = lambda(0), .min = 0,
              .max = 2});
    t.single(0x45, 1, C::Air,
             {.name = {"Relative Drosselklappenstellung", "Relative throttle position"},
              .unit = "%", .field = percent(0), .min = 0, .max = 100});
    t.single(0x46, 1, C::Temperatures,
             {.name = {"Umgebungstemperatur", "Ambient air temperature"}, .unit = "°C",
              .field = temp8(0), .min = -40, .max = 60});
    const Text positions[6] = {{"Drosselklappe absolut B", "Absolute throttle position B"},
                               {"Drosselklappe absolut C", "Absolute throttle position C"},
                               {"Fahrpedalstellung D", "Accelerator pedal position D"},
                               {"Fahrpedalstellung E", "Accelerator pedal position E"},
                               {"Fahrpedalstellung F", "Accelerator pedal position F"},
                               {"Drosselklappen-Ansteuerung", "Commanded throttle actuator"}};
    for (int i = 0; i < 6; ++i) {
        t.single(0x47 + i, 1, i >= 2 && i <= 4 ? C::Engine : C::Air,
                 {.name = positions[i], .unit = "%", .field = percent(0), .min = 0, .max = 100});
    }
    t.single(0x4D, 2, C::Diagnostics,
             {.name = {"Laufzeit mit Fehlerlampe an", "Run time with MIL on"}, .unit = "min",
              .field = raw16(0), .min = 0, .max = 65535});
    t.single(0x4E, 2, C::Diagnostics,
             {.name = {"Zeit seit dem Löschen", "Time since codes cleared"}, .unit = "min",
              .field = raw16(0), .min = 0, .max = 65535});
    // Scaling limits the control unit uses for other PIDs. 0 means the
    // standard scaling applies.
    t.add(0x4F, 4, C::Diagnostics, {"Skalierungsgrenzen", "Maximum scaling values"},
          {{.name = {"Höchstwert Lambda", "Maximum lambda"}, .field = raw8(0), .min = 0, .max = 255},
           {.name = {"Höchstwert Lambdasondenspannung", "Maximum O2 sensor voltage"}, .unit = "V",
            .field = raw8(1), .min = 0, .max = 255},
           {.name = {"Höchstwert Lambdasondenstrom", "Maximum O2 sensor current"}, .unit = "mA",
            .field = raw8(2), .min = 0, .max = 255},
           {.name = {"Höchstwert Saugrohrdruck", "Maximum intake manifold pressure"},
            .unit = "kPa", .field = scaled(3, 1, 10, 1), .min = 0, .max = 2550}});
    t.single(0x50, 4, C::Diagnostics,
             {.name = {"Höchstwert Luftmasse", "Maximum mass air flow"}, .unit = "g/s",
              .field = scaled(0, 1, 10, 1), .min = 0, .max = 2550});
    t.single(0x51, 1, C::Fuel,
             {.name = {"Kraftstoffart", "Fuel type"}, .field = raw8(0), .text = fuelTypeText});
    t.single(0x52, 1, C::Fuel,
             {.name = {"Ethanolanteil", "Ethanol content"}, .unit = "%", .field = percent(0),
              .min = 0, .max = 100});
    t.single(0x53, 2, C::Emissions,
             {.name = {"Tankdruck absolut", "Absolute fuel tank vapor pressure"}, .unit = "kPa",
              .field = scaled(0, 2, 1, 200), .min = 0, .max = 327.675});
    t.single(0x54, 2, C::Emissions,
             {.name = {"Tankdruck (großer Bereich)", "Fuel tank vapor pressure (wide range)"},
              .unit = "Pa", .field = scaled(0, 2, 1, 1, 0, true), .min = -32768, .max = 32767});
    const Text secondary[4][2] = {
        {{"Kurzzeit-Korrektur hintere Lambdasonde Bank 1", "Short-term secondary O2 trim bank 1"},
         {"Kurzzeit-Korrektur hintere Lambdasonde Bank 3", "Short-term secondary O2 trim bank 3"}},
        {{"Langzeit-Korrektur hintere Lambdasonde Bank 1", "Long-term secondary O2 trim bank 1"},
         {"Langzeit-Korrektur hintere Lambdasonde Bank 3", "Long-term secondary O2 trim bank 3"}},
        {{"Kurzzeit-Korrektur hintere Lambdasonde Bank 2", "Short-term secondary O2 trim bank 2"},
         {"Kurzzeit-Korrektur hintere Lambdasonde Bank 4", "Short-term secondary O2 trim bank 4"}},
        {{"Langzeit-Korrektur hintere Lambdasonde Bank 2", "Long-term secondary O2 trim bank 2"},
         {"Langzeit-Korrektur hintere Lambdasonde Bank 4", "Long-term secondary O2 trim bank 4"}}};
    for (int i = 0; i < 4; ++i) {
        t.add(0x55 + i, 2, C::Fuel, kNoTitle,
              {{.name = secondary[i][0], .unit = "%", .field = trim(0), .min = -30, .max = 30,
                .warnLow = -20, .warnHigh = 20},
               {.name = secondary[i][1], .unit = "%", .field = trim(1), .min = -30, .max = 30,
                .warnLow = -20, .warnHigh = 20}});
    }
    t.single(0x59, 2, C::Fuel,
             {.name = {"Raildruck absolut", "Fuel rail pressure (absolute)"}, .unit = "kPa",
              .field = scaled(0, 2, 10, 1), .min = 0, .max = 250000});
    t.single(0x5A, 1, C::Engine,
             {.name = {"Fahrpedalstellung relativ", "Relative accelerator pedal position"},
              .unit = "%", .field = percent(0), .min = 0, .max = 100});
    t.single(0x5B, 1, C::HybridEv,
             {.name = {"Restkapazität Hybridbatterie", "Hybrid battery remaining life"},
              .unit = "%", .field = percent(0), .min = 0, .max = 100});
    t.single(0x5C, 1, C::Temperatures,
             {.name = {"Öltemperatur", "Engine oil temperature"}, .unit = "°C", .field = temp8(0),
              .min = -40, .max = 150, .warnLow = -10, .warnHigh = 130});
    t.single(0x5D, 2, C::Fuel,
             {.name = {"Einspritzbeginn", "Fuel injection timing"}, .unit = "Grad v.OT",
              .field = scaled(0, 2, 1, 128, -210), .min = -30, .max = 60, .unitEn = "deg BTDC"});
    t.single(0x5E, 2, C::Fuel,
             {.name = {"Kraftstoffverbrauch", "Engine fuel rate"}, .unit = "l/h",
              .field = scaled(0, 2, 1, 20), .min = 0, .max = 30});
    t.single(0x5F, 1, C::Diagnostics,
             {.name = {"Ausgelegt für Abgasvorschrift", "Emission requirements designed for"},
              .field = raw8(0), .text = emissionDesignText});

    // --- 0x61 to 0x7F --------------------------------------------------------
    t.single(0x61, 1, C::Engine,
             {.name = {"Drehmoment angefordert", "Driver's demand engine torque"}, .unit = "%",
              .field = torque(0), .min = -25, .max = 100});
    t.single(0x62, 1, C::Engine,
             {.name = {"Drehmoment tatsächlich", "Actual engine torque"}, .unit = "%",
              .field = torque(0), .min = -25, .max = 100});
    t.single(0x63, 2, C::Engine,
             {.name = {"Bezugsdrehmoment des Motors", "Engine reference torque"}, .unit = "Nm",
              .field = raw16(0), .min = 0, .max = 1000});
    t.add(0x64, 5, C::Engine, {"Drehmomentkennlinie", "Engine percent torque data"},
          {{.name = {"Drehmoment im Leerlauf", "Torque at idle"}, .unit = "%", .field = torque(0),
            .min = -25, .max = 100},
           {.name = {"Drehmoment Kennlinienpunkt 1", "Torque at engine point 1"}, .unit = "%",
            .field = torque(1), .min = -25, .max = 100},
           {.name = {"Drehmoment Kennlinienpunkt 2", "Torque at engine point 2"}, .unit = "%",
            .field = torque(2), .min = -25, .max = 100},
           {.name = {"Drehmoment Kennlinienpunkt 3", "Torque at engine point 3"}, .unit = "%",
            .field = torque(3), .min = -25, .max = 100},
           {.name = {"Drehmoment Kennlinienpunkt 4", "Torque at engine point 4"}, .unit = "%",
            .field = torque(4), .min = -25, .max = 100}});
    t.add(0x65, 2, C::Vehicle, {"Zusätzliche Ein- und Ausgänge", "Auxiliary inputs and outputs"},
          {{.name = {"Nebenantrieb (PTO)", "Power take-off (PTO)"}, .field = bitOf(1, 0),
            .bit = 0, .text = activeText},
           {.name = {"Automatikgetriebe, Wählhebel", "Automatic transmission range"},
            .field = bitOf(1, 1), .bit = 1, .text = automaticRangeText},
           {.name = {"Schaltgetriebe, Gang", "Manual transmission gear"}, .field = bitOf(1, 2),
            .bit = 2, .text = manualGearText},
           {.name = {"Vorglühlampe", "Glow plug lamp"}, .field = bitOf(1, 3), .bit = 3,
            .text = onOffText}});
    t.add(0x66, 5, C::Air, {"Luftmassenmesser A und B", "Mass air flow sensors A and B"},
          {{.name = {"Luftmasse Sensor A", "Mass air flow sensor A"}, .unit = "g/s",
            .field = scaled(1, 2, 1, 32), .bit = 0, .min = 0, .max = 300},
           {.name = {"Luftmasse Sensor B", "Mass air flow sensor B"}, .unit = "g/s",
            .field = scaled(3, 2, 1, 32), .bit = 1, .min = 0, .max = 300}});
    t.add(0x67, 3, C::Temperatures, {"Kühlmitteltemperatur (Sensoren)", "Coolant temperature sensors"},
          {{.name = {"Kühlmitteltemperatur Sensor 1", "Coolant temperature sensor 1"}, .unit = "°C",
            .field = temp8(1), .bit = 0, .min = -40, .max = 130, .warnLow = -10, .warnHigh = 110},
           {.name = {"Kühlmitteltemperatur Sensor 2", "Coolant temperature sensor 2"}, .unit = "°C",
            .field = temp8(2), .bit = 1, .min = -40, .max = 130, .warnLow = -10,
            .warnHigh = 110}});
    // Older control units send three bytes (two sensors), newer ones seven.
    {
        const char* posDe[6] = {"Bank 1 Sensor 1", "Bank 1 Sensor 2", "Bank 1 Sensor 3",
                                "Bank 2 Sensor 1", "Bank 2 Sensor 2", "Bank 2 Sensor 3"};
        const char* posEn[6] = {"bank 1 sensor 1", "bank 1 sensor 2", "bank 1 sensor 3",
                                "bank 2 sensor 1", "bank 2 sensor 2", "bank 2 sensor 3"};
        std::vector<ValueDef> defs;
        for (int i = 0; i < 6; ++i) {
            defs.push_back({.name = keepText(std::string("Ansauglufttemperatur ") + posDe[i],
                                             std::string("Intake air temperature ") + posEn[i]),
                            .unit = "°C", .field = temp8(1 + i), .bit = i, .min = -40, .max = 100});
        }
        t.add(0x68, 7, C::Temperatures,
              {"Ansauglufttemperatur (Sensoren)", "Intake air temperature sensors"}, std::move(defs));
    }
    t.add(0x69, 7, C::Emissions, {"Abgasrückführung Soll und Ist", "EGR commanded and actual"},
          {{.name = {"Abgasrückführung A angefordert", "Commanded EGR A"}, .unit = "%",
            .field = percent(1), .bit = 0, .min = 0, .max = 100},
           {.name = {"Abgasrückführung A tatsächlich", "Actual EGR A"}, .unit = "%",
            .field = percent(2), .bit = 1, .min = 0, .max = 100},
           {.name = {"Abgasrückführung A Regelabweichung", "EGR A error"}, .unit = "%",
            .field = trim(3), .bit = 2, .min = -100, .max = 100, .warnLow = -20, .warnHigh = 20},
           {.name = {"Abgasrückführung B angefordert", "Commanded EGR B"}, .unit = "%",
            .field = percent(4), .bit = 3, .min = 0, .max = 100},
           {.name = {"Abgasrückführung B tatsächlich", "Actual EGR B"}, .unit = "%",
            .field = percent(5), .bit = 4, .min = 0, .max = 100},
           {.name = {"Abgasrückführung B Regelabweichung", "EGR B error"}, .unit = "%",
            .field = trim(6), .bit = 5, .min = -100, .max = 100, .warnLow = -20,
            .warnHigh = 20}});
    t.add(0x6A, 5, C::Air, {"Ansaugluftklappe (Diesel)", "Diesel intake air flow control"},
          {{.name = {"Ansaugluftklappe A angefordert", "Commanded intake air flow A"}, .unit = "%",
            .field = percent(1), .bit = 0, .min = 0, .max = 100},
           {.name = {"Ansaugluftklappe A Stellung", "Intake air flow position A"}, .unit = "%",
            .field = percent(2), .bit = 1, .min = 0, .max = 100},
           {.name = {"Ansaugluftklappe B angefordert", "Commanded intake air flow B"}, .unit = "%",
            .field = percent(3), .bit = 2, .min = 0, .max = 100},
           {.name = {"Ansaugluftklappe B Stellung", "Intake air flow position B"}, .unit = "%",
            .field = percent(4), .bit = 3, .min = 0, .max = 100}});
    t.add(0x6B, 5, C::Emissions, {"Temperatur Abgasrückführung", "EGR temperature"},
          {{.name = {"Temperatur Abgasrückführung Bank 1 Sensor 1", "EGR temperature bank 1 sensor 1"},
            .unit = "°C", .field = temp8Wide(1, 4), .bit = 0, .min = -40, .max = 600},
           {.name = {"Temperatur Abgasrückführung Bank 1 Sensor 2", "EGR temperature bank 1 sensor 2"},
            .unit = "°C", .field = temp8Wide(2, 5), .bit = 1, .min = -40, .max = 600},
           {.name = {"Temperatur Abgasrückführung Bank 2 Sensor 1", "EGR temperature bank 2 sensor 1"},
            .unit = "°C", .field = temp8Wide(3, 6), .bit = 2, .min = -40, .max = 600},
           {.name = {"Temperatur Abgasrückführung Bank 2 Sensor 2", "EGR temperature bank 2 sensor 2"},
            .unit = "°C", .field = temp8Wide(4, 7), .bit = 3, .min = -40, .max = 600}});
    t.add(0x6C, 5, C::Air, {"Drosselklappen-Ansteuerung A und B", "Throttle actuator A and B"},
          {{.name = {"Drosselklappe A angefordert", "Commanded throttle actuator A"}, .unit = "%",
            .field = percent(1), .bit = 0, .min = 0, .max = 100},
           {.name = {"Drosselklappe A Stellung relativ", "Relative throttle position A"},
            .unit = "%", .field = percent(2), .bit = 1, .min = 0, .max = 100},
           {.name = {"Drosselklappe B angefordert", "Commanded throttle actuator B"}, .unit = "%",
            .field = percent(3), .bit = 2, .min = 0, .max = 100},
           {.name = {"Drosselklappe B Stellung relativ", "Relative throttle position B"},
            .unit = "%", .field = percent(4), .bit = 3, .min = 0, .max = 100}});
    t.add(0x6D, 11, C::Fuel, {"Kraftstoffdruckregelung", "Fuel pressure control system"},
          {{.name = {"Raildruck A angefordert", "Commanded fuel rail pressure A"}, .unit = "kPa",
            .field = scaled(1, 2, 10, 1), .bit = 0, .min = 0, .max = 250000},
           {.name = {"Raildruck A", "Fuel rail pressure A"}, .unit = "kPa",
            .field = scaled(3, 2, 10, 1), .bit = 1, .min = 0, .max = 250000},
           {.name = {"Kraftstofftemperatur A", "Fuel temperature A"}, .unit = "°C",
            .field = temp8(5), .bit = 2, .min = -40, .max = 120},
           {.name = {"Raildruck B angefordert", "Commanded fuel rail pressure B"}, .unit = "kPa",
            .field = scaled(6, 2, 10, 1), .bit = 3, .min = 0, .max = 250000},
           {.name = {"Raildruck B", "Fuel rail pressure B"}, .unit = "kPa",
            .field = scaled(8, 2, 10, 1), .bit = 4, .min = 0, .max = 250000},
           {.name = {"Kraftstofftemperatur B", "Fuel temperature B"}, .unit = "°C",
            .field = temp8(10), .bit = 5, .min = -40, .max = 120}});
    t.add(0x6E, 9, C::Fuel, {"Einspritzdruckregelung", "Injection pressure control system"},
          {{.name = {"Einspritzdruck A angefordert", "Commanded injection control pressure A"},
            .unit = "kPa", .field = scaled(1, 2, 10, 1), .bit = 0, .min = 0, .max = 250000},
           {.name = {"Einspritzdruck A", "Injection control pressure A"}, .unit = "kPa",
            .field = scaled(3, 2, 10, 1), .bit = 1, .min = 0, .max = 250000},
           {.name = {"Einspritzdruck B angefordert", "Commanded injection control pressure B"},
            .unit = "kPa", .field = scaled(5, 2, 10, 1), .bit = 2, .min = 0, .max = 250000},
           {.name = {"Einspritzdruck B", "Injection control pressure B"}, .unit = "kPa",
            .field = scaled(7, 2, 10, 1), .bit = 3, .min = 0, .max = 250000}});
    t.add(0x6F, 3, C::Air, {"Druck vor dem Turbolader", "Turbocharger compressor inlet pressure"},
          {{.name = {"Druck vor Turbolader A", "Compressor inlet pressure A"}, .unit = "kPa",
            .field = raw8(1), .bit = 0, .min = 0, .max = 255},
           {.name = {"Druck vor Turbolader B", "Compressor inlet pressure B"}, .unit = "kPa",
            .field = raw8(2), .bit = 1, .min = 0, .max = 255}});
    t.add(0x70, 10, C::Air, {"Ladedruckregelung", "Boost pressure control"},
          {{.name = {"Ladedruck A angefordert", "Commanded boost pressure A"}, .unit = "kPa",
            .field = scaled(1, 2, 1, 32), .bit = 0, .min = 0, .max = 300},
           {.name = {"Ladedruck A", "Boost pressure A"}, .unit = "kPa",
            .field = scaled(3, 2, 1, 32), .bit = 1, .min = 0, .max = 300},
           {.name = {"Ladedruck B angefordert", "Commanded boost pressure B"}, .unit = "kPa",
            .field = scaled(5, 2, 1, 32), .bit = 2, .min = 0, .max = 300},
           {.name = {"Ladedruck B", "Boost pressure B"}, .unit = "kPa",
            .field = scaled(7, 2, 1, 32), .bit = 3, .min = 0, .max = 300},
           {.name = {"Ladedruckregelung A", "Boost pressure control A"}, .field = bitsOf(9, 0, 3),
            .bit = 4, .text = controlLoopText},
           {.name = {"Ladedruckregelung B", "Boost pressure control B"}, .field = bitsOf(9, 2, 3),
            .bit = 5, .text = controlLoopText}});
    t.add(0x71, 6, C::Air, {"Verstellbarer Turbolader (VTG)", "Variable geometry turbo control"},
          {{.name = {"Turbolader-Leitschaufeln A angefordert", "Commanded VGT A position"},
            .unit = "%", .field = percent(1), .bit = 0, .min = 0, .max = 100},
           {.name = {"Turbolader-Leitschaufeln A Stellung", "VGT A position"}, .unit = "%",
            .field = percent(2), .bit = 1, .min = 0, .max = 100},
           {.name = {"Turbolader-Leitschaufeln B angefordert", "Commanded VGT B position"},
            .unit = "%", .field = percent(3), .bit = 2, .min = 0, .max = 100},
           {.name = {"Turbolader-Leitschaufeln B Stellung", "VGT B position"}, .unit = "%",
            .field = percent(4), .bit = 3, .min = 0, .max = 100},
           {.name = {"Turbolader-Regelung A", "VGT A control"}, .field = bitsOf(5, 0, 3), .bit = 4,
            .text = controlLoopText},
           {.name = {"Turbolader-Regelung B", "VGT B control"}, .field = bitsOf(5, 2, 3), .bit = 5,
            .text = controlLoopText}});
    t.add(0x72, 5, C::Air, {"Wastegate (Ladedruckventil)", "Wastegate control"},
          {{.name = {"Wastegate A angefordert", "Commanded wastegate A"}, .unit = "%",
            .field = percent(1), .bit = 0, .min = 0, .max = 100},
           {.name = {"Wastegate A Stellung", "Wastegate A position"}, .unit = "%",
            .field = percent(2), .bit = 1, .min = 0, .max = 100},
           {.name = {"Wastegate B angefordert", "Commanded wastegate B"}, .unit = "%",
            .field = percent(3), .bit = 2, .min = 0, .max = 100},
           {.name = {"Wastegate B Stellung", "Wastegate B position"}, .unit = "%",
            .field = percent(4), .bit = 3, .min = 0, .max = 100}});
    t.add(0x73, 5, C::Exhaust, {"Abgasdruck", "Exhaust pressure"},
          {{.name = {"Abgasdruck Bank 1", "Exhaust pressure bank 1"}, .unit = "kPa",
            .field = scaled(1, 2, 1, 100), .bit = 0, .min = 0, .max = 300},
           {.name = {"Abgasdruck Bank 2", "Exhaust pressure bank 2"}, .unit = "kPa",
            .field = scaled(3, 2, 1, 100), .bit = 1, .min = 0, .max = 300}});
    t.add(0x74, 5, C::Air, {"Turboladerdrehzahl", "Turbocharger speed"},
          {{.name = {"Drehzahl Turbolader A", "Turbocharger A speed"}, .unit = "1/min",
            .field = scaled(1, 2, 10, 1), .bit = 0, .min = 0, .max = 250000},
           {.name = {"Drehzahl Turbolader B", "Turbocharger B speed"}, .unit = "1/min",
            .field = scaled(3, 2, 10, 1), .bit = 1, .min = 0, .max = 250000}});
    for (int i = 0; i < 2; ++i) {
        const std::string tc = i == 0 ? "A" : "B";
        t.add(0x75 + i, 7, C::Air,
              keepText("Turboladertemperaturen " + tc, "Turbocharger " + tc + " temperatures"),
              {{.name = keepText("Turbolader " + tc + ", Luft vor Verdichter",
                                 "Turbocharger " + tc + " compressor inlet temperature"),
                .unit = "°C", .field = temp8(1), .bit = 0, .min = -40, .max = 100},
               {.name = keepText("Turbolader " + tc + ", Luft nach Verdichter",
                                 "Turbocharger " + tc + " compressor outlet temperature"),
                .unit = "°C", .field = temp8(2), .bit = 1, .min = -40, .max = 250},
               {.name = keepText("Turbolader " + tc + ", Abgas vor Turbine",
                                 "Turbocharger " + tc + " turbine inlet temperature"),
                .unit = "°C", .field = temp16(3), .bit = 2, .min = 0, .max = 1000,
                .warnLow = -50, .warnHigh = 950},
               {.name = keepText("Turbolader " + tc + ", Abgas nach Turbine",
                                 "Turbocharger " + tc + " turbine outlet temperature"),
                .unit = "°C", .field = temp16(5), .bit = 3, .min = 0, .max = 1000}});
    }
    t.add(0x77, 5, C::Temperatures, {"Ladeluftkühler", "Charge air cooler temperature"},
          {{.name = {"Ladelufttemperatur Bank 1 Sensor 1", "Charge air temperature bank 1 sensor 1"},
            .unit = "°C", .field = temp8Wide(1, 4), .bit = 0, .min = -40, .max = 150},
           {.name = {"Ladelufttemperatur Bank 1 Sensor 2", "Charge air temperature bank 1 sensor 2"},
            .unit = "°C", .field = temp8Wide(2, 5), .bit = 1, .min = -40, .max = 150},
           {.name = {"Ladelufttemperatur Bank 2 Sensor 1", "Charge air temperature bank 2 sensor 1"},
            .unit = "°C", .field = temp8Wide(3, 6), .bit = 2, .min = -40, .max = 150},
           {.name = {"Ladelufttemperatur Bank 2 Sensor 2", "Charge air temperature bank 2 sensor 2"},
            .unit = "°C", .field = temp8Wide(4, 7), .bit = 3, .min = -40, .max = 150}});
    // Exhaust gas temperatures: 0x78/0x79 sensors 1 to 4, 0x98/0x99 sensors
    // 5 to 8, each with 0.1 C per bit and four support bits in byte A.
    auto addEgt = [&t](int pid, int bank, int firstSensor) {
        std::vector<ValueDef> defs;
        for (int s = 0; s < 4; ++s) {
            defs.push_back(
                {.name = keepText("Abgastemperatur Bank " + std::to_string(bank) + " Sensor " +
                                      std::to_string(firstSensor + s),
                                  "Exhaust gas temperature bank " + std::to_string(bank) +
                                      " sensor " + std::to_string(firstSensor + s)),
                 .unit = "°C", .field = temp16(1 + 2 * s), .bit = s, .min = 0, .max = 1000,
                 .warnLow = -50, .warnHigh = 950});
        }
        t.add(pid, 9, C::Exhaust,
              keepText("Abgastemperatur Bank " + std::to_string(bank),
                       "Exhaust gas temperature bank " + std::to_string(bank)),
              std::move(defs));
    };
    addEgt(0x78, 1, 1);
    addEgt(0x79, 2, 1);
    for (int bank = 1; bank <= 2; ++bank) {
        const std::string b = std::to_string(bank);
        t.add(0x79 + bank, 7, C::Emissions,
              keepText("Partikelfilter Bank " + b + ", Drücke", "Particulate filter bank " + b + " pressures"),
              {// Signed, 0.01 kPa per bit: a clogged filter shows a high value.
               {.name = keepText("Partikelfilter Bank " + b + ", Differenzdruck",
                                 "Particulate filter bank " + b + ", differential pressure"),
                .unit = "kPa", .field = scaled(1, 2, 1, 100, 0, true), .bit = 0, .min = 0,
                .max = 50},
               {.name = keepText("Partikelfilter Bank " + b + ", Druck davor",
                                 "Particulate filter bank " + b + ", inlet pressure"),
                .unit = "kPa", .field = scaled(3, 2, 1, 100), .bit = 1, .min = 0, .max = 300},
               {.name = keepText("Partikelfilter Bank " + b + ", Druck danach",
                                 "Particulate filter bank " + b + ", outlet pressure"),
                .unit = "kPa", .field = scaled(5, 2, 1, 100), .bit = 2, .min = 0, .max = 300}});
    }
    t.add(0x7C, 9, C::Emissions, {"Partikelfilter, Temperaturen", "Particulate filter temperatures"},
          {{.name = {"Partikelfilter Bank 1, Temperatur davor", "Particulate filter bank 1 inlet temperature"},
            .unit = "°C", .field = temp16(1), .bit = 0, .min = 0, .max = 1000},
           {.name = {"Partikelfilter Bank 1, Temperatur danach", "Particulate filter bank 1 outlet temperature"},
            .unit = "°C", .field = temp16(3), .bit = 1, .min = 0, .max = 1000},
           {.name = {"Partikelfilter Bank 2, Temperatur davor", "Particulate filter bank 2 inlet temperature"},
            .unit = "°C", .field = temp16(5), .bit = 2, .min = 0, .max = 1000},
           {.name = {"Partikelfilter Bank 2, Temperatur danach", "Particulate filter bank 2 outlet temperature"},
            .unit = "°C", .field = temp16(7), .bit = 3, .min = 0, .max = 1000}});
    t.single(0x7D, 1, C::Emissions,
             {.name = {"Stickoxid-Kontrollbereich (NTE)", "NOx NTE control area status"},
              .field = raw8(0), .text = nteText});
    t.single(0x7E, 1, C::Emissions,
             {.name = {"Partikel-Kontrollbereich (NTE)", "PM NTE control area status"},
              .field = raw8(0), .text = nteText});
    // Run time counters. The control unit counts seconds; hours read better.
    t.add(0x7F, 13, C::Vehicle, {"Motorlaufzeit gesamt", "Engine run time"},
          {{.name = {"Motorlaufzeit gesamt", "Total engine run time"}, .unit = "h",
            .field = scaled(1, 4, 1, 3600), .bit = 0, .min = 0, .max = 20000},
           {.name = {"Motorlaufzeit im Leerlauf", "Total idle run time"}, .unit = "h",
            .field = scaled(5, 4, 1, 3600), .bit = 1, .min = 0, .max = 20000},
           {.name = {"Laufzeit mit Nebenantrieb", "Total run time with PTO active"}, .unit = "h",
            .field = scaled(9, 4, 1, 3600), .bit = 2, .min = 0, .max = 20000}});

    // --- 0x81 to 0x9F --------------------------------------------------------
    // Run time of auxiliary emission control devices (AECD), two timers each.
    auto addAecd = [&t](int pid, int first) {
        std::vector<ValueDef> defs;
        for (int i = 0; i < 5; ++i) {
            for (int timer = 1; timer <= 2; ++timer) {
                const std::string n = std::to_string(first + i);
                const std::string tn = std::to_string(timer);
                defs.push_back({.name = keepText("Abgas-Sonderstrategie (AECD) " + n + ", Zeit " + tn,
                                                 "Emission control device (AECD) " + n + ", timer " + tn),
                                .unit = "s", .field = raw32(1 + 8 * i + 4 * (timer - 1)), .bit = i,
                                .min = 0, .max = 1e6});
            }
        }
        t.add(pid, 41, C::Diagnostics,
              keepText("Laufzeit Abgas-Sonderstrategien " + std::to_string(first) + " bis " +
                           std::to_string(first + 4),
                       "Run time of AECD " + std::to_string(first) + " to " + std::to_string(first + 4)),
              std::move(defs));
    };
    addAecd(0x81, 1);
    addAecd(0x82, 6);
    auto addNox = [&t](int pid, bool corrected) {
        const char* posDe[4] = {"Bank 1 Sensor 1", "Bank 1 Sensor 2", "Bank 2 Sensor 1", "Bank 2 Sensor 2"};
        const char* posEn[4] = {"bank 1 sensor 1", "bank 1 sensor 2", "bank 2 sensor 1", "bank 2 sensor 2"};
        const std::string de = corrected ? "Stickoxide korrigiert " : "Stickoxide ";
        const std::string en = corrected ? "NOx corrected " : "NOx ";
        std::vector<ValueDef> defs;
        for (int i = 0; i < 4; ++i) {
            defs.push_back({.name = keepText(de + posDe[i], en + posEn[i]), .unit = "ppm",
                            .field = raw16(1 + 2 * i), .bit = i, .min = 0, .max = 3000});
        }
        t.add(pid, 9, C::Emissions,
              corrected ? Text{"Stickoxidsensoren, korrigiert", "NOx sensors, corrected"}
                        : Text{"Stickoxidsensoren", "NOx sensors"},
              std::move(defs));
    };
    addNox(0x83, false);
    t.single(0x84, 1, C::Temperatures,
             {.name = {"Oberflächentemperatur Krümmer", "Manifold surface temperature"},
              .unit = "°C", .field = temp8(0), .min = -40, .max = 215});
    t.add(0x85, 10, C::Emissions, {"AdBlue-System", "NOx reagent system (AdBlue)"},
          {{.name = {"AdBlue-Verbrauch im Schnitt", "Average reagent consumption"}, .unit = "l/h",
            .field = scaled(1, 2, 5, 1000), .bit = 0, .min = 0, .max = 5},
           {.name = {"AdBlue-Bedarf im Schnitt", "Average demanded reagent consumption"},
            .unit = "l/h", .field = scaled(3, 2, 5, 1000), .bit = 1, .min = 0, .max = 5},
           {.name = {"AdBlue-Füllstand", "Reagent tank level"}, .unit = "%", .field = percent(5),
            .bit = 2, .min = 0, .max = 100, .warnLow = 10, .warnHigh = 101},
           {.name = {"Zeit mit AdBlue-Warnung", "NOx warning indicator time"}, .unit = "s",
            .field = raw32(6), .bit = 3, .min = 0, .max = 1e6}});
    t.add(0x86, 5, C::Emissions, {"Partikelsensor", "Particulate matter sensor"},
          {{.name = {"Rußkonzentration Bank 1", "Particulate matter bank 1"}, .unit = "mg/m3",
            .field = scaled(1, 2, 1, 80), .bit = 0, .min = 0, .max = 50},
           {.name = {"Rußkonzentration Bank 2", "Particulate matter bank 2"}, .unit = "mg/m3",
            .field = scaled(3, 2, 1, 80), .bit = 1, .min = 0, .max = 50}});
    t.add(0x87, 5, C::Air, {"Saugrohrdruck A und B", "Intake manifold pressure A and B"},
          {{.name = {"Saugrohrdruck A", "Intake manifold pressure A"}, .unit = "kPa",
            .field = scaled(1, 2, 1, 32), .bit = 0, .min = 0, .max = 300},
           {.name = {"Saugrohrdruck B", "Intake manifold pressure B"}, .unit = "kPa",
            .field = scaled(3, 2, 1, 32), .bit = 1, .min = 0, .max = 300}});
    addAecd(0x89, 11);
    addAecd(0x8A, 16);
    t.add(0x8B, 7, C::Emissions, {"Abgasnachbehandlung (Diesel)", "Diesel aftertreatment status"},
          {{.name = {"Partikelfilter-Regeneration", "Particulate filter regeneration"},
            .field = bitOf(1, 0), .bit = 0, .text = regenerationText},
           {.name = {"Art der Regeneration", "Regeneration type"}, .field = bitOf(1, 1), .bit = 1,
            .text = regenerationTypeText},
           {.name = {"NOx-Speicherkat Regeneration", "NOx adsorber regeneration"},
            .field = bitOf(1, 2), .bit = 2, .text = regenerationText},
           {.name = {"NOx-Speicherkat Entschwefelung", "NOx adsorber desulfurization"},
            .field = bitOf(1, 3), .bit = 3, .text = regenerationText},
           {.name = {"Beladung bis zur nächsten Regeneration", "Normalized trigger for regeneration"},
            .unit = "%", .field = percent(2), .bit = 4, .min = 0, .max = 100},
           {.name = {"Zeit zwischen Regenerationen im Schnitt", "Average time between regenerations"},
            .unit = "min", .field = raw16(3), .bit = 5, .min = 0, .max = 2000},
           {.name = {"Strecke zwischen Regenerationen im Schnitt",
                     "Average distance between regenerations"},
            .unit = "km", .field = raw16(5), .bit = 6, .min = 0, .max = 2000}});
    t.single(0x8D, 1, C::Air,
             {.name = {"Drosselklappe absolut G", "Absolute throttle position G"}, .unit = "%",
              .field = percent(0), .min = 0, .max = 100});
    t.single(0x8E, 1, C::Engine,
             {.name = {"Reibmoment des Motors", "Engine friction torque"}, .unit = "%",
              .field = torque(0), .min = -25, .max = 25});
    addEgt(0x98, 1, 5);
    addEgt(0x99, 2, 5);
    t.add(0x9A, 6, C::HybridEv, {"Antriebsbatterie", "Hybrid/EV battery"},
          {{.name = {"Spannung Antriebsbatterie", "Hybrid/EV battery voltage"}, .unit = "V",
            .field = scaled(2, 2, 1, 64), .bit = 1, .min = 0, .max = 1000},
           // Signed, 0.1 A per bit. The sign shows charging or discharging.
           {.name = {"Strom Antriebsbatterie", "Hybrid/EV battery current"}, .unit = "A",
            .field = scaled(4, 2, 1, 10, 0, true), .bit = 2, .min = -400, .max = 400}});
    t.single(0x9B, 4, C::Emissions,
             {.name = {"AdBlue-Sensor, Füllstand", "Diesel exhaust fluid sensor, level"},
              .unit = "%", .field = percent(3), .min = 0, .max = 100});
    t.add(0x9D, 4, C::Fuel, {"Kraftstoffmassenstrom", "Engine fuel rate (mass)"},
          {{.name = {"Kraftstoffmassenstrom Motor", "Engine fuel rate"}, .unit = "g/s",
            .field = scaled(0, 2, 2, 100), .min = 0, .max = 20},
           {.name = {"Kraftstoffmassenstrom Fahrzeug", "Vehicle fuel rate"}, .unit = "g/s",
            .field = scaled(2, 2, 2, 100), .min = 0, .max = 20}});
    t.single(0x9E, 2, C::Exhaust,
             {.name = {"Abgasmassenstrom", "Engine exhaust flow rate"}, .unit = "kg/h",
              .field = scaled(0, 2, 2, 10), .min = 0, .max = 1000});

    // --- 0xA1 to 0xC4 --------------------------------------------------------
    addNox(0xA1, true);
    t.single(0xA2, 2, C::Fuel,
             {.name = {"Kraftstoffmenge je Hub", "Cylinder fuel rate"}, .unit = "mg/Hub",
              .field = scaled(0, 2, 1, 32), .min = 0, .max = 100, .unitEn = "mg/stroke"});
    t.add(0xA4, 4, C::Vehicle, {"Gang", "Transmission gear"},
          {{.name = {"Eingelegter Gang", "Transmission actual gear"}, .field = bitsOf(1, 4, 0x0F),
            .bit = 0, .min = 0, .max = 10, .text = gearText},
           {.name = {"Übersetzung des Gangs", "Actual gear ratio"},
            .field = scaled(2, 2, 1, 1000), .bit = 1, .min = 0, .max = 6}});
    t.single(0xA5, 4, C::Emissions,
             {.name = {"AdBlue-Dosierung angefordert", "Commanded DEF dosing"}, .unit = "%",
              .field = scaled(1, 1, 1, 2), .bit = 0, .min = 0, .max = 127.5});
    t.single(0xA6, 4, C::Vehicle,
             {.name = {"Kilometerstand", "Odometer"}, .unit = "km", .field = scaled(0, 4, 1, 10),
              .min = 0, .max = 1e6});

    return t.take();
}

}  // namespace

const char* pidCategoryName(PidCategory category) {
    switch (category) {
        case PidCategory::Engine: return L("Motor", "Engine");
        case PidCategory::Fuel: return L("Kraftstoff", "Fuel");
        case PidCategory::Air: return L("Luft und Aufladung", "Air and boost");
        case PidCategory::Exhaust: return L("Abgas", "Exhaust");
        case PidCategory::Emissions: return L("Abgasreinigung", "Emission control");
        case PidCategory::Temperatures: return L("Temperaturen", "Temperatures");
        case PidCategory::Electrical: return L("Elektrik", "Electrical");
        case PidCategory::Vehicle: return L("Fahrzeug", "Vehicle");
        case PidCategory::HybridEv: return L("Hybrid und Elektro", "Hybrid and EV");
        case PidCategory::Diagnostics: return L("Diagnose", "Diagnostics");
    }
    return "";
}

const std::vector<PidCategory>& pidCategories() {
    static const std::vector<PidCategory> all = {
        PidCategory::Engine,  PidCategory::Fuel,         PidCategory::Air,
        PidCategory::Exhaust, PidCategory::Emissions,    PidCategory::Temperatures,
        PidCategory::Electrical, PidCategory::Vehicle,   PidCategory::HybridEv,
        PidCategory::Diagnostics};
    return all;
}

// One table per language, both built on first use. The active language
// decides which one is handed out, so names and units follow it.
const std::vector<PidSpec>& pidTable() {
    static const std::vector<PidSpec> german = buildTable(false);
    static const std::vector<PidSpec> english = buildTable(true);
    return language() == Language::English ? english : german;
}

const PidSpec* findPid(int pid) {
    for (const PidSpec& s : pidTable()) {
        if (s.pid == pid) return &s;
    }
    return nullptr;
}

const PidValueSpec* findPidValue(int key) {
    const PidSpec* spec = findPid(liveKeyPid(key));
    if (!spec) return nullptr;
    const int sub = liveKeySub(key);
    if (sub < 0 || sub >= static_cast<int>(spec->values.size())) return nullptr;
    return &spec->values[static_cast<size_t>(sub)];
}

int pidValueCount() {
    int count = 0;
    for (const PidSpec& s : pidTable()) count += static_cast<int>(s.values.size());
    return count;
}

std::vector<PidReading> decodePid(const PidSpec& spec, const std::vector<int>& data) {
    std::vector<PidReading> out;
    for (const PidValueSpec& v : spec.values) {
        if (!v.isAvailable(data)) continue;
        out.push_back({&v, v.calc(data)});
    }
    return out;
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
        {0x04, "Tankentlüftung"},      {0x08, "Sekundärluft"},
        {0x10, "Klimaanlage"},         {0x20, "Lambdasonde"},
        {0x40, "Lambdasondenheizung"}, {0x80, "Abgasrückführung"}};
    static const std::vector<MonitorBit> english = {
        {0x01, "Catalyst"},              {0x02, "Heated catalyst"},
        {0x04, "Evaporative system"},    {0x08, "Secondary air system"},
        {0x10, "A/C refrigerant"},       {0x20, "Oxygen sensor"},
        {0x40, "Oxygen sensor heater"},  {0x80, "EGR system"}};
    return language() == Language::English ? english : german;
}

const std::vector<MonitorBit>& monitorsCDiesel() {
    static const std::vector<MonitorBit> german = {
        {0x01, "Oxidationskatalysator (NMHC)"}, {0x02, "Stickoxid-Nachbehandlung (SCR)"},
        {0x08, "Ladedruck"},                     {0x20, "Abgassensor"},
        {0x40, "Partikelfilter"},                {0x80, "Abgasrückführung"}};
    static const std::vector<MonitorBit> english = {
        {0x01, "NMHC catalyst"},          {0x02, "NOx aftertreatment (SCR)"},
        {0x08, "Boost pressure"},         {0x20, "Exhaust gas sensor"},
        {0x40, "Particulate filter"},     {0x80, "EGR and VVT system"}};
    return language() == Language::English ? english : german;
}

}  // namespace caruspex
