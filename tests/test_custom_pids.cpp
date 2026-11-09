// Custom values: the Torque CSV format, the equation evaluator, reading
// through the adapter (demo car and recorded answers of real cars) and the
// built-in packs.
//
// The recorded answers of the Kona, the Leaf and the Zoe are test cases of
// OBDb (github.com/OBDb, tests/test_cases), taken from real cars; the
// expected numbers are the ones OBDb lists for them. So the packs are checked
// against an independent decoder, not against themselves.

#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include <algorithm>
#include <cctype>
#include <cstdio>
#include <map>
#include <memory>
#include <set>
#include <string>
#include <vector>

#include "core/CustomPids.h"
#include "core/Lang.h"
#include "core/Obd2.h"
#include "core/Transport.h"
#include "data/PidPacks.h"
#include "data/Pids.h"
#include "DemoRig.h"

using namespace caruspex;
using Catch::Approx;

namespace {

double eval(const std::string& equation, const std::vector<int>& data) {
    const TorqueEquation eq(equation);
    INFO(equation << ": " << eq.error());
    REQUIRE(eq.ok());
    double value = 0.0;
    std::string error;
    const bool ok = eq.evaluate(data, value, error);
    INFO(error);
    REQUIRE(ok);
    return value;
}

bool parses(const std::string& equation) { return TorqueEquation(equation).ok(); }

// A scripted ELM327 on CAN 11 bit that knows which control unit is
// addressed. Answers are recorded CAN frames as OBDb writes them:
// "7EC103E620101FFF7E7" is id 7EC with the eight data bytes after it. They
// come out with or without headers, the way the adapter prints them with CAN
// auto formatting.
class EcuScript : public ITransport {
public:
    std::map<std::string, std::vector<std::string>> frames;   // "7E4|220101" -> frames
    std::vector<std::string> sent;

    bool write(const std::string& data) override {
        std::string cmd;
        for (char c : data) {
            if (c == '\r' || c == '\n' || c == ' ') continue;
            cmd += static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
        }
        sent.push_back(cmd);
        if (cmd == "ATZ") {
            headers_ = false;
            header_ = "7DF";
            pending_ = "ELM327 v1.5\r\r>";
        } else if (cmd == "ATH1" || cmd == "ATH0") {
            headers_ = cmd == "ATH1";
            pending_ = "OK\r\r>";
        } else if (cmd == "ATDP") {
            pending_ = "ISO 15765-4 (CAN 11/500)\r\r>";
        } else if (cmd.rfind("ATSH", 0) == 0) {
            header_ = cmd.substr(4);
            pending_ = "OK\r\r>";
        } else if (cmd.rfind("AT", 0) == 0) {
            pending_ = "OK\r\r>";
        } else if (cmd == "0100") {
            pending_ = headers_ ? "7E8 06 41 00 BE 1F A8 13 AA\r\r>" : "41 00 BE 1F A8 13\r\r>";
        } else {
            const auto it = frames.find(header_ + "|" + cmd);
            if (it == frames.end()) {
                pending_ = "NO DATA\r\r>";
            } else {
                pending_.clear();
                for (const std::string& line : format(it->second)) pending_ += line + "\r";
                pending_ += "\r>";
            }
        }
        return true;
    }
    std::string read(int) override {
        std::string out;
        out.swap(pending_);
        return out;
    }
    void flushInput() override { pending_.clear(); }
    std::string name() const override { return "EcuScript"; }

    size_t count(const std::string& cmd) const {
        return static_cast<size_t>(std::count(sent.begin(), sent.end(), cmd));
    }

private:
    static std::string spaced(const std::string& hex) {
        std::string out;
        for (size_t i = 0; i + 1 < hex.size(); i += 2) {
            if (!out.empty()) out += ' ';
            out += hex.substr(i, 2);
        }
        return out;
    }

    std::vector<std::string> format(const std::vector<std::string>& raw) const {
        std::vector<std::string> out;
        for (const std::string& f : raw) {
            const std::string id = f.substr(0, 3);
            const std::string data = f.substr(3);
            if (headers_) {
                out.push_back(id + " " + spaced(data));
                continue;
            }
            const int pci = std::stoi(data.substr(0, 2), nullptr, 16);
            if ((pci >> 4) == 0) {
                out.push_back(spaced(data.substr(2, static_cast<size_t>(pci) * 2)));
            } else if ((pci >> 4) == 1) {
                const int length = ((pci & 0x0F) << 8) | std::stoi(data.substr(2, 2), nullptr, 16);
                char head[8];
                std::snprintf(head, sizeof(head), "%03X", length);
                out.push_back(head);
                out.push_back("0: " + spaced(data.substr(4)));
            } else {
                out.push_back(std::to_string(pci & 0x0F) + ": " + spaced(data.substr(2)));
            }
        }
        return out;
    }

    std::string pending_;
    std::string header_ = "7DF";
    bool headers_ = false;
};

struct ScriptCar {
    EcuScript* t = nullptr;
    Elm327 elm;
    std::string error;
    bool connected = false;

