// Everything known about the car that is connected right now, collected from
// the worker's signals in one place. The garage, the report and the start page
// read from here instead of each keeping their own copy.
#pragma once

#include <QDateTime>
#include <QHash>
#include <QObject>
#include <QString>
#include <QStringList>
#include <QVector>

#include "app/Session.h"

namespace caruspex {

struct ReadinessRow {
    QString name;
    bool ready = false;
};

class VehicleState : public QObject {
    Q_OBJECT

public:
    explicit VehicleState(QObject* parent = nullptr);

    void attach(Worker* worker);

    bool connected() const { return connected_; }
    QString adapter() const { return adapter_; }
    QString transport() const { return transport_; }
    QString protocol() const { return protocol_; }
    QDateTime connectedAt() const { return connectedAt_; }

    QString vin() const { return vin_; }
    // Name and short description, filled by whoever decodes the VIN.
    QString displayName() const { return displayName_; }
    QString description() const { return description_; }
    int bodyStyle() const { return bodyStyle_; }
    void setIdentity(const QString& name, const QString& description, int bodyStyle);

    bool haveFaults() const { return haveFaults_; }
    const QVector<FaultRow>& faults() const { return faults_; }
    bool haveStatus() const { return status_.valid; }
    bool mil() const { return status_.mil; }
    QVector<ReadinessRow> readiness() const;
    QString freezeCode() const { return freezeCode_; }
    const QVector<LiveRow>& freezeFrame() const { return freeze_; }
    const QVector<MonitorRow>& monitors() const { return monitors_; }
    QString calibration() const { return calibration_; }
    QString ecuName() const { return ecuName_; }
    // Latest reading per PID.
    const QHash<int, LiveRow>& live() const { return live_; }
    bool hasLive(int pid) const { return live_.contains(pid); }
    double liveValue(int pid, double fallback = 0.0) const;

    // Score of the last health evaluation, -1 if none yet.
    int score() const { return score_; }
    void setScore(int score) { score_ = score; }

signals:
    void changed();
    void vinChanged(const QString& vin);
    void faultsChanged();

private:
    void reset();

    bool connected_ = false;
    QString adapter_;
    QString transport_;
    QString protocol_;
    QDateTime connectedAt_;
    QString vin_;
    QString displayName_;
    QString description_;
    int bodyStyle_ = 0;
    bool haveFaults_ = false;
    QVector<FaultRow> faults_;
    StatusRow status_;
    QString freezeCode_;
    QVector<LiveRow> freeze_;
    QVector<MonitorRow> monitors_;
    QString calibration_;
    QString ecuName_;
    QHash<int, LiveRow> live_;
    int score_ = -1;
};

}  // namespace caruspex
