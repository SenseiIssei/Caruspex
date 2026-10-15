// Settings that are stored permanently.
//
// Deliberately a plain value type instead of many individual queries: the UI
// loads it once, passes it on to the worker thread and saves it on change. That
// way there is no place where a setting shows up in the form but has no effect
// anywhere.
#pragma once

#include <QMetaType>
#include <QString>

namespace caruspex {

struct AppSettings {
    // --- Connection ----------------------------------------------------------
    // 0 means: try the usual baud rates.
    int baudOverride = 0;
    // Order in which the protocols are tried.
    // "kline" suits the A2, "can" the A4 B7.
    QString protocolOrder = QStringLiteral("kline");
    // All timeouts are multiplied by this factor. Sluggish clones need more
    // time, a good cable gets by with less.
    int timeoutPercent = 100;
    // Last used port, so the selection doesn't start from scratch every time.
    QString lastPort;
    bool autoConnect = false;

    // --- Live values ---------------------------------------------------------
    // Interval between two requests. Zero means: as fast as possible.
    int pollIntervalMs = 20;
    // How far back the charts reach.
    int chartWindowSeconds = 60;

    // --- Clearing ------------------------------------------------------------
    int clearAttempts = 5;
    bool confirmClear = true;

    // --- K-Line --------------------------------------------------------------
    // Session type for basic settings on KWP2000. Not publicly documented,
    // hence selectable.
    int basicSettingSession = 0x85;
    int basicSettingIntervalMs = 180;
    int kklEcuAddress = 0x01;

    // --- Log -----------------------------------------------------------------
    bool logRaw = true;
    bool logToFile = false;
    QString logPath;

    // --- Window --------------------------------------------------------------
    bool rememberGeometry = true;

    // --- Language ------------------------------------------------------------
    // "auto" follows the system language, otherwise "de" or "en".
    // Takes effect at the next start.
    QString language = QStringLiteral("auto");
};

// Read from the registry or the configuration file, respectively.
AppSettings loadSettings();
void saveSettings(const AppSettings& values);

// Resolves "auto" against the system locale: German on a German system,
// English everywhere else. Returns "de" or "en".
QString effectiveLanguage(const AppSettings& values);

// The default state, for the "reset everything" button.
inline AppSettings defaultSettings() { return AppSettings{}; }

}  // namespace caruspex

Q_DECLARE_METATYPE(caruspex::AppSettings)
