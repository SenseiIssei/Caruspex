#include "app/Session.h"

#include <QElapsedTimer>
#include <QThread>

#ifdef CARUSPEX_HAS_BLE
#include "app/BleTransport.h"
#endif
#include <QTimer>

#include <algorithm>
#include <cmath>

#include "core/AdapterProbe.h"
#include "core/Uds.h"
#include "core/DemoTransport.h"
#include "core/Recording.h"
#include "data/PidPacks.h"

#include <QFile>
#include "core/analysis/DriveCycleGuide.h"
#include "core/analysis/HealthCheck.h"

#include "core/Serial.h"
#include "core/Timing.h"
#include "core/Transport.h"
#include "data/DtcDatabase.h"
#include "data/Pids.h"

namespace caruspex {

double appSeconds() {
    static QElapsedTimer clock = [] {
        QElapsedTimer t;
        t.start();
        return t;
    }();
    return clock.nsecsElapsed() / 1e9;
}

namespace {

LiveRow toRow(const LiveValue& v) {
    LiveRow row;
    // The key tells the values of one PID apart; it equals the PID for the
    // primary value, so everything keyed by PID keeps working.
    row.pid = v.key != 0 ? v.key : v.pid;
    row.name = QString::fromStdString(v.name);
    row.unit = QString::fromStdString(v.unit);
    row.text = QString::fromStdString(v.text);
    row.value = v.value;
    row.warn = v.warn;
    row.time = appSeconds();
    row.displayMin = v.displayMin;
    row.displayMax = v.displayMax;
    return row;
}

// The core reports the kind of a fault as display text in the active language.
// "stored" is the English wording the core uses for "gespeichert".
bool isStoredKind(const std::string& kind) {
    return kind == "gespeichert" || kind == "stored";
}

}  // namespace

// --- Worker -----------------------------------------------------------------

Worker::Worker() = default;

Worker::~Worker() {
    elm_.disconnect();
    if (kwp_) kwp_->disconnect();
    kline_.close();
}

bool Worker::finishConnect(std::unique_ptr<ITransport> transport, const QString& protocolOrder,
                           bool reportFailure) {
    std::string error;
    if (!elm_.connect(std::move(transport), protocolOrder.toStdString(), error)) {
        if (reportFailure) emit failed(QString::fromStdString(error));
        else emit logLine(QString::fromStdString(error));
        return false;
    }
    // On CAN several control units may answer; with headers on, every fault
    // keeps the unit it came from.
    if (elm_.isCan()) elm_.setHeaders(true);
    obd_ = std::make_unique<Obd2>(elm_);
    emit connected(QString::fromStdString(elm_.version()),
                   QString::fromStdString(elm_.transportName()),
                   QString::fromStdString(elm_.protocolText()));
    return true;
}

void Worker::openSerial(const QString& device, int baud, const QString& protocolOrder) {
    emit busy(true);
    elm_.setLog([this](const std::string& m) { emit logLine(QString::fromStdString(m)); });

    // An empty device means: look for the adapter on every port. Only the
    // usual rates are tried there, otherwise a computer with many ports
    // would keep the user waiting for minutes.
    QStringList devices;
    if (device.isEmpty()) {
        for (const PortInfo& p : enumeratePorts()) devices << QString::fromStdString(p.device);
        if (devices.isEmpty()) {
            emit busy(false);
            emit failed(tr("Kein Anschluss gefunden. Steckt der Adapter am Rechner, und ist "
                           "ein Bluetooth-Adapter gekoppelt?"));
            return;
        }
    } else {
        devices << device;
    }
    const bool searching = device.isEmpty();
    const QVector<int> bauds = baud > 0           ? QVector<int>{baud}
                               : searching ? QVector<int>{38400, 115200, 9600}
                                           : QVector<int>{38400, 9600, 115200, 500000};

    std::string error;
    for (const QString& candidateDevice : devices) {
        if (searching) emit logLine(tr("Probiere %1 ...").arg(candidateDevice));
        for (int b : bauds) {
            auto candidate = std::make_unique<SerialTransport>();
            if (!candidate->open(candidateDevice.toStdString(), b, error)) break;
            emit logLine(tr("Anschluss %1 offen bei %2 Baud").arg(candidateDevice).arg(b));
            const bool last = !searching && b == bauds.back();
            if (finishConnect(std::move(candidate), protocolOrder, last)) {
                if (searching) emit adapterFound(candidateDevice);
                emit busy(false);
                refreshStatusAndFaults();
                return;
            }
            if (!searching) break;
        }
        if (!searching) {
            emit busy(false);
            if (!error.empty()) emit failed(QString::fromStdString(error));
            return;
        }
    }
    emit busy(false);
    emit failed(searching ? tr("An keinem Anschluss hat ein ELM327 geantwortet. Zündung an und "
                               "Adapter richtig eingesteckt?")
                          : QString::fromStdString(error));
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
    const bool ok = finishConnect(std::move(transport), protocolOrder, true);
    emit busy(false);
    if (ok) refreshStatusAndFaults();
}

#ifdef CARUSPEX_HAS_BLE
void Worker::scanBle(int milliseconds) {
    emit busy(true);
    QString error;
    const QList<BleDevice> devices = scanBleDevices(milliseconds, error);
    QStringList ids;
    QStringList names;
    QList<int> scores;
    for (const BleDevice& d : devices) {
        ids << d.id;
        names << d.name;
        scores << d.score;
    }
    emit busy(false);
    emit bleScanFinished(ids, names, scores, error);
}

void Worker::openBle(const QString& id, const QString& name, const QString& protocolOrder) {
    emit busy(true);
    elm_.setLog([this](const std::string& m) { emit logLine(QString::fromStdString(m)); });

    QList<BleDevice> candidates;
    if (id.isEmpty()) {
        emit logLine(tr("Suche Bluetooth-LE-Adapter ..."));
        QString error;
        for (const BleDevice& d : scanBleDevices(6000, error)) {
            if (d.score >= 3) candidates << d;
        }
        if (candidates.isEmpty()) {
            emit busy(false);
            emit failed(!error.isEmpty() ? error
                                         : tr("Kein Bluetooth-LE-Adapter in der Nähe gefunden. Steckt "
                                              "er im Auto, und ist die Zündung an?"));
            return;
        }
    } else {
        candidates << BleDevice{id, name, 0, 0};
    }

    std::string error;
    for (const BleDevice& d : candidates) {
        const QString shown = d.name.isEmpty() ? d.id : d.name;
        emit logLine(tr("Verbinde mit %1 über Bluetooth LE ...").arg(shown));
        auto transport = std::make_unique<BleTransport>();
        if (!transport->open(d, error)) {
            emit logLine(QString::fromStdString(error));
            continue;
        }
        emit logLine(tr("Kanal gefunden (%1)").arg(transport->profile()));
        if (finishConnect(std::move(transport), protocolOrder, true)) {
            emit bleAdapterFound(d.id, d.name);
            emit busy(false);
            refreshStatusAndFaults();
            return;
        }
    }
    emit busy(false);
    emit failed(QString::fromStdString(error));
}
#endif

void Worker::openDemo(const QString& protocolOrder) {
    // The demo car speaks CAN; the K-line protocols would only cost seconds.
    Q_UNUSED(protocolOrder);
    emit busy(true);
    elm_.setLog([this](const std::string& m) { emit logLine(QString::fromStdString(m)); });
    const bool ok = finishConnect(std::make_unique<DemoTransport>(), QStringLiteral("6"), true);
    emit busy(false);
    if (ok) refreshStatusAndFaults();
}

namespace {

UdsEcuRow toUdsRow(const UdsEcu& ecu) {
    UdsEcuRow row;
    row.request = ecu.definition.request;
    row.response = ecu.definition.response;
    row.extended29 = ecu.definition.extended29;
    row.kind = static_cast<int>(ecu.definition.kind);
    row.name = QString::fromStdString(ecu.definition.name);
    row.present = ecu.present;
    row.dtcsRead = ecu.dtcsRead;
    row.partNumber = QString::fromStdString(ecu.info.partNumber);
    row.softwareVersion = QString::fromStdString(ecu.info.softwareVersion);
    row.hardwareVersion = QString::fromStdString(ecu.info.hardwareVersion);
    row.systemName = QString::fromStdString(ecu.info.systemName);
    row.serial = QString::fromStdString(ecu.info.serial);
    row.note = QString::fromStdString(ecu.note);
    for (const UdsDtc& d : ecu.dtcs) {
        UdsDtcRow dtc;
        dtc.code = QString::fromStdString(d.code);
        dtc.failureText = QString::fromStdString(d.failureText);
        dtc.text = QString::fromStdString(d.text);
        dtc.testFailed = d.testFailed;
        dtc.pending = d.pending;
        dtc.confirmed = d.confirmed;
        dtc.warningLamp = d.warningLamp;
        row.dtcs.push_back(dtc);
    }
    return row;
}

}  // namespace

void Worker::scanUds(const QString& family) {
    if (!obd_) {
        emit udsScanFinished(0, 0, false, tr("Erst verbinden."));
        return;
    }
    if (!elm_.isCan()) {
        emit udsScanFinished(0, 0, false,
                             tr("Das Auto spricht kein CAN. UDS gibt es erst bei CAN-Autos, "
                                "bei älteren VW und Audi gehen die Steuergeräte über das KKL-Kabel."));
        return;
    }
    stopLive();
    stopVoltagePoll();
    cancelScan_ = false;
    emit busy(true);

    std::vector<EcuDefinition> list = family.isEmpty()
                                          ? genericEcuSweep()
                                          : ecuDefinitions(family.toStdString());
    const bool generic = family.isEmpty() || list.empty();
    if (list.empty()) list = genericEcuSweep();
    const int total = static_cast<int>(list.size());
    emit udsScanStarted(total, generic);

    // One unit at a time, so every answer reaches the page right away.
    int present = 0;
    int withFaults = 0;
    int index = 0;
    for (const EcuDefinition& def : list) {
        if (cancelScan_) break;
        emit udsScanProgress(QString::fromStdString(def.name), index++, total);
        const std::vector<UdsEcu> result = scanEcus(elm_, {def}, nullptr, [this] { return cancelScan_.load(); });
        if (result.empty() || !result.front().present) continue;
        ++present;
        if (!result.front().dtcs.empty()) ++withFaults;
        emit udsEcuResult(toUdsRow(result.front()));
    }
    emit udsScanFinished(present, withFaults, cancelScan_.load(), QString());
    emit busy(false);
}

void Worker::clearUdsEcu(quint32 request, quint32 response, bool extended29, const QString& name) {
    if (!obd_) return;
    stopLive();
    emit busy(true);
    EcuDefinition def;
    def.request = request;
    def.response = response;
    def.extended29 = extended29;
    def.name = name.toStdString();
    std::string error;
    bool ok = false;
    {
        Uds uds(elm_);
        if (uds.select(def, error)) ok = uds.clearDtcs(error);
    }
    emit udsClearResult(name, ok,
                        ok ? tr("Fehlerspeicher von %1 gelöscht.").arg(name)
                           : tr("Löschen bei %1 hat nicht geklappt: %2")
                                 .arg(name, QString::fromStdString(error)));
    // Read that unit again so the page shows what is left.
    const std::vector<UdsEcu> again = scanEcus(elm_, {def}, nullptr, [] { return false; });
    if (!again.empty() && again.front().present) {
        UdsEcuRow row = toUdsRow(again.front());
        row.name = name;
        emit udsEcuResult(row);
    }
    emit busy(false);
}

void Worker::openReplay(const QString& path, bool realTime, const QString& protocolOrder) {
    emit busy(true);
    elm_.setLog([this](const std::string& m) { emit logLine(QString::fromStdString(m)); });
    auto replay = std::make_unique<ReplayTransport>(path.toStdString(),
                                                    realTime ? ReplayPacing::RealTime : ReplayPacing::Fast);
    if (!replay->isOpen()) {
        emit busy(false);
        emit failed(tr("Aufnahme lässt sich nicht öffnen: %1").arg(QString::fromStdString(replay->error())));
        return;
    }
    emit logLine(QString::fromStdString(recordingSummary(replay->info())));
    const QString order = QString::fromStdString(replay->protocolOrder(protocolOrder.toStdString()));
    const bool ok = finishConnect(std::move(replay), order, true);
    emit busy(false);
    if (ok) refreshStatusAndFaults();
}

void Worker::startRecording(const QString& path) {
    if (!elm_.connected()) {
        emit recordingState(false, path, 0, tr("Erst verbinden, dann aufzeichnen."));
        return;
    }
    std::string error;
    if (!caruspex::startRecording(elm_, path.toStdString(), error)) {
        emit recordingState(false, path, 0, QString::fromStdString(error));
        return;
    }
    emit recordingState(true, path, 0, QString());
}

void Worker::stopRecording() {
    RecordingTransport* active = activeRecording(elm_);
    const int exchanges = active ? active->exchanges() : 0;
    const QString path = active ? QString::fromStdString(active->path()) : QString();
    caruspex::stopRecording(elm_);
    emit recordingState(false, path, exchanges, QString());
}

const CustomPid* Worker::findCustom(int key) const {
    for (const CustomPid& p : builtInPacks()) {
        if (p.key == key) return &p;
    }
    for (const CustomPid& p : userPids_) {
        if (p.key == key) return &p;
    }
    return nullptr;
}

int Worker::readCustomNow(const QVector<int>& keys, int* failed, QString* firstError) {
    std::vector<CustomPid> list;
    for (int key : keys) {
        if (const CustomPid* p = findCustom(key)) list.push_back(*p);
    }
    int ok = 0;
    int bad = 0;
    {
        // The reader restores the normal OBD addressing when it goes out of
        // scope, so the next OBD request works as before.
        CustomPidReader reader(elm_);
        for (const CustomReading& r : reader.readAll(list)) {
            if (r.ok) {
                ++ok;
                emit liveValue(toRow(r.value));
            } else {
                ++bad;
                if (firstError && firstError->isEmpty()) *firstError = QString::fromStdString(r.error);
            }
        }
    }
    if (failed) *failed = bad;
    return ok;
}

void Worker::readCustom(const QVector<int>& keys) {
    if (!obd_) {
        emit customRead(0, keys.size(), tr("Erst verbinden."));
        return;
    }
    emit busy(true);
    int failed = 0;
    QString error;
    const int ok = readCustomNow(keys, &failed, &error);
    emit customRead(ok, failed, error);
    emit busy(false);
}

void Worker::startCustomPoll(const QVector<int>& keys) {
    if (!obd_) return;
    customPollKeys_ = keys;
    if (!pollTimer_) {
        pollTimer_ = new QTimer(this);
        pollTimer_->setInterval(settings_.pollIntervalMs);
        QObject::connect(pollTimer_, &QTimer::timeout, this, &Worker::pollTick);
    }
    if (!customPollKeys_.isEmpty() && !pollTimer_->isActive()) pollTimer_->start();
}

void Worker::importCustomCsv(const QString& path) {
    QFile file(path);
    QStringList errors;
    QVector<CustomRow> rows;
    if (!file.open(QIODevice::ReadOnly)) {
        errors << tr("Datei lässt sich nicht öffnen: %1").arg(file.errorString());
        emit customImported(rows, errors);
        return;
    }
    std::vector<std::string> problems;
    // New imports get keys after the ones already loaded.
    const int base = kUserKeyBase + static_cast<int>(userPids_.size());
    std::vector<CustomPid> parsed = parseTorqueCsv(file.readAll().toStdString(), problems, base);
    for (const std::string& p : problems) errors << QString::fromStdString(p);
    for (CustomPid& p : parsed) {
        p.pack.clear();
        userPids_.push_back(p);
        rows.push_back({p.key, QString::fromStdString(p.name), QString::fromStdString(p.shortName),
                        QString::fromStdString(p.unit), QString(), QString::fromStdString(p.source),
                        QString::fromStdString(p.request), QString::fromStdString(p.header)});
    }
    emit customImported(rows, errors);
}

void Worker::probeAdapter() {
    if (!elm_.connected()) {
        emit failed(tr("Für den Adaptertest erst verbinden."));
        return;
    }
    stopLive();
    stopVoltagePoll();
    emit busy(true);
    const AdapterReport report = caruspex::probeAdapter(elm_);
    AdapterRow row;
    row.reachable = report.reachable;
    row.version = QString::fromStdString(report.versionText);
    row.description = QString::fromStdString(report.description);
    row.isStn = report.isStn;
    row.stnId = QString::fromStdString(report.stnId);
    row.voltage = report.voltage;
    row.hasVoltage = report.hasVoltage;
    row.roundTripMs = report.roundTripMs;
    row.roundTripWithHintMs = report.roundTripWithHintMs;
    row.behavesLike = report.behavesLikeLevel > 0
                          ? QString::fromStdString(versionText(report.behavesLikeLevel))
                          : QString();
    row.acceptsUnknown = report.acceptsUnknownCommands;
    row.verdict = QString::fromStdString(report.verdict.get());
    for (const auto& f : report.facts) row.facts << QString::fromStdString(f.get());
    for (const auto& t : report.speedTips) row.speedTips << QString::fromStdString(t.get());
    for (const CommandCheck& c : report.checks) {
        row.checks.push_back({QString::fromStdString(c.command),
                              QString::fromStdString(versionText(c.introducedIn)), c.supported,
                              QString::fromStdString(c.purpose.get())});
    }
    emit adapterReport(row);
    emit busy(false);
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
                        QString::fromStdString(f.text), f.throttleRelated,
                        isStoredKind(f.kind), QString::fromStdString(f.ecu)});
    }
    emit faults(rows);
    emit logLine(rows.isEmpty()
                     ? tr("Fehlerspeicher gelesen: kein Eintrag.")
                     : tr("Fehlerspeicher gelesen: %1 Einträge.").arg(rows.size()));

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
    emit logLine(ok ? tr("Fehlerspeicher gelöscht.")
                    : tr("Löschen fehlgeschlagen: %1")
                          .arg(QString::fromStdString(error)));
    emit busy(false);
    refreshStatusAndFaults();
}

