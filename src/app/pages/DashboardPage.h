#pragma once

#include <QHash>
#include <QWidget>

#include "app/Session.h"

class QLabel;
class QPushButton;
class QFrame;
class QGridLayout;

namespace caruspex {

class GaugeWidget;

class DashboardPage : public QWidget {
    Q_OBJECT

public:
    explicit DashboardPage(QWidget* parent = nullptr);

public slots:
    void onConnected(const QString& adapter, const QString& transport, const QString& protocol);
    void onDisconnected();
    void onStatus(const caruspex::StatusRow& row);
    void onFaults(const QVector<caruspex::FaultRow>& rows);
    void onLiveValue(const caruspex::LiveRow& row);

signals:
    void readRequested();
    void detailsRequested();
    void liveRequested();

private:
    void updateHeadline();
    GaugeWidget* gauge(int pid);

    QFrame* card_ = nullptr;
    QLabel* headline_ = nullptr;
    QLabel* subline_ = nullptr;
    QLabel* connectionInfo_ = nullptr;
    QPushButton* readButton_ = nullptr;
    QPushButton* detailsButton_ = nullptr;
    QPushButton* liveButton_ = nullptr;
    QHash<int, GaugeWidget*> gauges_;

    bool connected_ = false;
    bool haveRead_ = false;
    bool mil_ = false;
    int faultCount_ = 0;
};

}  // namespace caruspex
