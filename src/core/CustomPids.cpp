#include "core/CustomPids.h"

#include <algorithm>
#include <cctype>
#include <cmath>
#include <cstddef>
#include <cstdio>
#include <map>
#include <utility>

#include "core/EcuAddress.h"
#include "core/Lang.h"
#include "core/Uds.h"
#include "data/EcuAddresses.h"

namespace caruspex {
namespace {

// ---------------------------------------------------------------------------
// Small text helpers
// ---------------------------------------------------------------------------

std::string trim(const std::string& s) {
    const size_t a = s.find_first_not_of(" \t\r\n");
    if (a == std::string::npos) return {};
    const size_t e = s.find_last_not_of(" \t\r\n");
    return s.substr(a, e - a + 1);
}

std::string upper(std::string s) {
    std::transform(s.begin(), s.end(), s.begin(),
                   [](unsigned char c) { return static_cast<char>(std::toupper(c)); });
    return s;
}

std::string lower(std::string s) {
    std::transform(s.begin(), s.end(), s.begin(),
                   [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
    return s;
}

bool isHexText(const std::string& s) {
    if (s.empty()) return false;
    return std::all_of(s.begin(), s.end(),
                       [](unsigned char c) { return std::isxdigit(c) != 0; });
}

std::string stripHexPrefix(const std::string& s) {
    if (s.size() >= 2 && s[0] == '0' && (s[1] == 'x' || s[1] == 'X')) return s.substr(2);
    return s;
}

// A decimal number in the C locale: optional sign, digits, optional point
// and fraction, optional exponent. strtod would follow the process locale,
// and the GUI may run under a German one where the point is a comma.
bool parseNumber(const std::string& text, double& out) {
    const std::string s = trim(text);
    size_t i = 0;
    bool negative = false;
    if (i < s.size() && (s[i] == '+' || s[i] == '-')) negative = s[i++] == '-';
    double value = 0.0;
    bool digits = false;
    while (i < s.size() && std::isdigit(static_cast<unsigned char>(s[i]))) {
        value = value * 10.0 + (s[i++] - '0');
        digits = true;
    }
    if (i < s.size() && s[i] == '.') {
        ++i;
        double scale = 0.1;
        while (i < s.size() && std::isdigit(static_cast<unsigned char>(s[i]))) {
            value += (s[i++] - '0') * scale;
            scale /= 10.0;
            digits = true;
        }
    }
    if (!digits) return false;
    if (i < s.size() && (s[i] == 'e' || s[i] == 'E')) {
        ++i;
        bool expNegative = false;
        if (i < s.size() && (s[i] == '+' || s[i] == '-')) expNegative = s[i++] == '-';
        int exponent = 0;
        bool expDigits = false;
        while (i < s.size() && std::isdigit(static_cast<unsigned char>(s[i]))) {
            exponent = std::min(exponent * 10 + (s[i++] - '0'), 400);
            expDigits = true;
        }
        if (!expDigits) return false;
        value *= std::pow(10.0, expNegative ? -exponent : exponent);
    }
    if (i != s.size() || !std::isfinite(value)) return false;
    out = negative ? -value : value;
    return true;
}

// The opposite, also independent of the locale: printf may write a comma.
std::string formatNumber(double value) {
    char buffer[40];
    std::snprintf(buffer, sizeof(buffer), "%.10g", value);
    std::string out = buffer;
    std::replace(out.begin(), out.end(), ',', '.');
    return out;
}

std::string lineText(int line) {
    return std::string(L("Zeile ", "Line ")) + std::to_string(line) + ": ";
}

}  // namespace

// ---------------------------------------------------------------------------
// Equations
// ---------------------------------------------------------------------------

namespace {

enum class Op {
    Number, Byte, Negate, Add, Subtract, Multiply, Divide, ShiftLeft, ShiftRight,
    Signed8, Signed16, Signed24, Signed32, Int16, Int24, Int32, Min, Max, Abs, Int, Bit,
};

struct FunctionSpec {
    const char* name;
    Op op;
    int arguments;
};

const FunctionSpec kFunctions[] = {
    {"SIGNED", Op::Signed8, 1},   {"SIGNED8", Op::Signed8, 1},   {"SIGNED16", Op::Signed16, 1},
    {"SIGNED24", Op::Signed24, 1}, {"SIGNED32", Op::Signed32, 1}, {"INT16", Op::Int16, 2},
    {"INT24", Op::Int24, 3},       {"INT32", Op::Int32, 4},       {"MIN", Op::Min, 2},
    {"MAX", Op::Max, 2},           {"ABS", Op::Abs, 1},           {"INT", Op::Int, 1},
    {"BIT", Op::Bit, 2},
};

// Deeper than any sensible equation, shallow enough for the stack.
constexpr int kMaxDepth = 64;

std::string byteName(int index) {
    if (index < 26) return std::string(1, static_cast<char>('A' + index));
    index -= 26;
    return std::string(1, static_cast<char>('A' + index / 26)) +
           static_cast<char>('A' + index % 26);
}

}  // namespace

struct TorqueEquation::Program {
    struct Node {
        Op op = Op::Number;
        double number = 0.0;      // Number
        int index = 0;            // Byte
        std::vector<int> args;    // children, by position in nodes
    };
    std::vector<Node> nodes;
    int root = -1;
};

namespace {

using Program = TorqueEquation::Program;
using Node = Program::Node;

// Recursive descent over the grammar
//   expr    := shift
//   shift   := sum (('<' | '>') sum)*
//   sum     := product (('+' | '-') product)*
//   product := unary (('*' | '/') unary)*
//   unary   := ('-' | '+') unary | primary
//   primary := number | byte | '(' expr ')' | '{' byte ':' integer '}'
//            | function '(' expr (':' expr)* ')'
class Parser {
public:
    Parser(const std::string& text, Program& program) : s_(text), p_(program) {}

    bool parse(std::string& error, int& bytesNeeded) {
        const int root = expression(0);
        if (root >= 0) {
            skipSpace();
            if (pos_ < s_.size()) fail(L("Unerwartetes Zeichen", "Unexpected character"));
        }
        if (!error_.empty()) {
            error = error_;
            return false;
        }
        p_.root = root;
        bytesNeeded = bytesNeeded_;
        return true;
    }

private:
    void skipSpace() {
        while (pos_ < s_.size() && std::isspace(static_cast<unsigned char>(s_[pos_]))) ++pos_;
    }
    char peek() {
        skipSpace();
        return pos_ < s_.size() ? s_[pos_] : '\0';
    }
    int fail(const char* what) {
        if (error_.empty()) {
            error_ = what;
            error_ += L(" an Stelle ", " at position ");
            error_ += std::to_string(pos_ + 1);
        }
        return -1;
    }
    int add(Node node) {
        p_.nodes.push_back(std::move(node));
        return static_cast<int>(p_.nodes.size()) - 1;
    }
    int binary(Op op, int a, int b) {
        Node n;
        n.op = op;
        n.args = {a, b};
        return add(std::move(n));
    }

    int expression(int depth) {
        if (depth > kMaxDepth)
            return fail(L("Gleichung zu tief verschachtelt", "Equation nested too deeply"));
        int left = sum(depth);
        for (;;) {
            if (left < 0) return -1;
            const char c = peek();
            if (c != '<' && c != '>') return left;
            ++pos_;
            const int right = sum(depth);
            if (right < 0) return -1;
            left = binary(c == '<' ? Op::ShiftLeft : Op::ShiftRight, left, right);
        }
    }
    int sum(int depth) {
        int left = product(depth);
        for (;;) {
            if (left < 0) return -1;
            const char c = peek();
            if (c != '+' && c != '-') return left;
            ++pos_;
            const int right = product(depth);
            if (right < 0) return -1;
            left = binary(c == '+' ? Op::Add : Op::Subtract, left, right);
        }
    }
    int product(int depth) {
        int left = unary(depth);
        for (;;) {
            if (left < 0) return -1;
            const char c = peek();
            if (c != '*' && c != '/') return left;
            ++pos_;
            const int right = unary(depth);
            if (right < 0) return -1;
            left = binary(c == '*' ? Op::Multiply : Op::Divide, left, right);
        }
    }
    int unary(int depth) {
        if (depth > kMaxDepth)
            return fail(L("Gleichung zu tief verschachtelt", "Equation nested too deeply"));
        const char c = peek();
        if (c == '+') {
            ++pos_;
            return unary(depth + 1);
        }
        if (c == '-') {
            ++pos_;
            const int inner = unary(depth + 1);
            if (inner < 0) return -1;
            Node n;
            n.op = Op::Negate;
            n.args = {inner};
            return add(std::move(n));
        }
        return primary(depth);
    }

    int byteNode(int index) {
        bytesNeeded_ = std::max(bytesNeeded_, index + 1);
        Node n;
        n.op = Op::Byte;
        n.index = index;
        return add(std::move(n));
    }

    std::string identifier() {
        const size_t start = pos_;
        while (pos_ < s_.size() && std::isalnum(static_cast<unsigned char>(s_[pos_]))) ++pos_;
        return s_.substr(start, pos_ - start);
    }

    int primary(int depth) {
        const char c = peek();
        if (c == '\0') return fail(L("Gleichung endet zu früh", "Equation ends too early"));
        if (c == '(') {
            ++pos_;
            const int inner = expression(depth + 1);
            if (inner < 0) return -1;
            if (peek() != ')') return fail(L("Schließende Klammer fehlt", "Missing closing parenthesis"));
            ++pos_;
            return inner;
        }
        if (std::isdigit(static_cast<unsigned char>(c)) || c == '.') {
            const size_t start = pos_;
            while (pos_ < s_.size() &&
                   (std::isdigit(static_cast<unsigned char>(s_[pos_])) || s_[pos_] == '.'))
                ++pos_;
            double value = 0.0;
            if (!parseNumber(s_.substr(start, pos_ - start), value)) {
                pos_ = start;
                return fail(L("Ungültige Zahl", "Invalid number"));
            }
            Node n;
            n.op = Op::Number;
            n.number = value;
            return add(std::move(n));
        }
        if (c == '{') {
            // {A:3}: bit 3 of byte A.
            ++pos_;
            skipSpace();
            const int index = TorqueEquation::byteIndex(identifier());
            if (index < 0) return fail(L("Byte wie A oder AB erwartet", "Byte like A or AB expected"));
            if (peek() != ':') return fail(L("Doppelpunkt erwartet", "Colon expected"));
            ++pos_;
            skipSpace();
            const size_t start = pos_;
            while (pos_ < s_.size() && std::isdigit(static_cast<unsigned char>(s_[pos_]))) ++pos_;
            if (start == pos_ || pos_ - start > 2) return fail(L("Bitnummer erwartet", "Bit number expected"));
            const int bit = std::stoi(s_.substr(start, pos_ - start));
            if (bit > 31) return fail(L("Bitnummer zu groß", "Bit number too large"));
            if (peek() != '}') return fail(L("Schließende Klammer fehlt", "Missing closing brace"));
            ++pos_;
            Node number;
            number.op = Op::Number;
            number.number = bit;
            return binary(Op::Bit, byteNode(index), add(std::move(number)));
        }
        if (std::isalpha(static_cast<unsigned char>(c))) {
            const size_t start = pos_;
            const std::string name = upper(identifier());
            const char next = peek();
            if (next == '{') {
                pos_ = start;
                return fail(L("Verweise auf andere Werte (val{...}) werden nicht unterstützt",
                              "References to other values (val{...}) are not supported"));
            }
            if (next != '(') {
                const int index = TorqueEquation::byteIndex(name);
                if (index < 0) {
                    pos_ = start;
                    return fail(L("Unbekannter Name", "Unknown name"));
                }
                return byteNode(index);
            }
            const FunctionSpec* spec = nullptr;
            for (const FunctionSpec& f : kFunctions) {
                if (name == f.name) spec = &f;
            }
            if (!spec) {
                pos_ = start;
                return fail(L("Unbekannte Funktion", "Unknown function"));
            }
            ++pos_;   // '('
            Node call;
            call.op = spec->op;
            for (;;) {
                const int arg = expression(depth + 1);
                if (arg < 0) return -1;
                call.args.push_back(arg);
                const char sep = peek();
                // Torque separates arguments with ':'; a comma is taken too.
                if (sep == ':' || sep == ',') {
                    ++pos_;
                    continue;
                }
                if (sep == ')') {
                    ++pos_;
                    break;
                }
                return fail(L("Schließende Klammer fehlt", "Missing closing parenthesis"));
            }
            if (static_cast<int>(call.args.size()) != spec->arguments) {
                pos_ = start;
                return fail(L("Falsche Anzahl von Argumenten", "Wrong number of arguments"));
            }
            return add(std::move(call));
        }
        return fail(L("Unerwartetes Zeichen", "Unexpected character"));
    }

    const std::string& s_;
    Program& p_;
    size_t pos_ = 0;
    int bytesNeeded_ = 0;
    std::string error_;
};

// Whole number part as a 64-bit integer, if it fits comfortably.
bool toInteger(double v, long long& out) {
    if (!std::isfinite(v) || std::fabs(v) >= 9.0e18) return false;
    out = static_cast<long long>(std::trunc(v));
    return true;
}

struct Evaluator {
    const Program& program;
    const std::vector<int>& data;
    std::string error;

    bool fail(const char* de, const char* en) {
        if (error.empty()) error = L(de, en);
        return false;
    }

    bool integer(double v, long long& out) {
        if (toInteger(v, out)) return true;
        return fail("Zahl zu groß für eine Bitoperation.", "Number too large for a bit operation.");
    }

    bool signedBits(double v, int bits, double& out) {
        long long n = 0;
        if (!integer(v, n)) return false;
        const unsigned long long mask = (1ULL << bits) - 1ULL;
        const unsigned long long raw = static_cast<unsigned long long>(n) & mask;
        const unsigned long long sign = 1ULL << (bits - 1);
        out = raw >= sign ? static_cast<double>(raw) - std::ldexp(1.0, bits)
                          : static_cast<double>(raw);
        return true;
    }

    bool joinBytes(const Node& node, double& out) {
        double total = 0.0;
        for (int arg : node.args) {
            double b = 0.0;
            if (!eval(arg, b)) return false;
            long long n = 0;
            if (!integer(b, n)) return false;
            total = total * 256.0 + static_cast<double>(static_cast<unsigned long long>(n) & 0xFFULL);
        }
        out = total;
        return true;
    }

    bool shiftCount(double v, int& out) {
        long long n = 0;
        if (!integer(v, n)) return false;
        if (n < 0 || n > 62) return fail("Verschiebung außerhalb von 0 bis 62 Bit.",
                                         "Shift outside 0 to 62 bits.");
        out = static_cast<int>(n);
        return true;
    }

    bool eval(int index, double& out) {
        const Node& node = program.nodes[static_cast<size_t>(index)];
        double a = 0.0, b = 0.0;
        switch (node.op) {
            case Op::Number: out = node.number; return true;
            case Op::Byte:
                if (node.index >= static_cast<int>(data.size())) {
                    if (error.empty()) {
                        error = std::string(L("Antwort zu kurz: Byte ", "Answer too short: byte ")) +
                                byteName(node.index) + L(" fehlt (", " is missing (") +
                                std::to_string(data.size()) + L(" Datenbytes erhalten).", " data bytes received).");
                    }
                    return false;
                }
                out = data[static_cast<size_t>(node.index)] & 0xFF;
                return true;
            case Op::Negate:
                if (!eval(node.args[0], a)) return false;
                out = -a;
                return true;
            case Op::Signed8:
            case Op::Signed16:
            case Op::Signed24:
            case Op::Signed32: {
                if (!eval(node.args[0], a)) return false;
                const int bits = node.op == Op::Signed8 ? 8 : node.op == Op::Signed16 ? 16
                                 : node.op == Op::Signed24 ? 24 : 32;
                return signedBits(a, bits, out);
            }
            case Op::Int16:
            case Op::Int24:
            case Op::Int32: return joinBytes(node, out);
            case Op::Abs:
                if (!eval(node.args[0], a)) return false;
                out = std::fabs(a);
                return true;
            case Op::Int:
                if (!eval(node.args[0], a)) return false;
                out = std::trunc(a);
                return true;
            default: break;
        }
        // Everything else takes two arguments.
        if (!eval(node.args[0], a) || !eval(node.args[1], b)) return false;
        switch (node.op) {
            case Op::Add: out = a + b; return true;
            case Op::Subtract: out = a - b; return true;
            case Op::Multiply: out = a * b; return true;
            case Op::Divide:
                if (b == 0.0) return fail("Division durch null.", "Division by zero.");
                out = a / b;
                return true;
            case Op::ShiftLeft:
            case Op::ShiftRight: {
                long long n = 0;
                int count = 0;
                if (!integer(a, n) || !shiftCount(b, count)) return false;
                // ldexp is exact for these magnitudes and avoids undefined
                // behaviour of shifting negative numbers; floor makes the right
                // shift arithmetic, as on two's complement integers.
                out = node.op == Op::ShiftLeft ? std::ldexp(static_cast<double>(n), count)
                                               : std::floor(std::ldexp(static_cast<double>(n), -count));
                return true;
            }
            case Op::Min: out = std::min(a, b); return true;
            case Op::Max: out = std::max(a, b); return true;
            case Op::Bit: {
                long long n = 0;
                int count = 0;
                if (!integer(a, n) || !shiftCount(b, count)) return false;
                out = static_cast<double>((static_cast<unsigned long long>(n) >> count) & 1ULL);
                return true;
            }
            default: break;
        }
        return fail("Unbekannte Operation.", "Unknown operation.");
    }
};

}  // namespace

TorqueEquation::TorqueEquation(const std::string& text) : text_(text) {
    auto program = std::make_shared<Program>();
    const std::string trimmed = trim(text);
    if (trimmed.empty()) {
        error_ = L("Leere Gleichung", "Empty equation");
        return;
    }
    Parser parser(trimmed, *program);
    if (!parser.parse(error_, bytesNeeded_)) {
        bytesNeeded_ = 0;
        return;
    }
    program_ = std::move(program);
}

bool TorqueEquation::evaluate(const std::vector<int>& data, double& value, std::string& error) const {
    error.clear();
    if (!program_) {
        error = error_;
        return false;
    }
    Evaluator ev{*program_, data, {}};
    double result = 0.0;
    if (!ev.eval(program_->root, result)) {
        error = ev.error;
        return false;
    }
    if (!std::isfinite(result)) {
        error = L("Ergebnis ist keine endliche Zahl.", "Result is not a finite number.");
        return false;
    }
    value = result;
    return true;
}

int TorqueEquation::byteIndex(const std::string& name) {
    const std::string n = upper(name);
    auto letter = [](char c) { return c >= 'A' && c <= 'Z'; };
    if (n.size() == 1 && letter(n[0])) return n[0] - 'A';
    if (n.size() == 2 && letter(n[0]) && letter(n[1])) return 26 + (n[0] - 'A') * 26 + (n[1] - 'A');
    return -1;
}

// ---------------------------------------------------------------------------
// Torque CSV
// ---------------------------------------------------------------------------

bool parseRequestHex(const std::string& text, std::vector<int>& bytes) {
    bytes.clear();
    std::string s = stripHexPrefix(trim(text));
    s.erase(std::remove(s.begin(), s.end(), ' '), s.end());
    if (!isHexText(s) || s.size() % 2 != 0 || s.size() < 2 || s.size() > 14) return false;
    for (size_t i = 0; i < s.size(); i += 2) bytes.push_back(std::stoi(s.substr(i, 2), nullptr, 16));
    return true;
}

namespace {

// One CSV record with the line it started on.
struct Record {
    int line = 0;
    std::vector<std::string> cells;
};

// Splits the text into records. Quoted fields may hold commas, doubled
// quotes and line breaks. A line whose first non-blank character is '#'
// outside a quoted field is a comment.
std::vector<Record> splitCsv(const std::string& text) {
    std::vector<Record> out;
    size_t i = 0;
    int line = 1;
    // A UTF-8 byte order mark, as spreadsheet programs like to write.
    if (text.compare(0, 3, "\xEF\xBB\xBF") == 0) i = 3;
    while (i < text.size()) {
        // Start of a line: comment or blank?
        size_t j = i;
        while (j < text.size() && (text[j] == ' ' || text[j] == '\t')) ++j;
        if (j < text.size() && text[j] == '#') {
            while (j < text.size() && text[j] != '\n') ++j;
            i = j + 1;
            ++line;
            continue;
        }
        Record record;
        record.line = line;
        std::string cell;
        bool quoted = false;
        bool done = false;
        while (i < text.size() && !done) {
            const char c = text[i];
            if (quoted) {
                if (c == '"') {
                    if (i + 1 < text.size() && text[i + 1] == '"') {
                        cell += '"';
                        i += 2;
                        continue;
                    }
                    quoted = false;
                } else {
                    if (c == '\n') ++line;
                    cell += c;
                }
                ++i;
                continue;
            }
            if (c == '"') quoted = true;
            else if (c == ',') {
                record.cells.push_back(cell);
                cell.clear();
            } else if (c == '\n') {
                ++line;
                done = true;
            } else if (c != '\r') cell += c;
            ++i;
        }
        record.cells.push_back(cell);
        bool empty = true;
        for (const std::string& s : record.cells) empty = empty && trim(s).empty();
        if (!empty) out.push_back(std::move(record));
    }
    return out;
}

enum Column { ColName, ColShort, ColRequest, ColEquation, ColMin, ColMax, ColUnit, ColHeader,
              ColResponse, ColumnCount };

int columnFor(const std::string& title) {
    std::string t;
    for (char c : lower(trim(title))) {
        if (c != ' ' && c != '_' && c != '-') t += c;
    }
    if (t == "name") return ColName;
    if (t == "shortname") return ColShort;
    if (t == "modeandpid" || t == "pid") return ColRequest;
    if (t == "equation") return ColEquation;
    if (t == "minvalue" || t == "min") return ColMin;
    if (t == "maxvalue" || t == "max") return ColMax;
    if (t == "units" || t == "unit") return ColUnit;
    if (t == "header") return ColHeader;
    if (t == "response") return ColResponse;
    return -1;
}

// "7E4" -> "7E4", "0x7e4" -> "7E4", "DA44F1" -> "18DA44F1". False if it is
// no CAN id. bits29 tells the width.
bool normalizeId(const std::string& text, std::string& out, bool& bits29) {
    std::string s = upper(stripHexPrefix(trim(text)));
    if (!isHexText(s)) return false;
    if (s.size() <= 3) {
        if (std::stoul(s, nullptr, 16) > 0x7FF) return false;
        while (s.size() < 3) s = "0" + s;
        bits29 = false;
    } else if (s.size() == 6 || s.size() == 8) {
        if (s.size() == 6) s = "18" + s;   // the usual priority byte
        if (std::stoul(s, nullptr, 16) > 0x1FFFFFFFUL) return false;
        bits29 = true;
    } else {
        return false;
    }
    out = s;
    return true;
}

// The id a control unit answers on when nothing else is said.
std::string defaultResponse(const std::string& header) {
    const unsigned long id = std::stoul(header, nullptr, 16);
    char buffer[16];
    if (header.size() > 3) {
        // 18DA<target><source>: the answer swaps target and source.
        const unsigned long r = (id & 0xFFFF0000UL) | ((id & 0xFFUL) << 8) | ((id >> 8) & 0xFFUL);
        std::snprintf(buffer, sizeof(buffer), "%08lX", r);
    } else {
        std::snprintf(buffer, sizeof(buffer), "%03lX", (id + 8) & 0x7FFUL);
    }
    return buffer;
}

std::string csvField(const std::string& s) {
    const bool needsQuotes = s.find_first_of(",\"\r\n") != std::string::npos ||
                             (!s.empty() && s.front() == '#');
    if (!needsQuotes) return s;
    std::string out = "\"";
    for (char c : s) {
        if (c == '"') out += '"';
        out += c;
    }
    return out + "\"";
}

}  // namespace

std::vector<CustomPid> parseTorqueCsv(const std::string& text, std::vector<std::string>& errors,
                                      int keyBase) {
    std::vector<CustomPid> out;
    std::vector<Record> records = splitCsv(text);

    // Column layout: by the header line if there is one, Torque's order otherwise.
    int map[ColumnCount];
    for (int c = 0; c < ColumnCount; ++c) map[c] = c;
    size_t first = 0;
    if (!records.empty()) {
        const std::vector<std::string>& head = records.front().cells;
        bool isHeader = false;
        for (const std::string& title : head) {
            const int col = columnFor(title);
            isHeader = isHeader || col == ColRequest || col == ColEquation;
        }
        if (isHeader) {
            for (int c = 0; c < ColumnCount; ++c) map[c] = -1;
            for (size_t i = 0; i < head.size(); ++i) {
                const int col = columnFor(head[i]);
                if (col >= 0 && map[col] < 0) map[col] = static_cast<int>(i);
            }
            first = 1;
            if (map[ColName] < 0 || map[ColRequest] < 0 || map[ColEquation] < 0) {
                errors.push_back(lineText(records.front().line) +
                                 L("Kopfzeile ohne die Spalten Name, ModeAndPID und Equation.",
                                   "Header line without the columns Name, ModeAndPID and Equation."));
                return out;
            }
        }
    }

    for (size_t r = first; r < records.size(); ++r) {
        const Record& record = records[r];
        auto cell = [&](int col) -> std::string {
            const int i = map[col];
            if (i < 0 || static_cast<size_t>(i) >= record.cells.size()) return {};
            return trim(record.cells[static_cast<size_t>(i)]);
        };
        auto reject = [&](const std::string& why) { errors.push_back(lineText(record.line) + why); };

        CustomPid pid;
        pid.name = cell(ColName);
        pid.shortName = cell(ColShort);
        pid.equation = cell(ColEquation);
        pid.unit = cell(ColUnit);
        if (pid.name.empty()) {
            reject(L("Name fehlt.", "Name is missing."));
            continue;
        }
        std::vector<int> bytes;
        const std::string request = cell(ColRequest);
        if (!parseRequestHex(request, bytes)) {
            reject(request.empty() ? L("ModeAndPID fehlt.", "ModeAndPID is missing.")
                                   : std::string(L("ModeAndPID ist keine Anfrage in Hex: ",
                                                   "ModeAndPID is no hex request: ")) + request);
            continue;
        }
        if (bytes.size() < 2) {
            reject(L("ModeAndPID braucht Dienst und Kennung, z. B. 220101.",
                     "ModeAndPID needs service and identifier, e.g. 220101."));
            continue;
        }
        pid.request.clear();
        for (int b : bytes) {
            char hex[4];
            std::snprintf(hex, sizeof(hex), "%02X", b);
            pid.request += hex;
        }

        const TorqueEquation equation(pid.equation);
        if (!equation.ok()) {
            reject(std::string(L("Gleichung: ", "Equation: ")) + equation.error());
            continue;
        }

        const std::string minText = cell(ColMin), maxText = cell(ColMax);
        if ((!minText.empty() && !parseNumber(minText, pid.minValue)) ||
            (!maxText.empty() && !parseNumber(maxText, pid.maxValue))) {
            reject(L("Min oder Max ist keine Zahl.", "Min or max is not a number."));
            continue;
        }

        // Header, optionally with the response id: "79B", "79B/7BB", "79B:7BB".
        std::string header = cell(ColHeader);
        std::string response = cell(ColResponse);
        const size_t split = header.find_first_of("/:");
        if (split != std::string::npos) {
            if (response.empty()) response = header.substr(split + 1);
            header = header.substr(0, split);
        }
        bool headerWide = false, responseWide = false;
        if (!trim(header).empty() && !normalizeId(header, pid.header, headerWide)) {
            reject(std::string(L("Header ist keine CAN-Adresse: ", "Header is no CAN id: ")) + header);
            continue;
        }
        if (!trim(response).empty()) {
            if (pid.header.empty()) {
                reject(L("Response ohne Header.", "Response without a header."));
                continue;
            }
            if (!normalizeId(response, pid.response, responseWide) || responseWide != headerWide) {
                reject(std::string(L("Response ist keine passende CAN-Adresse: ",
                                     "Response is no matching CAN id: ")) + response);
                continue;
            }
        }
        if (pid.shortName.empty()) pid.shortName = pid.name;
        pid.key = keyBase + static_cast<int>(out.size());
        out.push_back(std::move(pid));
    }
    return out;
}

std::string writeTorqueCsv(const std::vector<CustomPid>& pids) {
    const bool withResponse = std::any_of(pids.begin(), pids.end(),
                                          [](const CustomPid& p) { return !p.response.empty(); });
    std::string out = "Name,ShortName,ModeAndPID,Equation,Min Value,Max Value,Units,Header";
    if (withResponse) out += ",Response";
    out += "\n";
    for (const CustomPid& p : pids) {
        out += csvField(p.name) + "," + csvField(p.shortName) + "," + csvField(p.request) + "," +
               csvField(p.equation) + "," + formatNumber(p.minValue) + "," +
               formatNumber(p.maxValue) + "," + csvField(p.unit) + "," + csvField(p.header);
        if (withResponse) out += "," + csvField(p.response);
        out += "\n";
    }
    return out;
}

// ---------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------

namespace {

constexpr int kRequestTimeoutMs = 2000;

// Where to ask, as the adapter sees it. Empty for the default addressing.
std::string addressKey(const CustomPid& pid) {
    if (pid.header.empty()) return {};
    return pid.header + "/" + (pid.response.empty() ? defaultResponse(pid.header) : pid.response);
}

// One answer: the data bytes after the echo, or why there are none.
struct Answer {
    bool ok = false;
    std::vector<int> data;
    std::string error;
};

}  // namespace

struct CustomPidReader::Impl {
    Elm327& elm;
    Uds uds;
    bool addressed = false;
    std::string current;    // addressKey the adapter points at
    std::map<std::string, TorqueEquation> equations;

    explicit Impl(Elm327& e) : elm(e), uds(e) {}

    bool address(const CustomPid& pid, std::string& error) {
        const std::string key = addressKey(pid);
        if (key.empty()) {
            if (addressed) uds.restoreDefault();
            addressed = false;
            current.clear();
            return true;
        }
        if (addressed && key == current) return true;
        EcuDefinition def;
        def.request = static_cast<std::uint32_t>(std::stoul(pid.header, nullptr, 16));
        const std::string response = pid.response.empty() ? defaultResponse(pid.header) : pid.response;
        def.response = static_cast<std::uint32_t>(std::stoul(response, nullptr, 16));
        def.extended29 = pid.header.size() > 3;
        def.name = "ECU " + pid.header;
        addressed = false;
        current.clear();
        if (!uds.select(def, error)) return false;   // select puts the default back itself
        addressed = true;
        current = key;
        return true;
    }

    Answer ask(const CustomPid& pid) {
        Answer answer;
        std::vector<int> request;
        if (!parseRequestHex(pid.request, request) || request.size() < 2) {
            answer.error = L("Ungültige Anfrage.", "Invalid request.");
            return answer;
        }
        if (!address(pid, answer.error)) return answer;
        std::vector<int> response;
        if (!uds.request(request, response, answer.error, kRequestTimeoutMs)) return answer;
        // The positive answer repeats everything after the service byte
        // (the identifier), then come the data bytes.
        const size_t echo = request.size() - 1;
        if (response.size() < echo ||
            !std::equal(request.begin() + 1, request.end(), response.begin())) {
            answer.error = L("Die Antwort passt nicht zur Anfrage.", "The answer does not match the request.");
            return answer;
        }
        answer.data.assign(response.begin() + static_cast<std::ptrdiff_t>(echo), response.end());
        answer.ok = true;
        return answer;
    }

    const TorqueEquation& equation(const std::string& text) {
        auto it = equations.find(text);
        if (it == equations.end()) it = equations.emplace(text, TorqueEquation(text)).first;
        return it->second;
    }

    bool decode(const CustomPid& pid, const Answer& answer, LiveValue& out, std::string& error) {
        out = LiveValue{};
        out.pid = 0;
        out.sub = 0;
        out.key = pid.key;
        out.name = pid.name;
        out.unit = pid.unit;
        out.category = pid.category;
        out.displayMin = pid.minValue;
        out.displayMax = pid.maxValue;
        if (!pid.header.empty()) {
            out.ecuSource = pid.response.empty() ? defaultResponse(pid.header) : pid.response;
            out.ecu = ecuDisplayName(out.ecuSource);
        }
        if (!answer.ok) {
            error = answer.error;
            return false;
        }
        return equation(pid.equation).evaluate(answer.data, out.value, error);
    }
};

CustomPidReader::CustomPidReader(Elm327& elm) : impl_(std::make_unique<Impl>(elm)) {}

CustomPidReader::~CustomPidReader() { restoreDefault(); }

void CustomPidReader::restoreDefault() {
    if (impl_->addressed) impl_->uds.restoreDefault();
    impl_->addressed = false;
    impl_->current.clear();
}

bool CustomPidReader::read(const CustomPid& pid, LiveValue& out, std::string& error) {
    error.clear();
    const TorqueEquation& eq = impl_->equation(pid.equation);
    if (!eq.ok()) {
        error = eq.error();
        return false;
    }
    return impl_->decode(pid, impl_->ask(pid), out, error);
}

std::vector<CustomReading> CustomPidReader::readAll(const std::vector<CustomPid>& pids) {
    std::vector<CustomReading> out(pids.size());

    // Group by control unit, in the order the units first appear; the
    // control unit the adapter already points at goes first.
    std::vector<std::string> order;
    for (const CustomPid& pid : pids) {
        const std::string key = addressKey(pid);
        if (std::find(order.begin(), order.end(), key) == order.end()) order.push_back(key);
    }
    const std::string current = impl_->addressed ? impl_->current : std::string();
    auto here = std::find(order.begin(), order.end(), current);
    if (here != order.end()) std::rotate(order.begin(), here, here + 1);

    for (const std::string& unit : order) {
        std::map<std::string, Answer> answers;   // by request
        for (size_t i = 0; i < pids.size(); ++i) {
            const CustomPid& pid = pids[i];
            if (addressKey(pid) != unit) continue;
            CustomReading& reading = out[i];
            const TorqueEquation& eq = impl_->equation(pid.equation);
            if (!eq.ok()) {
                reading.value.key = pid.key;
                reading.value.name = pid.name;
                reading.error = eq.error();
                continue;
            }
            auto it = answers.find(pid.request);
            if (it == answers.end()) it = answers.emplace(pid.request, impl_->ask(pid)).first;
            reading.ok = impl_->decode(pid, it->second, reading.value, reading.error);
        }
    }
    return out;
}

bool readCustom(Elm327& elm, const CustomPid& pid, LiveValue& out, std::string& error) {
    CustomPidReader reader(elm);
    return reader.read(pid, out, error);
}

}  // namespace caruspex
