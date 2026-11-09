#include "app/VehicleState.h"

namespace caruspex {

VehicleState::VehicleState(QObject* parent) : QObject(parent) {}

void VehicleState::attach(Worker* worker) {
    connect(worker, &Worker::connected, this,
            [this](const QString& adapter, const QString& transport, const QString& protocol) {
                reset();
                connected_ = true;
                adapter_ = adapter;
                transport_ = transport;
                protocol_ = protocol;
                connectedAt_ = QDateTime::currentDateTime();
                emit changed();
            });
    connect(worker, &Worker::closed, this, [this] {
        reset();
        emit changed();
    });
    connect(worker, &Worker::faults, this, [this](const QVector<FaultRow>& rows) {
        faults_ = rows;
        haveFaults_ = true;
        emit faultsChanged();
        emit changed();
    });
    connect(worker, &Worker::status, this, [this](const StatusRow& row) {
        if (!row.valid) return;
        status_ = row;
        emit changed();
    });
    connect(worker, &Worker::vehicleInfo, this,
            [this](const QString& vin, const QString& code, const QVector<LiveRow>& frame) {
                freezeCode_ = code;
                freeze_ = frame;
                const QString clean = vin.trimmed().toUpper();
                if (!clean.isEmpty() && clean != vin_) {
                    vin_ = clean;
                    emit vinChanged(vin_);
                }
                emit changed();
            });
    connect(worker, &Worker::identity, this,
            [this](const QString& vin, const QString& calibration, const QString&,
                   const QString& ecuName) {
                calibration_ = calibration;
                ecuName_ = ecuName;
                const QString clean = vin.trimmed().toUpper();
                if (!clean.isEmpty() && clean != vin_) {
                    vin_ = clean;
                    emit vinChanged(vin_);
                }
                emit changed();
            });
    connect(worker, &Worker::monitorResults, this,
            [this](const QVector<MonitorRow>& rows, const QString&) {
                monitors_ = rows;
                emit changed();
            });
    connect(worker, &Worker::liveValue, this,
            [this](const LiveRow& row) { live_.insert(row.pid, row); });
}

void VehicleState::setIdentity(const QString& name, const QString& description, int bodyStyle) {
    displayName_ = name;
    description_ = description;
    bodyStyle_ = bodyStyle;
    emit changed();
}

QVector<ReadinessRow> VehicleState::readiness() const {
    QVector<ReadinessRow> rows;
    for (int i = 0; i < status_.monitorNames.size() && i < status_.monitorReady.size(); ++i) {
        rows.push_back({status_.monitorNames[i], status_.monitorReady[i]});
    }
    return rows;
}

double VehicleState::liveValue(int pid, double fallback) const {
    const auto it = live_.find(pid);
    return it == live_.end() ? fallback : it->value;
}

void VehicleState::reset() {
    connected_ = false;
    adapter_.clear();
    transport_.clear();
    protocol_.clear();
    vin_.clear();
    displayName_.clear();
    description_.clear();
    bodyStyle_ = 0;
    haveFaults_ = false;
    faults_.clear();
    status_ = StatusRow{};
    freezeCode_.clear();
    freeze_.clear();
    monitors_.clear();
    calibration_.clear();
    ecuName_.clear();
    live_.clear();
    score_ = -1;
}

}  // namespace caruspex
