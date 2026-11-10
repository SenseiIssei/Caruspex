#include "app/Settings.h"

#include "core/Lang.h"

#include <QGuiApplication>
#include <QStyleHints>

#include <QLocale>
#include <QSettings>

namespace caruspex {
namespace {

QSettings store() { return QSettings(QStringLiteral("Caruspex"), QStringLiteral("Caruspex")); }

}  // namespace

AppSettings loadSettings() {
    AppSettings v;
    QSettings s = store();

    s.beginGroup(QStringLiteral("verbindung"));
    v.baudOverride = s.value(QStringLiteral("baud"), v.baudOverride).toInt();
    v.protocolOrder = s.value(QStringLiteral("protokoll"), v.protocolOrder).toString();
    v.timeoutPercent = s.value(QStringLiteral("zeitlimit"), v.timeoutPercent).toInt();
    v.adaptiveTiming = qBound(0, s.value(QStringLiteral("taktung"), v.adaptiveTiming).toInt(), 2);
    v.lastPort = s.value(QStringLiteral("anschluss"), v.lastPort).toString();
    v.autoConnect = s.value(QStringLiteral("autoverbinden"), v.autoConnect).toBool();
    s.endGroup();

    s.beginGroup(QStringLiteral("messwerte"));
    v.pollIntervalMs = s.value(QStringLiteral("takt"), v.pollIntervalMs).toInt();
    v.chartWindowSeconds =
        s.value(QStringLiteral("zeitfenster"), v.chartWindowSeconds).toInt();
    s.endGroup();

    s.beginGroup(QStringLiteral("loeschen"));
    v.clearAttempts = s.value(QStringLiteral("versuche"), v.clearAttempts).toInt();
    v.confirmClear = s.value(QStringLiteral("nachfragen"), v.confirmClear).toBool();
    s.endGroup();

    s.beginGroup(QStringLiteral("kline"));
    v.basicSettingSession =
        s.value(QStringLiteral("sitzungsart"), v.basicSettingSession).toInt();
    v.basicSettingIntervalMs =
        s.value(QStringLiteral("takt"), v.basicSettingIntervalMs).toInt();
    v.kklEcuAddress = s.value(QStringLiteral("steuergeraet"), v.kklEcuAddress).toInt();
    s.endGroup();

    s.beginGroup(QStringLiteral("protokoll"));
    v.logRaw = s.value(QStringLiteral("roh"), v.logRaw).toBool();
    v.logToFile = s.value(QStringLiteral("datei"), v.logToFile).toBool();
    v.logPath = s.value(QStringLiteral("pfad"), v.logPath).toString();
    s.endGroup();

    s.beginGroup(QStringLiteral("fenster"));
    v.rememberGeometry = s.value(QStringLiteral("merken"), v.rememberGeometry).toBool();
    v.checkUpdates = s.value(QStringLiteral("updates"), v.checkUpdates).toBool();
    s.endGroup();

    v.language = s.value(QStringLiteral("sprache"), v.language).toString();
    v.appearance = s.value(QStringLiteral("erscheinungsbild"), v.appearance).toString();
    if (v.appearance != QStringLiteral("dark") && v.appearance != QStringLiteral("light")) {
        v.appearance = QStringLiteral("auto");
    }
    bool known = false;
    for (const LanguageInfo& info : languages()) known = known || v.language == QLatin1String(info.code);
    if (!known) v.language = QStringLiteral("auto");

    // Values from an older or hand-edited file could be nonsense. Better to
    // clamp them than to divide by zero later.
    if (v.timeoutPercent < 25) v.timeoutPercent = 25;
    if (v.timeoutPercent > 500) v.timeoutPercent = 500;
    if (v.pollIntervalMs < 0) v.pollIntervalMs = 0;
    if (v.pollIntervalMs > 5000) v.pollIntervalMs = 5000;
    if (v.chartWindowSeconds < 10) v.chartWindowSeconds = 10;
    if (v.chartWindowSeconds > 3600) v.chartWindowSeconds = 3600;
    if (v.clearAttempts < 1) v.clearAttempts = 1;
    if (v.clearAttempts > 20) v.clearAttempts = 20;
    if (v.basicSettingIntervalMs < 50) v.basicSettingIntervalMs = 50;
    if (v.basicSettingIntervalMs > 2000) v.basicSettingIntervalMs = 2000;
    return v;
}

void saveSettings(const AppSettings& v) {
    QSettings s = store();

    s.beginGroup(QStringLiteral("verbindung"));
    s.setValue(QStringLiteral("baud"), v.baudOverride);
    s.setValue(QStringLiteral("protokoll"), v.protocolOrder);
    s.setValue(QStringLiteral("zeitlimit"), v.timeoutPercent);
    s.setValue(QStringLiteral("taktung"), v.adaptiveTiming);
    s.setValue(QStringLiteral("anschluss"), v.lastPort);
    s.setValue(QStringLiteral("autoverbinden"), v.autoConnect);
    s.endGroup();

    s.beginGroup(QStringLiteral("messwerte"));
    s.setValue(QStringLiteral("takt"), v.pollIntervalMs);
    s.setValue(QStringLiteral("zeitfenster"), v.chartWindowSeconds);
    s.endGroup();

    s.beginGroup(QStringLiteral("loeschen"));
    s.setValue(QStringLiteral("versuche"), v.clearAttempts);
    s.setValue(QStringLiteral("nachfragen"), v.confirmClear);
    s.endGroup();

    s.beginGroup(QStringLiteral("kline"));
    s.setValue(QStringLiteral("sitzungsart"), v.basicSettingSession);
    s.setValue(QStringLiteral("takt"), v.basicSettingIntervalMs);
    s.setValue(QStringLiteral("steuergeraet"), v.kklEcuAddress);
    s.endGroup();

    s.beginGroup(QStringLiteral("protokoll"));
    s.setValue(QStringLiteral("roh"), v.logRaw);
    s.setValue(QStringLiteral("datei"), v.logToFile);
    s.setValue(QStringLiteral("pfad"), v.logPath);
    s.endGroup();

    s.beginGroup(QStringLiteral("fenster"));
    s.setValue(QStringLiteral("merken"), v.rememberGeometry);
    s.setValue(QStringLiteral("updates"), v.checkUpdates);
    s.endGroup();

    s.setValue(QStringLiteral("sprache"), v.language);
    s.setValue(QStringLiteral("erscheinungsbild"), v.appearance);

    s.sync();
}

QString effectiveLanguage(const AppSettings& v) {
    for (const LanguageInfo& info : languages())
        if (v.language == QLatin1String(info.code)) return v.language;
    // The system language if Caruspex has it, English otherwise.
    const std::string system = QLocale::system().name().toStdString();
    const Language guess = languageFromCode(system);
    const QString code = QString::fromLatin1(languageInfo(guess).code);
    return system.rfind(languageInfo(guess).code, 0) == 0 ? code : QStringLiteral("en");
}

}  // namespace caruspex

namespace caruspex {

bool effectiveLight(const AppSettings& values) {
    if (values.appearance == QStringLiteral("light")) return true;
    if (values.appearance == QStringLiteral("dark")) return false;
    if (!qGuiApp) return false;
    return qGuiApp->styleHints()->colorScheme() == Qt::ColorScheme::Light;
}

}  // namespace caruspex
