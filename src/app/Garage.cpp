#include "app/Garage.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QSaveFile>
#include <QStandardPaths>

namespace caruspex {

namespace {

QJsonObject toJson(const GarageCheck& c) {
    QJsonObject o;
    o["when"] = c.when.toString(Qt::ISODate);
    o["score"] = c.score;
    o["mil"] = c.mil;
    o["openMonitors"] = c.openMonitors;
    o["odometerKm"] = c.odometerKm;
    o["codes"] = QJsonArray::fromStringList(c.codes);
    o["texts"] = QJsonArray::fromStringList(c.texts);
    return o;
}

GarageCheck checkFromJson(const QJsonObject& o) {
    GarageCheck c;
    c.when = QDateTime::fromString(o["when"].toString(), Qt::ISODate);
    c.score = o["score"].toInt(-1);
    c.mil = o["mil"].toBool();
    c.openMonitors = o["openMonitors"].toInt();
    c.odometerKm = o["odometerKm"].toDouble(-1.0);
    for (const QJsonValue& v : o["codes"].toArray()) c.codes << v.toString();
    for (const QJsonValue& v : o["texts"].toArray()) c.texts << v.toString();
    return c;
}

}  // namespace

Garage::Garage(QObject* parent) : QObject(parent), path_(defaultPath()) {}

QString Garage::defaultPath() {
    // Lets tests and screenshots use their own garage.
    const QString override = qEnvironmentVariable("CARUSPEX_GARAGE");
    if (!override.isEmpty()) return override;
    const QString dir = QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);
    return QDir(dir).filePath(QStringLiteral("garage.json"));
}

void Garage::setPath(const QString& path) { path_ = path; }

bool Garage::load() {
    cars_.clear();
    QFile file(path_);
    if (!file.exists()) return true;
    if (!file.open(QIODevice::ReadOnly)) return false;
    const QJsonDocument doc = QJsonDocument::fromJson(file.readAll());
    for (const QJsonValue& value : doc.object()["cars"].toArray()) {
        const QJsonObject o = value.toObject();
        GarageCar car;
        car.vin = o["vin"].toString();
        car.name = o["name"].toString();
        car.description = o["description"].toString();
        car.bodyStyle = o["bodyStyle"].toInt();
        car.note = o["note"].toString();
        car.firstSeen = QDateTime::fromString(o["firstSeen"].toString(), Qt::ISODate);
        car.lastSeen = QDateTime::fromString(o["lastSeen"].toString(), Qt::ISODate);
        for (const QJsonValue& c : o["checks"].toArray()) car.checks.push_back(checkFromJson(c.toObject()));
        if (!car.vin.isEmpty()) cars_.push_back(car);
    }
    return true;
}

bool Garage::save() const {
    QJsonArray cars;
    for (const GarageCar& car : cars_) {
        QJsonObject o;
        o["vin"] = car.vin;
        o["name"] = car.name;
        o["description"] = car.description;
        o["bodyStyle"] = car.bodyStyle;
        o["note"] = car.note;
        o["firstSeen"] = car.firstSeen.toString(Qt::ISODate);
        o["lastSeen"] = car.lastSeen.toString(Qt::ISODate);
        QJsonArray checks;
        for (const GarageCheck& c : car.checks) checks.append(toJson(c));
        o["checks"] = checks;
        cars.append(o);
    }
    QJsonObject root;
    root["version"] = 1;
    root["cars"] = cars;

    QDir().mkpath(QFileInfo(path_).absolutePath());
    // QSaveFile writes to a temporary file first, so a crash mid-write never
    // leaves a broken garage behind.
    QSaveFile file(path_);
    if (!file.open(QIODevice::WriteOnly)) return false;
    file.write(QJsonDocument(root).toJson(QJsonDocument::Indented));
    return file.commit();
}

const GarageCar* Garage::find(const QString& vin) const {
    for (const GarageCar& car : cars_) {
        if (car.vin == vin) return &car;
    }
    return nullptr;
}

GarageCar* Garage::findMutable(const QString& vin) {
    for (GarageCar& car : cars_) {
        if (car.vin == vin) return &car;
    }
    return nullptr;
}

void Garage::touch(const QString& vin, const QString& name, const QString& description,
                   int bodyStyle) {
    if (vin.isEmpty()) return;
    GarageCar* car = findMutable(vin);
    const QDateTime now = QDateTime::currentDateTime();
    if (!car) {
        GarageCar fresh;
        fresh.vin = vin;
        fresh.firstSeen = now;
        cars_.push_back(fresh);
        car = &cars_.back();
    }
    if (!name.isEmpty()) car->name = name;
    if (!description.isEmpty()) car->description = description;
    car->bodyStyle = bodyStyle;
    car->lastSeen = now;
    save();
    emit changed();
}

void Garage::addCheck(const QString& vin, const GarageCheck& check) {
    GarageCar* car = findMutable(vin);
    if (!car) return;
    if (!car->checks.isEmpty() && car->checks.back().when.secsTo(check.when) < 15 * 60) {
        car->checks.back() = check;
    } else {
        car->checks.push_back(check);
    }
    car->lastSeen = check.when;
    save();
    emit changed();
}

void Garage::setNote(const QString& vin, const QString& note) {
    if (GarageCar* car = findMutable(vin)) {
        car->note = note;
        save();
        emit changed();
    }
}

void Garage::remove(const QString& vin) {
    for (int i = 0; i < cars_.size(); ++i) {
        if (cars_[i].vin == vin) {
            cars_.removeAt(i);
            save();
            emit changed();
            return;
        }
    }
}

}  // namespace caruspex
