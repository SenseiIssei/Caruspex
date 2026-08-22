#pragma once

#include <QVector>
#include <QWidget>

#include "app/Session.h"

class QLabel;
class QPushButton;
class QTableWidget;

namespace caruspex {

class FaultsPage : public QWidget {
    Q_OBJECT

public:
    explicit FaultsPage(QWidget* parent = nullptr);

public slots:
    void onFaults(const QVector<caruspex::FaultRow>& rows);
    void setConnected(bool connected);

signals:
    void readRequested();
    void clearRequested();
    void exportRequested();

private:
    void askAndClear();
    void exportCsv();

    QTableWidget* table_ = nullptr;
    QPushButton* readButton_ = nullptr;
    QPushButton* clearButton_ = nullptr;
    QPushButton* exportButton_ = nullptr;
    QLabel* summary_ = nullptr;
    QLabel* throttleHint_ = nullptr;
    QVector<FaultRow> rows_;
};

}  // namespace caruspex
