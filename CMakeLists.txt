cmake_minimum_required(VERSION 3.21)
project(Caruspex VERSION 0.2.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)

if(MSVC)
    add_compile_options(/W3 /utf-8 /permissive- /MP)
    add_compile_definitions(_CRT_SECURE_NO_WARNINGS NOMINMAX WIN32_LEAN_AND_MEAN)
else()
    add_compile_options(-Wall -Wextra -Wno-unused-parameter)
endif()

option(CARUSPEX_GUI   "Build the graphical application (needs Qt 6)" ON)
option(CARUSPEX_TESTS "Build the tests (needs Catch2 via Conan)"       OFF)

# ---------------------------------------------------------------------------
# Core: transport, adapter drivers, diagnostic services, databases. No Qt.
# ---------------------------------------------------------------------------
if(WIN32)
    set(CARUSPEX_SERIAL src/core/Serial_win.cpp)
else()
    set(CARUSPEX_SERIAL src/core/Serial_posix.cpp)
endif()

add_library(caruspex_core STATIC
    ${CARUSPEX_SERIAL}
    src/core/Lang.cpp
    src/core/Transport.cpp
    src/core/Elm327.cpp
    src/core/EcuAddress.cpp
    src/core/Obd2.cpp
    src/core/DemoTransport.cpp
    src/core/AdapterProbe.cpp
    src/core/Kwp1281.cpp
    src/core/Kwp2000.cpp
    src/core/VagKLine.cpp
    src/core/KLineStream.cpp
    src/data/DtcDatabase.cpp
    src/data/DtcDatabase.gen.cpp
    src/data/Pids.cpp
    src/data/Vehicles.cpp
    src/data/DtcLocation.cpp
    src/data/VagFaults.cpp
    src/data/MonitorTests.cpp
    src/core/analysis/Common.cpp
    src/core/analysis/Readiness.cpp
    src/core/analysis/DriveCycleGuide.cpp
    src/core/analysis/HealthCheck.cpp
    src/core/analysis/TripComputer.cpp
    src/core/analysis/PerformanceTimer.cpp
    src/core/analysis/BatteryTest.cpp
)
target_include_directories(caruspex_core PUBLIC src)

if(WIN32)
    target_link_libraries(caruspex_core PUBLIC setupapi ws2_32)
endif()

find_package(Threads REQUIRED)
target_link_libraries(caruspex_core PUBLIC Threads::Threads)

# ---------------------------------------------------------------------------
# Command-line tool on the same core, scriptable.
# ---------------------------------------------------------------------------
add_executable(caruspex_cli src/cli_main.cpp)
target_link_libraries(caruspex_cli PRIVATE caruspex_core)

# ---------------------------------------------------------------------------
# Graphical application (Qt 6 Widgets and Charts)
# ---------------------------------------------------------------------------
if(CARUSPEX_GUI)
    find_package(Qt6 6.5 REQUIRED COMPONENTS Widgets Charts Svg PrintSupport LinguistTools)
    qt_standard_project_setup()

    qt_add_executable(caruspex
        resources/resources.qrc
        src/app/main.cpp
        src/app/MainWindow.cpp
        src/app/MainWindow.h
        src/app/Session.cpp
        src/app/Session.h
        src/app/Theme.cpp
        src/app/Theme.h
        src/app/GaugeWidget.cpp
        src/app/GaugeWidget.h
        src/app/Garage.cpp
        src/app/Garage.h
        src/app/Icons.cpp
        src/app/Icons.h
        src/app/Report.cpp
        src/app/Report.h
        src/app/VehicleState.cpp
        src/app/VehicleState.h
        src/app/Widgets.cpp
        src/app/Widgets.h
        src/app/TitleBar.cpp
        src/app/TitleBar.h
        src/app/VehicleDiagram.cpp
        src/app/VehicleDiagram.h
        src/app/pages/ConnectPage.cpp
        src/app/pages/ConnectPage.h
        src/app/pages/DashboardPage.cpp
        src/app/pages/DashboardPage.h
        src/app/pages/AdapterPage.cpp
        src/app/pages/AdapterPage.h
        src/app/pages/BatteryPage.cpp
        src/app/pages/BatteryPage.h
        src/app/pages/FaultsPage.cpp
        src/app/pages/GaragePage.cpp
        src/app/pages/GaragePage.h
        src/app/pages/HealthPage.cpp
        src/app/pages/HealthPage.h
        src/app/pages/FaultsPage.h
        src/app/pages/LivePage.cpp
        src/app/pages/LivePage.h
        src/app/pages/ToolsPage.cpp
        src/app/pages/TripPage.cpp
        src/app/pages/TripPage.h
        src/app/pages/ToolsPage.h
        src/app/pages/ThrottlePage.cpp
        src/app/pages/ThrottlePage.h
        src/app/pages/StatsPage.cpp
        src/app/pages/StatsPage.h
        src/app/pages/ScanPage.cpp
        src/app/pages/ScanPage.h
        src/app/pages/KklToolsPage.cpp
        src/app/pages/KklToolsPage.h
        src/app/pages/MonitorPage.cpp
        src/app/pages/PerformancePage.cpp
        src/app/pages/PerformancePage.h
        src/app/pages/MonitorPage.h
        src/app/pages/LexiconPage.cpp
        src/app/pages/LexiconPage.h
        src/app/pages/SettingsPage.cpp
        src/app/pages/SettingsPage.h
        src/app/Settings.cpp
        src/app/Settings.h
    )
    target_link_libraries(caruspex PRIVATE caruspex_core Qt6::Widgets Qt6::Charts Qt6::Svg
                                           Qt6::PrintSupport)

    # The source strings are German; the English translation is compiled into
    # the binary as :/i18n/caruspex_en.qm. Refresh the .ts file after changing
    # UI strings with: cmake --build build --target update_translations
    qt_add_translations(caruspex
        TS_FILES translations/caruspex_en.ts
        RESOURCE_PREFIX /i18n
        LUPDATE_OPTIONS -no-obsolete -locations none -source-language de -target-language en
    )
    set_target_properties(caruspex PROPERTIES
        RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin
        WIN32_EXECUTABLE ON      # no console window next to it on Windows
        MACOSX_BUNDLE ON         # a proper caruspex.app bundle on macOS
    )
endif()

# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------
if(CARUSPEX_TESTS)
    find_package(Catch2 3 REQUIRED)
    enable_testing()
    add_executable(caruspex_tests
        tests/test_protocol.cpp
        tests/test_obd_extended.cpp
        tests/test_kwp1281.cpp
        tests/test_kwp2000.cpp
        tests/test_settings.cpp
        tests/test_analysis_health.cpp
        tests/test_analysis_drivecycle.cpp
        tests/test_analysis_trip.cpp
        tests/test_analysis_performance.cpp
        tests/test_analysis_battery.cpp
        tests/test_demo.cpp
        tests/test_probe.cpp
        tests/FakeEcu.cpp
        tests/FakeEcu2000.cpp
    )
    target_include_directories(caruspex_tests PRIVATE tests)
    target_link_libraries(caruspex_tests PRIVATE caruspex_core Catch2::Catch2WithMain)
    add_test(NAME protocol COMMAND caruspex_tests)
endif()
