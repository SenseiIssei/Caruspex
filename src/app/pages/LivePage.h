#pragma once

#include <QElapsedTimer>
#include <QHash>

#include <limits>
#include <QVector>
#include <QWidget>

#include "app/Session.h"

QT_BEGIN_NAMESPACE
class QCheckBox;
class QLabel;
class QListWidget;
class QPushButton;
class QTableWidget;
QT_END_NAMESPACE

namespace QtCharts {}

class QChart;
class QChartView;
class QLineSeries;
class QValueAxis;

namespace caruspex {

class LivePage : public QWidget {
    Q_OBJECT

public:
    // Zeitfenster der Kurven in Sekunden.
    void setWindowSeconds(int seconds);
    explicit LivePage(QWidget* parent = nullptr);

public slots:
    void onSupportedPids(const QVector<int>& pids);
    void onLiveValue(const caruspex::LiveRow& row);
    void setConnected(bool connected);
    void setRunning(bool running);
    void toggleFromOutside();

signals:
    void startRequested(const QVector<int>& pids);
    void stopRequested();

private:
    double windowSeconds_ = 90.0;
    void buildPidList();
    QVector<int> selectedPids() const;
    void toggleRun();
    void exportCsv();
    void resetChart();

    QListWidget* pidList_ = nullptr;
    QTableWidget* table_ = nullptr;
    QPushButton* runButton_ = nullptr;
    QPushButton* exportButton_ = nullptr;
    QCheckBox* recordBox_ = nullptr;
    QLabel* hint_ = nullptr;

    QChart* chart_ = nullptr;
    QChartView* chartView_ = nullptr;
    QValueAxis* axisX_ = nullptr;
    QValueAxis* axisY_ = nullptr;
    QHash<int, QLineSeries*> series_;

    QHash<int, int> tableRow_;
    QElapsedTimer clock_;
    QElapsedTimer rescaleClock_;
    double yLow_ = std::numeric_limits<double>::max();
    double yHigh_ = std::numeric_limits<double>::lowest();
    bool running_ = false;
    bool connected_ = false;

    struct Sample {
        double seconds;
        int pid;
        QString name;
        double value;
        QString unit;
    };
    QVector<Sample> recorded_;
};

}  // namespace caruspex
