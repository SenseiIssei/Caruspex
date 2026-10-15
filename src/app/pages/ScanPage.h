#pragma once

#include <QVector>
#include <QWidget>

#include "app/Session.h"

QT_BEGIN_NAMESPACE
class QComboBox;
class QLabel;
class QPlainTextEdit;
class QProgressBar;
class QPushButton;
class QTreeWidget;
QT_END_NAMESPACE

namespace caruspex {

// Polls every known control unit in turn and shows which ones the vehicle has
// and where something is stored in fault memory. Needs a KKL cable.
class ScanPage : public QWidget {
    Q_OBJECT

public:
    explicit ScanPage(QWidget* parent = nullptr);

public slots:
    void onScanStarted(int total);
    void onClearProgress(const QString& line);
    void onScanProgress(const QString& ecuName, int index, int total);
    void onEcuResult(const caruspex::EcuScanRow& row);
    void onScanFinished(int present, int withFaults, bool cancelled);

public:
    // Attempts when clearing all control units.
    void setClearAttempts(int attempts) { clearAttempts_ = attempts > 0 ? attempts : 1; }

signals:
    void scanRequested(const QString& device, int clearAttempts);
    void cancelRequested();
    void reportRequested();

private:
    int clearAttempts_ = 3;
    void refreshPorts();
    void exportReport();

    QComboBox* portBox_ = nullptr;
    QPushButton* rescanButton_ = nullptr;
    QPushButton* startButton_ = nullptr;
    QPushButton* clearAllButton_ = nullptr;
    QPushButton* reportButton_ = nullptr;
    QProgressBar* progress_ = nullptr;
    QLabel* status_ = nullptr;
    QPlainTextEdit* clearLog_ = nullptr;
    QTreeWidget* tree_ = nullptr;

    QVector<EcuScanRow> results_;
    bool running_ = false;
};

}  // namespace caruspex
