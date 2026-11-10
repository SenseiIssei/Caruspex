// Connecting, step by step: where the socket is, which adapter, then one
// button. Replaces the controls that used to crowd the sidebar.
#pragma once

#include <QHash>
#include <QWidget>

#include "app/Settings.h"
#include "data/VehicleCatalog.h"

class QButtonGroup;
class QCheckBox;
class QComboBox;
class QCompleter;
class QStringListModel;
class QFrame;
class QLabel;
class QLineEdit;
class QPushButton;
class QSpinBox;
class QStackedWidget;

namespace caruspex {

class Spinner;

class ConnectPage : public QWidget {
    Q_OBJECT

public:
    // The values are the pages of the mode stack, Ble was added last.
    enum class Mode { Cable, Wifi, Demo, Replay, Ble, Enet, Can };

    explicit ConnectPage(QWidget* parent = nullptr);

    void setSettings(const AppSettings& settings);
    // Fills the port list. Returns the number of ports found.
    int refreshPorts();
    // Tries to preselect a port, true if it is there.
    bool selectPort(const QString& device);
    void setWifiAddress(const QString& address);
    void setReplayPath(const QString& path);
    void setEnetAddress(const QString& host);
    // Lists the CAN interfaces of this computer again.
    void refreshCanDevices();
    // Selects a Bluetooth device by id, empty means automatic search.
    void selectBluetooth(const QString& id);
    void setMode(Mode mode);

    // The protocol order to try first, from the vehicle choice or the settings.
    QString protocolOrder() const;

    // Where the vehicle's OBD socket is, if known. Shown in step one.
    void setSocketHint(const QString& text);

    // Preselects a car from the catalog (or none for automatic).
    void setGeneration(const VehicleGeneration* generation, int year);

public slots:
    void startConnect();
    void onConnecting();
    void onConnected(const QString& adapter, const QString& transport, const QString& protocol);
    void onFailed(const QString& error);
    void onClosed();
    // Result of a Bluetooth LE scan (Worker::bleScanFinished).
    void onBleScan(const QStringList& ids, const QStringList& names, const QList<int>& scores,
                   const QList<int>& kinds, const QString& error);
    // Remembers the device the Bluetooth connection went to.
    void rememberBle(const QString& id, const QString& name, int kinds);

signals:
    // device empty means: try every port.
    void serialRequested(const QString& device, const QString& protocolOrder);
    void tcpRequested(const QString& host, int port, const QString& protocolOrder);
    void demoRequested(const QString& protocolOrder);
    void replayRequested(const QString& path, bool realTime, const QString& protocolOrder);
    // id empty means: search and take the first device that looks like an adapter.
    // kinds: 1 = Bluetooth LE, 2 = classic, 3 = both.
    void bleRequested(const QString& id, const QString& name, int kinds, const QString& protocolOrder);
    void bleScanRequested();
    // BMW over ENET, host empty means: look for the car.
    void enetRequested(const QString& host, const QString& protocolOrder);
    // A raw CAN interface (J2534, SLCAN, SocketCAN), see openCanInterface.
    void canRequested(const QString& spec, const QString& protocolOrder);
    void disconnectRequested();
    void vehicleChoiceChanged();

private:
    QWidget* buildSteps();
    QWidget* buildAdapter();
    void updateProfileNote();
    void showError(const QString& error);

    AppSettings settings_;
    bool connected_ = false;
    bool connecting_ = false;

    QButtonGroup* modes_ = nullptr;
    QStackedWidget* modeStack_ = nullptr;
    QComboBox* portBox_ = nullptr;
    QComboBox* bleBox_ = nullptr;
    QPushButton* bleScan_ = nullptr;
    QLabel* bleNote_ = nullptr;
    QLineEdit* addressEdit_ = nullptr;
    QLineEdit* enetEdit_ = nullptr;
    QComboBox* canDevice_ = nullptr;
    QComboBox* canRate_ = nullptr;
    QLineEdit* replayPath_ = nullptr;
    QCheckBox* replayRealTime_ = nullptr;
    QLineEdit* vehicleEdit_ = nullptr;
    QCompleter* completer_ = nullptr;
    QStringListModel* suggestions_ = nullptr;
    QHash<QString, const VehicleGeneration*> suggestionMap_;
    const VehicleGeneration* generation_ = nullptr;
    QSpinBox* yearBox_ = nullptr;
    QLabel* profileNote_ = nullptr;
    QLabel* socketHint_ = nullptr;
    QPushButton* connectButton_ = nullptr;
    Spinner* spinner_ = nullptr;
    QLabel* stateLabel_ = nullptr;
    QLabel* details_ = nullptr;
    QFrame* errorCard_ = nullptr;
    QLabel* errorText_ = nullptr;
};

}  // namespace caruspex
