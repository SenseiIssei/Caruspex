cmake_minimum_required(VERSION 3.21)
# Without this the app would need the macOS version of the build machine.
set(CMAKE_OSX_DEPLOYMENT_TARGET "13.0" CACHE STRING "Oldest macOS the app runs on")
project(Caruspex VERSION 0.7.0 LANGUAGES CXX)

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
    src/core/Catalog.gen.cpp
    src/core/Transport.cpp
    src/core/Elm327.cpp
    src/core/EcuAddress.cpp
    src/core/Obd2.cpp
    src/core/Uds.cpp
    src/core/CustomPids.cpp
    src/core/DemoTransport.cpp
    src/core/AdapterProbe.cpp
    src/core/BleProfile.cpp
    src/core/CanSignals.cpp
    src/core/CanBus.cpp
    src/core/J2534.cpp
    src/core/SocketCan.cpp
    src/core/Enet.cpp
    src/core/Recording.cpp
    src/core/Kwp1281.cpp
    src/core/Kwp2000.cpp
    src/core/VagKLine.cpp
    src/core/KLineStream.cpp
    src/data/DtcDatabase.cpp
    src/data/DtcGenericP.gen.cpp
    src/data/DtcGenericCBU.gen.cpp
    src/data/DtcBrands.gen.cpp
    src/data/DtcDetails.gen.cpp
    src/data/Pids.cpp
    src/data/Vehicles.cpp
    src/data/Vin.cpp
    src/data/VehicleCatalog.cpp
    src/data/VehicleCatalog.gen.cpp
    src/data/DtcLocation.cpp
    src/data/VagFaults.cpp
    src/data/MonitorTests.cpp
    src/data/EcuAddresses.cpp
    src/data/EcuAddresses.gen.cpp
    src/data/PidPacks.cpp
    src/data/PidPacks.gen.cpp
    src/data/FailureTypes.cpp
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
target_compile_definitions(caruspex_cli PRIVATE CARUSPEX_VERSION="${PROJECT_VERSION}")

# ---------------------------------------------------------------------------
# Graphical application (Qt 6 Widgets and Charts)
# ---------------------------------------------------------------------------
if(CARUSPEX_GUI)
    find_package(Qt6 6.5 REQUIRED COMPONENTS Widgets Charts Svg PrintSupport Network LinguistTools)
    qt_standard_project_setup()

    # The prebuilt Qt 6.8 still links the AGL framework on macOS, which the
    # macOS 15 SDK no longer has. Nothing here uses it, so drop the reference.
    if(APPLE AND TARGET WrapOpenGL::WrapOpenGL)
        get_target_property(_gl_libs WrapOpenGL::WrapOpenGL INTERFACE_LINK_LIBRARIES)
        if(_gl_libs)
            list(FILTER _gl_libs EXCLUDE REGEX "AGL")
            set_target_properties(WrapOpenGL::WrapOpenGL PROPERTIES
                INTERFACE_LINK_LIBRARIES "${_gl_libs}")
        endif()
    endif()

    # Icons for the installers and the file managers. resources/icons is
    # generated: caruspex --render-icon DIR, then tools/gen_icons.py DIR.
    set(CARUSPEX_ICON_ICO  ${CMAKE_CURRENT_SOURCE_DIR}/resources/icons/caruspex.ico)
    set(CARUSPEX_ICON_ICNS ${CMAKE_CURRENT_SOURCE_DIR}/resources/icons/caruspex.icns)
    set(CARUSPEX_PLATFORM_FILES)
    if(NOT EXISTS ${CARUSPEX_ICON_ICO})
        message(STATUS "No app icon yet, building without one")
    elseif(WIN32)
        configure_file(packaging/windows/caruspex.rc.in ${CMAKE_BINARY_DIR}/caruspex.rc @ONLY)
        list(APPEND CARUSPEX_PLATFORM_FILES ${CMAKE_BINARY_DIR}/caruspex.rc)
    elseif(APPLE)
        set_source_files_properties(${CARUSPEX_ICON_ICNS} PROPERTIES
            MACOSX_PACKAGE_LOCATION Resources)
        list(APPEND CARUSPEX_PLATFORM_FILES ${CARUSPEX_ICON_ICNS})
    endif()

    qt_add_executable(caruspex
        ${CARUSPEX_PLATFORM_FILES}
        resources/resources.qrc
        src/app/main.cpp
        src/app/AppIcon.cpp
        src/app/AppIcon.h
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
        src/app/VehicleIdentity.cpp
        src/app/VehicleIdentity.h
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
        src/app/pages/EcuPage.cpp
        src/app/pages/EcuPage.h
        src/app/pages/EvPage.cpp
        src/app/pages/EvPage.h
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
        src/app/pages/VehiclesPage.cpp
        src/app/pages/VehiclesPage.h
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
                                           Qt6::PrintSupport Qt6::Network)

    # Bluetooth LE adapters need the Qt module qtconnectivity. Without it the
    # app is built as before, only the Bluetooth LE choice is missing.
    find_package(Qt6 QUIET COMPONENTS Bluetooth)
    if(TARGET Qt6::Bluetooth)
        target_sources(caruspex PRIVATE src/app/BleTransport.cpp src/app/BleTransport.h)
        target_link_libraries(caruspex PRIVATE Qt6::Bluetooth)
        target_compile_definitions(caruspex PRIVATE CARUSPEX_HAS_BLE)
        message(STATUS "Bluetooth LE: on")
    else()
        message(STATUS "Bluetooth LE: off (Qt module qtconnectivity not found)")
    endif()

    # The source strings are German; the English translation is compiled into
    # the binary as :/i18n/caruspex_en.qm. Refresh the .ts file after changing
    # UI strings with: cmake --build build --target update_translations
    # One file per language; lupdate takes the target language from the
    # file name. The other languages are filled by tools/gen_i18n.py.
    qt_add_translations(caruspex
        TS_FILES
            translations/caruspex_en.ts
            translations/caruspex_es.ts
            translations/caruspex_fr.ts
            translations/caruspex_pt.ts
            translations/caruspex_it.ts
            translations/caruspex_nl.ts
            translations/caruspex_pl.ts
            translations/caruspex_cs.ts
            translations/caruspex_ru.ts
            translations/caruspex_uk.ts
            translations/caruspex_tr.ts
            translations/caruspex_ar.ts
            translations/caruspex_zh.ts
            translations/caruspex_ja.ts
            translations/caruspex_ko.ts
            translations/caruspex_hi.ts
            translations/caruspex_id.ts
        RESOURCE_PREFIX /i18n
        LUPDATE_OPTIONS -no-obsolete -locations none -source-language de
    )
    target_compile_definitions(caruspex PRIVATE CARUSPEX_VERSION="${PROJECT_VERSION}")
    set_target_properties(caruspex PROPERTIES
        RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin
        WIN32_EXECUTABLE ON      # no console window next to it on Windows
        MACOSX_BUNDLE ON         # a proper caruspex.app bundle on macOS
        MACOSX_BUNDLE_BUNDLE_NAME Caruspex
        MACOSX_BUNDLE_GUI_IDENTIFIER dev.senseiissei.caruspex
        MACOSX_BUNDLE_BUNDLE_VERSION ${PROJECT_VERSION}
        MACOSX_BUNDLE_SHORT_VERSION_STRING ${PROJECT_VERSION}
        MACOSX_BUNDLE_ICON_FILE caruspex.icns
        MACOSX_BUNDLE_COPYRIGHT "Copyright (c) 2026 SenseiIssei, MIT License"
        # Own template: macOS ends the app on first Bluetooth use without a
        # reason in NSBluetoothAlwaysUsageDescription.
        MACOSX_BUNDLE_INFO_PLIST ${CMAKE_CURRENT_SOURCE_DIR}/packaging/macos/Info.plist.in
    )
