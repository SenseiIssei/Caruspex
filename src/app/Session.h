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

struct FaultRow {
    QString code;
    QString kind;
    QString text;
    bool throttleRelated = false;
    // Confirmed (stored) fault, as opposed to intermittent/pending or
    // permanent. Kept separately because kind is display text and depends on
    // the language.
    bool stored = false;
};

struct LiveRow {
    int pid = 0;
    QString name;
    QString unit;
    QString text;   // set for status values
    double value = 0.0;
    bool warn = false;
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

private slots:
    void pollTick();
    void basicSettingTick();

private:
    void refreshStatusAndFaults();

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
