#include "app/MainWindow.h"

#ifdef Q_OS_WIN
#include <windows.h>
#include <windowsx.h>
#endif

#include <QApplication>
#include <QButtonGroup>
#include <QCloseEvent>
#include <QDateTime>
#include <QFrame>
#include <QHBoxLayout>
#include <QLabel>
#include <QMouseEvent>
#include <QPainter>
#include <QPainterPath>
#include <QPlainTextEdit>
#include <QPropertyAnimation>
#include <QPushButton>
#include <QScrollArea>
#include <QSettings>
#include <QTimer>
#include <QVBoxLayout>
#include <QWindow>

#include "app/Garage.h"
#include "app/Icons.h"
#include "app/Report.h"
#include "app/VehicleState.h"
#include "app/Theme.h"
#include "app/TitleBar.h"
#include "app/Widgets.h"
#include "app/pages/AdapterPage.h"
#include "app/pages/BatteryPage.h"
#include "app/pages/ConnectPage.h"
#include "app/pages/DashboardPage.h"
#include "app/pages/FaultsPage.h"
#include "app/pages/GaragePage.h"
#include "app/pages/HealthPage.h"
#include "app/pages/KklToolsPage.h"
#include "app/pages/LexiconPage.h"
#include "app/pages/LivePage.h"
#include "app/pages/MonitorPage.h"
#include "app/pages/PerformancePage.h"
#include "app/pages/ScanPage.h"
#include "app/pages/SettingsPage.h"
#include "app/pages/StatsPage.h"
#include "app/pages/ThrottlePage.h"
#include "app/pages/ToolsPage.h"
#include "app/pages/TripPage.h"

namespace caruspex {

using namespace theme;

namespace {

// Width of the strip along the window edges in which the pointer offers
// resizing.
constexpr int kResizeMargin = 6;
constexpr int kCornerRadius = 12;
constexpr int kSidebarWide = 248;
constexpr int kSidebarCompact = 68;
constexpr int kLogHeight = 210;

// Paints the rounded window background. Without a window frame nobody else
// would paint it.
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
        p.fillPath(path, color::background);
        // A faint wash of the accent in the upper right, like Odysync.
        QRadialGradient wash(QPointF(width() * 0.85, 0), width() * 0.6);
        QColor tint = color::accent;
        tint.setAlphaF(0.05f);
        wash.setColorAt(0, tint);
        wash.setColorAt(1, QColor(0, 0, 0, 0));
        p.fillPath(path, wash);
        p.setPen(QPen(color::borderBright, 1.0));
        p.drawPath(path);
    }

private:
    bool rounded_ = true;
};

}  // namespace

