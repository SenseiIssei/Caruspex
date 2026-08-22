#pragma once

#include <QMainWindow>
#include <QVector>

#include "app/Session.h"

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

class DashboardPage;
class FaultsPage;
class LivePage;
class ToolsPage;
class ThrottlePage;
class StatsPage;
class LexiconPage;

class MainWindow : public QMainWindow {
    Q_OBJECT

public:
    explicit MainWindow(QWidget* parent = nullptr);

    // Startparameter --tcp host:port
    void autoConnectTcp(const QString& address);

private slots:
    void refreshPorts();
    void toggleConnection();
    void onConnected(const QString& adapter, const QString& transport, const QString& protocol);
    void onFailed(const QString& error);
    void onClosed();
    void onBusy(bool active);
    void appendLog(const QString& line);
    void onBrandChanged();

private:
    QWidget* buildSidebar();
    void addNavigationEntry(const QString& text, QWidget* page);
    QString protocolOrderForSelection() const;

    Session session_;

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
    LexiconPage* lexicon_ = nullptr;

    bool connected_ = false;
};

}  // namespace caruspex