endif()

# ---------------------------------------------------------------------------
# Install layout, used by the Linux AppImage (tools/package.sh). Windows and
# macOS are packaged from the build folder with windeployqt and macdeployqt.
# ---------------------------------------------------------------------------
include(GNUInstallDirs)
install(TARGETS caruspex_cli RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR})
if(CARUSPEX_GUI)
    install(TARGETS caruspex
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
        BUNDLE DESTINATION .)
    if(UNIX AND NOT APPLE)
        install(FILES packaging/linux/caruspex.desktop
                DESTINATION ${CMAKE_INSTALL_DATADIR}/applications)
        install(FILES resources/icons/caruspex.png
                DESTINATION ${CMAKE_INSTALL_DATADIR}/icons/hicolor/256x256/apps)
    endif()
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
        tests/test_obd_batch.cpp
        tests/test_json.cpp
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
        tests/test_recording.cpp
        tests/test_dtc.cpp
        tests/test_vehicles.cpp
        tests/test_ecus.cpp
        tests/test_uds.cpp
        tests/test_custom_pids.cpp
        tests/test_ble_profile.cpp
        tests/test_enet.cpp
        tests/test_obd_on_uds.cpp
        tests/test_can_signals.cpp
        tests/test_lang.cpp
        tests/test_can_bus.cpp
        tests/test_j2534.cpp
        tests/FakeEcu.cpp
        tests/FakeEcu2000.cpp
    )
    target_include_directories(caruspex_tests PRIVATE tests)
    target_link_libraries(caruspex_tests PRIVATE caruspex_core Catch2::Catch2WithMain)
    add_test(NAME protocol COMMAND caruspex_tests)
    # A J2534 driver DLL without hardware, loaded by the J2534 test.
    if(WIN32)
        add_library(fake_j2534 SHARED tests/fake_j2534.cpp)
        set_target_properties(fake_j2534 PROPERTIES RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)
        add_dependencies(caruspex_tests fake_j2534)
        target_compile_definitions(caruspex_tests PRIVATE
            CARUSPEX_FAKE_J2534="$<TARGET_FILE:fake_j2534>")
    endif()
endif()
