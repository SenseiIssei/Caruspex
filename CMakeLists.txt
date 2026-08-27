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

option(CARUSPEX_GUI   "Die grafische Anwendung mitbauen (braucht Qt 6)" ON)
option(CARUSPEX_TESTS "Tests mitbauen (braucht Catch2 ueber Conan)"     OFF)

# ---------------------------------------------------------------------------
# Kern: Transport, Adaptertreiber, Diagnosedienste, Datenbanken. Kein Qt.
# ---------------------------------------------------------------------------
if(WIN32)
    set(CARUSPEX_SERIAL src/core/Serial_win.cpp)
else()
    set(CARUSPEX_SERIAL src/core/Serial_posix.cpp)
endif()

add_library(caruspex_core STATIC
    ${CARUSPEX_SERIAL}
    src/core/Transport.cpp
    src/core/Elm327.cpp
    src/core/Obd2.cpp
    src/core/Kwp1281.cpp
    src/core/KLineStream.cpp
    src/data/DtcDatabase.cpp
    src/data/DtcDatabase.gen.cpp
    src/data/Pids.cpp
    src/data/Vehicles.cpp
    src/data/DtcLocation.cpp
    src/data/VagFaults.cpp
)
target_include_directories(caruspex_core PUBLIC src)

if(WIN32)
    target_link_libraries(caruspex_core PUBLIC setupapi ws2_32)
endif()

find_package(Threads REQUIRED)
target_link_libraries(caruspex_core PUBLIC Threads::Threads)

# ---------------------------------------------------------------------------
# Konsolenwerkzeug, benutzt denselben Kern und laesst sich skripten.
# ---------------------------------------------------------------------------
add_executable(caruspex_cli src/cli_main.cpp)
target_link_libraries(caruspex_cli PRIVATE caruspex_core)

# ---------------------------------------------------------------------------
# Grafische Anwendung (Qt 6 Widgets und Charts)
# ---------------------------------------------------------------------------
if(CARUSPEX_GUI)
    find_package(Qt6 6.5 REQUIRED COMPONENTS Widgets Charts)
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
        src/app/TitleBar.cpp
        src/app/TitleBar.h
        src/app/VehicleDiagram.cpp
        src/app/VehicleDiagram.h
        src/app/pages/DashboardPage.cpp
        src/app/pages/DashboardPage.h
        src/app/pages/FaultsPage.cpp
        src/app/pages/FaultsPage.h
        src/app/pages/LivePage.cpp
        src/app/pages/LivePage.h
        src/app/pages/ToolsPage.cpp
        src/app/pages/ToolsPage.h
        src/app/pages/ThrottlePage.cpp
        src/app/pages/ThrottlePage.h
        src/app/pages/StatsPage.cpp
        src/app/pages/StatsPage.h
        src/app/pages/ScanPage.cpp
        src/app/pages/ScanPage.h
        src/app/pages/KklToolsPage.cpp
        src/app/pages/KklToolsPage.h
        src/app/pages/LexiconPage.cpp
        src/app/pages/LexiconPage.h
    )
    target_link_libraries(caruspex PRIVATE caruspex_core Qt6::Widgets Qt6::Charts)
    set_target_properties(caruspex PROPERTIES
        RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin
        WIN32_EXECUTABLE ON      # unter Windows kein Konsolenfenster daneben
        MACOSX_BUNDLE ON         # unter macOS ein richtiges Caruspex.app
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
        tests/test_kwp1281.cpp
        tests/FakeEcu.cpp
    )
    target_include_directories(caruspex_tests PRIVATE tests)
    target_link_libraries(caruspex_tests PRIVATE caruspex_core Catch2::Catch2WithMain)
    add_test(NAME protocol COMMAND caruspex_tests)
endif()
