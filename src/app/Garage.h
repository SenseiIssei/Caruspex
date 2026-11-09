// The garage: every car Caruspex has seen, recognised by its VIN, with the
// history of its checks, its trips, a fuel log and its service plan. Kept as
// a small JSON file in the user's data folder, nothing leaves the computer.
#pragma once

#include <QDate>
#include <QDateTime>
#include <QObject>
#include <QString>
#include <QStringList>
#include <QVector>

namespace caruspex {

struct GarageCheck {
    QDateTime when;
    int score = -1;            // health score, -1 if not rated
    bool mil = false;
    int openMonitors = 0;
    double odometerKm = -1.0;  // -1 when the car does not report it
    QStringList codes;         // "P0301"
    QStringList texts;         // plain text per code, same order
};

// One drive recorded by the trip computer.
struct GarageTrip {
    QDateTime start;
    QDateTime end;
    double distanceKm = 0.0;
    double fuelLitres = 0.0;
    double averageL100 = 0.0;
    double cost = 0.0;
    double co2Kg = 0.0;
    double maxSpeedKmh = 0.0;
    int ecoScore = -1;
};

// One refuelling, entered by hand.
struct GarageFuel {
    QDate date;
    double odometerKm = -1.0;
    double litres = 0.0;
    double pricePerLitre = 0.0;
    bool full = true;          // filled to the brim, needed to work out consumption
};

// A service item that comes back after a distance, a time, or whichever is
// first (oil change, inspection, emissions test, timing belt ...).
struct GarageService {
    QString id;                // stable, for editing
    QString title;
    int intervalKm = 0;        // 0 = not by distance
    int intervalMonths = 0;    // 0 = not by time
    QDate lastDate;            // invalid = never recorded
    double lastKm = -1.0;
    QString note;
};

struct GarageCar {
    QString vin;
    QString name;              // "Audi A2 (8Z)"
    QString description;       // engine, year, whatever is known
    int bodyStyle = 0;         // BodyStyle as int
    QString note;              // free text of the owner
    QDateTime firstSeen;
    QDateTime lastSeen;
    QVector<GarageCheck> checks;   // oldest first
    QVector<GarageTrip> trips;     // oldest first
    QVector<GarageFuel> fuel;      // oldest first
    QVector<GarageService> services;
};

// How a service item stands today.
struct ServiceStatus {
    const GarageService* service = nullptr;
    enum class State { Ok, Soon, Due, Unknown } state = State::Unknown;
    int kmLeft = 0;            // negative when overdue
    int daysLeft = 0;          // negative when overdue
    bool byKm = false;         // the distance limit is the closer one
    QString text;              // "noch 800 km" / "seit 3 Wochen überfällig"
};

// A drop of the odometer between two checks: a hint, not proof.
struct OdometerJump {
    QDateTime earlier;
    double earlierKm = 0.0;
    QDateTime later;
    double laterKm = 0.0;
};

class Garage : public QObject {
    Q_OBJECT

public:
    explicit Garage(QObject* parent = nullptr);

    // Where the file lives. Overridable for tests.
    static QString defaultPath();
    void setPath(const QString& path);

    bool load();
    bool save() const;

    const QVector<GarageCar>& cars() const { return cars_; }
    const GarageCar* find(const QString& vin) const;

    // Creates or updates the entry and marks it as seen now.
    void touch(const QString& vin, const QString& name, const QString& description, int bodyStyle);
    // Adds a check. A second check within fifteen minutes replaces the
    // first, so reading the faults three times in a row stays one entry.
    void addCheck(const QString& vin, const GarageCheck& check);
    void setNote(const QString& vin, const QString& note);
    void remove(const QString& vin);

    void addTrip(const QString& vin, const GarageTrip& trip);
    void addFuel(const QString& vin, const GarageFuel& fuel);
    void removeFuel(const QString& vin, int index);
    // Adds or replaces (same id) a service item.
    void setService(const QString& vin, const GarageService& service);
    void removeService(const QString& vin, const QString& id);
    void markServiceDone(const QString& vin, const QString& id, const QDate& date, double km);

    // Latest known odometer reading of a car: last check or fuel entry.
    double latestOdometer(const QString& vin) const;
    // Every service item with its state. currentKm < 0 means unknown.
    static QVector<ServiceStatus> serviceStatus(const GarageCar& car, double currentKm,
                                                const QDate& today);
    // Odometer readings that went down between two checks.
    static QVector<OdometerJump> odometerJumps(const GarageCar& car);
    // Consumption between full tanks: litres per 100 km, -1 if not enough data.
    static double fuelLogConsumption(const GarageCar& car);

    // Templates for the "add service" dialog.
    static QVector<GarageService> serviceTemplates();

    // Whole garage as a JSON file, and back. Import merges by VIN; for a car
    // that exists already, entries are added, nothing is removed.
    bool exportTo(const QString& path) const;
    int importFrom(const QString& path, QString* error = nullptr);

signals:
    void changed();

private:
    GarageCar* findMutable(const QString& vin);

    QString path_;
    QVector<GarageCar> cars_;
};

}  // namespace caruspex
