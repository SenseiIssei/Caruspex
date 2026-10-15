#pragma once

#include <QVector>
#include <QWidget>

#include "app/Session.h"

QT_BEGIN_NAMESPACE
class QLabel;
class QPushButton;
class QTreeWidget;
QT_END_NAMESPACE

namespace caruspex {

// Modes 06 and 05: what the control unit continuously checks by itself.
//
// This is the most detailed part of EOBD. Instead of just pass or fail, the
// control unit reports the measured value along with its limits. So you can
// see how close a component is to its limits long before the warning lamp
// comes on.
class MonitorPage : public QWidget {
    Q_OBJECT

public:
    explicit MonitorPage(QWidget* parent = nullptr);

public slots:
    void setConnected(bool connected);
    void onMonitorResults(const QVector<caruspex::MonitorRow>& rows, const QString& note);
    void onOxygenTests(const QVector<caruspex::OxygenRow>& rows, const QString& note);
    void onIdentity(const QString& vin, const QString& calibration,
                    const QString& checksum, const QString& ecuName);

signals:
    void readRequested();
    void exportRequested();

private:
    void exportReport();

    QPushButton* readButton_ = nullptr;
    QPushButton* exportButton_ = nullptr;
    QLabel* status_ = nullptr;
    QLabel* identity_ = nullptr;
    QTreeWidget* tree_ = nullptr;

    QVector<MonitorRow> monitors_;
    QVector<OxygenRow> oxygen_;
};

}  // namespace caruspex
