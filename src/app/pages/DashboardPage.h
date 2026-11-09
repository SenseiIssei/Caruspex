// Start page: the connected car as a picture, its health as a ring with every
// finding listed below, key values as tiles and the live gauges.
#pragma once

#include <QHash>
#include <QVector>
#include <QWidget>

#include "app/Session.h"
#include "app/VehicleDiagram.h"

class QLabel;
class QPushButton;
class QFrame;
class QVBoxLayout;

namespace caruspex {

class GaugeWidget;
class ScoreRing;
class StatTile;

class DashboardPage : public QWidget {
    Q_OBJECT

public:
    explicit DashboardPage(QWidget* parent = nullptr);

    // A single line per finding for the list, provided by whoever evaluates
    // the car (the health check). Without it the page rates faults itself.
    struct Finding {
        QString icon;
        QColor tint;
        QString title;
        QString text;
        QString meta;
    };
    void setHealth(int score, const QString& caption, const QVector<Finding>& findings);
    int score() const { return score_; }
    QString scoreCaption() const { return caption_; }
    const QVector<Finding>& findings() const { return findings_; }

public slots:
    void onConnected(const QString& adapter, const QString& transport, const QString& protocol);
    void onDisconnected();
    void onStatus(const caruspex::StatusRow& row);
    void onFaults(const QVector<caruspex::FaultRow>& rows);
    void onLiveValue(const caruspex::LiveRow& row);
    void onBusy(bool busy);
    void setVehicle(const QString& title, const QString& subtitle, caruspex::BodyStyle body);

signals:
    void readRequested();
    void detailsRequested();
    void liveRequested();
    void connectRequested();
    void faultsRequested();
    void reportRequested();
    void scoreChanged(int score);

private:
    void rebuildFindings();
    void showFindings(int score, const QString& caption, const QString& subcaption,
                      const QVector<Finding>& findings, bool clickableFaults);
    void updateHero();
    GaugeWidget* gauge(int pid);

    VehicleDiagram* car_ = nullptr;
    QLabel* eyebrow_ = nullptr;
    QLabel* vehicleTitle_ = nullptr;
    QLabel* vehicleSub_ = nullptr;
    QLabel* stateLine_ = nullptr;
    QPushButton* primary_ = nullptr;
    QPushButton* secondary_ = nullptr;
    QPushButton* report_ = nullptr;

    ScoreRing* ring_ = nullptr;
    QLabel* findingsTitle_ = nullptr;
    QLabel* findingsSub_ = nullptr;
    QVBoxLayout* findingsList_ = nullptr;

    StatTile* battery_ = nullptr;
    StatTile* coolant_ = nullptr;
    StatTile* odometer_ = nullptr;
    StatTile* fuel_ = nullptr;
    QHash<int, GaugeWidget*> gauges_;

    bool connected_ = false;
    bool busy_ = false;
    bool haveRead_ = false;
    bool haveStatus_ = false;
    bool mil_ = false;
    int incompleteMonitors_ = 0;
    QVector<FaultRow> faults_;
    bool externalHealth_ = false;
    QString vehicleName_;
    int score_ = -1;
    QString caption_;
    QVector<Finding> findings_;
};

}  // namespace caruspex