MainWindow::MainWindow(QWidget* parent) : QMainWindow(parent) {
    setWindowTitle(tr("Caruspex - Fahrzeugdiagnose"));
    resize(1440, 920);
    setMinimumSize(980, 640);

    // Frameless with a custom title bar. Translucent background so the rounded
    // corners are really round and not backed by a square.
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
    connect(titleBar_, &TitleBar::sidebarToggled, this, &MainWindow::toggleSidebar);
    connect(titleBar_, &TitleBar::logToggled, this, &MainWindow::toggleLog);
    connect(titleBar_, &TitleBar::connectionClicked, this, [this] { navigateTo(connect_); });

    pages_ = new AnimatedStack;
    connect_ = new ConnectPage;
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
    state_ = new VehicleState(this);
    garage_ = new Garage(this);
    garage_->load();
    garagePage_ = new GaragePage(garage_);
    health_ = new HealthPage;
    trip_ = new TripPage;
    performance_ = new PerformancePage;
    battery_ = new BatteryPage;
    adapter_ = new AdapterPage;

    auto* central = new QWidget;
    auto* layout = new QHBoxLayout(central);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(0);
    layout->addWidget(buildSidebar());

    auto* right = new QVBoxLayout;
    right->setContentsMargins(0, 0, 0, 0);
    right->setSpacing(0);
    right->addWidget(pages_, 1);
    right->addWidget(buildLogDrawer());
    layout->addLayout(right, 1);
    frameLayout->addWidget(central, 1);
    setCentralWidget(frame_);

    connect(settingsPage_, &SettingsPage::changed, this, &MainWindow::applySettings);

    Worker* worker = session_.worker();

    // Worker to UI
    connect(worker, &Worker::connected, this, &MainWindow::onConnected);
    connect(worker, &Worker::failed, this, &MainWindow::onFailed);
    connect(worker, &Worker::closed, this, &MainWindow::onClosed);
    connect(worker, &Worker::busy, this, &MainWindow::onBusy);
    connect(worker, &Worker::logLine, this, &MainWindow::appendLog);

    connect(worker, &Worker::connected, connect_, &ConnectPage::onConnected);
    connect(worker, &Worker::failed, connect_, &ConnectPage::onFailed);
    connect(worker, &Worker::closed, connect_, &ConnectPage::onClosed);
    connect(worker, &Worker::connected, dashboard_, &DashboardPage::onConnected);
    connect(worker, &Worker::status, dashboard_, &DashboardPage::onStatus);
    connect(worker, &Worker::faults, dashboard_, &DashboardPage::onFaults);
    connect(worker, &Worker::liveValue, dashboard_, &DashboardPage::onLiveValue);
    connect(worker, &Worker::busy, dashboard_, &DashboardPage::onBusy);
    connect(worker, &Worker::faults, faults_, &FaultsPage::onFaults);
    connect(worker, &Worker::faults, this, [this](const QVector<FaultRow>& rows) {
        faultsButton_->setBadge(rows.isEmpty() ? QString() : QString::number(rows.size()),
                                color::bad);
    });
    connect(worker, &Worker::liveValue, live_, &LivePage::onLiveValue);
    connect(worker, &Worker::supportedPids, live_, &LivePage::onSupportedPids);
    connect(worker, &Worker::status, tools_, &ToolsPage::onStatus);
    connect(worker, &Worker::vehicleInfo, tools_, &ToolsPage::onVehicleInfo);
    connect(worker, &Worker::rawReply, tools_, &ToolsPage::onRawReply);
    connect(worker, &Worker::monitorResults, monitor_, &MonitorPage::onMonitorResults);
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
    connect(worker, &Worker::adaptationResult, kklTools_, &KklToolsPage::onAdaptationResult);
    connect(worker, &Worker::actuatorResult, kklTools_, &KklToolsPage::onActuatorResult);
    connect(worker, &Worker::kklConnected, throttle_, &ThrottlePage::setKklConnected);
    connect(worker, &Worker::kklProtocol, throttle_, &ThrottlePage::setProtocol);
    connect(worker, &Worker::kklMessage, throttle_, &ThrottlePage::onKklMessage);
    connect(worker, &Worker::basicSettingValues, throttle_, &ThrottlePage::onBasicSettingValues);
    connect(worker, &Worker::kklMessage, this,
            [this](const QString& text, bool) { appendLog(text); });
    connect(worker, &Worker::adapterFound, this, [this](const QString& device) {
        connect_->refreshPorts();
        connect_->selectPort(device);
        settings_.lastPort = device;
        saveSettings(settings_);
        toast::show(this, tr("Adapter an %1 gefunden.").arg(device), toast::Kind::Success);
    });

    // The shared picture of the connected car. Attached after the pages, so
    // the start page has rated the faults before the garage stores them.
    state_->attach(worker);
    connect(dashboard_, &DashboardPage::scoreChanged, state_, &VehicleState::setScore);
    connect(state_, &VehicleState::vinChanged, this, [this](const QString& vin) {
        garage_->touch(vin, state_->displayName(), state_->description(), state_->bodyStyle());
        garagePage_->setCurrentVin(vin);
        recordCheck();
    });
    connect(state_, &VehicleState::faultsChanged, this, &MainWindow::recordCheck);
    connect(dashboard_, &DashboardPage::reportRequested, this, &MainWindow::showReport);
    connect(garagePage_, &GaragePage::reportRequested, this, &MainWindow::showReport);

    // New pages
    connect(worker, &Worker::healthProgress, health_, &HealthPage::onProgress);
    connect(worker, &Worker::healthResult, health_, &HealthPage::onResult);
    connect(worker, &Worker::healthResult, this, [this](const HealthRow& row) {
        // The start page shows the same result as the health check.
        QVector<DashboardPage::Finding> findings;
        for (const HealthFindingRow& f : row.findings) {
            findings.push_back({levelIcon(f.level), levelColour(f.level), f.title, f.explanation,
                                levelLabel(f.level)});
        }
        const QString caption = row.grade == 0   ? tr("Gut in Schuss")
                                : row.grade == 1 ? tr("Braucht Pflege")
                                                 : tr("Bitte ansehen");
        dashboard_->setHealth(row.score, caption, findings);
        toast::show(this, tr("Gesundheitscheck fertig: %1 Punkte.").arg(row.score),
                    row.grade == 0 ? toast::Kind::Success : toast::Kind::Warning);
    });
    connect(health_, &HealthPage::startRequested, &session_, &Session::healthCheckRequested);
    connect(health_, &HealthPage::cancelRequested, &session_, &Session::cancelScan);
    connect(worker, &Worker::liveValue, trip_, &TripPage::onLiveValue);
    connect(worker, &Worker::supportedPids, trip_, &TripPage::onSupportedPids);
    connect(trip_, &TripPage::startRequested, &session_, &Session::startLiveRequested);
    connect(trip_, &TripPage::stopRequested, &session_, &Session::stopLiveRequested);
    connect(worker, &Worker::liveValue, performance_, &PerformancePage::onLiveValue);
    connect(performance_, &PerformancePage::startRequested, &session_, &Session::startLiveRequested);
    connect(performance_, &PerformancePage::stopRequested, &session_, &Session::stopLiveRequested);
    connect(worker, &Worker::adapterVoltage, battery_, &BatteryPage::onVoltage);
    connect(battery_, &BatteryPage::pollRequested, &session_, &Session::voltagePollRequested);
    connect(battery_, &BatteryPage::pollStopRequested, &session_, &Session::voltagePollStopRequested);
    connect(worker, &Worker::adapterReport, adapter_, &AdapterPage::onReport);
    connect(adapter_, &AdapterPage::probeRequested, &session_, &Session::probeAdapterRequested);

    // UI to worker
    connect(connect_, &ConnectPage::serialRequested, this,
            [this](const QString& device, const QString& order) {
                appendLog(device.isEmpty() ? tr("Suche den Adapter an allen Anschlüssen ...")
                                           : tr("Verbinde über %1 ...").arg(device));
                // Remember the port so auto-connect knows where to go next time.
                if (!device.isEmpty() && settings_.lastPort != device) {
                    settings_.lastPort = device;
                    saveSettings(settings_);
                }
                titleBar_->setConnection(TitleBar::Link::Connecting, tr("Verbinde ..."));
                emit session_.openSerialRequested(device, settings_.baudOverride, order);
            });
    connect(connect_, &ConnectPage::tcpRequested, this,
            [this](const QString& host, int port, const QString& order) {
                appendLog(tr("Verbinde mit %1:%2 ...").arg(host).arg(port));
                titleBar_->setConnection(TitleBar::Link::Connecting, tr("Verbinde ..."));
                emit session_.openTcpRequested(host, port, order);
            });
    connect(connect_, &ConnectPage::demoRequested, this, [this](const QString& order) {
        appendLog(tr("Starte das Demo-Auto ..."));
        titleBar_->setConnection(TitleBar::Link::Connecting, tr("Verbinde ..."));
        emit session_.openDemoRequested(order);
    });
    connect(connect_, &ConnectPage::disconnectRequested, &session_, &Session::closeRequested);

    connect(dashboard_, &DashboardPage::readRequested, &session_, &Session::readFaultsRequested);
    connect(dashboard_, &DashboardPage::detailsRequested, &session_,
            &Session::readDetailsRequested);
    connect(dashboard_, &DashboardPage::connectRequested, this, [this] { navigateTo(connect_); });
    connect(dashboard_, &DashboardPage::faultsRequested, this, [this] { navigateTo(faults_); });
    connect(dashboard_, &DashboardPage::liveRequested, this, [this] {
        navigateTo(live_);
        live_->toggleFromOutside();
    });
    connect(faults_, &FaultsPage::readRequested, &session_, &Session::readFaultsRequested);
    connect(faults_, &FaultsPage::clearRequested, &session_, &Session::clearFaultsRequested);
    connect(faults_, &FaultsPage::bruteForceRequested, &session_,
            &Session::bruteForceClearRequested);
    connect(tools_, &ToolsPage::detailsRequested, &session_, &Session::readDetailsRequested);
    connect(tools_, &ToolsPage::rawRequested, &session_, &Session::sendRawRequested);
    connect(monitor_, &MonitorPage::readRequested, &session_, &Session::readMonitorsRequested);
    connect(live_, &LivePage::startRequested, &session_, &Session::startLiveRequested);
    connect(live_, &LivePage::stopRequested, &session_, &Session::stopLiveRequested);
    connect(throttle_, &ThrottlePage::kklConnectRequested, &session_, &Session::openKklRequested);
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
    connect(kklTools_, &KklToolsPage::connectRequested, &session_, &Session::openKklRequested);
    connect(kklTools_, &KklToolsPage::disconnectRequested, &session_,
            &Session::closeKklRequested);
    connect(kklTools_, &KklToolsPage::adaptationRequested, &session_,
            &Session::adaptationRequested);
    connect(kklTools_, &KklToolsPage::actuatorTestRequested, &session_,
            &Session::actuatorTestRequested);

    const int ports = connect_->refreshPorts();
    appendLog(ports == 0 ? tr("Kein COM-Anschluss gefunden. Steckt der Adapter am Rechner?")
                         : tr("%n Anschluss/Anschlüsse gefunden.", nullptr, ports));

    // Load the saved settings. showValues reports the values itself, so
    // everything takes the same path as a manual change.
    settings_ = loadSettings();
    connect_->setSettings(settings_);
    settingsPage_->showValues(settings_);

    if (settings_.rememberGeometry) {
        QSettings store(QStringLiteral("Caruspex"), QStringLiteral("Caruspex"));
        const QByteArray geometry = store.value(QStringLiteral("fenster/geometrie")).toByteArray();
        if (!geometry.isEmpty()) restoreGeometry(geometry);
    }

    appendLog(tr("Caruspex bereit. Adapter einstecken, Zündung an, dann verbinden."));

    if (settings_.autoConnect && !settings_.lastPort.isEmpty()) {
        if (connect_->selectPort(settings_.lastPort)) {
            appendLog(tr("Verbinde selbsttätig mit %1, so eingestellt.").arg(settings_.lastPort));
            QTimer::singleShot(0, connect_, &ConnectPage::startConnect);
        } else {
            appendLog(tr("Selbsttätiges Verbinden: Anschluss %1 ist nicht da.")
                          .arg(settings_.lastPort));
        }
    }
}

