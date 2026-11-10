// J2534 end to end through a real driver DLL (tests/fake_j2534.cpp): loading,
// connecting CAN, the pass filter, own echoes skipped, ISO-TP with flow
// control and the battery voltage. Windows only, like J2534 itself.

#include <catch2/catch_test_macros.hpp>

#include "core/CanBus.h"
#include "core/Elm327.h"
#include "core/Obd2.h"

using namespace caruspex;

#ifdef CARUSPEX_FAKE_J2534
TEST_CASE("J2534: a PassThru driver DLL carries OBD on CAN", "[can][j2534]") {
    std::string error;
    auto link = openJ2534(CARUSPEX_FAKE_J2534, 500000, error);
    REQUIRE(link);
    REQUIRE(link->batteryVoltage() > 12.4);
    Elm327 elm;
    REQUIRE(elm.connect(std::make_unique<CanElm>(std::move(link), "J2534 test", 500000), "6", error));
    Obd2 obd(elm);
    REQUIRE(obd.readVin() == "WAUZZZ8V5LA012345");
    REQUIRE(elm.send("ATRV", 1000).lines.front() == "12.5V");

    // A wrong bit rate is refused by the driver with a clear message.
    REQUIRE_FALSE(openJ2534(CARUSPEX_FAKE_J2534, 250000, error));
    REQUIRE_FALSE(error.empty());
    REQUIRE_FALSE(openJ2534("C:/no/such/driver.dll", 500000, error));
}
#endif
