#include "app/MainWindow.h"

#ifdef Q_OS_WIN
#include <windows.h>
#include <windowsx.h>
#endif

#include <QApplication>
#include <QButtonGroup>
#include <QCheckBox>
#include <QCloseEvent>
#include <QComboBox>
#include <QDateTime>
#include <QFrame>
#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QPlainTextEdit>
#include <QProgressBar>
#include <QPushButton>
#include <QSettings>
#include <QSpinBox>
#include <QTimer>
#include <QMouseEvent>
#include <QPainter>
#include <QPainterPath>
#include <QStackedWidget>
#include <QVBoxLayout>
#include <QWindow>

#include "app/Theme.h"
#include "app/TitleBar.h"
#include "app/pages/DashboardPage.h"
#include "app/pages/FaultsPage.h"
#include "app/pages/KklToolsPage.h"
#include "app/pages/LexiconPage.h"
#include "app/pages/MonitorPage.h"
#include "app/pages/SettingsPage.h"
#include "app/pages/LivePage.h"
#include "app/pages/ScanPage.h"
#include "app/pages/StatsPage.h"
#include "app/pages/ThrottlePage.h"
#include "app/pages/ToolsPage.h"
#include "core/Serial.h"
#include "data/Vehicles.h"

namespace caruspex {

namespace {

// Wie breit der Streifen an den Fensterkanten ist, in dem der Zeiger die
// Groessenaenderung anbietet.
constexpr int kResizeMargin = 6;
constexpr int kCornerRadius = 12;

// Zeichnet den abgerundeten Fensterhintergrund. Ohne Fensterrahmen malt das
// sonst niemand.
class WindowFrame : public QWidget {
public:
    explicit WindowFrame(QWidget* parent = nullptr) : QWidget(parent) {
        setAttribute(Qt::WA_StyledBackground, false);
    }
    void setRounded(bool rounded) {
        rounded_ = rounded;
        update();
    }

protected:
    void paintEvent(QPaintEvent*) override {
        QPainter p(this);
        p.setRenderHint(QPainter::Antialiasing);
        const double radius = rounded_ ? kCornerRadius : 0.0;
        const QRectF box = QRectF(rect()).adjusted(0.5, 0.5, -0.5, -0.5);
        QPainterPath path;
        path.addRoundedRect(box, radius, radius);
        p.fillPath(path, theme::color::background);
        p.setPen(QPen(theme::color::borderBright, 1.0));
        p.drawPath(path);
    }

private:
    bool rounded_ = true;
};

}  // namespace

MainWindow::MainWindow(QWidget* parent) : QMainWindow(parent) {
    setWindowTitle(tr("Caruspex - Fahrzeugdiagnose"));
    resize(1360, 880);

    // Rahmenlos mit eigener Titelleiste. Durchsichtiger Hintergrund, damit die
    // abgerundeten Ecken wirklich rund sind und nicht eckig unterlegt.
    setWindowFlags(Qt::Window | Qt::FramelessWindowHint);
    setAttribute(Qt::WA_TranslucentBackground);
    setMouseTracking(true);
    qApp->installEventFilter(this);

    auto* frame = new WindowFrame;
    frame_ = frame;
    auto* frameLayout = new QVBoxLayout(frame);
    frameLayout->setContentsMargins(1, 1, 1, 1);
    frameLayout->setSpacing(0);

    titleBar_ = new TitleBar;
    frameLayout->addWidget(titleBar_);
    connect(titleBar_, &TitleBar::minimiseRequested, this, &QWidget::showMinimized);
    connect(titleBar_, &TitleBar::maximiseToggled, this, &MainWindow::toggleMaximise);
    connect(titleBar_, &TitleBar::closeRequested, this, &QWidget::close);

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
    kklTools_ = new KklToolsPage;
    monitor_ = new MonitorPage;
    lexicon_ = new LexiconPage;
    settingsPage_ = new SettingsPage;
    addNavigationEntry(tr("Uebersicht"), dashboard_);
    addNavigationEntry(tr("Fehlerspeicher"), faults_);
    addNavigationEntry(tr("Live-Werte"), live_);
    addNavigationEntry(tr("Pruefen und Werkzeuge"), tools_);
    addNavigationEntry(tr("Bauteilueberwachung"), monitor_);
    addNavigationEntry(tr("Drosselklappe"), throttle_);
    addNavigationEntry(tr("Statistik"), stats_);
    addNavigationEntry(tr("Alle Steuergeraete"), scan_);
    addNavigationEntry(tr("Anpassung und Stellglieder"), kklTools_);
    addNavigationEntry(tr("Fehlercode-Lexikon"), lexicon_);
    addNavigationEntry(tr("Einstellungen"), settingsPage_);
    layout->addWidget(pages_, 1);
    frameLayout->addWidget(central, 1);
    setCentralWidget(frame_);

    connect(settingsPage_, &SettingsPage::changed, this, &MainWindow::applySettings);

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
    connect(worker, &Worker::monitorResults, monitor_,
            &MonitorPage::onMonitorResults);
    connect(worker, &Worker::oxygenTests, monitor_, &MonitorPage::onOxygenTests);
    connect(worker, &Worker::identity, monitor_, &MonitorPage::onIdentity);
    connect(worker, &Worker::liveValue, throttle_, &ThrottlePage::onLiveValue);
    connect(worker, &Worker::liveValue, stats_, &StatsPage::onLiveValue);
    connect(worker, &Worker::scanStarted, scan_, &ScanPage::onScanStarted);
    connect(worker, &Worker::scanProgress, scan_, &ScanPage::onScanProgress);
    connect(worker, &Worker::scanEcuResult, scan_, &ScanPage::onEcuResult);
    connect(worker, &Worker::scanFinished, scan_, &ScanPage::onScanFinished);
    connect(worker, &Worker::clearProgress, scan_, &ScanPage::onClearProgress);
    connect(worker, &Worker::clearProgress, faults_, &FaultsPage::onClearProgress);
    connect(worker, &Worker::clearFinished, faults_, &FaultsPage::onClearFinished);
    connect(worker, &Worker::kklConnected, kklTools_, &KklToolsPage::setKklConnected);
    connect(worker, &Worker::kklMessage, kklTools_, &KklToolsPage::onMessage);
    connect(worker, &Worker::adaptationResult, kklTools_,
            &KklToolsPage::onAdaptationResult);
    connect(worker, &Worker::actuatorResult, kklTools_,
            &KklToolsPage::onActuatorResult);
    connect(worker, &Worker::kklConnected, throttle_, &ThrottlePage::setKklConnected);
    connect(worker, &Worker::kklProtocol, throttle_, &ThrottlePage::setProtocol);
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
    connect(faults_, &FaultsPage::bruteForceRequested, &session_,
            &Session::bruteForceClearRequested);
    connect(tools_, &ToolsPage::detailsRequested, &session_, &Session::readDetailsRequested);
    connect(tools_, &ToolsPage::rawRequested, &session_, &Session::sendRawRequested);
    connect(monitor_, &MonitorPage::readRequested, &session_,
            &Session::readMonitorsRequested);
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
    connect(throttle_, &ThrottlePage::basicSettingSessionChanged, &session_,
            &Session::basicSettingSessionRequested);
    connect(scan_, &ScanPage::scanRequested, &session_, &Session::scanAllRequested);
    connect(scan_, &ScanPage::cancelRequested, &session_, &Session::cancelScan);
    connect(kklTools_, &KklToolsPage::connectRequested, &session_,
            &Session::openKklRequested);
    connect(kklTools_, &KklToolsPage::disconnectRequested, &session_,
            &Session::closeKklRequested);
    connect(kklTools_, &KklToolsPage::adaptationRequested, &session_,
            &Session::adaptationRequested);
    connect(kklTools_, &KklToolsPage::actuatorTestRequested, &session_,
            &Session::actuatorTestRequested);

    refreshPorts();
    onBrandChanged();

    // Gespeicherte Einstellungen laden. showValues meldet die Werte selbst,
    // dadurch laeuft alles ueber denselben Weg wie eine Aenderung von Hand.
    settings_ = loadSettings();
    settingsPage_->showValues(settings_);

    if (settings_.rememberGeometry) {
        QSettings store(QStringLiteral("Caruspex"), QStringLiteral("Caruspex"));
        const QByteArray geometry =
            store.value(QStringLiteral("fenster/geometrie")).toByteArray();
        if (!geometry.isEmpty()) restoreGeometry(geometry);
    }

    appendLog(tr("Caruspex bereit. Fahrzeug und Adapter waehlen, dann verbinden."));

    if (settings_.autoConnect && !settings_.lastPort.isEmpty()) {
        const int index = portBox_->findData(settings_.lastPort);
        if (index >= 0) {
            portBox_->setCurrentIndex(index);
            appendLog(tr("Verbinde selbsttaetig mit %1, so eingestellt.")
                          .arg(settings_.lastPort));
            QTimer::singleShot(0, this, &MainWindow::toggleConnection);
        } else {
            appendLog(tr("Selbsttaetiges Verbinden: Anschluss %1 ist nicht da.")
                          .arg(settings_.lastPort));
        }
    }
}

void MainWindow::applySettings(const AppSettings& values) {
    settings_ = values;
    saveSettings(settings_);

    // An den Arbeitsthread: Takt, Zeitlimits, Rohprotokoll, Sitzungsart.
    emit session_.settingsChanged(settings_);

    // An die Seiten.
    live_->setWindowSeconds(settings_.chartWindowSeconds);
    faults_->setClearOptions(settings_.clearAttempts, settings_.confirmClear);
    scan_->setClearAttempts(settings_.clearAttempts);

    // Protokolldatei oeffnen oder schliessen.
    const bool wantFile = settings_.logToFile && !settings_.logPath.isEmpty();
    const bool pathChanged = logFile_.isOpen() && logFile_.fileName() != settings_.logPath;
    if (logFile_.isOpen() && (!wantFile || pathChanged)) {
        logStream_.flush();
        logStream_.setDevice(nullptr);
        logFile_.close();
    }
    if (wantFile && !logFile_.isOpen()) {
        logFile_.setFileName(settings_.logPath);
        if (logFile_.open(QIODevice::WriteOnly | QIODevice::Append | QIODevice::Text)) {
            logStream_.setDevice(&logFile_);
            logStream_.setEncoding(QStringConverter::Utf8);
            logStream_ << "\n=== Caruspex, "
                       << QDateTime::currentDateTime().toString("dd.MM.yyyy HH:mm")
                       << " ===\n";
            logStream_.flush();
            appendLog(tr("Protokoll wird zusaetzlich nach %1 geschrieben.")
                          .arg(settings_.logPath));
        } else {
            // Das still zu schlucken waere die schlechteste Loesung: der
            // Anwender glaubt sonst, es werde mitgeschrieben.
            appendLog(tr("Protokolldatei %1 laesst sich nicht schreiben. Es wird nur "
                         "hier im Fenster protokolliert.")
                          .arg(settings_.logPath));
        }
    }
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
    theme::addGlow(brand, theme::color::accent, 1.1);
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
    root->addWidget(new QLabel(tr("Fahrzeug (optional)")));
    brandBox_ = new QComboBox;
    for (const std::string& b : vehicleBrands()) brandBox_->addItem(QString::fromStdString(b));
    // Standardmaessig automatisch. Die Auswahl bestimmt nur, welches Protokoll
    // zuerst probiert wird, sie ist eine Abkuerzung und keine Voraussetzung.
    brandBox_->setCurrentText(QStringLiteral("Automatisch erkennen"));
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
    theme::pulseGlow(connectButton_, theme::color::accent);
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
    if (profile) return QString::fromStdString(profile->protocolOrder);
    // Ohne passendes Fahrzeugprofil entscheidet die Einstellung, womit zuerst
    // versucht wird. Das spart beim A2 die Sekunden, die das Durchprobieren
    // der CAN-Protokolle kostet.
    return QString::fromStdString(settings_.protocolOrder == QStringLiteral("can")
                                      ? kProtocolOrderCanFirst
                                      : kProtocolOrderKLineFirst);
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
        // Anschluss merken, damit selbsttaetiges Verbinden beim naechsten Mal
        // weiss, wohin.
        if (settings_.lastPort != device) {
            settings_.lastPort = device;
            saveSettings(settings_);
        }
        emit session_.openSerialRequested(device, settings_.baudOverride, order);
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
    stateLabel_->setText(tr("VERBUNDEN"));
    stateLabel_->setStyleSheet(QStringLiteral("color: %1; font-weight: 700;")
                                   .arg(theme::color::good.name()));
    theme::addGlow(connectButton_, theme::color::bad, 0.8);
    theme::addGlow(stateLabel_, theme::color::good, 0.7);
    adapterLabel_->setText(tr("Adapter: %1\nWeg: %2\nProtokoll: %3")
                               .arg(adapter, transport, protocol));
    faults_->setConnected(true);
    live_->setConnected(true);
    tools_->setConnected(true);
    monitor_->setConnected(true);
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
    theme::removeGlow(stateLabel_);
    theme::pulseGlow(connectButton_, theme::color::accent);
    adapterLabel_->clear();
    dashboard_->onDisconnected();
    faults_->setConnected(false);
    live_->setConnected(false);
    tools_->setConnected(false);
    monitor_->setConnected(false);
    throttle_->setEobdConnected(false);
    stats_->setConnected(false);
    appendLog(tr("Verbindung getrennt."));
}

void MainWindow::onBusy(bool active) {
    busyBar_->setVisible(active);
}

void MainWindow::appendLog(const QString& line) {
    const QString stamped =
        QDateTime::currentDateTime().toString("HH:mm:ss") + "  " + line;
    log_->appendPlainText(stamped);
    if (logStream_.device()) {
        logStream_ << stamped << '\n';
        logStream_.flush();
    }
}

// --- Rahmenloses Fenster ----------------------------------------------------

Qt::Edges MainWindow::edgesAt(const QPoint& p) const {
    if (isMaximized() || isFullScreen()) return {};
    Qt::Edges edges;
    if (p.x() <= kResizeMargin) edges |= Qt::LeftEdge;
    if (p.x() >= width() - kResizeMargin) edges |= Qt::RightEdge;
    if (p.y() <= kResizeMargin) edges |= Qt::TopEdge;
    if (p.y() >= height() - kResizeMargin) edges |= Qt::BottomEdge;
    return edges;
}

// Unter Windows macht das WM_NCHITTEST weiter unten. Auf den anderen Systemen
// wird der Randstreifen hier abgefangen, weil die Kinderwidgets die
// Mausereignisse sonst zuerst bekommen.
bool MainWindow::eventFilter(QObject* watched, QEvent* event) {
#ifdef Q_OS_WIN
    return QMainWindow::eventFilter(watched, event);
#else
    const QEvent::Type type = event->type();
    if (type != QEvent::MouseMove && type != QEvent::MouseButtonPress) {
        return QMainWindow::eventFilter(watched, event);
    }
    auto* widget = qobject_cast<QWidget*>(watched);
    if (!widget || widget->window() != this) {
        return QMainWindow::eventFilter(watched, event);
    }

    auto* mouse = static_cast<QMouseEvent*>(event);
    const QPoint here = mapFromGlobal(mouse->globalPosition().toPoint());
    const Qt::Edges edges = edgesAt(here);

    if (type == QEvent::MouseMove) {
        if (edges == (Qt::LeftEdge | Qt::TopEdge) ||
            edges == (Qt::RightEdge | Qt::BottomEdge)) {
            setCursor(Qt::SizeFDiagCursor);
        } else if (edges == (Qt::RightEdge | Qt::TopEdge) ||
                   edges == (Qt::LeftEdge | Qt::BottomEdge)) {
            setCursor(Qt::SizeBDiagCursor);
        } else if (edges & (Qt::LeftEdge | Qt::RightEdge)) {
            setCursor(Qt::SizeHorCursor);
        } else if (edges & (Qt::TopEdge | Qt::BottomEdge)) {
            setCursor(Qt::SizeVerCursor);
        } else {
            unsetCursor();
        }
        return QMainWindow::eventFilter(watched, event);
    }

    if (edges && mouse->button() == Qt::LeftButton) {
        // Die Groessenaenderung uebernimmt der Fenstermanager, damit sie sich
        // genauso anfuehlt wie bei jedem anderen Fenster.
        if (QWindow* handle = windowHandle()) {
            handle->startSystemResize(edges);
            return true;   // nicht an das darunterliegende Bedienelement geben
        }
    }
    return QMainWindow::eventFilter(watched, event);
#endif
}

#ifdef Q_OS_WIN
// Windows fragt bei jeder Mausbewegung, welcher Teil des Fensters unter dem
// Zeiger liegt. Meldet man hier einen Rand, erledigt das Betriebssystem die
// Groessenaenderung selbst, samt passendem Zeiger und Andocken. Gerechnet wird
// in echten Bildpunkten, damit die Bildschirmskalierung keine Rolle spielt.
bool MainWindow::nativeEvent(const QByteArray& type, void* message, qintptr* result) {
    auto* msg = static_cast<MSG*>(message);
    if (!msg || msg->message != WM_NCHITTEST || isMaximized() || isFullScreen()) {
        return QMainWindow::nativeEvent(type, message, result);
    }

    RECT frame{};
    if (!GetWindowRect(msg->hwnd, &frame)) {
        return QMainWindow::nativeEvent(type, message, result);
    }

    const int x = GET_X_LPARAM(msg->lParam);
    const int y = GET_Y_LPARAM(msg->lParam);
    const int margin = static_cast<int>(kResizeMargin * devicePixelRatioF());

    const bool left = x < frame.left + margin;
    const bool right = x >= frame.right - margin;
    const bool top = y < frame.top + margin;
    const bool bottom = y >= frame.bottom - margin;

    LRESULT where = 0;
    if (top && left) where = HTTOPLEFT;
    else if (top && right) where = HTTOPRIGHT;
    else if (bottom && left) where = HTBOTTOMLEFT;
    else if (bottom && right) where = HTBOTTOMRIGHT;
    else if (left) where = HTLEFT;
    else if (right) where = HTRIGHT;
    else if (top) where = HTTOP;
    else if (bottom) where = HTBOTTOM;

    if (where != 0) {
        *result = static_cast<qintptr>(where);
        return true;
    }
    return QMainWindow::nativeEvent(type, message, result);
}
#endif

void MainWindow::toggleMaximise() {
    if (isMaximized()) {
        showNormal();
    } else {
        showMaximized();
    }
}

void MainWindow::changeEvent(QEvent* event) {
    QMainWindow::changeEvent(event);
    if (event->type() != QEvent::WindowStateChange) return;
    const bool maximised = isMaximized() || isFullScreen();
    if (titleBar_) titleBar_->updateMaximiseState(maximised);
    // Maximiert muessen die Ecken eckig sein, sonst klaffen an den
    // Bildschirmecken Luecken.
    // Der Typ steht fest, frame_ wird nur an einer Stelle gesetzt.
    if (frame_) static_cast<WindowFrame*>(frame_)->setRounded(!maximised);
}

void MainWindow::closeEvent(QCloseEvent* event) {
    if (settings_.rememberGeometry) {
        QSettings store(QStringLiteral("Caruspex"), QStringLiteral("Caruspex"));
        store.setValue(QStringLiteral("fenster/geometrie"), saveGeometry());
    }
    if (logFile_.isOpen()) {
        logStream_.flush();
        logStream_.setDevice(nullptr);
        logFile_.close();
    }
    QMainWindow::closeEvent(event);
}

void MainWindow::autoConnectTcp(const QString& address) {
    wifiBox_->setChecked(true);
    addressEdit_->setText(address);
    toggleConnection();
}

}  // namespace caruspex
