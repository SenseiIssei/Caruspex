// The fault memory: every code with how urgent it is, and for the selected one
// what it means, whether the car may still be driven, where on the car to
// look, the usual causes and what one can check without a workshop.
#pragma once

#include <QVector>
#include <QWidget>

#include "app/Session.h"

class QLabel;
class QListWidget;
class QPushButton;
class QPlainTextEdit;
class QVBoxLayout;

namespace caruspex {

class VehicleDiagram;

// Colour for a Severity value (as int).
QColor severityColour(int severity);

class FaultsPage : public QWidget {
    Q_OBJECT

public:
    explicit FaultsPage(QWidget* parent = nullptr);

    // Brand of the connected car, so manufacturer codes get their meaning.
    void setBrand(const QString& brand);

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
    void clearDetail();

    QString brand_;
    QListWidget* list_ = nullptr;
    QPushButton* readButton_ = nullptr;
    QPushButton* clearButton_ = nullptr;
    QPushButton* bruteButton_ = nullptr;
    QPlainTextEdit* clearLog_ = nullptr;
    QPushButton* exportButton_ = nullptr;
    QLabel* summary_ = nullptr;
    QLabel* throttleHint_ = nullptr;
    VehicleDiagram* diagram_ = nullptr;

    QWidget* detail_ = nullptr;
    QWidget* empty_ = nullptr;
    QLabel* emptyText_ = nullptr;
    QLabel* code_ = nullptr;
    QLabel* severity_ = nullptr;
    QLabel* title_ = nullptr;
    QLabel* meta_ = nullptr;
    QLabel* advice_ = nullptr;
    QLabel* where_ = nullptr;
    QVBoxLayout* causes_ = nullptr;
    QVBoxLayout* checks_ = nullptr;
    QLabel* cost_ = nullptr;
    QWidget* causesBox_ = nullptr;
    QWidget* checksBox_ = nullptr;
    QVector<FaultRow> rows_;
};

}  // namespace caruspex
