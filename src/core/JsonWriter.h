// A small JSON writer for machine-readable output (the console tool's
// --json, reports). No dependency, nothing clever: containers are opened and
// closed explicitly, inside an object every value is preceded by key(), and
// the output is indented by two spaces.
//
// Strings are escaped after RFC 8259. Bytes that are not well-formed UTF-8
// (a clone adapter may send any byte in its version string) become U+FFFD,
// so the output always parses. Numbers that are not finite become null.
#pragma once

#include <cmath>
#include <cstdio>
#include <string>
#include <vector>

namespace caruspex {

class JsonWriter {
public:
    JsonWriter& beginObject() { return open('{'); }
    JsonWriter& endObject() { return close('}'); }
    JsonWriter& beginArray() { return open('['); }
    JsonWriter& endArray() { return close(']'); }

    JsonWriter& key(const std::string& name) {
        separate();
        quote(name);
        out_ += ": ";
        afterKey_ = true;
        return *this;
    }

    JsonWriter& value(const std::string& s) {
        separate();
        quote(s);
        return *this;
    }
    JsonWriter& value(const char* s) { return s ? value(std::string(s)) : null(); }
    JsonWriter& value(bool b) {
        separate();
        out_ += b ? "true" : "false";
        return *this;
    }
    JsonWriter& value(int n) {
        separate();
        out_ += std::to_string(n);
        return *this;
    }
    JsonWriter& value(double d) {
        if (!std::isfinite(d)) return null();
        separate();
        char buffer[32];
        std::snprintf(buffer, sizeof(buffer), "%.10g", d);
        out_ += buffer;
        return *this;
    }
    JsonWriter& null() {
        separate();
        out_ += "null";
        return *this;
    }

    // Shorthands for a key with its value.
    template <typename T>
    JsonWriter& field(const std::string& name, const T& v) {
        key(name);
        return value(v);
    }
    JsonWriter& stringList(const std::string& name, const std::vector<std::string>& list) {
        key(name).beginArray();
        for (const std::string& s : list) value(s);
        return endArray();
    }

    std::string text() const { return out_ + "\n"; }

private:
    struct Level {
        bool empty = true;
    };

    JsonWriter& open(char bracket) {
        separate();
        out_ += bracket;
        levels_.push_back({});
        return *this;
    }

    JsonWriter& close(char bracket) {
        const bool empty = levels_.empty() || levels_.back().empty;
        if (!levels_.empty()) levels_.pop_back();
        if (!empty) newline();
        out_ += bracket;
        return *this;
    }

    // Comma and line break before the next element, nothing after a key.
    void separate() {
        if (afterKey_) {
            afterKey_ = false;
            return;
        }
        if (levels_.empty()) return;
        if (!levels_.back().empty) out_ += ',';
        levels_.back().empty = false;
        newline();
    }

    void newline() {
        out_ += '\n';
        out_.append(levels_.size() * 2, ' ');
    }

    void quote(const std::string& s) {
        out_ += '"';
        size_t i = 0;
        while (i < s.size()) {
            const unsigned char c = static_cast<unsigned char>(s[i]);
            if (c < 0x80) {
                switch (c) {
                    case '"': out_ += "\\\""; break;
                    case '\\': out_ += "\\\\"; break;
                    case '\b': out_ += "\\b"; break;
                    case '\f': out_ += "\\f"; break;
                    case '\n': out_ += "\\n"; break;
                    case '\r': out_ += "\\r"; break;
                    case '\t': out_ += "\\t"; break;
                    default:
                        if (c < 0x20) {
                            char buffer[8];
                            std::snprintf(buffer, sizeof(buffer), "\\u%04x", c);
                            out_ += buffer;
                        } else {
                            out_ += static_cast<char>(c);
                        }
                }
                ++i;
                continue;
            }
            const size_t length = utf8Length(s, i);
            if (length == 0) {
                out_ += "\\ufffd";
                ++i;
            } else {
                out_.append(s, i, length);
                i += length;
            }
        }
        out_ += '"';
    }

    // Length of the well-formed UTF-8 sequence starting at s[i], 0 if it is
    // not one (stray continuation byte, overlong form, surrogate, cut off).
    static size_t utf8Length(const std::string& s, size_t i) {
        const auto at = [&s](size_t k) { return static_cast<unsigned char>(s[k]); };
        const unsigned char c = at(i);
        size_t length = 0;
        unsigned min = 0, code = 0;
        if (c >= 0xC2 && c <= 0xDF) {
            length = 2;
            code = c & 0x1F;
            min = 0x80;
        } else if (c >= 0xE0 && c <= 0xEF) {
            length = 3;
            code = c & 0x0F;
            min = 0x800;
        } else if (c >= 0xF0 && c <= 0xF4) {
            length = 4;
            code = c & 0x07;
            min = 0x10000;
        } else {
            return 0;
        }
        if (i + length > s.size()) return 0;
        for (size_t k = 1; k < length; ++k) {
            if ((at(i + k) & 0xC0) != 0x80) return 0;
            code = (code << 6) | (at(i + k) & 0x3F);
        }
        if (code < min || code > 0x10FFFF || (code >= 0xD800 && code <= 0xDFFF)) return 0;
        return length;
    }

    std::string out_;
    std::vector<Level> levels_;
    bool afterKey_ = false;
};

}  // namespace caruspex
