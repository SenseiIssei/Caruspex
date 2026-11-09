// Link between the UI and the diagnostic core.
//
// The core knows nothing about Qt. This class runs in its own thread and
// passes results to the UI via signals, so the window stays usable during the
// slow K-Line communication.
#pragma once

#include <QList>
#include <QMetaType>
#include <QObject>
#include <QString>
#include <QStringList>
#include <QThread>
#include <QVector>

#include <atomic>
#include <memory>

#include "core/Elm327.h"
#include "core/KLineStream.h"
#include "core/Kwp1281.h"
#include "core/VagKLine.h"
#include "core/Obd2.h"
#include "app/Settings.h"

class QTimer;

namespace caruspex {
// Seconds since the program started, monotonic.
double appSeconds();
}

namespace caruspex {

struct FaultRow {
    QString code;
    QString kind;
    QString text;
    bool throttleRelated = false;
    // Confirmed (stored) fault, as opposed to intermittent/pending or
    // permanent. Kept separately because kind is display text and depends on
    // the language.
    bool stored = false;
    // Which control unit reported it, e.g. "Motor (7E8)". Empty on the
    // K-line or when only one unit answers.
    QString ecu;
};

struct LiveRow {
    int pid = 0;
    QString name;
    QString unit;
    QString text;   // set for status values
    double value = 0.0;
    bool warn = false;
    // Seconds since the program started, taken in the worker right after the
    // reply. The trip computer and the stopwatch rely on it.
    double time = 0.0;
    // Sensible scale for charts and gauges, 0/0 when the table has none.
    double displayMin = 0.0;
    double displayMax = 0.0;
};

// Result of the health check, prepared for the UI.
struct HealthFindingRow {
    int level = 0;          // analysis::Level: 0 good, 1 info, 2 warning, 3 critical
    QString area;
    QString title;
    QString explanation;
    QString advice;
};

struct MonitorGuideRow {
    QString name;
    bool complete = false;
    QString explanation;
    QString drivePattern;
    QString typicalTime;
    bool coldStart = false;
};

// Result of the adapter test, prepared for the UI.
struct AdapterCheckRow {
    QString command;
    QString version;     // the ELM version that introduced it
    bool supported = false;
    QString purpose;
};

struct AdapterRow {
    bool reachable = false;
    QString version;
    QString description;
    bool isStn = false;
    QString stnId;
    double voltage = 0.0;
    bool hasVoltage = false;
    double roundTripMs = 0.0;
    double roundTripWithHintMs = 0.0;
    QString behavesLike;
    bool acceptsUnknown = false;
    QString verdict;
    QStringList facts;
    QStringList speedTips;
    QVector<AdapterCheckRow> checks;
};

struct HealthRow {
    int score = -1;
    int grade = 0;          // analysis::TrafficLight: 0 green, 1 yellow, 2 red
    QString summary;
    QVector<HealthFindingRow> findings;
    QStringList notChecked;         // "title: reason"
    QVector<MonitorGuideRow> monitors;
    int checksRun = 0;
    bool diesel = false;
};

// Result for one control unit when scanning the whole vehicle.
struct EcuScanRow {
    int address = 0;
    QString name;
    bool present = false;
    QString identification;
    QVector<FaultRow> faults;
    QString note;
};

// A test result from component monitoring, prepared for the UI.
struct MonitorRow {
    QString name;
    QString component;
    double value = 0.0;
    double minimum = 0.0;
    double maximum = 0.0;
    QString unit;
    bool hasMinimum = false;
    bool hasMaximum = false;
    bool passed = true;
};

struct OxygenRow {
    QString name;
    int sensor = 0;
    double value = 0.0;
    QString unit;
};

struct StatusRow {
    bool valid = false;
    bool mil = false;
    int faultCount = 0;
    QStringList monitorNames;
    QList<bool> monitorReady;
};

class Worker : public QObject {
    Q_OBJECT

public:
    Worker();
    ~Worker() override;

public slots:
    void openSerial(const QString& device, int baud, const QString& protocolOrder);
    void openTcp(const QString& host, int port, const QString& protocolOrder);
    // The built-in demo car, no adapter needed.
    void openDemo(const QString& protocolOrder);
    void closeAdapter();
    void readFaults();
    void clearFaults();
    void readDetails();
    void startLive(const QVector<int>& pids);
    void stopLive();
    void sendRaw(const QString& command);