void MainWindow::applySettings(const AppSettings& values) {
    settings_ = values;
    saveSettings(settings_);
    connect_->setSettings(settings_);

    // To the worker thread: poll interval, timeouts, raw log, session type.
    emit session_.settingsChanged(settings_);

    // To the pages.
    live_->setWindowSeconds(settings_.chartWindowSeconds);
    faults_->setClearOptions(settings_.clearAttempts, settings_.confirmClear);
    scan_->setClearAttempts(settings_.clearAttempts);

    // Open or close the log file.
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
                       << QDateTime::currentDateTime().toString("dd.MM.yyyy HH:mm") << " ===\n";
            logStream_.flush();
            appendLog(tr("Protokoll wird zusätzlich nach %1 geschrieben.").arg(settings_.logPath));
        } else {
            // Swallowing this silently would be the worst option: the user
            // would otherwise believe the log is being written.
            appendLog(tr("Protokolldatei %1 lässt sich nicht schreiben. Es wird nur "
                         "hier im Fenster protokolliert.")
                          .arg(settings_.logPath));
        }
    }
}

QWidget* MainWindow::buildSidebar() {
    sidebar_ = new QFrame;
    sidebar_->setObjectName("Sidebar");
    sidebar_->setFixedWidth(kSidebarWide);
    auto* root = new QVBoxLayout(sidebar_);
    root->setContentsMargins(10, 14, 10, 12);
    root->setSpacing(2);

    navigation_ = new QButtonGroup(this);
    navigation_->setExclusive(true);

    auto* scroll = new QScrollArea;
    scroll->setWidgetResizable(true);
    scroll->setFrameShape(QFrame::NoFrame);
    scroll->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    auto* list = new QWidget;
    navigationLayout_ = new QVBoxLayout(list);
    navigationLayout_->setContentsMargins(0, 0, 0, 0);
    navigationLayout_->setSpacing(2);
    scroll->setWidget(list);
    root->addWidget(scroll, 1);

    addEntry(QStringLiteral("home"), tr("Start"), dashboard_);
    addEntry(QStringLiteral("plug"), tr("Verbinden"), connect_);
    addSection(tr("Diagnose"));
    addEntry(QStringLiteral("heart"), tr("Gesundheitscheck"), health_);
    faultsButton_ = addEntry(QStringLiteral("alert"), tr("Fehlerspeicher"), faults_);
    addEntry(QStringLiteral("shield"), tr("Prüfen und AU"), tools_);
    addEntry(QStringLiteral("stethoscope"), tr("Bauteilprüfung"), monitor_);
    addSection(tr("Messen"));
    addEntry(QStringLiteral("activity"), tr("Live-Werte"), live_);
    addEntry(QStringLiteral("fuel"), tr("Bordcomputer"), trip_);
    addEntry(QStringLiteral("timer"), tr("Leistung"), performance_);
    addEntry(QStringLiteral("battery"), tr("Batterie"), battery_);
    addEntry(QStringLiteral("chart"), tr("Statistik"), stats_);
    addSection(tr("Werkstatt"));
    addEntry(QStringLiteral("cpu"), tr("Alle Steuergeräte"), scan_);
    addEntry(QStringLiteral("throttle"), tr("Drosselklappe"), throttle_);
    addEntry(QStringLiteral("sliders"), tr("Anpassung und Stellglieder"), kklTools_);
    addSection(tr("Wissen"));
    addEntry(QStringLiteral("book"), tr("Fehlercode-Lexikon"), lexicon_);
    navigationLayout_->addStretch();

    auto* divider = new QFrame;
    divider->setObjectName("Divider");
    root->addSpacing(6);
    root->addWidget(divider);
    root->addSpacing(6);
    bottomNavigation_ = new QVBoxLayout;
    bottomNavigation_->setSpacing(2);
    root->addLayout(bottomNavigation_);
    addEntry(QStringLiteral("garage"), tr("Garage"), garagePage_, bottomNavigation_);
    addEntry(QStringLiteral("cable"), tr("Adapter"), adapter_, bottomNavigation_);
    addEntry(QStringLiteral("settings"), tr("Einstellungen"), settingsPage_, bottomNavigation_);

    buttonForPage_.value(dashboard_)->setChecked(true);
    pages_->setCurrentWidget(dashboard_);

    sidebarAnimation_ = new QPropertyAnimation(sidebar_, "maximumWidth", this);
    sidebarAnimation_->setDuration(240);
    sidebarAnimation_->setEasingCurve(QEasingCurve::OutCubic);
    connect(sidebarAnimation_, &QPropertyAnimation::valueChanged, this, [this](const QVariant& v) {
        sidebar_->setMinimumWidth(v.toInt());
    });
    return sidebar_;
}

