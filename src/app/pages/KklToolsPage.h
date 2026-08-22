#pragma once

#include <QVector>
#include <QWidget>

#include "app/Session.h"

QT_BEGIN_NAMESPACE
class QComboBox;
class QLabel;
class QPlainTextEdit;
class QPushButton;
class QSpinBox;
class QTableWidget;
QT_END_NAMESPACE

namespace caruspex {

// Die beiden herstellereigenen Dienste, die kein ELM327 kann:
// Anpassungskanaele lesen und Stellglieddiagnose.
class KklToolsPage : public QWidget {
    Q_OBJECT

public:
    explicit KklToolsPage(QWidget* parent = nullptr);

public slots:
    void setKklConnected(bool connected, const QString& identification);
    void onAdaptationResult(int channel, int value,
                            const QVector<caruspex::LiveRow>& shown);
    void onActuatorResult(const QString& actuator);
    void onMessage(const QString& message, bool isError);

signals:
    void connectRequested(const QString& port, int ecuAddress);
    void disconnectRequested();
    void adaptationRequested(int channel);
    void actuatorTestRequested();

private:
    void refreshPorts();
    void updateAvailability();

    QComboBox* portBox_ = nullptr;
    QComboBox* ecuBox_ = nullptr;
    QPushButton* rescanButton_ = nullptr;
    QPushButton* connectButton_ = nullptr;
    QLabel* state_ = nullptr;
    QLabel* identification_ = nullptr;

    QSpinBox* channelBox_ = nullptr;
    QPushButton* readChannelButton_ = nullptr;
    QLabel* channelValue_ = nullptr;
    QTableWidget* channelTable_ = nullptr;

    QPushButton* actuatorButton_ = nullptr;
    QLabel* actuatorLabel_ = nullptr;
    QPlainTextEdit* actuatorLog_ = nullptr;

    QLabel* message_ = nullptr;
    bool connected_ = false;
};

}  // namespace caruspex
