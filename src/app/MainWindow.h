#pragma once

#include <QFile>
#include <QHash>
#include <QMainWindow>
#include <QTextStream>
#include <QVector>

#include "app/Session.h"
#include "app/Settings.h"

QT_BEGIN_NAMESPACE
class QButtonGroup;
class QFrame;
class QLabel;
class QPlainTextEdit;
class QPropertyAnimation;
class QVBoxLayout;
QT_END_NAMESPACE

namespace caruspex {

class AnimatedStack;
class NavButton;
class TitleBar;
class ConnectPage;
class DashboardPage;
class FaultsPage;
class LivePage;
class ToolsPage;
class ThrottlePage;
class StatsPage;
class ScanPage;
class KklToolsPage;
class MonitorPage;
class SettingsPage;
class LexiconPage;
class GaragePage;
class HealthPage;
class TripPage;
class PerformancePage;
class BatteryPage;
class AdapterPage;
class VehiclesPage;
class Garage;
class VehicleState;

class MainWindow : public QMainWindow {
    Q_OBJECT

public:
    explicit MainWindow(QWidget* parent = nullptr);

    // Command-line option --tcp host:port
    void autoConnectTcp(const QString& address);
    // Command-line option --demo
    void autoConnectDemo();
    // Command-line option --page: start, connect, faults, tools, monitor, live,
    // stats, scan, throttle, kkl, lexicon, settings, plus the newer pages.
    void showPageByName(const QString& name);

protected:
    // Intercepts mouse moves to offer resizing at the window edges. Otherwise
    // the child widgets get the events first.
    bool eventFilter(QObject* watched, QEvent* event) override;
    void changeEvent(QEvent* event) override;
    void closeEvent(QCloseEvent* event) override;
#ifdef Q_OS_WIN
    // On Windows the operating system handles resizing if WM_NCHITTEST tells
    // it where the edges are. That gives the right cursors, snapping and
    // dragging across screen boundaries.
    bool nativeEvent(const QByteArray& type, void* message, qintptr* result) override;
#endif

private slots:
    void onConnected(const QString& adapter, const QString& transport, const QString& protocol);
    void onFailed(const QString& error);
    void onClosed();
    void onBusy(bool active);
    void appendLog(const QString& line);
    // Applies changed settings: save them, pass them on to the worker thread
    // and update the pages.
    void applySettings(const caruspex::AppSettings& values);

private:
    QWidget* buildSidebar();
    QWidget* buildLogDrawer();
    // Returns the edges the pointer is currently on, otherwise empty.
    Qt::Edges edgesAt(const QPoint& windowPosition) const;
    void toggleMaximise();
    void toggleSidebar();
    void toggleLog();
    void addSection(const QString& title);
    NavButton* addEntry(const QString& icon, const QString& text, QWidget* page,
                        QVBoxLayout* target = nullptr);
    void navigateTo(QWidget* page);
    // Stores the current check in the garage, once VIN and faults are known.
    void recordCheck();
    void showReport();

    Session session_;

    TitleBar* titleBar_ = nullptr;
    QWidget* frame_ = nullptr;
    AnimatedStack* pages_ = nullptr;
    QButtonGroup* navigation_ = nullptr;
    QVBoxLayout* navigationLayout_ = nullptr;
    QVBoxLayout* bottomNavigation_ = nullptr;
    QFrame* sidebar_ = nullptr;
    QVector<QLabel*> sectionLabels_;
    QVector<NavButton*> navButtons_;
    QHash<QWidget*, NavButton*> buttonForPage_;
    QPropertyAnimation* sidebarAnimation_ = nullptr;
    bool sidebarCompact_ = false;
    QWidget* logDrawer_ = nullptr;
    QPropertyAnimation* logAnimation_ = nullptr;
    bool logOpen_ = false;
    QPlainTextEdit* log_ = nullptr;

    ConnectPage* connect_ = nullptr;
    DashboardPage* dashboard_ = nullptr;
    FaultsPage* faults_ = nullptr;
    LivePage* live_ = nullptr;
    ToolsPage* tools_ = nullptr;
    ThrottlePage* throttle_ = nullptr;
    StatsPage* stats_ = nullptr;
    ScanPage* scan_ = nullptr;
    KklToolsPage* kklTools_ = nullptr;
    MonitorPage* monitor_ = nullptr;
    SettingsPage* settingsPage_ = nullptr;
    LexiconPage* lexicon_ = nullptr;
    GaragePage* garagePage_ = nullptr;
    HealthPage* health_ = nullptr;
    TripPage* trip_ = nullptr;
    PerformancePage* performance_ = nullptr;
    BatteryPage* battery_ = nullptr;
    AdapterPage* adapter_ = nullptr;
    VehiclesPage* vehicles_ = nullptr;
    NavButton* faultsButton_ = nullptr;
    VehicleState* state_ = nullptr;
    Garage* garage_ = nullptr;

    bool connected_ = false;
    QString adapterText_;

    AppSettings settings_;
    QFile logFile_;
    QTextStream logStream_;
};

}  // namespace caruspex