QWidget* MainWindow::buildLogDrawer() {
    logDrawer_ = new QWidget;
    logDrawer_->setMaximumHeight(0);
    auto* layout = new QVBoxLayout(logDrawer_);
    layout->setContentsMargins(20, 8, 20, 14);
    layout->setSpacing(6);
    auto* head = new QHBoxLayout;
    auto* icon = new QLabel;
    icon->setPixmap(icons::pixmap(QStringLiteral("terminal"), 16, color::textMuted));
    head->addWidget(icon);
    head->addWidget(makeLabel(tr("Protokoll"), nullptr));
    head->addStretch();
    auto* clear = new QPushButton(tr("Leeren"));
    clear->setObjectName("Ghost");
    auto* close = new QPushButton;
    close->setObjectName("Ghost");
    close->setIcon(icons::icon(QStringLiteral("chevron-down"), color::textMuted, color::text, 16));
    head->addWidget(clear);
    head->addWidget(close);
    layout->addLayout(head);
    log_ = new QPlainTextEdit;
    log_->setReadOnly(true);
    log_->setFont(mono(12));
    log_->setMaximumBlockCount(5000);
    layout->addWidget(log_, 1);
    connect(clear, &QPushButton::clicked, log_, &QPlainTextEdit::clear);
    connect(close, &QPushButton::clicked, this, &MainWindow::toggleLog);
    logDrawer_->setStyleSheet(QStringLiteral("QWidget#LogDrawer { border-top: 1px solid %1; }")
                                  .arg(color::border.name()));
    logDrawer_->setObjectName("LogDrawer");

    logAnimation_ = new QPropertyAnimation(logDrawer_, "maximumHeight", this);
    logAnimation_->setDuration(260);
    logAnimation_->setEasingCurve(QEasingCurve::OutCubic);
    return logDrawer_;
}

