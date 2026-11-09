#include "app/BleTransport.h"

#include <QBluetoothAddress>
#include <QBluetoothDeviceDiscoveryAgent>
#include <QBluetoothLocalDevice>
#include <QBluetoothUuid>
#include <QCoreApplication>
#include <QDeadlineTimer>
#include <QEventLoop>
#include <QLowEnergyController>
#include <QLowEnergyDescriptor>
#include <QLowEnergyService>
#include <QTimer>

#include <algorithm>
#include <functional>

#include "core/BleProfile.h"

namespace caruspex {
namespace {

std::string uuidText(const QBluetoothUuid& uuid) { return uuid.toString(QUuid::WithoutBraces).toStdString(); }

}  // namespace

QList<BleDevice> scanBleDevices(int milliseconds, QString& error) {
    QList<BleDevice> found;
    QBluetoothDeviceDiscoveryAgent agent;
    agent.setLowEnergyDiscoveryTimeout(milliseconds);
    QEventLoop loop;
    bool failed = false;
    QObject::connect(&agent, &QBluetoothDeviceDiscoveryAgent::finished, &loop, &QEventLoop::quit);
    QObject::connect(&agent, &QBluetoothDeviceDiscoveryAgent::canceled, &loop, &QEventLoop::quit);
    QObject::connect(&agent, &QBluetoothDeviceDiscoveryAgent::errorOccurred, &loop,
                     [&](QBluetoothDeviceDiscoveryAgent::Error e) {
                         failed = true;
                         switch (e) {
                         case QBluetoothDeviceDiscoveryAgent::PoweredOffError:
                             error = QCoreApplication::translate(
                                 "caruspex::BleTransport", "Bluetooth ist ausgeschaltet.");
                             break;
                         case QBluetoothDeviceDiscoveryAgent::MissingPermissionsError:
                             error = QCoreApplication::translate(
                                 "caruspex::BleTransport",
                                 "Caruspex darf Bluetooth nicht verwenden. Bitte in den "
                                 "Systemeinstellungen unter Datenschutz erlauben.");
                             break;
                         case QBluetoothDeviceDiscoveryAgent::InvalidBluetoothAdapterError:
                         case QBluetoothDeviceDiscoveryAgent::UnsupportedPlatformError:
                         case QBluetoothDeviceDiscoveryAgent::UnsupportedDiscoveryMethod:
                             error = QCoreApplication::translate(
                                 "caruspex::BleTransport",
                                 "Dieser Rechner hat kein Bluetooth LE, das Caruspex nutzen kann.");
                             break;
                         default:
                             error = agent.errorString();
                         }
                         loop.quit();
                     });
    // Guard against a backend that never reports back.
    QTimer::singleShot(milliseconds + 5000, &loop, &QEventLoop::quit);
    agent.start(QBluetoothDeviceDiscoveryAgent::LowEnergyMethod);
    if (agent.isActive()) loop.exec();
    agent.stop();
    if (failed && agent.discoveredDevices().isEmpty()) return found;

    for (const QBluetoothDeviceInfo& info : agent.discoveredDevices()) {
        if (!(info.coreConfigurations() & QBluetoothDeviceInfo::LowEnergyCoreConfiguration)) continue;
        BleDevice d;
        d.id = info.address().isNull() ? info.deviceUuid().toString(QUuid::WithoutBraces)
                                       : info.address().toString();
        d.name = info.name().trimmed();
        d.rssi = info.rssi();
        d.score = obdAdapterNameScore(d.name.toStdString());
        if (d.id.isEmpty()) continue;
        found.push_back(d);
    }
    std::sort(found.begin(), found.end(), [](const BleDevice& a, const BleDevice& b) {
        if (a.score != b.score) return a.score > b.score;
        if (a.name.isEmpty() != b.name.isEmpty()) return !a.name.isEmpty();
        return a.rssi > b.rssi;
    });
    return found;
}

BleTransport::BleTransport() = default;

BleTransport::~BleTransport() {
    services_.clear();
    if (controller_) {
        controller_->disconnectFromDevice();
        delete controller_;
    }
}

bool BleTransport::waitFor(const std::function<bool()>& done, int timeoutMs) {
    const QDeadlineTimer deadline(timeoutMs);
    while (!done()) {
        if (deadline.hasExpired() || disconnected_) return done();
        QEventLoop loop;
        QTimer::singleShot(qMin<qint64>(20, qMax<qint64>(1, deadline.remainingTime())), &loop,
                           &QEventLoop::quit);
        loop.exec();
    }
    return true;
}

bool BleTransport::open(const BleDevice& device, std::string& error) {
    const QBluetoothAddress address(device.id);
    QBluetoothDeviceInfo info = address.isNull()
                                    ? QBluetoothDeviceInfo(QBluetoothUuid(QUuid(device.id)), device.name, 0)
                                    : QBluetoothDeviceInfo(address, device.name, 0);
    info.setCoreConfigurations(QBluetoothDeviceInfo::LowEnergyCoreConfiguration);
    name_ = ("Bluetooth LE " + (device.name.isEmpty() ? device.id : device.name)).toStdString();

    controller_ = QLowEnergyController::createCentral(info);
    if (!controller_) {
        error = tr("Bluetooth LE ist auf diesem Rechner nicht verfügbar.").toStdString();
        return false;
    }
    // Connections that capture locals of this function; dropped at the end.
    QList<QMetaObject::Connection> scoped;
    struct Drop {
        QList<QMetaObject::Connection>& list;
        ~Drop() {
            for (const auto& c : list) QObject::disconnect(c);
        }
    } drop{scoped};

    QString lastError;
    scoped << connect(controller_, &QLowEnergyController::errorOccurred, this,
                      [this, &lastError](QLowEnergyController::Error) { lastError = controller_->errorString(); });
    connect(controller_, &QLowEnergyController::disconnected, this, [this] { disconnected_ = true; });

    controller_->connectToDevice();
    if (!waitFor([this] { return controller_->state() == QLowEnergyController::ConnectedState ||
                                 controller_->state() == QLowEnergyController::DiscoveredState; },
                 15000)) {
        error = (tr("Verbindung zu %1 kam nicht zustande").arg(device.name.isEmpty() ? device.id : device.name) +
                 (lastError.isEmpty() ? QString() : QStringLiteral(": ") + lastError))
                    .toStdString();
        return false;
    }

    bool discovered = false;
    scoped << connect(controller_, &QLowEnergyController::discoveryFinished, this,
                      [&discovered] { discovered = true; });
    controller_->discoverServices();
    if (!waitFor([&] { return discovered; }, 10000)) {
        error = tr("Das Gerät hat seine Dienste nicht gemeldet.").toStdString();
        return false;
    }

    // Look into every service: the characteristics are only known after
    // the details of a service were discovered.
    std::vector<BleServiceDesc> descs;
    for (const QBluetoothUuid& uuid : controller_->services()) {
        std::unique_ptr<QLowEnergyService> service(controller_->createServiceObject(uuid));
        if (!service) continue;
        service->discoverDetails();
        QLowEnergyService* raw = service.get();
        waitFor([raw] { return raw->state() == QLowEnergyService::RemoteServiceDiscovered ||
                               raw->state() == QLowEnergyService::InvalidService; },
                5000);
        BleServiceDesc desc;
        desc.uuid = uuidText(uuid);
        for (const QLowEnergyCharacteristic& c : raw->characteristics()) {
            BleCharacteristicDesc cd;
            cd.uuid = uuidText(c.uuid());
            const auto p = c.properties();
            cd.notify = p.testFlag(QLowEnergyCharacteristic::Notify);
            cd.indicate = p.testFlag(QLowEnergyCharacteristic::Indicate);
            cd.write = p.testFlag(QLowEnergyCharacteristic::Write);
            cd.writeNoResponse = p.testFlag(QLowEnergyCharacteristic::WriteNoResponse);
            desc.characteristics.push_back(cd);
        }
        descs.push_back(desc);
        services_.push_back(std::move(service));
    }

    const BleChannel channel = chooseBleChannel(descs);
    if (!channel.valid()) {
        error = tr("Auf diesem Gerät wurde kein serieller Kanal für OBD gefunden. Ist es ein "
                   "OBD-Adapter mit Bluetooth LE?")
                    .toStdString();
        return false;
    }
    serial_ = services_[channel.service].get();
    const QList<QLowEnergyCharacteristic> chars = serial_->characteristics();
    const QLowEnergyCharacteristic notifyChar = chars[channel.notifyCharacteristic];
    writeChar_ = chars[channel.writeCharacteristic];
    writeWithoutResponse_ = channel.writeWithoutResponse;
    profile_ = QString::fromStdString(channel.profile);
    chunk_ = qBound(20, controller_->mtu() - 3, 180);

    connect(serial_, &QLowEnergyService::characteristicChanged, this,
            [this](const QLowEnergyCharacteristic&, const QByteArray& value) { buffer_.append(value); });

    // Switch on notifications (or indications) through the CCC descriptor.
    const QLowEnergyDescriptor ccc =
        notifyChar.descriptor(QBluetoothUuid::DescriptorType::ClientCharacteristicConfiguration);
    if (ccc.isValid()) {
        const bool indicateOnly = !(notifyChar.properties() & QLowEnergyCharacteristic::Notify);
        bool written = false;
        scoped << connect(serial_, &QLowEnergyService::descriptorWritten, this,
                          [&written](const QLowEnergyDescriptor&, const QByteArray&) { written = true; });
        // 0x0001 switches notifications on, 0x0002 indications (little endian).
        serial_->writeDescriptor(ccc, indicateOnly ? QByteArray::fromHex("0200")
                                                   : QByteArray::fromHex("0100"));
        waitFor([&written] { return written; }, 3000);
    }
    return !disconnected_;
}

bool BleTransport::write(const std::string& data) {
    if (!serial_ || disconnected_) return false;
    const QByteArray bytes = QByteArray::fromStdString(data);
    const auto mode = writeWithoutResponse_ ? QLowEnergyService::WriteWithoutResponse
                                            : QLowEnergyService::WriteWithResponse;
    for (qsizetype at = 0; at < bytes.size(); at += chunk_) {
        serial_->writeCharacteristic(writeChar_, bytes.mid(at, chunk_), mode);
    }
    // Let the stack send it before the caller starts waiting for the answer.
    QCoreApplication::processEvents(QEventLoop::AllEvents, 5);
    return !disconnected_;
}

std::string BleTransport::read(int timeoutMs) {
    if (buffer_.isEmpty()) waitFor([this] { return !buffer_.isEmpty(); }, timeoutMs);
    std::string out = buffer_.toStdString();
    buffer_.clear();
    return out;
}

void BleTransport::flushInput() {
    QCoreApplication::processEvents(QEventLoop::AllEvents, 5);
    buffer_.clear();
}

}  // namespace caruspex
