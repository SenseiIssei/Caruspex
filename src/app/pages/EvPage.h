// The traction battery of electric cars and hybrids, from the manufacturer
// values of the built-in packs: health, charge, cells, temperatures.
#pragma once

#include <QHash>
#include <QWidget>

#include "app/Session.h"

class QComboBox;
class QLabel;
class QPushButton;
class QTableWidget;

namespace caruspex {

class ScoreRing;
class StatTile;

class EvPage : public QWidget {
    Q_OBJECT

public:
    explicit EvPage(QWidget* parent = nullptr);

    // Preselects the pack that fits the connected car.
    void setVehicle(const QString& brand, const QString& title);
    // Same as pressing the read button (used by --page ev-run).
    void readNow();

public slots:
    void setConnected(bool connected);
    void onLiveValue(const caruspex::LiveRow& row);
    void onCustomRead(int ok, int failed, const QString& firstError);

signals:
    void readRequested(const QVector<int>& keys);
    void pollRequested(const QVector<int>& keys);
    void stopRequested();

private:
    QVector<int> packKeys() const;
    void rebuildTable();
    void refreshTiles();

    bool connected_ = false;
    bool running_ = false;
    QComboBox* pack_ = nullptr;
    QPushButton* read_ = nullptr;
    QPushButton* live_ = nullptr;
    QLabel* status_ = nullptr;
    ScoreRing* health_ = nullptr;
    QLabel* charge_ = nullptr;
    QLabel* chargeNote_ = nullptr;
    StatTile* voltage_ = nullptr;
    StatTile* current_ = nullptr;
    StatTile* power_ = nullptr;
    StatTile* cells_ = nullptr;
    StatTile* spread_ = nullptr;
    StatTile* temperature_ = nullptr;
    StatTile* aux_ = nullptr;
    QLabel* advice_ = nullptr;
    QTableWidget* table_ = nullptr;
    QHash<int, QString> shortNames_;   // key -> ShortName of the pack
    QHash<int, int> rowOf_;
    QHash<QString, double> values_;    // ShortName -> last value
};

}  // namespace caruspex
