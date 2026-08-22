#pragma once

#include <QElapsedTimer>
#include <QHash>
#include <QWidget>

#include "app/Session.h"

QT_BEGIN_NAMESPACE
class QLabel;
class QPushButton;
class QTableWidget;
QT_END_NAMESPACE

namespace caruspex {

// Zahlen ueber die laufende Messung: Kleinstwert, Mittel, Groesstwert je
// Messgroesse, dazu abgeleitete Werte wie Verbrauch und Strecke.
class StatsPage : public QWidget {
    Q_OBJECT

public:
    explicit StatsPage(QWidget* parent = nullptr);

public slots:
    void onLiveValue(const caruspex::LiveRow& row);
    void setConnected(bool connected);
    void reset();

private:
    struct Series {
        QString name;
        QString unit;
        double minimum = 0.0;
        double maximum = 0.0;
        double sum = 0.0;
        double last = 0.0;
        long long count = 0;
        int row = -1;
    };

    void refreshCards();
    void exportCsv();

    QTableWidget* table_ = nullptr;
    QHash<int, Series> series_;

    QLabel* durationValue_ = nullptr;
    QLabel* samplesValue_ = nullptr;
    QLabel* maxRpmValue_ = nullptr;
    QLabel* maxSpeedValue_ = nullptr;
    QLabel* consumptionValue_ = nullptr;
    QLabel* distanceValue_ = nullptr;
    QPushButton* resetButton_ = nullptr;
    QPushButton* exportButton_ = nullptr;
    QLabel* note_ = nullptr;

    QElapsedTimer clock_;
    // Fuer Verbrauch und Strecke wird ueber die Zeit aufsummiert.
    double fuelLitres_ = 0.0;
    double distanceKm_ = 0.0;
    double lastMaf_ = 0.0;
    double lastSpeed_ = 0.0;
    qint64 lastMafMs_ = -1;
    qint64 lastSpeedMs_ = -1;
    bool sawMaf_ = false;
};

}  // namespace caruspex