    ScriptCar() {
        auto owner = std::make_unique<EcuScript>();
        t = owner.get();
        connected = elm.connect(std::move(owner), "6", error);
    }
};

// Recorded by OBDb on a 2020 Kona Electric (test_cases/2020).
const std::vector<std::string> kKona0101 = {
    "7EC103E620101FFF7E7", "7EC21FF490000000080", "7EC2200000DD9090908",
    "7EC23080908000007B4", "7EC2402B40100007E00", "7EC2500DB540000D915",
    "7EC260000525900004F", "7EC2757007B02130000", "7EC2800000000000BB8"};
const std::vector<std::string> kKona0105 = {
    "7EC102E620105003FFF", "7EC2190000000000000", "7EC220000077E000041",
    "7EC238C3E7800006407", "7EC244B03E032003137", "7EC254B0000B4B40000",
    "7EC260900000000AAAA"};
// A 2013 Leaf (test_cases/2013 and 2015).
const std::vector<std::string> kLeaf2101 = {
    "7BB1029610100000012", "7BB210289FFFFFDE0FF", "7BB22FFFFFF0D5A2AF8",
    "7BB23968F381C038B00", "7BB24601D46000B9B35", "7BB2500081340800001"};
const std::vector<std::string> kLeaf2161 = {"7BB100C6161140F1C9A", "7BB21FF1CA41C9A03FF"};
// A 2017 Zoe (test_cases/2017).
const std::vector<std::string> kZoe3203 = {"7EC0562320302A2AAAA"};
const std::vector<std::string> kZoe3204 = {"7EC056232047DEAAAAA"};

const CustomPid* byName(const std::vector<CustomPid>& pids, const std::string& name) {
    for (const CustomPid& p : pids) {
        if (p.name == name) return &p;
    }
    return nullptr;
}

// Reads a pack in English and returns name -> value of everything read.
std::map<std::string, double> readPack(Elm327& elm, const std::string& pack,
                                       std::vector<std::string>* failures = nullptr) {
    setLanguage(Language::English);
    const std::vector<CustomPid> pids = packPids(pack);
    std::map<std::string, double> out;
    CustomPidReader reader(elm);
    const std::vector<CustomReading> readings = reader.readAll(pids);
    REQUIRE(readings.size() == pids.size());
    for (size_t i = 0; i < pids.size(); ++i) {
        if (readings[i].ok) {
            REQUIRE(readings[i].value.key == pids[i].key);
            REQUIRE(readings[i].value.name == pids[i].name);
            out[pids[i].name] = readings[i].value.value;
        } else if (failures) {
            failures->push_back(pids[i].name + ": " + readings[i].error);
        }
    }
    setLanguage(Language::German);
    return out;
}

}  // namespace

// ---------------------------------------------------------------------------
// Equations
// ---------------------------------------------------------------------------

TEST_CASE("Equations: arithmetic, precedence and bytes", "[custom]") {
    const std::vector<int> d = {0x12, 0x34, 0xFF, 0x80, 0x7F, 0x00, 0x05};
    REQUIRE(eval("2+3*4", d) == 14);
    REQUIRE(eval("(2+3)*4", d) == 20);
    REQUIRE(eval("10-4-3", d) == 3);
    REQUIRE(eval("100/10/4", d) == Approx(2.5));
    REQUIRE(eval("-A+5", d) == -0x12 + 5);
    REQUIRE(eval("--3", d) == 3);
    REQUIRE(eval("2*-3", d) == -6);
    REQUIRE(eval("+4", d) == 4);
    REQUIRE(eval(" 1.5 * 2 ", d) == 3);
    REQUIRE(eval(".25*4", d) == 1);
    REQUIRE(eval("A", d) == 0x12);
    REQUIRE(eval("b", d) == 0x34);           // case does not matter
    REQUIRE(eval("G", d) == 5);
    REQUIRE(eval("(A*256)+B", d) == 0x1234);
    REQUIRE(eval("A*256+B/4", d) == 0x12 * 256 + 0x34 / 4.0);
}

TEST_CASE("Equations: byte names beyond Z", "[custom]") {
    REQUIRE(TorqueEquation::byteIndex("A") == 0);
    REQUIRE(TorqueEquation::byteIndex("Z") == 25);
    REQUIRE(TorqueEquation::byteIndex("AA") == 26);
    REQUIRE(TorqueEquation::byteIndex("ab") == 27);
    REQUIRE(TorqueEquation::byteIndex("AZ") == 51);
    REQUIRE(TorqueEquation::byteIndex("BA") == 52);
    REQUIRE(TorqueEquation::byteIndex("BV") == 73);
    REQUIRE(TorqueEquation::byteIndex("ZZ") == 701);
    REQUIRE(TorqueEquation::byteIndex("ABC") == -1);
    REQUIRE(TorqueEquation::byteIndex("A1") == -1);
    REQUIRE(TorqueEquation::byteIndex("") == -1);

    std::vector<int> d(80, 0);
    d[26] = 7;
    d[52] = 9;
    d[73] = 50;
    REQUIRE(eval("AA", d) == 7);
    REQUIRE(eval("ba", d) == 9);
    REQUIRE(eval("BV-40", d) == 10);
    REQUIRE(TorqueEquation("BV-40").bytesNeeded() == 74);
    REQUIRE(TorqueEquation("A+C").bytesNeeded() == 3);
    REQUIRE(TorqueEquation("12").bytesNeeded() == 0);
}

