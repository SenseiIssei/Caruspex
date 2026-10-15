#pragma once

#include <QVector>
#include <QWidget>

#include "app/Session.h"

QT_BEGIN_NAMESPACE
class QComboBox;
class QLabel;
class QProgressBar;
class QPushButton;
class QTableWidget;
QT_END_NAMESPACE

namespace caruspex {

class GaugeWidget;

// Throttle body adaptation.
//
// Two ways, depending on which adapter is plugged in:
//  - KKL cable: real basic settings via KWP1281, measuring block 060 or 098
//  - ELM327: observe only, plus the adaptation attempt without a scan tool
class ThrottlePage : public QWidget {
    Q_OBJECT

public:
    explicit ThrottlePage(QWidget* parent = nullptr);

public slots:
    void setEobdConnected(bool connected);
    void setKklConnected(bool connected, const QString& identification);
    void setProtocol(const QString& protocolName, bool isKwp2000);
    void onLiveValue(const caruspex::LiveRow& row);
    void onBasicSettingValues(const QVector<caruspex::LiveRow>& values);
    void onKklMessage(const QString& message, bool isError);

signals:
    void kklConnectRequested(const QString& port, int ecuAddress);
    void kklDisconnectRequested();
    void basicSettingRequested(int group);
    void basicSettingStopRequested();
    void basicSettingSessionChanged(int mode);

private:
    void refreshPorts();
    void updateAvailability();

    QComboBox* portBox_ = nullptr;
    QComboBox* groupBox_ = nullptr;
    QComboBox* sessionBox_ = nullptr;
    QWidget* sessionRow_ = nullptr;
    QLabel* sessionHint_ = nullptr;
    QPushButton* rescanButton_ = nullptr;
    QPushButton* connectButton_ = nullptr;
    QPushButton* runButton_ = nullptr;
    QLabel* kklState_ = nullptr;
    QLabel* identification_ = nullptr;
    QLabel* checklistHint_ = nullptr;
    QLabel* message_ = nullptr;
    QTableWidget* values_ = nullptr;
    QProgressBar* activity_ = nullptr;
    GaugeWidget* angleGauge_ = nullptr;
    GaugeWidget* rpmGauge_ = nullptr;
    QLabel* eobdHint_ = nullptr;

    bool kklConnected_ = false;
    bool isKwp2000_ = false;
    bool eobdConnected_ = false;
    bool running_ = false;
};

}  // namespace caruspex
