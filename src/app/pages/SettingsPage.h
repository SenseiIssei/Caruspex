// Einstellungen.
//
// Jede Stellschraube hier wirkt tatsaechlich. Was nur nach Auswahl aussieht,
// aber nirgends ankommt, gehoert nicht in diese Maske.
#pragma once

#include <QWidget>

#include "app/Settings.h"

class QCheckBox;
class QComboBox;
class QLabel;
class QLineEdit;
class QPushButton;
class QSpinBox;

namespace caruspex {

class SettingsPage : public QWidget {
    Q_OBJECT

public:
    explicit SettingsPage(QWidget* parent = nullptr);

    // Setzt die Maske auf die uebergebenen Werte, ohne dabei Aenderungen zu
    // melden.
    void showValues(const AppSettings& values);
    AppSettings values() const;

signals:
    // Wird bei jeder Aenderung gesendet, damit nichts erst auf einen
    // Speichern-Knopf wartet.
    void changed(const caruspex::AppSettings& values);

private:
    void emitChange();
    void resetToDefaults();
    void chooseLogPath();
    void openConfigLocation();

    bool loading_ = false;

    QComboBox* baud_ = nullptr;
    QComboBox* protocolOrder_ = nullptr;
    QSpinBox* timeout_ = nullptr;
    QCheckBox* autoConnect_ = nullptr;

    QSpinBox* pollInterval_ = nullptr;
    QLabel* pollHint_ = nullptr;
    QSpinBox* chartWindow_ = nullptr;

    QSpinBox* clearAttempts_ = nullptr;
    QCheckBox* confirmClear_ = nullptr;

    QComboBox* basicSession_ = nullptr;
    QSpinBox* basicInterval_ = nullptr;
    QComboBox* kklEcu_ = nullptr;

    QCheckBox* logRaw_ = nullptr;
    QCheckBox* logToFile_ = nullptr;
    QLineEdit* logPath_ = nullptr;
    QPushButton* logBrowse_ = nullptr;

    QCheckBox* rememberGeometry_ = nullptr;
};

}  // namespace caruspex