void Worker::readDetails() {
    if (!obd_) return;
    emit busy(true);
    const QString vin = QString::fromStdString(obd_->readVin());
    // All freeze frames; the first one also goes out the old way for the
    // pages that only know a single frame.
    QVector<FreezeFrameRow> frames;
    for (const FreezeFrame& f : obd_->readFreezeFrames()) {
        FreezeFrameRow row;
        row.frame = f.frame;
        row.dtc = QString::fromStdString(f.dtc);
        row.ecu = QString::fromStdString(f.ecu);
        for (const LiveValue& v : f.values) row.values.push_back(toRow(v));
        frames.push_back(row);
    }
    emit vehicleInfo(vin, frames.isEmpty() ? QString() : frames.front().dtc,
                     frames.isEmpty() ? QVector<LiveRow>() : frames.front().values);
    emit freezeFrames(frames);

    // One reading of the values the start page shows as tiles, so they are
    // not empty until a measurement runs.
    const std::set<int> supported = obd_->supportedPids();
    for (int pid : {0x05, 0x2F, 0x42, 0xA6}) {
        if (!supported.empty() && !supported.count(pid)) continue;
        LiveValue value;
        if (obd_->readLive(pid, value)) emit liveValue(toRow(value));
    }
    emit busy(false);
}

