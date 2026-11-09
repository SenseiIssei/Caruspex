// Caruspex - vehicle diagnostics over EOBD.
//
//   caruspex.exe
//   caruspex.exe --tcp 192.168.0.10:35000
//   caruspex.exe --language en

#include <QApplication>
#include <QCommandLineOption>
#include <QCommandLineParser>
#include <QPainter>
#include <QTimer>
#include <QTranslator>

#include "app/MainWindow.h"
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
    QApplication::setApplicationVersion(QStringLiteral("0.3.0"));
    QApplication::setOrganizationName(QStringLiteral("Caruspex"));

    // Language first, so every widget is built with the right texts. The UI
    // strings go through Qt's translator, the core picks its own via L().
    QTranslator translator;
    caruspex::AppSettings startup = caruspex::loadSettings();
    const QString requested = languageFromArguments(QCoreApplication::arguments());
    if (!requested.isEmpty()) startup.language = requested;
    const bool english = caruspex::effectiveLanguage(startup) == QStringLiteral("en");
    caruspex::setLanguage(english ? caruspex::Language::English
                                  : caruspex::Language::German);
    if (english && translator.load(QStringLiteral(":/i18n/caruspex_en.qm"))) {
        QApplication::installTranslator(&translator);
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
    QCommandLineOption languageOption(
        QStringList() << QStringLiteral("language"),
        QCoreApplication::translate("main",
                                    "Sprache für diesen Start: de oder en"),
        QStringLiteral("de|en"));
    parser.addOption(languageOption);
    QCommandLineOption demoOption(
        QStringList() << QStringLiteral("demo"),
        QCoreApplication::translate("main", "Gleich mit dem eingebauten Demo-Auto verbinden"));
    parser.addOption(demoOption);
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
    QCommandLineOption themeOption(
        QStringList() << QStringLiteral("theme"),
        QCoreApplication::translate("main", "Erscheinungsbild für diesen Start: dark oder light"),
        QStringLiteral("dark|light"));
    parser.addOption(themeOption);
    parser.process(app);

    if (parser.isSet(themeOption)) startup.appearance = parser.value(themeOption);
    caruspex::theme::setLight(caruspex::effectiveLight(startup));
    caruspex::theme::apply(app);

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
    if (parser.isSet(tcpOption)) window.autoConnectTcp(parser.value(tcpOption));
    if (parser.isSet(demoOption)) window.autoConnectDemo();
    if (parser.isSet(pageOption)) {
        const QString page = parser.value(pageOption);
        // Give a connection a moment, it jumps to the start page when done.
        QTimer::singleShot(parser.isSet(tcpOption) || parser.isSet(demoOption) ? 2600 : 0, &window,
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