void MainWindow::toggleLog() {
    logOpen_ = !logOpen_;
    titleBar_->setLogOpen(logOpen_);
    logAnimation_->stop();
    logAnimation_->setStartValue(logDrawer_->maximumHeight());
    logAnimation_->setEndValue(logOpen_ ? kLogHeight : 0);
    logAnimation_->start();
}

void MainWindow::toggleSidebar() {
    sidebarCompact_ = !sidebarCompact_;
    for (NavButton* b : std::as_const(navButtons_)) b->setCompact(sidebarCompact_);
    for (QLabel* l : std::as_const(sectionLabels_)) {
        l->setText(sidebarCompact_ ? QString() : l->property("fullText").toString());
        l->setMaximumHeight(sidebarCompact_ ? 10 : 40);
    }
    sidebarAnimation_->stop();
    sidebarAnimation_->setStartValue(sidebar_->width());
    sidebarAnimation_->setEndValue(sidebarCompact_ ? kSidebarCompact : kSidebarWide);
    sidebar_->setMinimumWidth(0);
    sidebarAnimation_->start();
}

void MainWindow::addSection(const QString& title) {
    auto* label = new QLabel(title.toUpper());
    label->setObjectName("NavSection");
    label->setProperty("fullText", title.toUpper());
    navigationLayout_->addSpacing(2);
    navigationLayout_->addWidget(label);
    sectionLabels_.push_back(label);
}

