// Einstellungen, die dauerhaft gespeichert werden.
//
// Absichtlich ein schlichter Wertetyp statt vieler Einzelabfragen: die
// Oberflaeche laedt ihn einmal, reicht ihn an den Arbeitsthread weiter und
// speichert ihn beim Aendern. Damit gibt es keine Stelle, an der eine
// Einstellung zwar in der Maske steht, aber nirgends wirkt.
#pragma once

#include <QMetaType>
#include <QString>

namespace caruspex {

struct AppSettings {
    // --- Verbindung ----------------------------------------------------------
    // 0 heisst: die ueblichen Baudraten durchprobieren.
    int baudOverride = 0;
    // Reihenfolge, in der die Protokolle versucht werden.
    // "kline" passt zum A2, "can" zum A4 B7.
    QString protocolOrder = QStringLiteral("kline");
    // Alle Zeitlimits werden mit diesem Faktor multipliziert. Traege Klone
    // brauchen mehr Zeit, ein gutes Kabel kommt mit weniger aus.
    int timeoutPercent = 100;
    // Zuletzt benutzter Anschluss, damit die Auswahl nicht jedes Mal neu ist.
    QString lastPort;
    bool autoConnect = false;

    // --- Messwerte -----------------------------------------------------------
    // Abstand zwischen zwei Abfragen. Null heisst: so schnell es geht.
    int pollIntervalMs = 20;
    // Wie weit die Diagramme zurueckreichen.
    int chartWindowSeconds = 60;

    // --- Loeschen ------------------------------------------------------------
    int clearAttempts = 5;
    bool confirmClear = true;

    // --- K-Line --------------------------------------------------------------
    // Sitzungsart fuer die Grundeinstellung bei KWP2000. Nicht oeffentlich
    // dokumentiert, deshalb waehlbar.
    int basicSettingSession = 0x85;
    int basicSettingIntervalMs = 180;
    int kklEcuAddress = 0x01;

    // --- Protokoll -----------------------------------------------------------
    bool logRaw = true;
    bool logToFile = false;
    QString logPath;

    // --- Fenster -------------------------------------------------------------
    bool rememberGeometry = true;
};

// Aus der Registrierung beziehungsweise der Konfigurationsdatei lesen.
AppSettings loadSettings();
void saveSettings(const AppSettings& values);

// Der Vorgabezustand, fuer den Knopf "Alles zuruecksetzen".
inline AppSettings defaultSettings() { return AppSettings{}; }

}  // namespace caruspex

Q_DECLARE_METATYPE(caruspex::AppSettings)