TEST_CASE("Equations: shifts as the EV lists write them", "[custom]") {
    const std::vector<int> d = {0x01, 0x02, 0x03, 0x04, 0xF0};
    REQUIRE(eval("(A<8)+B", d) == 0x0102);
    REQUIRE(eval("((A<24)+(B<16)+(C<8)+D)/10", d) == Approx(0x01020304 / 10.0));
    REQUIRE(eval("E>4", d) == 0x0F);
    // Shifts bind weaker than + and -, as in C.
    REQUIRE(eval("A<8+B", d) == 1 << 10);
    REQUIRE(eval("(0-16)>2", d) == -4);   // arithmetic right shift
    REQUIRE(eval("2.9<1", d) == 4);       // the fraction is dropped first
    REQUIRE(eval("(E<24)<8", d) == Approx(4026531840.0 * 256.0));
}

TEST_CASE("Equations: Signed and the integer functions", "[custom]") {
    const std::vector<int> d = {0xFF, 0x80, 0x7F, 0x00, 0xFF, 0x38, 0x9C};
    REQUIRE(eval("Signed(A)", d) == -1);
    REQUIRE(eval("SIGNED(B)", d) == -128);
    REQUIRE(eval("signed(C)", d) == 127);
    REQUIRE(eval("SIGNED8(D)", d) == 0);
    // The current of the Hyundai/Kia BMS: signed high byte times 256 plus
    // the low byte.
    REQUIRE(eval("((Signed(A)*256)+G)/10", d) == Approx(-10.0));
    REQUIRE(eval("INT16(E:F)", d) == 0xFF38);
    REQUIRE(eval("SIGNED16(INT16(E:F))", d) == -200);
    REQUIRE(eval("SIGNED16(INT16(E,F))", d) == -200);   // comma as separator too
    REQUIRE(eval("INT24(A:B:C)", d) == 0xFF807F);
    REQUIRE(eval("SIGNED24(INT24(A:B:C))", d) == static_cast<double>(0xFF807F) - 16777216.0);
    REQUIRE(eval("INT32(A:B:C:D)", d) == Approx(4286611200.0));
    REQUIRE(eval("SIGNED32(INT32(A:B:C:D))", d) == Approx(4286611200.0 - 4294967296.0));
    REQUIRE(eval("MIN(A:C)", d) == 0x7F);
    REQUIRE(eval("MAX(A:C)", d) == 0xFF);
    REQUIRE(eval("ABS(0-5)", d) == 5);
    REQUIRE(eval("INT(7/2)", d) == 3);
    REQUIRE(eval("INT(0-7/2)", d) == -3);
}

TEST_CASE("Equations: bit access", "[custom]") {
    const std::vector<int> d = {0x80, 0x01, 0x28};
    REQUIRE(eval("{A:7}", d) == 1);
    REQUIRE(eval("{A:6}", d) == 0);
    REQUIRE(eval("{b:0}", d) == 1);
    REQUIRE(eval("{C:3}+{C:5}", d) == 2);
    REQUIRE(eval("BIT(C:5)", d) == 1);
    REQUIRE(eval("BIT(C:4)", d) == 0);
    REQUIRE(eval("BIT((A<8)+B:15)", d) == 1);
}

TEST_CASE("Equations: errors instead of crashes", "[custom]") {
    for (const char* bad : {"", "   ", "A+", "(A", "A)", "A B", "2A", "foo(A)", "ABC", "1.2.3",
                            "val{000_Battery Current}", "{A:40}", "{A}", "{1:2}", "MIN(A)",
                            "SIGNED(A:B)", "A*/B", "#", "A=B", "INT16(A:B"}) {
        const TorqueEquation eq(bad);
        INFO(bad);
        REQUIRE_FALSE(eq.ok());
        REQUIRE_FALSE(eq.error().empty());
        double value = 0.0;
        std::string error;
        REQUIRE_FALSE(eq.evaluate({1, 2, 3}, value, error));
        REQUIRE_FALSE(error.empty());
    }
    // Nesting deep enough to hurt a naive parser.
    REQUIRE_FALSE(parses(std::string(5000, '(') + "A" + std::string(5000, ')')));
    REQUIRE_FALSE(parses(std::string(5000, '-') + "A"));
    REQUIRE(parses(std::string(20, '(') + "A" + std::string(20, ')')));

    double value = 0.0;
    std::string error;
    REQUIRE_FALSE(TorqueEquation("A/B").evaluate({5, 0}, value, error));
    REQUIRE(error.find("null") != std::string::npos);   // German: "Division durch null."

    setLanguage(Language::English);
    REQUIRE_FALSE(TorqueEquation("A/(B-B)").evaluate({5, 3}, value, error));
    REQUIRE(error == "Division by zero.");
    REQUIRE_FALSE(TorqueEquation("((AA<8)+AB)/10").evaluate({1, 2, 3}, value, error));
    REQUIRE(error.find("AA") != std::string::npos);
    REQUIRE(error.find("3 data bytes") != std::string::npos);
    REQUIRE_FALSE(TorqueEquation("A<70").evaluate({1}, value, error));
    REQUIRE_FALSE(error.empty());
    REQUIRE_FALSE(TorqueEquation("BIT(A:0-1)").evaluate({1}, value, error));
    REQUIRE_FALSE(TorqueEquation("10000000000*10000000000<1").evaluate({}, value, error));
    REQUIRE(TorqueEquation("val{x}").error().find("val{") != std::string::npos);
    setLanguage(Language::German);
}