void Worker::startLive(const QVector<int>& pids) {
    pollPids_ = pids;
    if (!obd_ || pids.isEmpty()) return;
    pollIndex_ = 0;
    if (!pollTimer_) {
        pollTimer_ = new QTimer(this);
        // No faster than 50 requests per second. On the K-Line a request takes
        // longer anyway, but a fast CAN adapter or the simulator would
        // otherwise flood the UI with signals.
        pollTimer_->setInterval(settings_.pollIntervalMs);
        QObject::connect(pollTimer_, &QTimer::timeout, this, &Worker::pollTick);
    }
    pollTimer_->start();
    emit logLine(tr("Messung gestartet, %1 Werte im Umlauf.").arg(pids.size()));
}

void Worker::stopLive() {
    customPollKeys_.clear();
    if (pollTimer_ && pollTimer_->isActive()) {
        pollTimer_->stop();
        emit logLine(tr("Messung gestoppt."));
    }
}

// One value per tick. That keeps the event loop free and stop requests from
// the UI get through immediately.
void Worker::pollTick() {
    if (!obd_) return;
    // Every other tick belongs to the manufacturer values when some are
    // selected. They change slowly and cost an addressing round each time.
    if (!customPollKeys_.isEmpty()) {
        customTurn_ = !customTurn_;
        if (customTurn_ || pollPids_.isEmpty()) {
            readCustomNow(customPollKeys_, nullptr, nullptr);
            return;
        }
    }
    if (pollPids_.isEmpty()) return;
    // Up to six PIDs per tick: on CAN they go out as one request, elsewhere
    // the core reads them one by one. Either way the event loop gets back
    // control after each slice, so a stop request is not delayed.
    std::vector<int> slice;
    for (int i = 0; i < 6 && i < pollPids_.size(); ++i) {
        slice.push_back(pollPids_[pollIndex_ % pollPids_.size()]);
        pollIndex_ = (pollIndex_ + 1) % pollPids_.size();
    }
    for (const LiveValue& value : obd_->readLiveBatch(slice)) emit liveValue(toRow(value));
}