    // KWP1281 over a KKL cable: the functions EOBD cannot reach.
    void openKkl(const QString& device, int ecuAddress);
    void closeKkl();
    void startBasicSetting(int group);
    void stopBasicSetting();
    // Contact all known control units one after another.
    // clearAttempts greater than zero means: clear everywhere and verify,
    // several times if necessary.
    void scanAllEcus(const QString& device, int clearAttempts);
    // Persistent clear over EOBD: clear several times, read back each time,
    // re-establish the connection if needed.
    void bruteForceClear(int attempts);
    void readAdaptationChannel(int channel);
    void runActuatorTest();
    void readMonitors();
    void setBasicSettingSession(int mode);
    // Full health check: faults, readiness, component monitoring and a
    // series of live values over the given number of seconds.
    void runHealthCheck(int seconds);
    // Tests what the adapter really is and can do.
    void probeAdapter();
    // Adapter voltage (ATRV) in a fast loop, for the battery test.
    void startVoltagePoll(int intervalMs);
    void stopVoltagePoll();
    // Applies the settings. Called at startup and after every change; also
    // takes effect during a running measurement.
    void applySettings(const caruspex::AppSettings& values);

public:
    // Cancels a running scan. Allowed from any thread.
    void cancelScan() { cancelScan_ = true; }

signals:
    void logLine(const QString& line);
    void connected(const QString& adapter, const QString& transport, const QString& protocol);
    void failed(const QString& error);
    // The automatic search found the adapter on this port.
    void adapterFound(const QString& device);
    void closed();
    void busy(bool active);
    void faults(const QVector<caruspex::FaultRow>& rows);
    void status(const caruspex::StatusRow& row);
    void liveValue(const caruspex::LiveRow& row);
    void vehicleInfo(const QString& vin, const QString& freezeCode,
                     const QVector<caruspex::LiveRow>& freezeRows);
    void supportedPids(const QVector<int>& pids);
    void rawReply(const QString& reply);
    void kklConnected(bool connected, const QString& identification);
    void kklMessage(const QString& message, bool isError);
    void basicSettingValues(const QVector<caruspex::LiveRow>& values);
    void scanStarted(int total);
    void scanProgress(const QString& ecuName, int index, int total);
    void scanEcuResult(const caruspex::EcuScanRow& row);
    void scanFinished(int present, int withFaults, bool cancelled);
    void clearProgress(const QString& line);
    void clearFinished(bool everythingGone, int remaining);
    void adaptationResult(int channel, int value, const QVector<caruspex::LiveRow>& shown);
    void actuatorResult(const QString& actuator);
    void kklProtocol(const QString& protocolName, bool isKwp2000);
    void monitorResults(const QVector<caruspex::MonitorRow>& rows,
                        const QString& note);
    void oxygenTests(const QVector<caruspex::OxygenRow>& rows,
                     const QString& note);
    void identity(const QString& vin, const QString& calibration,
                  const QString& checksum, const QString& ecuName);
    void healthProgress(const QString& step, int percent);
    void healthResult(const caruspex::HealthRow& row);
    void adapterVoltage(double volts, double time);
    void adapterReport(const caruspex::AdapterRow& row);

private slots:
    void pollTick();
    void basicSettingTick();

private:
    void refreshStatusAndFaults();
    // Hands a ready transport to the ELM327 driver and reports the result.
    bool finishConnect(std::unique_ptr<ITransport> transport, const QString& protocolOrder,
                       bool reportFailure);

    Elm327 elm_;
    std::unique_ptr<Obd2> obd_;
    QTimer* pollTimer_ = nullptr;
    QVector<int> pollPids_;
    int pollIndex_ = 0;

    // Set from the UI thread, hence atomic. The scan blocks the worker's event
    // loop, so a signal would not get through.
    std::atomic<bool> cancelScan_{false};

    KLineStream kline_;
    std::unique_ptr<VagKLine> kwp_;
    QTimer* basicSettingTimer_ = nullptr;
    QTimer* voltageTimer_ = nullptr;
    int basicSettingGroup_ = 60;
    AppSettings settings_;
};

// Owns the thread and the worker, used by the UI.
class Session : public QObject {
    Q_OBJECT

public:
    explicit Session(QObject* parent = nullptr);
    ~Session() override;

    Worker* worker() const { return worker_; }
    // May be called from the UI thread.
    void cancelScan();
    bool isConnected() const { return connected_; }

signals:
    void openSerialRequested(const QString& device, int baud, const QString& protocolOrder);
    void openTcpRequested(const QString& host, int port, const QString& protocolOrder);
    void openDemoRequested(const QString& protocolOrder);
    void closeRequested();
    void readFaultsRequested();
    void clearFaultsRequested();
    void readDetailsRequested();
    void startLiveRequested(const QVector<int>& pids);
    void stopLiveRequested();
    void sendRawRequested(const QString& command);
    void openKklRequested(const QString& device, int ecuAddress);
    void closeKklRequested();
    void startBasicSettingRequested(int group);
    void stopBasicSettingRequested();
    void scanAllRequested(const QString& device, int clearAttempts);
    void bruteForceClearRequested(int attempts);
    void adaptationRequested(int channel);
    void actuatorTestRequested();
    void basicSettingSessionRequested(int mode);
    void readMonitorsRequested();
    void healthCheckRequested(int seconds);
    void probeAdapterRequested();
    void voltagePollRequested(int intervalMs);
    void voltagePollStopRequested();
    void settingsChanged(const caruspex::AppSettings& values);

private:
    QThread thread_;
    Worker* worker_ = nullptr;
    bool connected_ = false;
};

}  // namespace caruspex

Q_DECLARE_METATYPE(caruspex::FaultRow)
Q_DECLARE_METATYPE(caruspex::LiveRow)
Q_DECLARE_METATYPE(caruspex::StatusRow)
Q_DECLARE_METATYPE(caruspex::EcuScanRow)
Q_DECLARE_METATYPE(caruspex::MonitorRow)
Q_DECLARE_METATYPE(caruspex::OxygenRow)
Q_DECLARE_METATYPE(caruspex::HealthRow)
Q_DECLARE_METATYPE(caruspex::AdapterRow)
