// Connecting, step by step: where the socket is, which adapter, then one
// button. Replaces the controls that used to crowd the sidebar.
#pragma once

#include <QHash>
#include <QWidget>

#include "app/Settings.h"
#include "data/VehicleCatalog.h"

class QButtonGroup;
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
    enum class Mode { Cable, Wifi, Demo };

    explicit ConnectPage(QWidget* parent = nullptr);

    void setSettings(const AppSettings& settings);
    // Fills the port list. Returns the number of ports found.
    int refreshPorts();
    // Tries to preselect a port, true if it is there.
    bool selectPort(const QString& device);
    void setWifiAddress(const QString& address);
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

signals:
    // device empty means: try every port.
    void serialRequested(const QString& device, const QString& protocolOrder);
    void tcpRequested(const QString& host, int port, const QString& protocolOrder);
    void demoRequested(const QString& protocolOrder);
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
    QLineEdit* addressEdit_ = nullptr;
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
