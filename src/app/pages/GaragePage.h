// The garage: all cars seen so far, each with its file: checks, service plan,
// trips and fuel log.
#pragma once

#include <QWidget>

class QLabel;
class QPlainTextEdit;
class QPushButton;
class QTabWidget;
class QVBoxLayout;

namespace caruspex {

class Garage;
class StatTile;
class VehicleDiagram;

class GaragePage : public QWidget {
    Q_OBJECT

public:
    GaragePage(Garage* garage, QWidget* parent = nullptr);

    // Highlights the car that is connected right now.
    void setCurrentVin(const QString& vin);
    // Odometer of the connected car, for the service plan.
    void setCurrentOdometer(double km);
    // Opens one of the tabs (0 history, 1 service, 2 trips, 3 fuel, 4 note).
    void showTab(int index);

signals:
    void reportRequested();

private:
    void rebuildList();
    void showCar(const QString& vin);
    void fillHistory(const struct GarageCar& car);
    void fillServices(const struct GarageCar& car);
    void fillTrips(const struct GarageCar& car);
    void fillFuel(const struct GarageCar& car);
    void addService();
    void addFuel();

    Garage* garage_ = nullptr;
    QString selected_;
    QString current_;
    double currentKm_ = -1.0;
    QVBoxLayout* list_ = nullptr;
    QWidget* detail_ = nullptr;
    QWidget* empty_ = nullptr;
    VehicleDiagram* picture_ = nullptr;
    QLabel* name_ = nullptr;
    QLabel* meta_ = nullptr;
    QTabWidget* tabs_ = nullptr;
    QVBoxLayout* history_ = nullptr;
    QVBoxLayout* services_ = nullptr;
    QVBoxLayout* trips_ = nullptr;
    QVBoxLayout* fuel_ = nullptr;
    StatTile* tripDistance_ = nullptr;
    StatTile* tripConsumption_ = nullptr;
    StatTile* tripCost_ = nullptr;
    StatTile* fuelConsumption_ = nullptr;
    StatTile* fuelCost_ = nullptr;
    QPlainTextEdit* note_ = nullptr;
    QPushButton* remove_ = nullptr;
    QPushButton* report_ = nullptr;
};

}  // namespace caruspex
