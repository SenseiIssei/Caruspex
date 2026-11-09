// Settings.
//
// Every knob here actually has an effect. Anything that only looks like a
// choice but doesn't reach anything does not belong on this page.
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

    // Sets the page to the given values without reporting any changes.
    void showValues(const AppSettings& values);
    AppSettings values() const;

signals:
    // Emitted on every change, so nothing has to wait for a save button.
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
    QComboBox* language_ = nullptr;
    QComboBox* timing_ = nullptr;
    QComboBox* appearance_ = nullptr;
};

}  // namespace caruspex
