// Caruspex - vehicle diagnostics over EOBD.
//
//   caruspex.exe
//   caruspex.exe --tcp 192.168.0.10:35000
//   caruspex.exe --language en

#include <QApplication>
#include <QCommandLineOption>
#include <QCommandLineParser>
#include <QTranslator>

#include "app/MainWindow.h"
#include "app/Settings.h"
#include "app/Theme.h"
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
    QApplication::setApplicationVersion(QStringLiteral("0.2.0"));
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
                                    "Fahrzeugdiagnose ueber EOBD mit ELM327-kompatiblen "
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
                                    "Sprache fuer diesen Start: de oder en"),
        QStringLiteral("de|en"));
    parser.addOption(languageOption);
    parser.process(app);

    caruspex::theme::apply(app);

    caruspex::MainWindow window;
    window.show();
    if (parser.isSet(tcpOption)) window.autoConnectTcp(parser.value(tcpOption));

    return app.exec();
}
