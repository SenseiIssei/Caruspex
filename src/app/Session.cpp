#include "app/Session.h"

#include <QTimer>

#include "core/Timing.h"
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

    kwp_ = std::make_unique<VagKLine>(kline_,
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
    emit kklProtocol(QString::fromStdString(kwp_->protocolName()),
                     kwp_->protocol() == KLineProtocol::Kwp2000);
    emit kklMessage(QObject::tr("Verbunden mit %1 ueber %2. Vor dem Start die Punkte "
                                "oben pruefen.")
                        .arg(QString::fromStdString(ecuName(ecu)),
                             QString::fromStdString(kwp_->protocolName())),
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

// Sucht der Reihe nach alle bekannten Steuergeraete. Jeder Versuch braucht
// wegen der 5-Baud-Weckadresse ein paar Sekunden, bei nicht vorhandenen
// Steuergeraeten kommt die Wartezeit auf die Antwort dazu.
void Worker::scanAllEcus(const QString& device, int clearAttempts) {
    closeKkl();
    cancelScan_ = false;

    std::string error;
    if (!kline_.open(device.toStdString(), 9600, error)) {
        emit kklMessage(QString::fromStdString(error), true);
        emit scanFinished(0, 0, false);
        return;
    }

    const std::vector<Ecu>& list = knownEcus();
    const int total = static_cast<int>(list.size());
    emit scanStarted(total);
    emit busy(true);

    int present = 0;
    int withFaults = 0;
    int index = 0;

    for (Ecu ecu : list) {
        if (cancelScan_) break;
        ++index;
        EcuScanRow row;
        row.address = static_cast<int>(ecu);
        row.name = QString::fromStdString(ecuName(ecu));
        emit scanProgress(row.name, index, total);

        VagKLine probe(kline_);
        std::string localError;
        if (!probe.connect(ecu, localError)) {
            row.present = false;
            row.note = QObject::tr("meldet sich nicht, vermutlich nicht verbaut");
            emit scanEcuResult(row);
            continue;
        }

        row.present = true;
        ++present;
        QStringList identification;
        for (const std::string& line : probe.identification()) {
            identification << QString::fromStdString(line).trimmed();
        }
        row.identification = QStringLiteral("[%1]  %2")
                                 .arg(QString::fromStdString(probe.protocolName()),
                                      identification.join(QStringLiteral("  |  ")));

        std::vector<Kwp1281Fault> faults;
        if (probe.readFaults(faults, localError)) {
            for (const Kwp1281Fault& f : faults) {
                FaultRow fault;
                fault.code = QStringLiteral("%1").arg(f.code, 5, 10, QChar('0'));
                fault.kind = f.sporadic ? QObject::tr("sporadisch")
                                        : QObject::tr("gespeichert");
                fault.text = QString::fromStdString(f.text);
                row.faults.push_back(fault);
            }
            if (!row.faults.isEmpty()) ++withFaults;
            row.note = row.faults.isEmpty()
                           ? QObject::tr("kein Eintrag")
                           : QObject::tr("%1 Eintraege").arg(row.faults.size());

            // Hartnaeckig loeschen: nach jedem Versuch nachlesen. Manche
            // Steuergeraete nehmen den Loeschbefehl erst im zweiten Anlauf an.
            if (clearAttempts > 0 && !faults.empty()) {
                bool cleared = false;
                for (int attempt = 1; attempt <= clearAttempts && !cleared; ++attempt) {
                    emit clearProgress(QObject::tr("%1: loesche, Versuch %2 von %3")
                                           .arg(row.name)
                                           .arg(attempt)
                                           .arg(clearAttempts));
                    if (!probe.clearFaults(localError)) {
                        emit clearProgress(QObject::tr("   abgelehnt: %1")
                                               .arg(QString::fromStdString(localError)));
                        sleepMs(300);
                        continue;
                    }
                    sleepMs(400);
                    std::vector<Kwp1281Fault> rest;
                    if (probe.readFaults(rest, localError) && rest.empty()) cleared = true;
                }
                if (cleared) {
                    row.faults.clear();
                    row.note = QObject::tr("geloescht");
                    --withFaults;
                    emit clearProgress(QObject::tr("   %1: leer").arg(row.name));
                } else {
                    row.note = QObject::tr("%1 Eintraege, Loeschen ohne Wirkung")
                                   .arg(row.faults.size());
                    emit clearProgress(
                        QObject::tr("   %1: bleibt bestehen, die Ursache ist noch da")
                            .arg(row.name));
                }
            }
        } else {
            row.note = QString::fromStdString(localError);
        }
        probe.disconnect();
        emit scanEcuResult(row);
    }

    kline_.close();
    emit busy(false);
    emit scanFinished(present, withFaults, cancelScan_.load());
}

// Hartnaeckiges Loeschen ueber EOBD. Manche Steuergeraete nehmen den Befehl
// nur direkt nach dem Verbindungsaufbau an, andere brauchen mehrere Anlaeufe.
// Deshalb wird zwischendurch die Verbindung neu aufgebaut.
void Worker::bruteForceClear(int attempts) {
    if (!obd_) {
        emit clearProgress(QObject::tr("Nicht verbunden."));
        emit clearFinished(false, 0);
        return;
    }
    emit busy(true);

    int remaining = 0;
    for (int attempt = 1; attempt <= attempts; ++attempt) {
        emit clearProgress(QObject::tr("Versuch %1 von %2 ...").arg(attempt).arg(attempts));

        std::string error;
        const bool acknowledged = obd_->clearFaults(error);
        if (!acknowledged) {
            emit clearProgress(QObject::tr("   Steuergeraet quittiert nicht: %1")
                                   .arg(QString::fromStdString(error)));
        }
        sleepMs(600);

        const std::vector<Fault> left = obd_->readFaults(error);
        remaining = static_cast<int>(left.size());
        if (left.empty()) {
            emit clearProgress(QObject::tr("   Speicher ist leer."));
            emit busy(false);
            emit clearFinished(true, 0);
            refreshStatusAndFaults();
            return;
        }
        emit clearProgress(QObject::tr("   noch %1 Eintraege").arg(remaining));

        // Ab dem zweiten Fehlschlag die Verbindung neu aufbauen. Einige
        // Steuergeraete oeffnen das Loeschfenster nur direkt nach dem Wecken.
        if (attempt >= 2 && attempt < attempts) {
            emit clearProgress(QObject::tr("   Verbindung wird neu aufgebaut ..."));
            ElmResult reset = elm_.send("ATZ", 6000);
            Q_UNUSED(reset);
            sleepMs(400);
            elm_.send("ATE0");
            elm_.send("ATL0");
            elm_.send("ATS0");
            elm_.send("ATH0");
            elm_.send("0100", 10000);
            sleepMs(200);
        }
    }

    emit busy(false);
    emit clearFinished(false, remaining);
    refreshStatusAndFaults();
}

void Worker::setBasicSettingSession(int mode) {
    if (kwp_) kwp_->setBasicSettingSession(static_cast<std::uint8_t>(mode));
    emit logLine(QStringLiteral("Sitzungsart fuer die Grundeinstellung: 0x%1")
                     .arg(mode, 2, 16, QChar('0')).toUpper());
}

void Worker::readAdaptationChannel(int channel) {
    if (!kwp_ || !kwp_->connected()) {
        emit kklMessage(QObject::tr("Erst das KKL-Kabel verbinden."), true);
        return;
    }
    emit busy(true);
    int value = 0;
    std::vector<MeasuringValue> shown;
    std::string error;
    if (!kwp_->readAdaptation(channel, value, shown, error)) {
        emit busy(false);
        emit kklMessage(QString::fromStdString(error), true);
        return;
    }

    QVector<LiveRow> rows;
    int index = 1;
    for (const MeasuringValue& m : shown) {
        if (m.valid) {
            LiveRow row;
            row.pid = index;
            row.name = QObject::tr("Feld %1: %2").arg(index).arg(
                QString::fromStdString(m.label));
            row.unit = QString::fromStdString(m.unit);
            row.value = m.value;
            rows.push_back(row);
        }
        ++index;
    }
    emit busy(false);
    emit adaptationResult(channel, value, rows);
    emit kklMessage(QObject::tr("Anpassungskanal %1 steht auf %2.").arg(channel).arg(value),
                    false);
}

void Worker::runActuatorTest() {
    if (!kwp_ || !kwp_->connected()) {
        emit kklMessage(QObject::tr("Erst das KKL-Kabel verbinden."), true);
        return;
    }
    emit busy(true);
    std::string actuator;
    std::string error;
    if (!kwp_->actuatorTest(actuator, error)) {
        emit busy(false);
        emit kklMessage(QString::fromStdString(error), true);
        return;
    }
    emit busy(false);
    emit actuatorResult(QString::fromStdString(actuator));
}

// --- Session ----------------------------------------------------------------

Session::Session(QObject* parent) : QObject(parent) {
    qRegisterMetaType<FaultRow>("caruspex::FaultRow");
    qRegisterMetaType<QVector<FaultRow>>("QVector<caruspex::FaultRow>");
    qRegisterMetaType<LiveRow>("caruspex::LiveRow");
    qRegisterMetaType<QVector<LiveRow>>("QVector<caruspex::LiveRow>");
    qRegisterMetaType<StatusRow>("caruspex::StatusRow");
    qRegisterMetaType<EcuScanRow>("caruspex::EcuScanRow");
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
    connect(this, &Session::scanAllRequested, worker_, &Worker::scanAllEcus);
    connect(this, &Session::bruteForceClearRequested, worker_,
            &Worker::bruteForceClear);
    connect(this, &Session::adaptationRequested, worker_,
            &Worker::readAdaptationChannel);
    connect(this, &Session::actuatorTestRequested, worker_,
            &Worker::runActuatorTest);
    connect(this, &Session::basicSettingSessionRequested, worker_,
            &Worker::setBasicSettingSession);

    connect(worker_, &Worker::connected, this,
            [this](const QString&, const QString&, const QString&) { connected_ = true; });
    connect(worker_, &Worker::closed, this, [this] { connected_ = false; });

    thread_.start();
}

void Session::cancelScan() {
    if (worker_) worker_->cancelScan();
}

Session::~Session() {
    cancelScan();
    QMetaObject::invokeMethod(worker_, &Worker::closeAdapter, Qt::BlockingQueuedConnection);
    thread_.quit();
    thread_.wait(3000);
}

}  // namespace caruspex
