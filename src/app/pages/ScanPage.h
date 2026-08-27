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

// Klappert alle bekannten Steuergeraete ab und zeigt, welche das Fahrzeug hat
// und wo etwas im Speicher steht. Braucht ein KKL-Kabel.
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

signals:
    void scanRequested(const QString& device, int clearAttempts);
    void cancelRequested();
    void reportRequested();

private:
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
