// Bindeglied zwischen Oberflaeche und Diagnosekern.
//
// Der Kern kennt kein Qt. Diese Klasse laeuft in einem eigenen Thread und
// reicht Ergebnisse per Signal an die Oberflaeche weiter, damit das Fenster
// waehrend der langsamen K-Line-Kommunikation bedienbar bleibt.
#pragma once

#include <QList>
#include <QMetaType>
#include <QObject>
#include <QString>
#include <QStringList>
#include <QThread>
#include <QVector>

#include <memory>

#include "core/Elm327.h"
#include "core/KLineStream.h"
#include "core/Kwp1281.h"
#include "core/Obd2.h"

class QTimer;

namespace caruspex {

struct FaultRow {
    QString code;
    QString kind;
    QString text;
    bool throttleRelated = false;
};

struct LiveRow {
    int pid = 0;
    QString name;
    QString unit;
    QString text;   // gesetzt bei Statuswerten
    double value = 0.0;
    bool warn = false;
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

    // KWP1281 ueber ein KKL-Kabel: die Wege, an die EOBD nicht herankommt.
    void openKkl(const QString& device, int ecuAddress);
    void closeKkl();
    void startBasicSetting(int group);
    void stopBasicSetting();

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

    KLineStream kline_;
    std::unique_ptr<Kwp1281> kwp_;
    QTimer* basicSettingTimer_ = nullptr;
    int basicSettingGroup_ = 60;
};

// Besitzt den Thread und den Worker, wird von der Oberflaeche benutzt.
class Session : public QObject {
    Q_OBJECT

public:
    explicit Session(QObject* parent = nullptr);
    ~Session() override;

    Worker* worker() const { return worker_; }
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

private:
    QThread thread_;
    Worker* worker_ = nullptr;
    bool connected_ = false;
};

}  // namespace caruspex

Q_DECLARE_METATYPE(caruspex::FaultRow)
Q_DECLARE_METATYPE(caruspex::LiveRow)
Q_DECLARE_METATYPE(caruspex::StatusRow)
