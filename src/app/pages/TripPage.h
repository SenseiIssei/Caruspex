// Trip computer: consumption live and on average, distance, cost, CO2 and a
// gentle score for the driving style. Fuel comes from the engine's fuel rate,
// the air mass, or a speed-density estimate, whichever the car offers.
#pragma once

#include <QDateTime>
#include <QSet>
#include <QWidget>

#include <memory>

#include "app/Garage.h"
#include "app/Session.h"

class QComboBox;
class QDoubleSpinBox;
class QLabel;
class QPushButton;
class QTimer;

namespace caruspex {

namespace analysis {
class TripComputer;
}
class ScoreRing;
class StatTile;

class TripPage : public QWidget {
    Q_OBJECT

public:
    explicit TripPage(QWidget* parent = nullptr);
    ~TripPage() override;

public slots:
    void setConnected(bool connected);
    void onSupportedPids(const QVector<int>& pids);
    void onLiveValue(const caruspex::LiveRow& row);

signals:
    void startRequested(const QVector<int>& pids);
    void stopRequested();
    // A trip worth keeping has ended (reset or disconnect).
    void tripCompleted(const caruspex::GarageTrip& trip);

private:
    void toggle();
    void resetTrip();
    void refresh();
    void rebuildComputer();
    void handOverTrip();

    bool connected_ = false;
    bool running_ = false;
    QDateTime tripStart_;
    QSet<int> supported_;
    std::unique_ptr<analysis::TripComputer> trip_;

    QLabel* liveValue_ = nullptr;
    QLabel* liveUnit_ = nullptr;
    QLabel* liveCaption_ = nullptr;
    QLabel* fuelNote_ = nullptr;
    ScoreRing* eco_ = nullptr;
    QLabel* ecoText_ = nullptr;
    StatTile* distance_ = nullptr;
    StatTile* average_ = nullptr;
    StatTile* fuel_ = nullptr;
    StatTile* cost_ = nullptr;
    StatTile* co2_ = nullptr;
    StatTile* speed_ = nullptr;
    StatTile* maxSpeed_ = nullptr;
    StatTile* idle_ = nullptr;
    QPushButton* start_ = nullptr;
    QPushButton* reset_ = nullptr;
    QComboBox* fuelType_ = nullptr;
    QDoubleSpinBox* price_ = nullptr;
    QDoubleSpinBox* displacement_ = nullptr;
    QTimer* refresh_ = nullptr;
};

}  // namespace caruspex
