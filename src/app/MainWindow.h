#pragma once

#include <QMainWindow>
#include <QVector>

#include <QFile>
#include <QTextStream>

#include "app/Session.h"
#include "app/Settings.h"

QT_BEGIN_NAMESPACE
class QButtonGroup;
class QCheckBox;
class QComboBox;
class QLabel;
class QLineEdit;
class QPlainTextEdit;
class QProgressBar;
class QPushButton;
class QSpinBox;
class QStackedWidget;
class QVBoxLayout;
QT_END_NAMESPACE

namespace caruspex {

class TitleBar;
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

class MainWindow : public QMainWindow {
    Q_OBJECT

public:
    explicit MainWindow(QWidget* parent = nullptr);

    // Command-line option --tcp host:port
    void autoConnectTcp(const QString& address);

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
    void refreshPorts();
    void toggleConnection();
    void onConnected(const QString& adapter, const QString& transport, const QString& protocol);
    void onFailed(const QString& error);
    void onClosed();
    void onBusy(bool active);
    void appendLog(const QString& line);
    void onBrandChanged();
    // Applies changed settings: save them, pass them on to the worker thread
    // and update the pages.
    void applySettings(const caruspex::AppSettings& values);

private:
    QWidget* buildSidebar();
    // Returns the edges the pointer is currently on, otherwise empty.
    Qt::Edges edgesAt(const QPoint& windowPosition) const;
    void toggleMaximise();
    void addNavigationEntry(const QString& text, QWidget* page);
    QString protocolOrderForSelection() const;

    Session session_;

    TitleBar* titleBar_ = nullptr;
    QWidget* frame_ = nullptr;
    QStackedWidget* pages_ = nullptr;
    QButtonGroup* navigation_ = nullptr;
    QVBoxLayout* navigationLayout_ = nullptr;

    QComboBox* brandBox_ = nullptr;
    QSpinBox* yearBox_ = nullptr;
    QLabel* profileNote_ = nullptr;
    QCheckBox* wifiBox_ = nullptr;
    QComboBox* portBox_ = nullptr;
    QLineEdit* addressEdit_ = nullptr;
    QPushButton* rescanButton_ = nullptr;
    QPushButton* connectButton_ = nullptr;
    QLabel* stateLabel_ = nullptr;
    QLabel* adapterLabel_ = nullptr;
    QProgressBar* busyBar_ = nullptr;
    QPlainTextEdit* log_ = nullptr;

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

    bool connected_ = false;

    AppSettings settings_;
    QFile logFile_;
    QTextStream logStream_;
};

}  // namespace caruspex
