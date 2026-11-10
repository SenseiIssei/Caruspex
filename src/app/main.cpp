// Caruspex - vehicle diagnostics over EOBD.
//
//   caruspex.exe
//   caruspex.exe --tcp 192.168.0.10:35000
//   caruspex.exe --language en

#include <QApplication>

#include <cstdio>
#include <QCommandLineOption>
#include <QCommandLineParser>
#include <QDir>
#include <QPainter>
#include <QTimer>
#include <QTranslator>

#include "app/AppIcon.h"
#ifdef CARUSPEX_HAS_BLE
#include "app/BleTransport.h"
#endif
#include "app/Garage.h"
#include "app/MainWindow.h"
#include "app/Report.h"
#include "app/Settings.h"
#include "app/Theme.h"
#include "app/VehicleDiagram.h"
#include "core/Lang.h"

namespace {

// --language overrides the saved setting for this run only. It has to be
// known before the parser runs, because the help text is translated too.
QString languageFromArguments(const QStringList& args) {
    for (int i = 1; i < args.size(); ++i) {
        if (args[i].startsWith(QStringLiteral("--language="))) {
            return args[i].mid(11);
        }
        if (args[i] == QStringLiteral("--language") && i + 1 < args.size()) {
            return args[i + 1];
        }
    }
    return {};
}

}  // namespace

