#include "app/BleTransport.h"

#include <QBluetoothAddress>
#include <QBluetoothDeviceDiscoveryAgent>
#include <QBluetoothLocalDevice>
#include <QBluetoothSocket>
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

QList<BleDevice> scanBleDevices(int milliseconds, QString& error, bool classic) {
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
    // Classic inquiry is not offered everywhere (macOS lists paired devices
    // only); then the low energy scan alone runs.
    const auto supported = QBluetoothDeviceDiscoveryAgent::supportedDiscoveryMethods();
    QBluetoothDeviceDiscoveryAgent::DiscoveryMethods methods = QBluetoothDeviceDiscoveryAgent::LowEnergyMethod;
    if (classic && supported.testFlag(QBluetoothDeviceDiscoveryAgent::ClassicMethod)) {
        methods |= QBluetoothDeviceDiscoveryAgent::ClassicMethod;
    }
    agent.start(methods);
    if (agent.isActive()) loop.exec();
    agent.stop();
    if (failed && agent.discoveredDevices().isEmpty()) return found;

    for (const QBluetoothDeviceInfo& info : agent.discoveredDevices()) {
        const auto config = info.coreConfigurations();
        BleDevice d;
        d.lowEnergy = config.testFlag(QBluetoothDeviceInfo::LowEnergyCoreConfiguration);
        d.classic = config.testFlag(QBluetoothDeviceInfo::BaseRateCoreConfiguration) ||
                    config.testFlag(QBluetoothDeviceInfo::BaseRateAndLowEnergyCoreConfiguration);
        if (config.testFlag(QBluetoothDeviceInfo::BaseRateAndLowEnergyCoreConfiguration)) d.lowEnergy = true;
        if (!d.lowEnergy && !(classic && d.classic)) continue;
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

namespace caruspex {

RfcommTransport::RfcommTransport() = default;

RfcommTransport::~RfcommTransport() {
    if (socket_) {
        socket_->abort();
        delete socket_;
    }
}

bool RfcommTransport::open(const BleDevice& device, std::string& error) {
    const QString shown = device.name.isEmpty() ? device.id : device.name;
    name_ = ("Bluetooth " + shown).toStdString();
    socket_ = new QBluetoothSocket(QBluetoothServiceInfo::RfcommProtocol);
    connect(socket_, &QBluetoothSocket::readyRead, this, [this] { buffer_.append(socket_->readAll()); });
    connect(socket_, &QBluetoothSocket::disconnected, this, [this] { disconnected_ = true; });
    QString lastError;
    const auto errorConnection = connect(socket_, &QBluetoothSocket::errorOccurred, this,
                                         [this, &lastError](QBluetoothSocket::SocketError) {
                                             lastError = socket_->errorString();
                                         });

    // The serial port profile; the adapters offer nothing else.
    const QBluetoothAddress address(device.id);
    if (address.isNull()) {
        error = tr("Klassisches Bluetooth braucht die Adresse des Geräts.").toStdString();
        disconnect(errorConnection);
        return false;
    }
    socket_->connectToService(address, QBluetoothUuid(QBluetoothUuid::ServiceClassUuid::SerialPort));

    const QDeadlineTimer deadline(20000);
    while (socket_->state() != QBluetoothSocket::SocketState::ConnectedState) {
        if (deadline.hasExpired() || socket_->state() == QBluetoothSocket::SocketState::UnconnectedState) {
            break;
        }
        QEventLoop loop;
        QTimer::singleShot(20, &loop, &QEventLoop::quit);
        loop.exec();
    }
    disconnect(errorConnection);
    if (socket_->state() != QBluetoothSocket::SocketState::ConnectedState) {
        error = (tr("Verbindung zu %1 kam nicht zustande. Ist der Adapter im System gekoppelt "
                    "(PIN meist 1234 oder 0000)?")
                     .arg(shown) +
                 (lastError.isEmpty() ? QString() : QStringLiteral(" (") + lastError + QStringLiteral(")")))
                    .toStdString();
        return false;
    }
    return true;
}

bool RfcommTransport::write(const std::string& data) {
    if (!socket_ || disconnected_) return false;
    const qint64 written = socket_->write(data.data(), static_cast<qint64>(data.size()));
    QCoreApplication::processEvents(QEventLoop::AllEvents, 5);
    return written == static_cast<qint64>(data.size()) && !disconnected_;
}

std::string RfcommTransport::read(int timeoutMs) {
    const QDeadlineTimer deadline(timeoutMs);
    while (buffer_.isEmpty() && !disconnected_ && !deadline.hasExpired()) {
        QEventLoop loop;
        QTimer::singleShot(qMin<qint64>(20, qMax<qint64>(1, deadline.remainingTime())), &loop,
                           &QEventLoop::quit);
        loop.exec();
    }
    std::string out = buffer_.toStdString();
    buffer_.clear();
    return out;
}

void RfcommTransport::flushInput() {
    QCoreApplication::processEvents(QEventLoop::AllEvents, 5);
    buffer_.clear();
}

}  // namespace caruspex
