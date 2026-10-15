#pragma once

#include <QVector>
#include <QWidget>

#include "app/Session.h"

class QLabel;
class QPushButton;
class QPlainTextEdit;
class QSplitter;
class QTableWidget;

namespace caruspex {

class VehicleDiagram;

class FaultsPage : public QWidget {
    Q_OBJECT

public:
    explicit FaultsPage(QWidget* parent = nullptr);

public slots:
    void onFaults(const QVector<caruspex::FaultRow>& rows);
    void setConnected(bool connected);
    void onClearProgress(const QString& line);
    void onClearFinished(bool everythingGone, int remaining);

public:
    // Attempts for the persistent clear and whether to ask first.
    void setClearOptions(int attempts, bool confirm);

signals:
    void readRequested();
    void clearRequested();
    void bruteForceRequested(int attempts);
    void exportRequested();

private:
    int clearAttempts_ = 5;
    bool confirmClear_ = true;
    void askAndClear();
    void askAndBruteForce();
    void exportCsv();
    void showSelected(int row);

    QTableWidget* table_ = nullptr;
    QPushButton* readButton_ = nullptr;
    QPushButton* clearButton_ = nullptr;
    QPushButton* bruteButton_ = nullptr;
    QPlainTextEdit* clearLog_ = nullptr;
    QPushButton* exportButton_ = nullptr;
    QLabel* summary_ = nullptr;
    QLabel* throttleHint_ = nullptr;
    VehicleDiagram* diagram_ = nullptr;
    QLabel* whereLabel_ = nullptr;
    QVector<FaultRow> rows_;
};

}  // namespace caruspex
