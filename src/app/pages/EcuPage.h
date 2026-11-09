// All control units over CAN: asks each one through UDS whether it is there,
// what it is and what it has stored. Works with a plain ELM327 on most cars
// from about 2008. Older VW and Audi go through the KKL page instead.
#pragma once

#include <QHash>
#include <QWidget>

#include "app/Session.h"

class QComboBox;
class QLabel;
class QListWidget;
class QProgressBar;
class QPushButton;
class QVBoxLayout;

namespace caruspex {

class EcuPage : public QWidget {
    Q_OBJECT

public:
    explicit EcuPage(QWidget* parent = nullptr);

    // Brand of the connected car; preselects its address list.
    void setBrand(const QString& brand);

    // Results of the last scan, for the report.
    QVector<UdsEcuRow> results() const;

public slots:
    void setConnected(bool connected);
    void onScanStarted(int total, bool generic);
    void onScanProgress(const QString& name, int index, int total);
    void onEcuResult(const caruspex::UdsEcuRow& row);
    void onScanFinished(int present, int withFaults, bool cancelled, const QString& note);
    void onClearResult(const QString& name, bool ok, const QString& message);

signals:
    void scanRequested(const QString& family);
    void cancelRequested();
    void clearRequested(quint32 request, quint32 response, bool extended29, const QString& name);

private:
    void showEcu(quint32 request);
    void refreshList();

    QString brand_;
    bool connected_ = false;
    bool running_ = false;
    QComboBox* family_ = nullptr;
    QPushButton* start_ = nullptr;
    QPushButton* cancel_ = nullptr;
    QProgressBar* progress_ = nullptr;
    QLabel* status_ = nullptr;
    QListWidget* list_ = nullptr;
    QWidget* detail_ = nullptr;
    QWidget* empty_ = nullptr;
    QLabel* emptyText_ = nullptr;
    QLabel* name_ = nullptr;
    QLabel* address_ = nullptr;
    QVBoxLayout* facts_ = nullptr;
    QVBoxLayout* faults_ = nullptr;
    QPushButton* clear_ = nullptr;
    QHash<quint32, UdsEcuRow> rows_;
    QVector<quint32> order_;
    quint32 selected_ = 0;
};

}  // namespace caruspex
