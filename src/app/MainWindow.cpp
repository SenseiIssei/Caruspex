#include "app/MainWindow.h"

#include <QButtonGroup>
#include <QCheckBox>
#include <QComboBox>
#include <QDateTime>
#include <QFrame>
#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QPlainTextEdit>
#include <QProgressBar>
#include <QPushButton>
#include <QSpinBox>
#include <QStackedWidget>
#include <QVBoxLayout>

#include "app/Theme.h"
#include "app/pages/DashboardPage.h"
#include "app/pages/FaultsPage.h"
#include "app/pages/LexiconPage.h"
#include "app/pages/LivePage.h"
#include "app/pages/ScanPage.h"
#include "app/pages/StatsPage.h"
#include "app/pages/ThrottlePage.h"
#include "app/pages/ToolsPage.h"
#include "core/Serial.h"
#include "data/Vehicles.h"

namespace caruspex {

MainWindow::MainWindow(QWidget* parent) : QMainWindow(parent) {
    setWindowTitle(tr("Caruspex - Fahrzeugdiagnose"));
    resize(1360, 880);

    auto* central = new QWidget;
    auto* layout = new QHBoxLayout(central);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(0);
    layout->addWidget(buildSidebar());

    pages_ = new QStackedWidget;
    dashboard_ = new DashboardPage;
    faults_ = new FaultsPage;
    live_ = new LivePage;
    tools_ = new ToolsPage;
    throttle_ = new ThrottlePage;
    stats_ = new StatsPage;
    scan_ = new ScanPage;
    lexicon_ = new LexiconPage;
    addNavigationEntry(tr("Uebersicht"), dashboard_);
    addNavigationEntry(tr("Fehlerspeicher"), faults_);
    addNavigationEntry(tr("Live-Werte"), live_);
    addNavigationEntry(tr("Pruefen und Werkzeuge"), tools_);
    addNavigationEntry(tr("Drosselklappe"), throttle_);
    addNavigationEntry(tr("Statistik"), stats_);
    addNavigationEntry(tr("Alle Steuergeraete"), scan_);
    addNavigationEntry(tr("Fehlercode-Lexikon"), lexicon_);
    layout->addWidget(pages_, 1);
    setCentralWidget(central);

    Worker* worker = session_.worker();

    // Worker zur Oberflaeche
    connect(worker, &Worker::connected, this, &MainWindow::onConnected);
    connect(worker, &Worker::failed, this, &MainWindow::onFailed);
    connect(worker, &Worker::closed, this, &MainWindow::onClosed);
    connect(worker, &Worker::busy, this, &MainWindow::onBusy);
    connect(worker, &Worker::logLine, this, &MainWindow::appendLog);

    connect(worker, &Worker::connected, dashboard_, &DashboardPage::onConnected);
    connect(worker, &Worker::status, dashboard_, &DashboardPage::onStatus);
    connect(worker, &Worker::faults, dashboard_, &DashboardPage::onFaults);
    connect(worker, &Worker::liveValue, dashboard_, &DashboardPage::onLiveValue);
    connect(worker, &Worker::faults, faults_, &FaultsPage::onFaults);
    connect(worker, &Worker::liveValue, live_, &LivePage::onLiveValue);
    connect(worker, &Worker::supportedPids, live_, &LivePage::onSupportedPids);
    connect(worker, &Worker::status, tools_, &ToolsPage::onStatus);
    connect(worker, &Worker::vehicleInfo, tools_, &ToolsPage::onVehicleInfo);
    connect(worker, &Worker::rawReply, tools_, &ToolsPage::onRawReply);
    connect(worker, &Worker::liveValue, throttle_, &ThrottlePage::onLiveValue);
    connect(worker, &Worker::liveValue, stats_, &StatsPage::onLiveValue);
    connect(worker, &Worker::scanStarted, scan_, &ScanPage::onScanStarted);
    connect(worker, &Worker::scanProgress, scan_, &ScanPage::onScanProgress);
    connect(worker, &Worker::scanEcuResult, scan_, &ScanPage::onEcuResult);
    connect(worker, &Worker::scanFinished, scan_, &ScanPage::onScanFinished);
    connect(worker, &Worker::kklConnected, throttle_, &ThrottlePage::setKklConnected);
    connect(worker, &Worker::kklMessage, throttle_, &ThrottlePage::onKklMessage);
    connect(worker, &Worker::basicSettingValues, throttle_,
            &ThrottlePage::onBasicSettingValues);
    connect(worker, &Worker::kklMessage, this,
            [this](const QString& text, bool) { appendLog(text); });

    // Oberflaeche zum Worker
    connect(dashboard_, &DashboardPage::readRequested, &session_, &Session::readFaultsRequested);
    connect(dashboard_, &DashboardPage::detailsRequested, &session_,
            &Session::readDetailsRequested);
    connect(dashboard_, &DashboardPage::liveRequested, this, [this] {
        pages_->setCurrentWidget(live_);
        navigation_->button(pages_->indexOf(live_))->setChecked(true);
        live_->toggleFromOutside();
    });
    connect(faults_, &FaultsPage::readRequested, &session_, &Session::readFaultsRequested);
    connect(faults_, &FaultsPage::clearRequested, &session_, &Session::clearFaultsRequested);
    connect(tools_, &ToolsPage::detailsRequested, &session_, &Session::readDetailsRequested);
    connect(tools_, &ToolsPage::rawRequested, &session_, &Session::sendRawRequested);
    connect(live_, &LivePage::startRequested, &session_, &Session::startLiveRequested);
    connect(live_, &LivePage::stopRequested, &session_, &Session::stopLiveRequested);
    connect(throttle_, &ThrottlePage::kklConnectRequested, &session_,
            &Session::openKklRequested);
    connect(throttle_, &ThrottlePage::kklDisconnectRequested, &session_,
            &Session::closeKklRequested);
    connect(throttle_, &ThrottlePage::basicSettingRequested, &session_,
            &Session::startBasicSettingRequested);
    connect(throttle_, &ThrottlePage::basicSettingStopRequested, &session_,
            &Session::stopBasicSettingRequested);
    connect(scan_, &ScanPage::scanRequested, &session_, &Session::scanAllRequested);
    connect(scan_, &ScanPage::cancelRequested, &session_, &Session::cancelScan);

    refreshPorts();
    onBrandChanged();
    appendLog(tr("Caruspex bereit. Fahrzeug und Adapter waehlen, dann verbinden."));
}

QWidget* MainWindow::buildSidebar() {
    auto* frame = new QFrame;
    frame->setObjectName("Sidebar");
    frame->setFixedWidth(340);
    auto* root = new QVBoxLayout(frame);
    root->setContentsMargins(18, 18, 18, 16);
    root->setSpacing(12);

    auto* brand = new QLabel(QStringLiteral("CARUSPEX"));
    brand->setObjectName("Brand");
    auto* claim = new QLabel(tr("Fahrzeugdiagnose"));
    claim->setObjectName("Subtle");
    root->addWidget(brand);
    root->addWidget(claim);

    // Navigation
    navigation_ = new QButtonGroup(this);
    navigation_->setExclusive(true);
    navigationLayout_ = new QVBoxLayout;
    navigationLayout_->setSpacing(3);
    root->addSpacing(6);
    root->addLayout(navigationLayout_);

    auto* separator = new QFrame;
    separator->setFrameShape(QFrame::HLine);
    separator->setStyleSheet(
        QStringLiteral("color: %1;").arg(theme::color::border.name()));
    root->addSpacing(6);
    root->addWidget(separator);

    // Fahrzeug
    root->addWidget(new QLabel(tr("Fahrzeug")));
    brandBox_ = new QComboBox;
    for (const std::string& b : vehicleBrands()) brandBox_->addItem(QString::fromStdString(b));
    brandBox_->setCurrentText(QStringLiteral("Audi"));
    root->addWidget(brandBox_);

    auto* yearRow = new QHBoxLayout;
    yearRow->addWidget(new QLabel(tr("Baujahr")));
    yearBox_ = new QSpinBox;
    yearBox_->setRange(1996, 2030);
    yearBox_->setValue(2002);
    yearRow->addWidget(yearBox_, 1);
    root->addLayout(yearRow);

    profileNote_ = new QLabel;
    profileNote_->setObjectName("Hint");
    profileNote_->setWordWrap(true);
    root->addWidget(profileNote_);

    connect(brandBox_, &QComboBox::currentTextChanged, this, &MainWindow::onBrandChanged);
    connect(yearBox_, &QSpinBox::valueChanged, this, &MainWindow::onBrandChanged);

    // Adapter
    root->addSpacing(4);
    root->addWidget(new QLabel(tr("Adapter")));
    wifiBox_ = new QCheckBox(tr("WLAN-Adapter statt Kabel"));
    root->addWidget(wifiBox_);

    portBox_ = new QComboBox;
    root->addWidget(portBox_);
    rescanButton_ = new QPushButton(tr("Anschluesse neu suchen"));
    root->addWidget(rescanButton_);
    addressEdit_ = new QLineEdit(QStringLiteral("192.168.0.10:35000"));
    addressEdit_->setVisible(false);
    root->addWidget(addressEdit_);
    connect(rescanButton_, &QPushButton::clicked, this, &MainWindow::refreshPorts);
    connect(wifiBox_, &QCheckBox::toggled, this, [this](bool wifi) {
        portBox_->setVisible(!wifi);
        rescanButton_->setVisible(!wifi);
        addressEdit_->setVisible(wifi);
    });

    connectButton_ = new QPushButton(tr("Verbinden"));
    connectButton_->setObjectName("Primary");
    connectButton_->setMinimumHeight(44);
    root->addWidget(connectButton_);
    connect(connectButton_, &QPushButton::clicked, this, &MainWindow::toggleConnection);

    busyBar_ = new QProgressBar;
    busyBar_->setRange(0, 0);
    busyBar_->setTextVisible(false);
    busyBar_->setVisible(false);
    root->addWidget(busyBar_);

    stateLabel_ = new QLabel(tr("nicht verbunden"));
    stateLabel_->setStyleSheet(
        QStringLiteral("color: %1;").arg(theme::color::textMuted.name()));
    adapterLabel_ = new QLabel;
    adapterLabel_->setObjectName("Hint");
    adapterLabel_->setWordWrap(true);
    root->addWidget(stateLabel_);
    root->addWidget(adapterLabel_);

    // Protokoll
    root->addWidget(new QLabel(tr("Protokoll")));
    log_ = new QPlainTextEdit;
    log_->setReadOnly(true);
    log_->setMinimumHeight(150);
    QFont mono(QStringLiteral("Consolas"));
    mono.setStyleHint(QFont::Monospace);
    mono.setPointSizeF(8.5);
    log_->setFont(mono);
    root->addWidget(log_, 1);

    auto* note = new QLabel(tr("Zuendung an, Motor darf aus sein."));
    note->setObjectName("Hint");
    root->addWidget(note);
    return frame;
}

void MainWindow::addNavigationEntry(const QString& text, QWidget* page) {
    const int index = pages_->addWidget(page);
    auto* button = new QPushButton(text);
    button->setObjectName("NavItem");
    button->setCheckable(true);
    button->setCursor(Qt::PointingHandCursor);
    if (index == 0) button->setChecked(true);
    navigationLayout_->addWidget(button);
    navigation_->addButton(button, index);
    connect(button, &QPushButton::clicked, this,
            [this, index] { pages_->setCurrentIndex(index); });
}

void MainWindow::onBrandChanged() {
    const VehicleProfile* profile =
        findProfile(brandBox_->currentText().toStdString(), yearBox_->value());
    if (!profile) {
        profileNote_->setText(tr("Kein Profil hinterlegt, alle Protokolle werden probiert."));
        return;
    }
    profileNote_->setText(QStringLiteral("%1\n%2")
                              .arg(QString::fromStdString(busName(profile->bus)),
                                   QString::fromStdString(profile->note))
                              .trimmed());
}

QString MainWindow::protocolOrderForSelection() const {
    const VehicleProfile* profile =
        findProfile(brandBox_->currentText().toStdString(), yearBox_->value());
    return QString::fromStdString(profile ? profile->protocolOrder : kAutoProtocolOrder);
}

void MainWindow::refreshPorts() {
    portBox_->clear();
    const std::vector<PortInfo> ports = enumeratePorts();
    for (const PortInfo& p : ports) {
        QString label = QString::fromStdString(p.device);
        if (!p.description.empty()) label += "  " + QString::fromStdString(p.description);
        if (!p.chipGuess.empty()) label += "  [" + QString::fromStdString(p.chipGuess) + "]";
        portBox_->addItem(label, QString::fromStdString(p.device));
    }
    if (ports.empty()) {
        portBox_->addItem(tr("kein Anschluss gefunden"), QString());
        appendLog(tr("Kein COM-Anschluss gefunden. Steckt der Adapter am Rechner?"));
    } else {
        appendLog(tr("%1 Anschluss/Anschluesse gefunden.").arg(ports.size()));
    }
}

void MainWindow::toggleConnection() {
    if (connected_) {
        emit session_.closeRequested();
        return;
    }
    const QString order = protocolOrderForSelection();
    if (wifiBox_->isChecked()) {
        const QString address = addressEdit_->text().trimmed();
        const int colon = address.indexOf(':');
        const QString host = colon < 0 ? address : address.left(colon);
        const int port = colon < 0 ? 35000 : address.mid(colon + 1).toInt();
        appendLog(tr("Verbinde mit %1 ...").arg(address));
        emit session_.openTcpRequested(host, port, order);
    } else {
        const QString device = portBox_->currentData().toString();
        if (device.isEmpty()) {
            appendLog(tr("Kein Anschluss ausgewaehlt."));
            return;
        }
        appendLog(tr("Verbinde ueber %1 ...").arg(device));
        emit session_.openSerialRequested(device, 0, order);
    }
    connectButton_->setEnabled(false);
    stateLabel_->setText(tr("verbinde ..."));
    stateLabel_->setStyleSheet(QStringLiteral("color: %1;").arg(theme::color::warn.name()));
}

void MainWindow::onConnected(const QString& adapter, const QString& transport,
                             const QString& protocol) {
    connected_ = true;
    connectButton_->setEnabled(true);
    connectButton_->setText(tr("Trennen"));
    connectButton_->setObjectName("Danger");
    connectButton_->style()->unpolish(connectButton_);
    connectButton_->style()->polish(connectButton_);
    stateLabel_->setText(tr("verbunden"));
    stateLabel_->setStyleSheet(QStringLiteral("color: %1;").arg(theme::color::good.name()));
    adapterLabel_->setText(tr("Adapter: %1\nWeg: %2\nProtokoll: %3")
                               .arg(adapter, transport, protocol));
    faults_->setConnected(true);
    live_->setConnected(true);
    tools_->setConnected(true);
    throttle_->setEobdConnected(true);
    stats_->setConnected(true);
}

void MainWindow::onFailed(const QString& error) {
    connected_ = false;
    connectButton_->setEnabled(true);
    stateLabel_->setText(tr("nicht verbunden"));
    stateLabel_->setStyleSheet(QStringLiteral("color: %1;").arg(theme::color::bad.name()));
    adapterLabel_->setText(error);
    appendLog(error);
}

void MainWindow::onClosed() {
    connected_ = false;
    connectButton_->setEnabled(true);
    connectButton_->setText(tr("Verbinden"));
    connectButton_->setObjectName("Primary");
    connectButton_->style()->unpolish(connectButton_);
    connectButton_->style()->polish(connectButton_);
    stateLabel_->setText(tr("nicht verbunden"));
    stateLabel_->setStyleSheet(
        QStringLiteral("color: %1;").arg(theme::color::textMuted.name()));
    adapterLabel_->clear();
    dashboard_->onDisconnected();
    faults_->setConnected(false);
    live_->setConnected(false);
    tools_->setConnected(false);
    throttle_->setEobdConnected(false);
    stats_->setConnected(false);
    appendLog(tr("Verbindung getrennt."));
}

void MainWindow::onBusy(bool active) {
    busyBar_->setVisible(active);
}

void MainWindow::appendLog(const QString& line) {
    log_->appendPlainText(QDateTime::currentDateTime().toString("HH:mm:ss") + "  " + line);
}

void MainWindow::autoConnectTcp(const QString& address) {
    wifiBox_->setChecked(true);
    addressEdit_->setText(address);
    toggleConnection();
}

}  // namespace caruspex