TEST_CASE("Equations: parse once, evaluate many times", "[custom]") {
    const TorqueEquation eq("((Signed(K)*256)+L)/10");
    const TorqueEquation copy = eq;   // copies share the parsed form
    std::vector<int> d(12, 0);
    for (int raw = -300; raw <= 300; raw += 7) {
        const int u = raw & 0xFFFF;
        d[10] = u >> 8;
        d[11] = u & 0xFF;
        double value = 0.0;
        std::string error;
        REQUIRE(copy.evaluate(d, value, error));
        REQUIRE(value == Approx(raw / 10.0));
    }
    REQUIRE(eq.text() == "((Signed(K)*256)+L)/10");
}

// ---------------------------------------------------------------------------
// Torque CSV
// ---------------------------------------------------------------------------

TEST_CASE("Torque CSV: header, quotes, comments", "[custom]") {
    const std::string text =
        "\xEF\xBB\xBF# My Kona values\r\n"
        "Name,ShortName,ModeAndPID,Equation,Min Value,Max Value,Units,Header\r\n"
        "\r\n"
        "  # an indented comment\r\n"
        "000_State of Charge BMS,SOC BMS,0x220101,e/2,0,100,%,7E4\r\n"
        "\"Current, signed\",\"Batt \"\"A\"\"\",220101,((Signed(K)*256)+L)/10,-230,230,A,7e4\r\n"
        "\"Two\nlines\",2L,2101,A,0,1,,\r\n"
        "Odometer,Odo,22b002,INT24(g:h:i),0,16777215,km,7C6\r\n"
        "Engine speed,RPM,010C,((A*256)+B)/4,0,8000,rpm,\r\n";
    std::vector<std::string> errors;
    const std::vector<CustomPid> pids = parseTorqueCsv(text, errors);
    INFO((errors.empty() ? std::string() : errors.front()));
    REQUIRE(errors.empty());
    REQUIRE(pids.size() == 5);

    REQUIRE(pids[0].name == "000_State of Charge BMS");
    REQUIRE(pids[0].shortName == "SOC BMS");
    REQUIRE(pids[0].request == "220101");
    REQUIRE(pids[0].equation == "e/2");
    REQUIRE(pids[0].maxValue == 100);
    REQUIRE(pids[0].unit == "%");
    REQUIRE(pids[0].header == "7E4");
    REQUIRE(pids[0].response.empty());
    REQUIRE(pids[0].key == kUserKeyBase);
    REQUIRE(pids[0].category == PidCategory::Custom);
    REQUIRE(pids[0].pack.empty());

    REQUIRE(pids[1].name == "Current, signed");
    REQUIRE(pids[1].shortName == "Batt \"A\"");
    REQUIRE(pids[1].minValue == -230);
    REQUIRE(pids[1].header == "7E4");
    REQUIRE(pids[1].key == kUserKeyBase + 1);

    REQUIRE(pids[2].name == "Two\nlines");
    REQUIRE(pids[2].header.empty());
    REQUIRE(pids[3].request == "22B002");
    REQUIRE(pids[3].header == "7C6");
    REQUIRE(pids[4].request == "010C");
    REQUIRE(isCustomKey(pids[4].key));
    REQUIRE_FALSE(isCustomKey(liveKey(0x0C, 3)));
}

TEST_CASE("Torque CSV: without header, by column name, extra columns", "[custom]") {
    std::vector<std::string> errors;
    // Torque's order without a header line, Header column left out.
    std::vector<CustomPid> pids =
        parseTorqueCsv("Oil,Oil,221001,A,0,100,%\nTemp,T,221002,A-40,-40,215,C,7E0\n", errors, 0x300000);
    REQUIRE(errors.empty());
    REQUIRE(pids.size() == 2);
    REQUIRE(pids[0].header.empty());
    REQUIRE(pids[1].header == "7E0");
    REQUIRE(pids[1].key == 0x300001);

    // Columns in another order, an unknown column, our Response column.
    pids = parseTorqueCsv(
        "Equation,Header,Name,Comment,ModeAndPID,Response,Units\n"
        "H/2,79B,SOH LBC,ignored,2161,7BB,%\n"
        "A,79B:7BB,Pair,,2101,,\n"
        "A,79b/7bb,Pair 2,,2101,,\n"
        "A,DA44F1,Wide,,22F190,,\n",
        errors);
    REQUIRE(errors.empty());
    REQUIRE(pids.size() == 4);
    REQUIRE(pids[0].name == "SOH LBC");
    REQUIRE(pids[0].shortName == "SOH LBC");   // falls back to the name
    REQUIRE(pids[0].request == "2161");
    REQUIRE(pids[0].header == "79B");
    REQUIRE(pids[0].response == "7BB");
    REQUIRE(pids[0].minValue == 0);
    REQUIRE(pids[1].header == "79B");
    REQUIRE(pids[1].response == "7BB");
    REQUIRE(pids[2].response == "7BB");
    REQUIRE(pids[3].header == "18DA44F1");
}

