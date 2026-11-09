// Guided battery and charging test. The voltage comes from the adapter itself
// (pin 16 of the OBD socket), so it works on every car, old or new.
#pragma once

#include <QVector>
#include <QWidget>

#include <optional>

#include "core/analysis/BatteryTest.h"

class QCheckBox;
class QComboBox;
class QLabel;
class QPushButton;
class QVBoxLayout;

namespace caruspex {

class GaugeWidget;

class BatteryPage : public QWidget {
    Q_OBJECT

public:
    explicit BatteryPage(QWidget* parent = nullptr);

public slots:
    void setConnected(bool connected);
    void onVoltage(double volts, double time);

signals:
    void pollRequested(int intervalMs);
    void pollStopRequested();

private:
    enum class Recording { None, Resting, Cranking, Idle, Loaded };
    void record(Recording what, int seconds);
    void finishRecording();
    void evaluate();

    bool connected_ = false;
    GaugeWidget* gauge_ = nullptr;
    QComboBox* type_ = nullptr;
    QCheckBox* startStop_ = nullptr;
    QVector<QPushButton*> stepButtons_;
    QVector<QLabel*> stepValues_;
    QLabel* summary_ = nullptr;
    QVBoxLayout* results_ = nullptr;

    Recording recording_ = Recording::None;
    double recordUntil_ = 0.0;
    std::vector<analysis::VoltageSample> buffer_;
    analysis::BatteryInput input_;
};

}  // namespace caruspex