NavButton* MainWindow::addEntry(const QString& icon, const QString& text, QWidget* page,
                                QVBoxLayout* target) {
    const int index = pages_->addWidget(page);
    auto* button = new NavButton(icon, text);
    (target ? target : navigationLayout_)->addWidget(button);
    navigation_->addButton(button, index);
    navButtons_.push_back(button);
    buttonForPage_.insert(page, button);
    connect(button, &QAbstractButton::clicked, this, [this, page] { navigateTo(page); });
    return button;
}

void MainWindow::showPageByName(const QString& name) {
    const QHash<QString, QWidget*> pages = {
        {QStringLiteral("start"), dashboard_}, {QStringLiteral("connect"), connect_},
        {QStringLiteral("faults"), faults_},   {QStringLiteral("tools"), tools_},
        {QStringLiteral("monitor"), monitor_}, {QStringLiteral("live"), live_},
        {QStringLiteral("stats"), stats_},     {QStringLiteral("scan"), scan_},
        {QStringLiteral("throttle"), throttle_}, {QStringLiteral("kkl"), kklTools_},
        {QStringLiteral("lexicon"), lexicon_}, {QStringLiteral("settings"), settingsPage_},
        {QStringLiteral("garage"), garagePage_},  {QStringLiteral("health"), health_},
        {QStringLiteral("trip"), trip_},          {QStringLiteral("performance"), performance_},
        {QStringLiteral("battery"), battery_},    {QStringLiteral("adapter"), adapter_},
    };
    if (QWidget* page = pages.value(name.toLower())) navigateTo(page);
    // Developer aids for screenshots: open a page and start what it does.
    if (name == QStringLiteral("health-run")) {
        navigateTo(health_);
        health_->onProgress(tr("Starte ..."), 0);
        emit session_.healthCheckRequested(12);
    }
    if (name == QStringLiteral("adapter-run")) {
        navigateTo(adapter_);
        emit session_.probeAdapterRequested();
    }
    if (name == QStringLiteral("live-run")) {
        navigateTo(live_);
        live_->toggleFromOutside();
    }
    if (name == QStringLiteral("log") && !logOpen_) toggleLog();
}

