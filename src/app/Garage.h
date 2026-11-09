// The garage: every car Caruspex has seen, recognised by its VIN, with the
// history of its checks. Kept as a small JSON file in the user's data folder,
// nothing leaves the computer.
#pragma once

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

struct GarageCar {
    QString vin;
    QString name;              // "Audi A2 (8Z)"
    QString description;       // engine, year, whatever is known
    int bodyStyle = 0;         // BodyStyle as int
    QString note;              // free text of the owner
    QDateTime firstSeen;
    QDateTime lastSeen;
    QVector<GarageCheck> checks;   // oldest first
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

signals:
    void changed();

private:
    GarageCar* findMutable(const QString& vin);

    QString path_;
    QVector<GarageCar> cars_;
};

}  // namespace caruspex