TEST_CASE("Torque CSV: bad lines are skipped and named", "[custom]") {
    const std::string text =
        "Name,ShortName,ModeAndPID,Equation,Min Value,Max Value,Units,Header\n"
        "Good,G,220101,A,0,1,,7E4\n"                                  // 2
        ",NoName,220101,A,0,1,,7E4\n"                                 // 3
        "Bad request,B,22ZZ,A,0,1,,7E4\n"                            // 4
        "Odd request,B,22010,A,0,1,,7E4\n"                           // 5
        "Bad equation,B,220101,A+*,0,1,,7E4\n"                       // 6
        "Bad min,B,220101,A,low,1,,7E4\n"                            // 7
        "Bad header,B,220101,A,0,1,,7E4X\n"                          // 8
        "~Modifications by someone,List Credit,,,,,,\n"              // 9
        "!_ABRP_Battery,B,,val{000_Battery Current},-230,230,A,\n"   // 10
        "Reference,R,220101,val{000_Battery Current},0,1,,7E4\n"     // 11
        "Only service,S,22,A,0,1,,7E4\n"                             // 12
        "Too long,L,2201010101010101,A,0,1,,7E4\n"                   // 13
        "Header too big,H,220101,A,0,1,,FFF\n"                       // 14
        "Good too,G2,2101,B,0,1,,\n";                                // 15
    std::vector<std::string> errors;
    setLanguage(Language::English);
    const std::vector<CustomPid> pids = parseTorqueCsv(text, errors);
    setLanguage(Language::German);
    REQUIRE(pids.size() == 2);
    REQUIRE(pids[0].name == "Good");
    REQUIRE(pids[1].name == "Good too");
    REQUIRE(pids[1].key == kUserKeyBase + 1);   // keys stay dense
    REQUIRE(errors.size() == 12);
    for (int line = 3; line <= 14; ++line) {
        const std::string prefix = "Line " + std::to_string(line) + ": ";
        const bool found = std::any_of(errors.begin(), errors.end(),
                                       [&](const std::string& e) { return e.rfind(prefix, 0) == 0; });
        INFO(prefix);
        REQUIRE(found);
    }

    // A header line without the essential columns.
    errors.clear();
    REQUIRE(parseTorqueCsv("Name,ModeAndPID,Units\nA,2101,%\n", errors).empty());
    REQUIRE(errors.size() == 1);

    // Nothing at all.
    errors.clear();
    REQUIRE(parseTorqueCsv("", errors).empty());
    REQUIRE(parseTorqueCsv("# only a comment\n\n", errors).empty());
    REQUIRE(errors.empty());
}

TEST_CASE("Torque CSV: writing and reading back", "[custom]") {
    std::vector<CustomPid> pids(3);
    pids[0].name = "Plain";
    pids[0].shortName = "P";
    pids[0].request = "220101";
    pids[0].equation = "E/2";
    pids[0].minValue = 0;
    pids[0].maxValue = 100;
    pids[0].unit = "%";
    pids[0].header = "7E4";
    pids[1].name = "With, comma and \"quotes\"";
    pids[1].shortName = " padded ";
    pids[1].request = "2101";
    pids[1].equation = "MIN(A:B)";
    pids[1].minValue = -0.25;
    pids[1].maxValue = 1234.5;
    pids[1].unit = "°C";
    pids[2].name = "#not a comment";
    pids[2].shortName = "N";
    pids[2].request = "2161";
    pids[2].equation = "H/2";
    pids[2].header = "79B";

    std::string text = writeTorqueCsv(pids);
    REQUIRE(text.rfind("Name,ShortName,ModeAndPID,Equation,Min Value,Max Value,Units,Header\n", 0) == 0);
    std::vector<std::string> errors;
    std::vector<CustomPid> back = parseTorqueCsv(text, errors);
    REQUIRE(errors.empty());
    REQUIRE(back.size() == 3);
    for (size_t i = 0; i < pids.size(); ++i) {
        REQUIRE(back[i].name == pids[i].name);
        REQUIRE(back[i].request == pids[i].request);
        REQUIRE(back[i].equation == pids[i].equation);
        REQUIRE(back[i].minValue == pids[i].minValue);
        REQUIRE(back[i].maxValue == pids[i].maxValue);
        REQUIRE(back[i].unit == pids[i].unit);
        REQUIRE(back[i].header == pids[i].header);
    }
    REQUIRE(back[1].shortName == "padded");   // cells are trimmed

    // The Response column appears only when needed.
    pids[2].response = "7BB";
    text = writeTorqueCsv(pids);
    REQUIRE(text.rfind("Name,ShortName,ModeAndPID,Equation,Min Value,Max Value,Units,Header,Response\n", 0) == 0);
    back = parseTorqueCsv(text, errors);
    REQUIRE(back.size() == 3);
    REQUIRE(back[2].response == "7BB");
    REQUIRE(back[0].response.empty());

    // A pack survives the round trip with every value.
    const std::vector<CustomPid>& all = builtInPacks();
    back = parseTorqueCsv(writeTorqueCsv(all), errors);
    REQUIRE(errors.empty());
    REQUIRE(back.size() == all.size());
}

