// Caruspex - Fahrzeugdiagnose ueber EOBD.
//
//   caruspex.exe
//   caruspex.exe --tcp 192.168.0.10:35000

#include <QApplication>
#include <QCommandLineOption>
#include <QCommandLineParser>

#include "app/MainWindow.h"
#include "app/Theme.h"

int main(int argc, char** argv) {
    QApplication app(argc, argv);
    QApplication::setApplicationName(QStringLiteral("Caruspex"));
    QApplication::setApplicationVersion(QStringLiteral("0.2.0"));
    QApplication::setOrganizationName(QStringLiteral("Caruspex"));

    QCommandLineParser parser;
    parser.setApplicationDescription(
        QStringLiteral("Fahrzeugdiagnose ueber EOBD mit ELM327-kompatiblen Adaptern"));
    parser.addHelpOption();
    parser.addVersionOption();
    QCommandLineOption tcpOption(
        QStringList() << QStringLiteral("tcp"),
        QStringLiteral("WLAN-Adapter direkt verbinden, z.B. 192.168.0.10:35000"),
        QStringLiteral("host:port"));
    parser.addOption(tcpOption);
    parser.process(app);

    caruspex::theme::apply(app);

    caruspex::MainWindow window;
    window.show();
    if (parser.isSet(tcpOption)) window.autoConnectTcp(parser.value(tcpOption));

    return app.exec();
}