void MainWindow::navigateTo(QWidget* page) {
    if (NavButton* button = buttonForPage_.value(page)) button->setChecked(true);
    pages_->slideTo(page);
}

void MainWindow::onConnected(const QString& adapter, const QString& transport,
                             const QString& protocol) {
    connected_ = true;
    adapterText_ = adapter;
    titleBar_->setConnection(TitleBar::Link::Online, tr("Verbunden · %1").arg(protocol));
    faults_->setConnected(true);
    live_->setConnected(true);
    tools_->setConnected(true);
    monitor_->setConnected(true);
    throttle_->setEobdConnected(true);
    stats_->setConnected(true);
    health_->setConnected(true);
    trip_->setConnected(true);
    performance_->setConnected(true);
    battery_->setConnected(true);
    adapter_->setConnected(true);
    toast::show(this, tr("Verbunden über %1. Protokoll: %2").arg(transport, protocol),
                toast::Kind::Success);
    // After connecting the start page is the natural place to go. The VIN
    // comes right after, so the garage recognises the car.
    navigateTo(dashboard_);
    QTimer::singleShot(0, this, [this] { emit session_.readDetailsRequested(); });
}

void MainWindow::onFailed(const QString& error) {
    if (!connected_) {
        titleBar_->setConnection(TitleBar::Link::Error, tr("Keine Verbindung"));
    }
    appendLog(error);
    toast::show(this, error, toast::Kind::Error, 6000);
}

void MainWindow::onClosed() {
    connected_ = false;
    titleBar_->setConnection(TitleBar::Link::Offline, tr("Nicht verbunden"));
    faultsButton_->setBadge(QString());
    garagePage_->setCurrentVin(QString());
    dashboard_->onDisconnected();
    faults_->setConnected(false);
    live_->setConnected(false);
    tools_->setConnected(false);
    monitor_->setConnected(false);
    throttle_->setEobdConnected(false);
    stats_->setConnected(false);
    health_->setConnected(false);
    trip_->setConnected(false);
    performance_->setConnected(false);
    battery_->setConnected(false);
    adapter_->setConnected(false);
    appendLog(tr("Verbindung getrennt."));
}