TEST_CASE("Request hex", "[custom]") {
    std::vector<int> b;
    REQUIRE(parseRequestHex("220101", b));
    REQUIRE(b == std::vector<int>{0x22, 0x01, 0x01});
    REQUIRE(parseRequestHex("0x2101", b));
    REQUIRE(b == std::vector<int>{0x21, 0x01});
    REQUIRE(parseRequestHex("22 f1 90", b));
    REQUIRE(b.size() == 3);
    REQUIRE_FALSE(parseRequestHex("", b));
    REQUIRE_FALSE(parseRequestHex("221", b));
    REQUIRE_FALSE(parseRequestHex("22G101", b));
    REQUIRE_FALSE(parseRequestHex("2201010101010101", b));   // eight bytes
}

// ---------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------

TEST_CASE("Reading the demo pack from the demo car", "[custom][demo]") {
    for (bool headers : {false, true}) {
        DemoRig rig(600.0);
        REQUIRE(rig.connected);
        if (headers) REQUIRE(rig.elm.setHeaders(true));
        rig.sent.clear();

        std::vector<std::string> failures;
        const std::map<std::string, double> v = readPack(rig.elm, "caruspex-demo", &failures);
        INFO((failures.empty() ? std::string() : failures.front()));
        REQUIRE(failures.empty());
        REQUIRE(v.size() == packPids("caruspex-demo").size());

        REQUIRE(v.at("Oil level") >= 60);
        REQUIRE(v.at("Oil level") <= 80);
        REQUIRE(v.at("Oil temperature") > 30);
        REQUIRE(v.at("Oil temperature") < 110);
        REQUIRE(v.at("Engine starts") == 1835);
        REQUIRE(v.at("Engine running (1 = yes)") == 1);
        // 2 456 000 s before the drive plus the run time: just over 682 h.
        REQUIRE(v.at("Engine hours") > 682.2);
        REQUIRE(v.at("Engine hours") < 682.5);
        REQUIRE(v.at("Supply voltage (engine control unit)") > 13.5);
        REQUIRE(v.at("Supply voltage (engine control unit)") < 14.5);
        REQUIRE(v.at("Engine speed (OBD)") > 600);

        // The engine was addressed once for all its values (one request for
        // the two values of 1010 too), and the default addressing is back.
        // Elm327 logs one line per command: "  > COMMAND   ->   answer".
        auto count = [&](const std::string& what) {
            return std::count_if(rig.sent.begin(), rig.sent.end(), [&](const std::string& line) {
                return line.find("> " + what + " ") != std::string::npos;
            });
        };
        REQUIRE(count("ATSH7E0") == 1);
        REQUIRE(count("221010") == 1);
        REQUIRE(count("ATSH7DF") >= 1);

        Obd2 obd(rig.elm);
        LiveValue speed;
        REQUIRE(obd.readLive(0x0D, speed));
    }
}

TEST_CASE("Reading single values and failures", "[custom][demo]") {
    DemoRig rig(600.0);
    REQUIRE(rig.connected);
    setLanguage(Language::English);

    CustomPid oil;
    oil.name = "Oil";
    oil.request = "221002";
    oil.header = "7E0";
    oil.equation = "SIGNED16(INT16(A:B))/10";
    oil.key = kUserKeyBase + 5;
    oil.minValue = -20;
    oil.maxValue = 150;
    oil.unit = "°C";
    LiveValue value;
    std::string error;
    REQUIRE(readCustom(rig.elm, oil, value, error));
    REQUIRE(error.empty());
    REQUIRE(value.key == kUserKeyBase + 5);
    REQUIRE(value.category == PidCategory::Custom);
    REQUIRE(value.displayMax == 150);
    REQUIRE(value.unit == "°C");
    REQUIRE(value.ecuSource == "7E8");
    REQUIRE_FALSE(value.ecu.empty());

    // Without a header: functional OBD addressing.
    CustomPid rpm;
    rpm.name = "RPM";
    rpm.request = "010C";
    rpm.equation = "((A*256)+B)/4";
    REQUIRE(readCustom(rig.elm, rpm, value, error));
    REQUIRE(value.value > 600);
    REQUIRE(value.ecuSource.empty());

    CustomPidReader reader(rig.elm);
    // An identifier the engine does not know: the refusal comes through.
    CustomPid unknown = oil;
    unknown.request = "22FFFF";
    REQUIRE_FALSE(reader.read(unknown, value, error));
    REQUIRE(error.find("0x31") != std::string::npos);

    // A control unit that is not there.
    CustomPid nobody = oil;
    nobody.header = "7A0";
    REQUIRE_FALSE(reader.read(nobody, value, error));
    REQUIRE_FALSE(error.empty());

    // The answer is shorter than the equation needs.
    CustomPid shortAnswer = oil;
    shortAnswer.equation = "Z";
    REQUIRE_FALSE(reader.read(shortAnswer, value, error));
    REQUIRE(error.find("byte Z") != std::string::npos);

    // A broken equation is reported without asking the car.
    CustomPid broken = oil;
    broken.equation = "A+";
    rig.sent.clear();
    REQUIRE_FALSE(reader.read(broken, value, error));
    REQUIRE_FALSE(error.empty());
    REQUIRE(rig.sent.empty());

    // A K-line style request id without CAN is refused by the adapter
    // check, a 29-bit header on an 11-bit bus too.
    CustomPid wide = oil;
    wide.header = "18DA10F1";
    REQUIRE_FALSE(reader.read(wide, value, error));
    REQUIRE_FALSE(error.empty());

    // And afterwards the oil temperature still reads.
    REQUIRE(reader.read(oil, value, error));
    reader.restoreDefault();
    setLanguage(Language::German);

    Obd2 obd(rig.elm);
    LiveValue speed;
    REQUIRE(obd.readLive(0x0C, speed));
}