void Worker::sendRaw(const QString& command) {
    ElmResult result = elm_.send(command.toStdString(), 8000);
    QStringList lines;
    for (const std::string& line : result.lines) lines << QString::fromStdString(line);
    if (lines.isEmpty()) lines << tr("(keine Antwort)");
    emit rawReply(lines.join('\n'));
}

// --- KWP1281 over KKL -------------------------------------------------------

void Worker::openKkl(const QString& device, int ecuAddress) {
    closeKkl();
    emit busy(true);
    emit kklMessage(tr("Wecke das Steuergerät mit der 5-Baud-Adresse, "
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
    // The configured session type must be set before basic settings can be
    // started.
    kwp_->setBasicSettingSession(
        static_cast<std::uint8_t>(settings_.basicSettingSession));
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
    emit kklMessage(tr("Verbunden mit %1 über %2. Vor dem Start die Punkte "
                       "oben prüfen.")
                        .arg(QString::fromStdString(ecuName(ecu)),
                             QString::fromStdString(kwp_->protocolName())),
                    false);
    emit logLine(tr("KWP1281 verbunden: %1").arg(identification.join(QStringLiteral(" / "))));
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
        emit kklMessage(tr("Erst das KKL-Kabel verbinden."), true);
        return;
    }
    basicSettingGroup_ = group;
    if (!basicSettingTimer_) {
        basicSettingTimer_ = new QTimer(this);
        // The control unit drops out of basic settings if nothing arrives for
        // about a second. We ask considerably more often.
        basicSettingTimer_->setInterval(settings_.basicSettingIntervalMs);
        QObject::connect(basicSettingTimer_, &QTimer::timeout, this,
                         &Worker::basicSettingTick);
    }
    basicSettingTimer_->start();
    emit kklMessage(tr("Grundeinstellung im Messwertblock %1 läuft. Gaspedal "
                       "nicht antippen.")
                        .arg(group),
                    false);
    emit logLine(tr("Grundeinstellung Block %1 gestartet").arg(group));
}

void Worker::stopBasicSetting() {
    if (basicSettingTimer_ && basicSettingTimer_->isActive()) {
        basicSettingTimer_->stop();
        emit kklMessage(tr("Grundeinstellung beendet."), false);
        emit logLine(tr("Grundeinstellung beendet"));
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
        row.name = tr("Feld %1: %2").arg(index).arg(QString::fromStdString(m.label));
        row.unit = QString::fromStdString(m.unit);
        row.value = m.value;
        rows.push_back(row);
        ++index;
    }
    emit basicSettingValues(rows);
}

// Searches all known control units one after another. Each attempt takes a few
// seconds because of the 5-baud wake-up address; for control units that are not
// fitted, the time spent waiting for a reply comes on top.
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
            row.note = tr("meldet sich nicht, vermutlich nicht verbaut");
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
                fault.kind = f.sporadic ? tr("sporadisch") : tr("gespeichert");
                fault.stored = !f.sporadic;
                fault.text = QString::fromStdString(f.text);
                row.faults.push_back(fault);
            }
            if (!row.faults.isEmpty()) ++withFaults;
            row.note = row.faults.isEmpty()
                           ? tr("kein Eintrag")
                           : tr("%1 Einträge").arg(row.faults.size());

            // Persistent clear: read back after every attempt. Some control
            // units only accept the clear command on the second try.
            if (clearAttempts > 0 && !faults.empty()) {
                bool cleared = false;
                for (int attempt = 1; attempt <= clearAttempts && !cleared; ++attempt) {
                    emit clearProgress(tr("%1: lösche, Versuch %2 von %3")
                                           .arg(row.name)
                                           .arg(attempt)
                                           .arg(clearAttempts));
                    if (!probe.clearFaults(localError)) {
                        emit clearProgress(tr("   abgelehnt: %1")
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
                    row.note = tr("gelöscht");
                    --withFaults;
                    emit clearProgress(tr("   %1: leer").arg(row.name));
                } else {
                    row.note = tr("%1 Einträge, Löschen ohne Wirkung")
                                   .arg(row.faults.size());
                    emit clearProgress(
                        tr("   %1: bleibt bestehen, die Ursache ist noch da")
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

// Persistent clear over EOBD. Some control units only accept the command right
// after the connection is established, others need several attempts. That is
// why the connection is re-established in between.
void Worker::bruteForceClear(int attempts) {
    if (!obd_) {
        emit clearProgress(tr("Nicht verbunden."));
        emit clearFinished(false, 0);
        return;
    }
    emit busy(true);

    int remaining = 0;
    for (int attempt = 1; attempt <= attempts; ++attempt) {
        emit clearProgress(tr("Versuch %1 von %2 ...").arg(attempt).arg(attempts));

        std::string error;
        const bool acknowledged = obd_->clearFaults(error);
        if (!acknowledged) {
            emit clearProgress(tr("   Steuergerät quittiert nicht: %1")
                                   .arg(QString::fromStdString(error)));
        }
        sleepMs(600);

        const std::vector<Fault> left = obd_->readFaults(error);
        remaining = static_cast<int>(left.size());
        if (left.empty()) {
            emit clearProgress(tr("   Speicher ist leer."));
            emit busy(false);
            emit clearFinished(true, 0);
            refreshStatusAndFaults();
            return;
        }
        emit clearProgress(tr("   noch %1 Einträge").arg(remaining));

        // From the second failure on, re-establish the connection. Some control
        // units only open the clear window right after waking up.
        if (attempt >= 2 && attempt < attempts) {
            emit clearProgress(tr("   Verbindung wird neu aufgebaut ..."));
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

void Worker::applySettings(const AppSettings& values) {
    settings_ = values;

    // Update running timers right away, otherwise a change would only take
    // effect after the next connection.
    if (pollTimer_) pollTimer_->setInterval(settings_.pollIntervalMs);
    if (basicSettingTimer_) {
        basicSettingTimer_->setInterval(settings_.basicSettingIntervalMs);
    }

    elm_.setTimeoutScale(static_cast<double>(settings_.timeoutPercent) / 100.0);
    elm_.setAdaptiveTiming(settings_.adaptiveTiming == 2   ? AdaptiveTiming::Aggressive
                           : settings_.adaptiveTiming == 1 ? AdaptiveTiming::Auto
                                                           : AdaptiveTiming::Standard);
    elm_.setVerbose(settings_.logRaw);
    if (kwp_) {
        kwp_->setBasicSettingSession(
            static_cast<std::uint8_t>(settings_.basicSettingSession));
    }
}

void Worker::setBasicSettingSession(int mode) {
    if (kwp_) kwp_->setBasicSettingSession(static_cast<std::uint8_t>(mode));
    emit logLine(tr("Sitzungsart für die Grundeinstellung: 0x%1")
                     .arg(QStringLiteral("%1").arg(mode, 2, 16, QChar('0')).toUpper()));
}

void Worker::readAdaptationChannel(int channel) {
    if (!kwp_ || !kwp_->connected()) {
        emit kklMessage(tr("Erst das KKL-Kabel verbinden."), true);
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
            row.name = tr("Feld %1: %2").arg(index).arg(QString::fromStdString(m.label));
            row.unit = QString::fromStdString(m.unit);
            row.value = m.value;
            rows.push_back(row);
        }
        ++index;
    }
    emit busy(false);
    emit adaptationResult(channel, value, rows);
    emit kklMessage(tr("Anpassungskanal %1 steht auf %2.").arg(channel).arg(value),
                    false);
}

void Worker::runActuatorTest() {
    if (!kwp_ || !kwp_->connected()) {
        emit kklMessage(tr("Erst das KKL-Kabel verbinden."), true);
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

// Modes 06, 05 and 09 in one go: everything the control unit reveals about
// itself and its running self-tests.
void Worker::runHealthCheck(int seconds) {
    using namespace analysis;
    if (!obd_) {
        emit failed(tr("Für den Gesundheitscheck erst verbinden."));
        return;
    }
    stopLive();
    cancelScan_ = false;
    emit busy(true);
    HealthInput input;

    emit healthProgress(tr("Bereitschaft und Fehlerlampe"), 4);
    const VehicleStatus st = obd_->readStatus();
    input.setStatus(st);
    StatusRow statusRow;
    statusRow.valid = st.valid;
    statusRow.mil = st.milOn;
    statusRow.faultCount = st.faultCount;
    for (const ReadinessItem& item : st.readiness) {
        statusRow.monitorNames << QString::fromStdString(item.name);
        statusRow.monitorReady << item.ready;
    }
    emit status(statusRow);

    emit healthProgress(tr("Fehlerspeicher"), 10);
    std::string error;
    input.faults = obd_->readFaults(error);
    input.faultsRead = error.empty();
    QVector<FaultRow> faultRows;
    for (const Fault& f : input.faults) {
        faultRows.push_back({QString::fromStdString(f.code), QString::fromStdString(f.kind),
                             QString::fromStdString(f.text), f.throttleRelated,
                             isStoredKind(f.kind), QString::fromStdString(f.ecu)});
    }
    emit faults(faultRows);
    input.faultLevel = [](const Fault& f) {
        // The database knows how urgent a code is; pending codes count one
        // step lower, they may still disappear on their own.
        const Severity severity = lookupDtc(f.code).severity;
        Level level = Level::Info;
        if (severity == Severity::Critical) level = Level::Critical;
        else if (severity == Severity::High || severity == Severity::Medium) level = Level::Warning;
        if (!isStoredKind(f.kind) && level == Level::Warning) level = Level::Info;
        return level;
    };

    emit healthProgress(tr("Bauteilprüfung (Modus 06)"), 18);
    std::string monitorNote;
    input.monitorResults = obd_->readMonitorResults(monitorNote);
    input.monitorResultsRead = true;

    LiveValue fuelType;
    if (obd_->readLive(0x51, fuelType)) input.fuel = fuelTypeFromObd(static_cast<int>(fuelType.value));

    // Live series: only what the car offers.
    const std::set<int> supported = obd_->supportedPids();
    std::vector<int> pids;
    for (int pid : healthLivePids()) {
        if (supported.empty() || supported.count(pid)) pids.push_back(pid);
    }
    const double start = appSeconds();
    const int total = std::max(1, seconds);
    while (!pids.empty() && appSeconds() - start < seconds && !cancelScan_) {
        const std::vector<LiveValue> values = obd_->readLiveBatch(pids);
        input.addSnapshot(values, appSeconds() - start);
        for (const LiveValue& value : values) emit liveValue(toRow(value));
        const double elapsed = appSeconds() - start;
        const int left = std::max(0, seconds - static_cast<int>(elapsed));
        const int percent = 25 + static_cast<int>(70.0 * elapsed / total);
        emit healthProgress(tr("Messwerte sammeln, noch %1 s").arg(left), std::min(95, percent));
    }

    const HealthReport report = analysis::runHealthCheck(input);
    HealthRow row;
    row.score = report.score;
    row.grade = static_cast<int>(report.grade);
    row.summary = QString::fromStdString(report.summary.text());
    row.checksRun = report.checksRun;
    row.diesel = input.fuel == FuelType::Diesel;
    for (const Finding& f : report.findings) {
        HealthFindingRow fr;
        fr.level = static_cast<int>(f.level);
        fr.title = QString::fromStdString(f.title.text());
        fr.explanation = QString::fromStdString(f.explanation.text());
        fr.advice = QString::fromStdString(f.advice.text());
        row.findings.push_back(fr);
    }
    for (const SkippedCheck& skipped : report.notChecked) {
        row.notChecked << QStringLiteral("%1: %2").arg(QString::fromStdString(skipped.title.text()),
                                                       QString::fromStdString(skipped.reason.text()));
    }
    for (const MonitorState& m : input.readiness) {
        if (!m.supported) continue;
        const MonitorGuide& guide = driveCycleGuide(m.monitor);
        MonitorGuideRow g;
        g.name = QString::fromUtf8(guide.name.get());
        g.complete = m.complete;
        g.explanation = QString::fromUtf8(guide.explanation.get());
        g.drivePattern = QString::fromUtf8(guide.drivePattern.get());
        g.typicalTime = QString::fromUtf8(guide.typicalTime.get());
        g.coldStart = guide.needsColdStart;
        row.monitors.push_back(g);
    }
    emit healthProgress(tr("Fertig"), 100);
    emit healthResult(row);
    emit busy(false);
}

void Worker::startVoltagePoll(int intervalMs) {
    if (!elm_.connected()) return;
    stopLive();
    if (!voltageTimer_) {
        voltageTimer_ = new QTimer(this);
        connect(voltageTimer_, &QTimer::timeout, this, [this] {
            const ElmResult r = elm_.send("ATRV", 1000);
            for (const std::string& line : r.lines) {
                QString text = QString::fromStdString(line).trimmed();
                text.remove(QLatin1Char('V'));
                bool ok = false;
                const double volts = text.toDouble(&ok);
                if (ok && volts > 3.0 && volts < 32.0) {
                    emit adapterVoltage(volts, appSeconds());
                    break;
                }
            }
        });
    }
    voltageTimer_->setInterval(std::max(20, intervalMs));
    voltageTimer_->start();
}

void Worker::stopVoltagePoll() {
    if (voltageTimer_) voltageTimer_->stop();
}

void Worker::readMonitors() {
    if (!obd_) {
        emit monitorResults({}, tr("Nicht verbunden."));
        return;
    }
    emit busy(true);

    const VehicleIdentity id = obd_->readIdentity();
    emit identity(QString::fromStdString(id.vin),
                  QString::fromStdString(id.calibrationId),
                  QString::fromStdString(id.calibrationVerification),
                  QString::fromStdString(id.ecuName));

    std::string note;
    QVector<MonitorRow> rows;
    for (const MonitorResult& r : obd_->readMonitorResults(note)) {
        MonitorRow row;
        row.name = QString::fromStdString(r.name);
        row.component = QString::fromStdString(r.component);
        row.value = r.value;
        row.minimum = r.minimum;
        row.maximum = r.maximum;
        row.unit = QString::fromStdString(r.unit);
        row.hasMinimum = r.hasMinimum;
        row.hasMaximum = r.hasMaximum;
        row.passed = r.passed;
        rows.push_back(row);
    }
    emit monitorResults(rows, QString::fromStdString(note));

    std::string oxygenNote;
    QVector<OxygenRow> oxygenRows;
    for (const OxygenSensorTest& t : obd_->readOxygenSensorTests(oxygenNote)) {
        OxygenRow row;
        row.name = QString::fromStdString(t.name);
        row.sensor = t.sensor;
        row.value = t.value;
        row.unit = QString::fromStdString(t.unit);
        oxygenRows.push_back(row);
    }
    emit oxygenTests(oxygenRows, QString::fromStdString(oxygenNote));

    emit busy(false);
    emit logLine(tr("Bauteilüberwachung gelesen: %1 Tests, %2 Sondenprüfungen.")
                     .arg(rows.size())
                     .arg(oxygenRows.size()));
}

// --- Session ----------------------------------------------------------------

Session::Session(QObject* parent) : QObject(parent) {
    qRegisterMetaType<FaultRow>("caruspex::FaultRow");
    qRegisterMetaType<QVector<FaultRow>>("QVector<caruspex::FaultRow>");
    qRegisterMetaType<LiveRow>("caruspex::LiveRow");
    qRegisterMetaType<QVector<LiveRow>>("QVector<caruspex::LiveRow>");
    qRegisterMetaType<StatusRow>("caruspex::StatusRow");
    qRegisterMetaType<EcuScanRow>("caruspex::EcuScanRow");
    qRegisterMetaType<MonitorRow>("caruspex::MonitorRow");
    qRegisterMetaType<QVector<MonitorRow>>("QVector<caruspex::MonitorRow>");
    qRegisterMetaType<OxygenRow>("caruspex::OxygenRow");
    qRegisterMetaType<QVector<OxygenRow>>("QVector<caruspex::OxygenRow>");
    qRegisterMetaType<QVector<int>>("QVector<int>");
    qRegisterMetaType<AppSettings>("caruspex::AppSettings");
    qRegisterMetaType<HealthRow>("caruspex::HealthRow");
    qRegisterMetaType<AdapterRow>("caruspex::AdapterRow");
    qRegisterMetaType<FreezeFrameRow>("caruspex::FreezeFrameRow");
    qRegisterMetaType<UdsEcuRow>("caruspex::UdsEcuRow");
    qRegisterMetaType<CustomRow>("caruspex::CustomRow");
    qRegisterMetaType<QVector<CustomRow>>("QVector<caruspex::CustomRow>");
    qRegisterMetaType<QVector<FreezeFrameRow>>("QVector<caruspex::FreezeFrameRow>");

    worker_ = new Worker();
    worker_->moveToThread(&thread_);
    QObject::connect(&thread_, &QThread::finished, worker_, &QObject::deleteLater);

    connect(this, &Session::openSerialRequested, worker_, &Worker::openSerial);
    connect(this, &Session::openTcpRequested, worker_, &Worker::openTcp);
    connect(this, &Session::openDemoRequested, worker_, &Worker::openDemo);
#ifdef CARUSPEX_HAS_BLE
    connect(this, &Session::scanBleRequested, worker_, &Worker::scanBle);
    connect(this, &Session::openBleRequested, worker_, &Worker::openBle);
#endif
    connect(this, &Session::openReplayRequested, worker_, &Worker::openReplay);
    connect(this, &Session::startRecordingRequested, worker_, &Worker::startRecording);
    connect(this, &Session::customReadRequested, worker_, &Worker::readCustom);
    connect(this, &Session::customPollRequested, worker_, &Worker::startCustomPoll);
    connect(this, &Session::customImportRequested, worker_, &Worker::importCustomCsv);
    connect(this, &Session::stopRecordingRequested, worker_, &Worker::stopRecording);
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
    connect(this, &Session::settingsChanged, worker_, &Worker::applySettings);
    connect(this, &Session::adaptationRequested, worker_,
            &Worker::readAdaptationChannel);
    connect(this, &Session::actuatorTestRequested, worker_,
            &Worker::runActuatorTest);
    connect(this, &Session::healthCheckRequested, worker_, &Worker::runHealthCheck);
    connect(this, &Session::probeAdapterRequested, worker_, &Worker::probeAdapter);
    connect(this, &Session::udsScanRequested, worker_, &Worker::scanUds);
    connect(this, &Session::udsClearRequested, worker_, &Worker::clearUdsEcu);
    connect(this, &Session::voltagePollRequested, worker_, &Worker::startVoltagePoll);
    connect(this, &Session::voltagePollStopRequested, worker_, &Worker::stopVoltagePoll);
    connect(this, &Session::readMonitorsRequested, worker_,
            &Worker::readMonitors);
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
