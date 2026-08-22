#include "app/Session.h"

#include <QTimer>

#include "core/Transport.h"
#include "data/Pids.h"

namespace caruspex {
namespace {

LiveRow toRow(const LiveValue& v) {
    LiveRow row;
    row.pid = v.pid;
    row.name = QString::fromStdString(v.name);
    row.unit = QString::fromStdString(v.unit);
    row.text = QString::fromStdString(v.text);
    row.value = v.value;
    row.warn = v.warn;
    return row;
}

}  // namespace

// --- Worker -----------------------------------------------------------------

Worker::Worker() = default;

Worker::~Worker() {
    elm_.disconnect();
    if (kwp_) kwp_->disconnect();
    kline_.close();
}

void Worker::openSerial(const QString& device, int baud, const QString& protocolOrder) {
    emit busy(true);
    elm_.setLog([this](const std::string& m) { emit logLine(QString::fromStdString(m)); });

    std::string error;
    std::unique_ptr<ITransport> transport;
    // Ohne Vorgabe die ueblichen Baudraten durchprobieren, Klone sind da eigen.
    const QVector<int> bauds = baud > 0 ? QVector<int>{baud}
                                        : QVector<int>{38400, 9600, 115200, 500000};
    for (int b : bauds) {
        auto candidate = std::make_unique<SerialTransport>();
        if (candidate->open(device.toStdString(), b, error)) {
            transport = std::move(candidate);
            emit logLine(QStringLiteral("Anschluss %1 offen bei %2 Baud").arg(device).arg(b));
            break;
        }
    }
    if (!transport) {
        emit busy(false);
        emit failed(QString::fromStdString(error));
        return;
    }

    if (!elm_.connect(std::move(transport), protocolOrder.toStdString(), error)) {
        emit busy(false);
        emit failed(QString::fromStdString(error));
        return;
    }
    obd_ = std::make_unique<Obd2>(elm_);
    emit connected(QString::fromStdString(elm_.version()),
                   QString::fromStdString(elm_.transportName()),
                   QString::fromStdString(elm_.protocolText()));
    emit busy(false);
    refreshStatusAndFaults();
}

void Worker::openTcp(const QString& host, int port, const QString& protocolOrder) {
    emit busy(true);
    elm_.setLog([this](const std::string& m) { emit logLine(QString::fromStdString(m)); });

    std::string error;
    auto transport = std::make_unique<TcpTransport>();
    if (!transport->open(host.toStdString(), port, error)) {
        emit busy(false);
        emit failed(QString::fromStdString(error));
        return;
    }
    if (!elm_.connect(std::move(transport), protocolOrder.toStdString(), error)) {
        emit busy(false);
        emit failed(QString::fromStdString(error));
        return;
    }
    obd_ = std::make_unique<Obd2>(elm_);
    emit connected(QString::fromStdString(elm_.version()),
                   QString::fromStdString(elm_.transportName()),
                   QString::fromStdString(elm_.protocolText()));
    emit busy(false);
    refreshStatusAndFaults();
}

void Worker::closeAdapter() {
    stopLive();
    elm_.disconnect();
    obd_.reset();
    emit closed();
}

void Worker::refreshStatusAndFaults() {
    if (!obd_) return;
    emit busy(true);

    const VehicleStatus st = obd_->readStatus();
    StatusRow row;
    row.valid = st.valid;
    row.mil = st.milOn;
    row.faultCount = st.faultCount;
    for (const ReadinessItem& item : st.readiness) {
        row.monitorNames << QString::fromStdString(item.name);
        row.monitorReady << item.ready;
    }
    emit status(row);

    std::string error;
    QVector<FaultRow> rows;
    for (const Fault& f : obd_->readFaults(error)) {
        rows.push_back({QString::fromStdString(f.code), QString::fromStdString(f.kind),
                        QString::fromStdString(f.text), f.throttleRelated});
    }
    emit faults(rows);
    emit logLine(rows.isEmpty()
                     ? QStringLiteral("Fehlerspeicher gelesen: kein Eintrag.")
                     : QStringLiteral("Fehlerspeicher gelesen: %1 Eintraege.").arg(rows.size()));

    QVector<int> supported;
    for (int pid : obd_->supportedPids()) supported.push_back(pid);
    emit supportedPids(supported);
    emit busy(false);
}

void Worker::readFaults() {
    refreshStatusAndFaults();
}

void Worker::clearFaults() {
    if (!obd_) return;
    emit busy(true);
    std::string error;
    const bool ok = obd_->clearFaults(error);
    emit logLine(ok ? QStringLiteral("Fehlerspeicher geloescht.")
                    : QStringLiteral("Loeschen fehlgeschlagen: %1")
                          .arg(QString::fromStdString(error)));
    emit busy(false);
    refreshStatusAndFaults();
}

void Worker::readDetails() {
    if (!obd_) return;
    emit busy(true);
    const QString vin = QString::fromStdString(obd_->readVin());
    std::string code;
    QVector<LiveRow> frame;
    for (const LiveValue& v : obd_->readFreezeFrame(code)) frame.push_back(toRow(v));
    emit vehicleInfo(vin, QString::fromStdString(code), frame);
    emit busy(false);
}

void Worker::startLive(const QVector<int>& pids) {
    if (!obd_ || pids.isEmpty()) return;
    pollPids_ = pids;
    pollIndex_ = 0;
    if (!pollTimer_) {
        pollTimer_ = new QTimer(this);
        // Nicht schneller als 50 Abfragen je Sekunde. Auf der K-Line dauert eine
        // Abfrage ohnehin laenger, aber ein schneller CAN-Adapter oder der
        // Simulator wuerden die Oberflaeche sonst mit Signalen zuschuetten.
        pollTimer_->setInterval(20);
        QObject::connect(pollTimer_, &QTimer::timeout, this, &Worker::pollTick);
    }
    pollTimer_->start();
    emit logLine(QStringLiteral("Messung gestartet, %1 Werte im Umlauf.").arg(pids.size()));
}

void Worker::stopLive() {
    if (pollTimer_ && pollTimer_->isActive()) {
        pollTimer_->stop();
        emit logLine(QStringLiteral("Messung gestoppt."));
    }
}

// Ein Messwert pro Durchlauf. So bleibt die Ereignisschleife frei und
// Stopp-Anforderungen der Oberflaeche kommen sofort durch.
void Worker::pollTick() {
    if (!obd_ || pollPids_.isEmpty()) return;
    const int pid = pollPids_[pollIndex_ % pollPids_.size()];
    pollIndex_ = (pollIndex_ + 1) % pollPids_.size();

    LiveValue value;
    if (obd_->readLive(pid, value)) emit liveValue(toRow(value));
}

void Worker::sendRaw(const QString& command) {
    ElmResult result = elm_.send(command.toStdString(), 8000);
    QStringList lines;
    for (const std::string& line : result.lines) lines << QString::fromStdString(line);
    if (lines.isEmpty()) lines << QStringLiteral("(keine Antwort)");
    emit rawReply(lines.join('\n'));
}

// --- KWP1281 ueber KKL ------------------------------------------------------

void Worker::openKkl(const QString& device, int ecuAddress) {
    closeKkl();
    emit busy(true);
    emit kklMessage(QObject::tr("Wecke das Steuergeraet mit der 5-Baud-Adresse, "
                                "das dauert etwa drei Sekunden ..."), false);

    std::string error;
    if (!kline_.open(device.toStdString(), 9600, error)) {
        emit busy(false);
        emit kklMessage(QString::fromStdString(error), true);
        return;
    }

    kwp_ = std::make_unique<Kwp1281>(kline_,
                                     [this](const std::string& m) {
                                         emit logLine(QString::fromStdString(m));
                                     });
    const Ecu ecu = static_cast<Ecu>(ecuAddress);
    if (!kwp_->connect(ecu, error)) {
        kwp_.reset();
        kline_.close();
        emit busy(false);
        emit kklConnected(false, QString());
        emit kklMessage(QString::fromStdString(error), true);
        return;
    }

    QStringList identification;
    for (const std::string& line : kwp_->identification()) {
        identification << QString::fromStdString(line).trimmed();
    }
    emit busy(false);
    emit kklConnected(true, identification.join(QStringLiteral("  |  ")));
    emit kklMessage(QObject::tr("Verbunden mit %1. Vor dem Start die Punkte oben pruefen.")
                        .arg(QString::fromStdString(ecuName(ecu))),
                    false);
    emit logLine(QStringLiteral("KWP1281 verbunden: ") + identification.join(" / "));
}

void Worker::closeKkl() {
    stopBasicSetting();
    if (kwp_) {
        kwp_->disconnect();
        kwp_.reset();
    }
    kline_.close();
    emit kklConnected(false, QString());
}

void Worker::startBasicSetting(int group) {
    if (!kwp_ || !kwp_->connected()) {
        emit kklMessage(QObject::tr("Erst das KKL-Kabel verbinden."), true);
        return;
    }
    basicSettingGroup_ = group;
    if (!basicSettingTimer_) {
        basicSettingTimer_ = new QTimer(this);
        // Das Steuergeraet faellt aus der Grundeinstellung, wenn laenger als
        // etwa eine Sekunde nichts kommt. Wir fragen deutlich haeufiger nach.
        basicSettingTimer_->setInterval(180);
        QObject::connect(basicSettingTimer_, &QTimer::timeout, this,
                         &Worker::basicSettingTick);
    }
    basicSettingTimer_->start();
    emit kklMessage(QObject::tr("Grundeinstellung im Messwertblock %1 laeuft. Gaspedal "
                                "nicht antippen.")
                        .arg(group),
                    false);
    emit logLine(QStringLiteral("Grundeinstellung Block %1 gestartet").arg(group));
}

void Worker::stopBasicSetting() {
    if (basicSettingTimer_ && basicSettingTimer_->isActive()) {
        basicSettingTimer_->stop();
        emit kklMessage(QObject::tr("Grundeinstellung beendet."), false);
        emit logLine(QStringLiteral("Grundeinstellung beendet"));
    }
}

void Worker::basicSettingTick() {
    if (!kwp_ || !kwp_->connected()) {
        stopBasicSetting();
        return;
    }
    std::vector<MeasuringValue> measured;
    std::string error;
    if (!kwp_->basicSetting(basicSettingGroup_, measured, error)) {
        stopBasicSetting();
        emit kklMessage(QString::fromStdString(error), true);
        return;
    }

    QVector<LiveRow> rows;
    int index = 1;
    for (const MeasuringValue& m : measured) {
        if (!m.valid) {
            ++index;
            continue;
        }
        LiveRow row;
        row.pid = index;
        row.name = QObject::tr("Feld %1: %2").arg(index).arg(QString::fromStdString(m.label));
        row.unit = QString::fromStdString(m.unit);
        row.value = m.value;
        rows.push_back(row);
        ++index;
    }
    emit basicSettingValues(rows);
}

// --- Session ----------------------------------------------------------------

Session::Session(QObject* parent) : QObject(parent) {
    qRegisterMetaType<FaultRow>("caruspex::FaultRow");
    qRegisterMetaType<QVector<FaultRow>>("QVector<caruspex::FaultRow>");
    qRegisterMetaType<LiveRow>("caruspex::LiveRow");
    qRegisterMetaType<QVector<LiveRow>>("QVector<caruspex::LiveRow>");
    qRegisterMetaType<StatusRow>("caruspex::StatusRow");
    qRegisterMetaType<QVector<int>>("QVector<int>");

    worker_ = new Worker();
    worker_->moveToThread(&thread_);
    QObject::connect(&thread_, &QThread::finished, worker_, &QObject::deleteLater);

    connect(this, &Session::openSerialRequested, worker_, &Worker::openSerial);
    connect(this, &Session::openTcpRequested, worker_, &Worker::openTcp);
    connect(this, &Session::closeRequested, worker_, &Worker::closeAdapter);
    connect(this, &Session::readFaultsRequested, worker_, &Worker::readFaults);
    connect(this, &Session::clearFaultsRequested, worker_, &Worker::clearFaults);
    connect(this, &Session::readDetailsRequested, worker_, &Worker::readDetails);
    connect(this, &Session::startLiveRequested, worker_, &Worker::startLive);
    connect(this, &Session::stopLiveRequested, worker_, &Worker::stopLive);
    connect(this, &Session::sendRawRequested, worker_, &Worker::sendRaw);
    connect(this, &Session::openKklRequested, worker_, &Worker::openKkl);
    connect(this, &Session::closeKklRequested, worker_, &Worker::closeKkl);
    connect(this, &Session::startBasicSettingRequested, worker_, &Worker::startBasicSetting);
    connect(this, &Session::stopBasicSettingRequested, worker_, &Worker::stopBasicSetting);

    connect(worker_, &Worker::connected, this,
            [this](const QString&, const QString&, const QString&) { connected_ = true; });
    connect(worker_, &Worker::closed, this, [this] { connected_ = false; });

    thread_.start();
}

Session::~Session() {
    QMetaObject::invokeMethod(worker_, &Worker::closeAdapter, Qt::BlockingQueuedConnection);
    thread_.quit();
    thread_.wait(3000);
}

}  // namespace caruspex