int main(int argc, char** argv) {
    QApplication app(argc, argv);
    QApplication::setApplicationName(QStringLiteral("Caruspex"));
    QApplication::setApplicationVersion(QStringLiteral(CARUSPEX_VERSION));
    QApplication::setOrganizationName(QStringLiteral("Caruspex"));
    QApplication::setWindowIcon(caruspex::appIcon());

    // Language first, so every widget is built with the right texts. The UI
    // strings go through Qt's translator, the core picks its own via L().
    QTranslator translator;
    caruspex::AppSettings startup = caruspex::loadSettings();
    const QString requested = languageFromArguments(QCoreApplication::arguments());
    if (!requested.isEmpty()) startup.language = requested;
    const QString code = caruspex::effectiveLanguage(startup);
    const caruspex::Language active = caruspex::languageFromCode(code.toStdString());
    caruspex::setLanguage(code == QStringLiteral("de") ? caruspex::Language::German : active);
    if (code != QStringLiteral("de") &&
        translator.load(QStringLiteral(":/i18n/caruspex_%1.qm").arg(code))) {
        QApplication::installTranslator(&translator);
    }
    // Arabic reads from right to left; Qt mirrors the layouts.
    if (caruspex::languageInfo(active).rightToLeft && code != QStringLiteral("de")) {
        QApplication::setLayoutDirection(Qt::RightToLeft);
    }

    QCommandLineParser parser;
    parser.setApplicationDescription(
        QCoreApplication::translate("main",
                                    "Fahrzeugdiagnose über EOBD mit ELM327-kompatiblen "
                                    "Adaptern"));
    parser.addHelpOption();
    parser.addVersionOption();
    QCommandLineOption tcpOption(
        QStringList() << QStringLiteral("tcp"),
        QCoreApplication::translate("main",
                                    "WLAN-Adapter direkt verbinden, z.B. 192.168.0.10:35000"),
        QCoreApplication::translate("main", "host:port"));
    parser.addOption(tcpOption);
    QCommandLineOption portOption(
        QStringList() << QStringLiteral("port"),
        QCoreApplication::translate("main",
                                    "Adapter am seriellen Anschluss verbinden, z.B. COM3 oder "
                                    "/dev/ttyUSB0, auto sucht an allen"),
        QStringLiteral("port|auto"));
    parser.addOption(portOption);
    QCommandLineOption bluetoothOption(
        QStringList() << QStringLiteral("bluetooth"),
        QCoreApplication::translate("main",
                                    "Bluetooth-Adapter verbinden, über die Adresse oder auto für "
                                    "die Suche"),
        QStringLiteral("address|auto"));
#ifdef CARUSPEX_HAS_BLE
    parser.addOption(bluetoothOption);
#endif
    QCommandLineOption enetOption(
        QStringList() << QStringLiteral("enet"),
        QCoreApplication::translate("main",
                                    "BMW oder Mini über ein ENET-Kabel verbinden, über die Adresse "
                                    "oder auto für die Suche"),
        QStringLiteral("address|auto"));
    parser.addOption(enetOption);
    QCommandLineOption canOption(
        QStringList() << QStringLiteral("can"),
        QCoreApplication::translate("main", "CAN-Interface verbinden: j2534:<treiber.dll>, slcan:COM5 oder "
                                            "socketcan:can0, mit @250000 für die Bitrate"),
        QStringLiteral("spec"));
    parser.addOption(canOption);
    QCommandLineOption reportOption(
        QStringList() << QStringLiteral("report"),
        QCoreApplication::translate("main",
                                    "Nach dem Verbinden einen Prüfbericht schreiben (PDF, oder "
                                    "HTML bei .html) und beenden"),
        QStringLiteral("file"));
    parser.addOption(reportOption);
    QCommandLineOption reportWaitOption(
        QStringList() << QStringLiteral("report-wait"),
        QStringLiteral("Milliseconds between connecting and writing --report"), QStringLiteral("ms"),
        QStringLiteral("8000"));
    reportWaitOption.setFlags(QCommandLineOption::HiddenFromHelp);
    parser.addOption(reportWaitOption);
    QCommandLineOption languageOption(
        QStringList() << QStringLiteral("language"),
        QCoreApplication::translate("main",
                                    "Sprache für diesen Start: de oder en"),
        QStringLiteral("de|en|fr|..."));
    parser.addOption(languageOption);
    QCommandLineOption demoOption(
        QStringList() << QStringLiteral("demo"),
        QCoreApplication::translate("main", "Gleich mit dem eingebauten Demo-Auto verbinden"));
    parser.addOption(demoOption);
    QCommandLineOption replayOption(
        QStringList() << QStringLiteral("replay"),
        QCoreApplication::translate("main", "Eine aufgezeichnete Sitzung abspielen"),
        QStringLiteral("file.jsonl"));
    parser.addOption(replayOption);
    QCommandLineOption pageOption(
        QStringList() << QStringLiteral("page"),
        QCoreApplication::translate("main", "Mit dieser Seite starten, z.B. faults oder live"),
        QStringLiteral("name"));
    parser.addOption(pageOption);
    QCommandLineOption shotOption(
        QStringList() << QStringLiteral("shot"),
        QCoreApplication::translate("main",
                                    "Nach ein paar Sekunden ein Bild des Fensters speichern und "
                                    "beenden (für Dokumentation)"),
        QStringLiteral("file.png"));
    parser.addOption(shotOption);
    QCommandLineOption sizeOption(
        QStringList() << QStringLiteral("size"),
        QCoreApplication::translate("main", "Fenstergröße, z.B. 1440x920"),
        QStringLiteral("WxH"));
    parser.addOption(sizeOption);
    QCommandLineOption shotDelayOption(QStringList() << QStringLiteral("shot-delay"),
                                       QStringLiteral("Developer aid: milliseconds before --shot"),
                                       QStringLiteral("ms"), QStringLiteral("5200"));
    shotDelayOption.setFlags(QCommandLineOption::HiddenFromHelp);
    parser.addOption(shotDelayOption);
    QCommandLineOption carsOption(
        QStringList() << QStringLiteral("render-cars"),
        QStringLiteral("Developer aid: draw every body style into one image and quit"),
        QStringLiteral("file.png"));
    carsOption.setFlags(QCommandLineOption::HiddenFromHelp);
    parser.addOption(carsOption);
    QCommandLineOption aktePdf(QStringList() << QStringLiteral("garage-pdf"),
                               QStringLiteral("Developer aid: write the garage file of the first car as PDF and quit"),
                               QStringLiteral("file.pdf"));
    aktePdf.setFlags(QCommandLineOption::HiddenFromHelp);
    parser.addOption(aktePdf);
    QCommandLineOption iconOption(QStringList() << QStringLiteral("render-icon"),
                                  QStringLiteral("Developer aid: write the app icon in all sizes as PNG into a folder and quit"),
                                  QStringLiteral("folder"));
    iconOption.setFlags(QCommandLineOption::HiddenFromHelp);
    parser.addOption(iconOption);
    QCommandLineOption bleProbeOption(
        QStringList() << QStringLiteral("ble-probe"),
        QStringLiteral("Developer aid: list Bluetooth LE devices, connect to the given one (or the "
                       "first adapter), send ATZ and print everything"),
        QStringLiteral("id|auto"));
    bleProbeOption.setFlags(QCommandLineOption::HiddenFromHelp);
    parser.addOption(bleProbeOption);

    QCommandLineOption themeOption(
        QStringList() << QStringLiteral("theme"),
        QCoreApplication::translate("main", "Erscheinungsbild für diesen Start: dark oder light"),
        QStringLiteral("dark|light"));
    parser.addOption(themeOption);
    parser.process(app);

    if (parser.isSet(themeOption)) startup.appearance = parser.value(themeOption);
    caruspex::theme::setLight(caruspex::effectiveLight(startup));
    caruspex::theme::apply(app);

#ifdef CARUSPEX_HAS_BLE
    if (parser.isSet(bleProbeOption)) {
        QString error;
        const QList<caruspex::BleDevice> devices = caruspex::scanBleDevices(5000, error);
        std::printf("%lld device(s)%s%s\n", static_cast<long long>(devices.size()),
                    error.isEmpty() ? "" : ", error: ", error.toUtf8().constData());
        for (const caruspex::BleDevice& d : devices) {
            std::printf("  %-40s %-30s %-8s rssi %4d score %d\n", d.id.toUtf8().constData(),
                        d.name.toUtf8().constData(),
                        d.lowEnergy && d.classic ? "LE+BR" : (d.classic ? "classic" : "LE"), d.rssi,
                        d.score);
        }
        caruspex::BleDevice target;
        const QString wanted = parser.value(bleProbeOption);
        for (const caruspex::BleDevice& d : devices) {
            if (wanted == QStringLiteral("auto") ? d.score >= 3 : d.id == wanted) {
                target = d;
                break;
            }
        }
        if (target.id.isEmpty() && wanted != QStringLiteral("auto")) target.id = wanted;
        if (target.id.isEmpty()) return 2;
        std::printf("connecting to %s ...\n", target.id.toUtf8().constData());
        std::string failure;
        std::unique_ptr<caruspex::ITransport> transport;
        if (target.lowEnergy || !target.classic) {
            auto ble = std::make_unique<caruspex::BleTransport>();
            if (ble->open(target, failure)) {
                std::printf("LE channel %s\n", ble->profile().toUtf8().constData());
                transport = std::move(ble);
            } else {
                std::printf("LE open failed: %s\n", failure.c_str());
            }
        }
        if (!transport && target.classic) {
            auto rfcomm = std::make_unique<caruspex::RfcommTransport>();
            if (rfcomm->open(target, failure)) {
                std::printf("classic serial port open\n");
                transport = std::move(rfcomm);
            } else {
                std::printf("classic open failed: %s\n", failure.c_str());
            }
        }
        if (!transport) return 1;
        // Only adapters get a command; other devices are just looked at.
        if (target.score < 3) return 0;
        std::printf("sending ATZ\n");
        transport->write("ATZ\r");
        std::string reply;
        for (int i = 0; i < 20 && reply.find('>') == std::string::npos; ++i) reply += transport->read(250);
        std::printf("reply: %s\n", reply.c_str());
        return reply.empty() ? 1 : 0;
    }
#endif
    if (parser.isSet(iconOption)) {
        // tools/gen_icons.py packs these into the .ico, .icns and .png files.
        const QDir dir(parser.value(iconOption));
        if (!dir.mkpath(QStringLiteral("."))) return 1;
        for (int size : {16, 20, 24, 32, 40, 48, 64, 128, 256, 512, 1024}) {
            const QString file = dir.filePath(QStringLiteral("icon-%1.png").arg(size));
            if (!caruspex::renderAppIcon(size).save(file)) return 1;
        }
        return 0;
    }
    if (parser.isSet(aktePdf)) {
        caruspex::Garage garage;
        garage.load();
        if (garage.cars().isEmpty()) return 1;
        return caruspex::writeGaragePdf(garage.cars().front(), parser.value(aktePdf)) ? 0 : 1;
    }
    if (parser.isSet(carsOption)) {
        // Every body style, plain and see-through, in a grid.
        const int w = 560;
        const int h = 236;
        QImage sheet(w * 2, h * 8, QImage::Format_ARGB32_Premultiplied);
        sheet.fill(caruspex::theme::color::surface);
        QPainter painter(&sheet);
        for (int i = 0; i < 8; ++i) {
            for (int see = 0; see < 2; ++see) {
                caruspex::VehicleDiagram car;
                car.resize(w, h);
                car.setBodyStyle(static_cast<caruspex::BodyStyle>(i));
                car.setAnatomyVisible(see == 1);
                if (see == 1) {
                    car.setAffected({static_cast<int>(caruspex::CarZone::Exhaust),
                                     static_cast<int>(caruspex::CarZone::Throttle)});
                    car.setFocus(caruspex::CarZone::Throttle);
                }
                car.render(&painter, QPoint(see * w, i * h));
            }
        }
        painter.end();
        sheet.save(parser.value(carsOption));
        return 0;
    }

    caruspex::MainWindow window;
    if (parser.isSet(sizeOption)) {
        const QStringList parts = parser.value(sizeOption).split('x');
        if (parts.size() == 2) window.resize(parts[0].toInt(), parts[1].toInt());
    }
    window.show();
    if (parser.isSet(reportOption)) {
        const bool connects = parser.isSet(tcpOption) || parser.isSet(demoOption) ||
                              parser.isSet(replayOption) || parser.isSet(portOption) ||
                              parser.isSet(bluetoothOption) || parser.isSet(enetOption) || parser.isSet(canOption);
        if (!connects) {
            std::fprintf(stderr, "%s\n",
                         QCoreApplication::translate("main", "--report braucht eine Verbindung: --demo, "
                                                             "--port, --tcp, --enet, --bluetooth oder --replay")
                             .toLocal8Bit()
                             .constData());
            return 2;
        }
        window.reportAndQuit(parser.value(reportOption), parser.value(reportWaitOption).toInt());
    }
    if (parser.isSet(portOption)) window.autoConnectSerial(parser.value(portOption));
    if (parser.isSet(enetOption)) window.autoConnectEnet(parser.value(enetOption));
    if (parser.isSet(canOption)) window.autoConnectCan(parser.value(canOption));
#ifdef CARUSPEX_HAS_BLE
    if (parser.isSet(bluetoothOption)) window.autoConnectBluetooth(parser.value(bluetoothOption));
#endif
    if (parser.isSet(tcpOption)) window.autoConnectTcp(parser.value(tcpOption));
    if (parser.isSet(demoOption)) window.autoConnectDemo();
    if (parser.isSet(replayOption)) window.autoConnectReplay(parser.value(replayOption));
    if (parser.isSet(pageOption)) {
        const QString page = parser.value(pageOption);
        // Give a connection a moment, it jumps to the start page when done.
        QTimer::singleShot(parser.isSet(tcpOption) || parser.isSet(demoOption) || parser.isSet(replayOption) ? 2600 : 0, &window,
                           [&window, page] { window.showPageByName(page); });
    }
    if (parser.isSet(shotOption)) {
        const QString file = parser.value(shotOption);
        QTimer::singleShot(parser.value(shotDelayOption).toInt(), &window, [&window, file] {
            window.grab().save(file);
            QApplication::quit();
        });
    }

    return app.exec();
}