TEST_CASE("Kona Electric: recorded multi-frame answers decode like OBDb", "[custom][packs]") {
    for (bool headers : {false, true}) {
        ScriptCar car;
        REQUIRE(car.connected);
        if (headers) REQUIRE(car.elm.setHeaders(true));
        car.t->frames["7E4|220101"] = kKona0101;
        car.t->frames["7E4|220105"] = kKona0105;
        car.t->sent.clear();

        std::vector<std::string> failures;
        const std::map<std::string, double> v = readPack(car.elm, "hyundai-kona-niro-ev", &failures);
        INFO((failures.empty() ? std::string() : failures.front()));
        REQUIRE(failures.empty());

        REQUIRE(v.at("State of charge (BMS)") == Approx(36.5));
        REQUIRE(v.at("State of charge (display)") == Approx(37.5));
        REQUIRE(v.at("State of health") == Approx(99.2));
        REQUIRE(v.at("Minimum cell deterioration") == Approx(4.9));
        REQUIRE(v.at("Battery current (positive = discharge)") == Approx(0.0));
        REQUIRE(v.at("Battery voltage") == Approx(354.5));
        REQUIRE(v.at("Battery power (positive = discharge)") == Approx(0.0));
        REQUIRE(v.at("Available charge power") == Approx(0.0));
        REQUIRE(v.at("Maximum cell voltage") == Approx(3.6));
        REQUIRE(v.at("Cell with the highest voltage") == 2);
        REQUIRE(v.at("Minimum cell voltage") == Approx(3.6));
        REQUIRE(v.at("Cell with the lowest voltage") == 1);
        REQUIRE(v.at("Cell voltage spread") == Approx(0.0));
        REQUIRE(v.at("Battery temperature maximum") == 9);
        REQUIRE(v.at("Battery temperature minimum") == 9);
        REQUIRE(v.at("Battery module 1 temperature") == 8);
        REQUIRE(v.at("Battery module 3 temperature") == 9);
        REQUIRE(v.at("Battery inlet temperature") == 7);
        REQUIRE(v.at("Battery heater temperature") == 7);
        REQUIRE(v.at("12 V battery voltage") == Approx(12.6));
        REQUIRE(v.at("Charging (1 = yes)") == 1);
        REQUIRE(v.at("Fast charging plug connected (1 = yes)") == 0);
        REQUIRE(v.at("Main relay closed (1 = yes)") == 0);
        REQUIRE(v.at("Total charge (current)") == Approx(5614.8));
        REQUIRE(v.at("Total discharge (current)") == Approx(5557.3));
        REQUIRE(v.at("Total energy charged") == Approx(2108.1));
        REQUIRE(v.at("Total energy discharged") == Approx(2031.1));
        REQUIRE(v.at("Operating time") == Approx(8061459.0 / 3600.0));
        REQUIRE(v.at("Isolation resistance") == 3000);

        // 33 values, two requests, one addressing of the BMS.
        REQUIRE(car.t->count("220101") == 1);
        REQUIRE(car.t->count("220105") == 1);
        REQUIRE(car.t->count("ATSH7E4") == 1);
        REQUIRE(car.t->count("ATCRA7EC") == 1);
        REQUIRE(car.t->count("ATSH7DF") == 1);
    }
}

TEST_CASE("Kona Electric: negative current and power", "[custom][packs]") {
    // The same answer with 0xFF9C (-10.0 A) in K and L: charging. The
    // first frame holds 62 01 01 and A to C, the next ones seven bytes
    // each, so K and L open the third frame.
    std::vector<std::string> frames = kKona0101;
    frames[2].replace(5, 4, "FF9C");
    ScriptCar car;
    REQUIRE(car.connected);
    car.t->frames["7E4|220101"] = frames;
    car.t->frames["7E4|220105"] = kKona0105;
    const std::map<std::string, double> v = readPack(car.elm, "hyundai-kona-niro-ev");
    REQUIRE(v.at("Battery current (positive = discharge)") == Approx(-10.0));
    REQUIRE(v.at("Battery power (positive = discharge)") == Approx(-10.0 * 354.5 / 1000.0));
}

