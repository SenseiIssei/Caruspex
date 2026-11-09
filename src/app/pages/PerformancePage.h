// Stopwatch for 0 to 100, 80 to 120, the quarter mile and braking, from the
// speed signal alone. Only speed is polled while it runs, as fast as the
// adapter allows.
#pragma once

#include <QVector>
#include <QWidget>

#include "app/Session.h"
#include "core/analysis/PerformanceTimer.h"

class QLabel;
class QPushButton;
class QTimer;

namespace caruspex {

class StatTile;

class PerformancePage : public QWidget {
    Q_OBJECT

public:
    explicit PerformancePage(QWidget* parent = nullptr);

public slots:
    void setConnected(bool connected);
    void onLiveValue(const caruspex::LiveRow& row);

signals:
    void startRequested(const QVector<int>& pids);
    void stopRequested();

private:
    void toggle();
    void evaluate();
    void showRun(StatTile* tile, const analysis::TimedRun& run, bool distance);

    bool connected_ = false;
    bool running_ = false;
    std::vector<analysis::SpeedSample> samples_;
    QLabel* speed_ = nullptr;
    QLabel* state_ = nullptr;
    QLabel* accuracy_ = nullptr;
    QPushButton* start_ = nullptr;
    StatTile* zeroHundred_ = nullptr;
    StatTile* eightyTwenty_ = nullptr;
    StatTile* zeroSixty_ = nullptr;
    StatTile* quarter_ = nullptr;
    StatTile* braking_ = nullptr;
    QTimer* evaluateTimer_ = nullptr;
};

}  // namespace caruspex
