// The small JSON writer of the console tool.

#include <catch2/catch_test_macros.hpp>

#include <cmath>
#include <limits>
#include <string>

#include "core/JsonWriter.h"

using namespace caruspex;

namespace {

// One string value as the writer puts it out, without the line break.
std::string quoted(const std::string& s) {
    JsonWriter json;
    json.value(s);
    std::string out = json.text();
    out.pop_back();
    return out;
}

}  // namespace

TEST_CASE("JSON strings are escaped", "[json]") {
    REQUIRE(quoted("plain") == "\"plain\"");
    REQUIRE(quoted("say \"hi\"") == "\"say \\\"hi\\\"\"");
    REQUIRE(quoted("C:\\obd") == "\"C:\\\\obd\"");
    REQUIRE(quoted("a\nb\r\tc") == "\"a\\nb\\r\\tc\"");
    REQUIRE(quoted(std::string("\x01\x1f", 2)) == "\"\\u0001\\u001f\"");
    REQUIRE(quoted(std::string("a\0b", 3)) == "\"a\\u0000b\"");
    REQUIRE(quoted("/") == "\"/\"");

    // Well-formed UTF-8 stays as it is.
    REQUIRE(quoted("Kühlmittel 90 °C, Škoda, \xE2\x82\xAC, \xF0\x9F\x9A\x97") ==
            "\"Kühlmittel 90 °C, Škoda, \xE2\x82\xAC, \xF0\x9F\x9A\x97\"");

    // Anything else becomes U+FFFD, byte by byte.
    REQUIRE(quoted("ELM\xFF") == "\"ELM\\ufffd\"");
    REQUIRE(quoted("\x80") == "\"\\ufffd\"");                 // stray continuation byte
    REQUIRE(quoted("\xC3") == "\"\\ufffd\"");                 // cut off
    REQUIRE(quoted("\xC0\xAF") == "\"\\ufffd\\ufffd\"");      // overlong
    REQUIRE(quoted("\xED\xA0\x80") == "\"\\ufffd\\ufffd\\ufffd\"");   // surrogate
    REQUIRE(quoted("\xC3x") == "\"\\ufffdx\"");
}

TEST_CASE("JSON structure, numbers and indentation", "[json]") {
    JsonWriter json;
    json.beginObject()
        .field("name", "demo")
        .field("count", 3)
        .field("ok", true)
        .field("ratio", 0.25)
        .field("nan", std::numeric_limits<double>::quiet_NaN())
        .field("inf", std::numeric_limits<double>::infinity())
        .key("none")
        .null()
        .key("empty")
        .beginArray()
        .endArray()
        .key("list")
        .beginArray()
        .value(1)
        .beginObject()
        .endObject()
        .endArray()
        .stringList("words", {"a", "b"})
        .endObject();
    REQUIRE(json.text() ==
            "{\n"
            "  \"name\": \"demo\",\n"
            "  \"count\": 3,\n"
            "  \"ok\": true,\n"
            "  \"ratio\": 0.25,\n"
            "  \"nan\": null,\n"
            "  \"inf\": null,\n"
            "  \"none\": null,\n"
            "  \"empty\": [],\n"
            "  \"list\": [\n"
            "    1,\n"
            "    {}\n"
            "  ],\n"
            "  \"words\": [\n"
            "    \"a\",\n"
            "    \"b\"\n"
            "  ]\n"
            "}\n");

    JsonWriter numbers;
    numbers.beginArray().value(-0.5).value(1234.5678).value(1e-7).value(-12).endArray();
    REQUIRE(numbers.text() == "[\n  -0.5,\n  1234.5678,\n  1e-07,\n  -12\n]\n");
}