TEST_CASE("Nissan Leaf and Renault Zoe: recorded answers decode like OBDb", "[custom][packs]") {
    {
        ScriptCar car;
        REQUIRE(car.connected);
        car.t->frames["79B|2101"] = kLeaf2101;
        car.t->frames["79B|2161"] = kLeaf2161;
        car.t->sent.clear();
        const std::map<std::string, double> v = readPack(car.elm, "nissan-leaf-ze0-aze0");
        REQUIRE(v.at("State of charge (battery controller)") == Approx(76.0629));
        REQUIRE(v.at("Battery capacity") == Approx(52.9216));
        REQUIRE(v.at("Battery voltage") == Approx(385.43));
        REQUIRE(v.at("State of health") == Approx(73.22));
        // The battery controller answers on 7BB, not 79B + 8.
        REQUIRE(car.t->count("ATSH79B") == 1);
        REQUIRE(car.t->count("ATCRA7BB") == 1);
        REQUIRE(car.t->count("ATFCSH79B") == 1);
        // Group 04 was not recorded: NO DATA, the temperatures are missing.
        REQUIRE(v.count("Battery temperature sensor 1") == 0);
    }
    {
        ScriptCar car;
        REQUIRE(car.connected);
        car.t->frames["7E4|223203"] = kZoe3203;
        car.t->frames["7E4|223204"] = kZoe3204;
        const std::map<std::string, double> v = readPack(car.elm, "renault-zoe-ph1");
        REQUIRE(v.at("Battery voltage") == Approx(337.0));
        REQUIRE(v.at("Battery current") == Approx(-133.5));
    }
}

// ---------------------------------------------------------------------------
// Packs
// ---------------------------------------------------------------------------

TEST_CASE("Built-in packs load and every equation parses", "[custom][packs]") {
    const std::vector<std::string> names = packNames();
    REQUIRE(names.size() >= 5);
    REQUIRE(names.front() == "caruspex-demo");
    REQUIRE(std::set<std::string>(names.begin(), names.end()).size() == names.size());
    for (const char* id : {"hyundai-kona-niro-ev", "hyundai-ioniq-28", "renault-zoe-ph1",
                           "nissan-leaf-ze0-aze0"}) {
        REQUIRE(std::find(names.begin(), names.end(), id) != names.end());
        REQUIRE(packCategory(id) == PidCategory::HybridEv);
    }
    REQUIRE(packCategory("caruspex-demo") == PidCategory::Custom);
    REQUIRE(packTitle("nope").empty());
    REQUIRE(packPids("nope").empty());

    for (Language lang : {Language::German, Language::English}) {
        setLanguage(lang);
        const std::vector<CustomPid>& all = builtInPacks();
        REQUIRE(all.size() >= 90);
        std::set<int> keys;
        size_t counted = 0;
        for (const std::string& n : names) {
            REQUIRE_FALSE(packTitle(n).empty());
            const std::vector<CustomPid> pack = packPids(n);
            REQUIRE_FALSE(pack.empty());
            counted += pack.size();
            std::set<std::string> seen;
            for (const CustomPid& p : pack) REQUIRE(seen.insert(p.name).second);
        }
        REQUIRE(counted == all.size());
        for (const CustomPid& p : all) {
            INFO(p.pack << ": " << p.name << " = " << p.equation);
            const TorqueEquation eq(p.equation);
            REQUIRE(eq.ok());
            REQUIRE(eq.bytesNeeded() <= 80);
            std::vector<int> request;
            REQUIRE(parseRequestHex(p.request, request));
            REQUIRE(request.size() >= 2);
            REQUIRE(keys.insert(p.key).second);
            REQUIRE(isCustomKey(p.key));
            REQUIRE(p.key >= kPackKeyBase);
            REQUIRE(p.key < kUserKeyBase);
            REQUIRE_FALSE(p.name.empty());
            REQUIRE_FALSE(p.shortName.empty());
            REQUIRE_FALSE(p.source.empty());
            REQUIRE(p.minValue < p.maxValue);
            REQUIRE(p.category == packCategory(p.pack));
            for (const std::string& text : {p.name, p.shortName, p.unit, p.source}) {
                REQUIRE(text.find("\xE2\x80\x94") == std::string::npos);   // em dash
                REQUIRE(text.find("\xE2\x80\x93") == std::string::npos);   // en dash
            }
            // With every byte at zero the equation must not fail.
            double value = 0.0;
            std::string error;
            REQUIRE(eq.evaluate(std::vector<int>(80, 0), value, error));
        }
    }
    setLanguage(Language::German);
    REQUIRE(packPids("hyundai-kona-niro-ev").front().name == "Ladestand (BMS)");
    setLanguage(Language::English);
    REQUIRE(packPids("hyundai-kona-niro-ev").front().name == "State of charge (BMS)");
    setLanguage(Language::German);
}

TEST_CASE("The custom category has a name and a place", "[custom]") {
    const std::vector<PidCategory>& all = pidCategories();
    REQUIRE(std::find(all.begin(), all.end(), PidCategory::Custom) != all.end());
    setLanguage(Language::English);
    REQUIRE(std::string(pidCategoryName(PidCategory::Custom)) == "Manufacturer and custom values");
    setLanguage(Language::German);
    REQUIRE(std::string(pidCategoryName(PidCategory::Custom)) == "Hersteller und eigene Werte");
}
