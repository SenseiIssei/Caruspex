// The health check: one button, about half a minute of reading, then a score
// with every finding explained in plain words, plus what is still missing for
// the emissions test and how to drive to finish it.
#pragma once

#include <QWidget>

#include "app/Session.h"

class QComboBox;
class QLabel;
class QProgressBar;
class QPushButton;
class QVBoxLayout;

namespace caruspex {

class ScoreRing;

// Icon and colour for an analysis level (0 good .. 3 critical).
QString levelIcon(int level);
QColor levelColour(int level);
QString levelLabel(int level);

class HealthPage : public QWidget {
    Q_OBJECT

public:
    explicit HealthPage(QWidget* parent = nullptr);

public slots:
    void setConnected(bool connected);
    void onProgress(const QString& step, int percent);
    void onResult(const caruspex::HealthRow& row);

signals:
    void startRequested(int seconds);
    void cancelRequested();

private:
    void clearList(QVBoxLayout* list);
    void setRunning(bool running);

    bool connected_ = false;
    bool running_ = false;
    ScoreRing* ring_ = nullptr;
    QLabel* headline_ = nullptr;
    QLabel* summary_ = nullptr;
    QComboBox* duration_ = nullptr;
    QPushButton* start_ = nullptr;
    QPushButton* cancel_ = nullptr;
    QProgressBar* progress_ = nullptr;
    QLabel* step_ = nullptr;
    QVBoxLayout* findings_ = nullptr;
    QVBoxLayout* monitors_ = nullptr;
    QLabel* monitorSummary_ = nullptr;
    QVBoxLayout* skipped_ = nullptr;
    QWidget* skippedCard_ = nullptr;
};

}  // namespace caruspex