void MainWindow::onBusy(bool active) { Q_UNUSED(active); }

void MainWindow::recordCheck() {
    if (state_->vin().isEmpty() || !state_->haveFaults()) return;
    GarageCheck check;
    check.when = QDateTime::currentDateTime();
    check.score = state_->score();
    check.mil = state_->mil();
    for (const ReadinessRow& r : state_->readiness()) {
        if (!r.ready) ++check.openMonitors;
    }
    check.odometerKm = state_->hasLive(0xA6) ? state_->liveValue(0xA6) : -1.0;
    for (const FaultRow& f : state_->faults()) {
        check.codes << f.code;
        check.texts << f.text;
    }
    garage_->addCheck(state_->vin(), check);
}

void MainWindow::showReport() {
    ReportExtras extras;
    extras.score = dashboard_->score();
    extras.scoreCaption = dashboard_->scoreCaption();
    for (const DashboardPage::Finding& f : dashboard_->findings()) {
        QString level = tr("Hinweis");
        if (f.tint == color::bad) level = tr("Hoch");
        else if (f.tint == color::warn) level = tr("Mittel");
        else if (f.tint == color::good) level = tr("In Ordnung");
        extras.findings.push_back({level, f.title, f.text});
    }
    const QString path = saveReportPdf(this, *state_, extras);
    if (!path.isEmpty()) {
        toast::show(this, tr("Prüfbericht gespeichert: %1").arg(path), toast::Kind::Success);
    }
}

void MainWindow::appendLog(const QString& line) {
    const QString stamped = QDateTime::currentDateTime().toString("HH:mm:ss") + "  " + line;
    log_->appendPlainText(stamped);
    if (logStream_.device()) {
        logStream_ << stamped << '\n';
        logStream_.flush();
    }
}

// --- Frameless window -------------------------------------------------------

Qt::Edges MainWindow::edgesAt(const QPoint& p) const {
    if (isMaximized() || isFullScreen()) return {};
    Qt::Edges edges;
    if (p.x() <= kResizeMargin) edges |= Qt::LeftEdge;
    if (p.x() >= width() - kResizeMargin) edges |= Qt::RightEdge;
    if (p.y() <= kResizeMargin) edges |= Qt::TopEdge;
    if (p.y() >= height() - kResizeMargin) edges |= Qt::BottomEdge;
    return edges;
}

// On Windows, WM_NCHITTEST further down handles this. On the other systems the
// edge strip is caught here, because otherwise the child widgets get the mouse
// events first.
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
        // The window manager does the resizing, so it feels exactly like
        // any other window.
        if (QWindow* handle = windowHandle()) {
            handle->startSystemResize(edges);
            return true;   // don't pass it on to the control underneath
        }
    }
    return QMainWindow::eventFilter(watched, event);
#endif
}

#ifdef Q_OS_WIN
// On every mouse move Windows asks which part of the window is under the
// pointer. Reporting an edge here makes the operating system do the resizing
// itself, including the right cursor and snapping. The maths is done in
// physical pixels so display scaling doesn't matter.
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
    if (titleBar_) {
        titleBar_->updateMaximiseState(maximised);
        titleBar_->update();
    }
    // When maximised the corners must be square, otherwise gaps show at the
    // screen corners.
    // The type is fixed, frame_ is set in only one place.
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
    connect_->setMode(ConnectPage::Mode::Wifi);
    connect_->setWifiAddress(address);
    connect_->startConnect();
}

void MainWindow::autoConnectDemo() {
    connect_->setMode(ConnectPage::Mode::Demo);
    connect_->startConnect();
}

}  // namespace caruspex
