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

// Modus 06 und 05: was das Steuergeraet laufend selbst prueft.
//
// Das ist der ausfuehrlichste Teil von EOBD. Statt nur bestanden oder nicht
// nennt das Steuergeraet den Messwert samt Grenzen. Man sieht also, wie knapp
// ein Bauteil noch im Rahmen liegt, lange bevor die Fehlerlampe angeht.
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
