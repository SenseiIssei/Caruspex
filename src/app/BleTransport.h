// ELM327 over Bluetooth LE. Lives on the worker thread like the other
// transports; the blocking read of ITransport is built from short local
// event loops, so the Qt Bluetooth signals keep flowing while it waits.
#pragma once

#include <QBluetoothDeviceInfo>
#include <QByteArray>
#include <QList>
#include <QLowEnergyCharacteristic>
#include <QObject>
#include <QPointer>

#include <functional>
#include <memory>
#include <vector>

#include "core/Transport.h"

class QLowEnergyController;
class QLowEnergyService;
class QBluetoothSocket;

namespace caruspex {

// One adapter seen by a scan.
struct BleDevice {
    QString id;      // address on Windows and Linux, a UUID on macOS
    QString name;
    int rssi = 0;
    int score = 0;   // obdAdapterNameScore
    bool lowEnergy = true;   // offers Bluetooth LE
    bool classic = false;    // offers classic Bluetooth (RFCOMM serial port)
};

// Bit mask for passing the two flags through signals.
inline int bluetoothKinds(const BleDevice& d) { return (d.lowEnergy ? 1 : 0) | (d.classic ? 2 : 0); }

// Scans for Bluetooth devices for the given time and returns them, OBD
// adapters first. With classic set, classic devices (paired or visible) are
// listed too. Blocks with a local event loop; error is set when the system
// has no usable Bluetooth.
QList<BleDevice> scanBleDevices(int milliseconds, QString& error, bool classic = true);

// Classic Bluetooth adapter over an RFCOMM serial port, without a COM port
// or an rfcomm binding in the system. The adapter has to be paired once in
// the system settings (PIN usually 1234 or 0000).
class RfcommTransport : public QObject, public ITransport {
    Q_OBJECT

public:
    RfcommTransport();
    ~RfcommTransport() override;

    bool open(const BleDevice& device, std::string& error);
    bool write(const std::string& data) override;
    std::string read(int timeoutMs) override;
    void flushInput() override;
    std::string name() const override { return name_; }

private:
    QBluetoothSocket* socket_ = nullptr;
    QByteArray buffer_;
    bool disconnected_ = false;
    std::string name_;
};

class BleTransport : public QObject, public ITransport {
    Q_OBJECT

public:
    BleTransport();
    ~BleTransport() override;

    // Connects, finds the serial channel and switches notifications on.
    bool open(const BleDevice& device, std::string& error);

    bool write(const std::string& data) override;
    std::string read(int timeoutMs) override;
    void flushInput() override;
    std::string name() const override { return name_; }

    // Which channel was used ("FFF0", "vLinker", "generic", ...), for the log.
    QString profile() const { return profile_; }

private:
    bool waitFor(const std::function<bool()>& done, int timeoutMs);

    QLowEnergyController* controller_ = nullptr;
    std::vector<std::unique_ptr<QLowEnergyService>> services_;
    QLowEnergyService* serial_ = nullptr;
    QLowEnergyCharacteristic writeChar_;
    bool writeWithoutResponse_ = false;
    int chunk_ = 20;
    QByteArray buffer_;
    bool disconnected_ = false;
    std::string name_;
    QString profile_;
};

}  // namespace caruspex
