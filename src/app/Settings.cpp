#include "app/Settings.h"

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
    s.endGroup();

    // Werte aus einer aelteren oder von Hand bearbeiteten Datei koennten
    // unsinnig sein. Lieber begrenzen als spaeter mit einer Null teilen.
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
    s.endGroup();

    s.sync();
}

}  // namespace caruspex
