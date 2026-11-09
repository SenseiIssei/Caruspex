#include "app/Garage.h"

#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QSaveFile>
#include <QStandardPaths>
#include <QUuid>

#include <algorithm>
#include <cmath>

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

QJsonObject toJson(const GarageTrip& t) {
    QJsonObject o;
    o["start"] = t.start.toString(Qt::ISODate);
    o["end"] = t.end.toString(Qt::ISODate);
    o["distanceKm"] = t.distanceKm;
    o["fuelLitres"] = t.fuelLitres;
    o["averageL100"] = t.averageL100;
    o["cost"] = t.cost;
    o["co2Kg"] = t.co2Kg;
    o["maxSpeedKmh"] = t.maxSpeedKmh;
    o["ecoScore"] = t.ecoScore;
    return o;
}

GarageTrip tripFromJson(const QJsonObject& o) {
    GarageTrip t;
    t.start = QDateTime::fromString(o["start"].toString(), Qt::ISODate);
    t.end = QDateTime::fromString(o["end"].toString(), Qt::ISODate);
    t.distanceKm = o["distanceKm"].toDouble();
    t.fuelLitres = o["fuelLitres"].toDouble();
    t.averageL100 = o["averageL100"].toDouble();
    t.cost = o["cost"].toDouble();
    t.co2Kg = o["co2Kg"].toDouble();
    t.maxSpeedKmh = o["maxSpeedKmh"].toDouble();
    t.ecoScore = o["ecoScore"].toInt(-1);
    return t;
}

QJsonObject toJson(const GarageFuel& f) {
    QJsonObject o;
    o["date"] = f.date.toString(Qt::ISODate);
    o["odometerKm"] = f.odometerKm;
    o["litres"] = f.litres;
    o["pricePerLitre"] = f.pricePerLitre;
    o["full"] = f.full;
    return o;
}

GarageFuel fuelFromJson(const QJsonObject& o) {
    GarageFuel f;
    f.date = QDate::fromString(o["date"].toString(), Qt::ISODate);
    f.odometerKm = o["odometerKm"].toDouble(-1.0);
    f.litres = o["litres"].toDouble();
    f.pricePerLitre = o["pricePerLitre"].toDouble();
    f.full = o["full"].toBool(true);
    return f;
}

QJsonObject toJson(const GarageService& s) {
    QJsonObject o;
    o["id"] = s.id;
    o["title"] = s.title;
    o["intervalKm"] = s.intervalKm;
    o["intervalMonths"] = s.intervalMonths;
    o["lastDate"] = s.lastDate.isValid() ? s.lastDate.toString(Qt::ISODate) : QString();
    o["lastKm"] = s.lastKm;
    o["note"] = s.note;
    return o;
}

GarageService serviceFromJson(const QJsonObject& o) {
    GarageService s;
    s.id = o["id"].toString();
    s.title = o["title"].toString();
    s.intervalKm = o["intervalKm"].toInt();
    s.intervalMonths = o["intervalMonths"].toInt();
    s.lastDate = QDate::fromString(o["lastDate"].toString(), Qt::ISODate);
    s.lastKm = o["lastKm"].toDouble(-1.0);
    s.note = o["note"].toString();
    if (s.id.isEmpty()) s.id = QUuid::createUuid().toString(QUuid::WithoutBraces);
    return s;
}

QJsonObject toJson(const GarageCar& car) {
    QJsonObject o;
    o["vin"] = car.vin;
    o["name"] = car.name;
    o["description"] = car.description;
    o["bodyStyle"] = car.bodyStyle;
    o["note"] = car.note;
    o["firstSeen"] = car.firstSeen.toString(Qt::ISODate);
    o["lastSeen"] = car.lastSeen.toString(Qt::ISODate);
    QJsonArray checks, trips, fuel, services;
    for (const GarageCheck& c : car.checks) checks.append(toJson(c));
    for (const GarageTrip& t : car.trips) trips.append(toJson(t));
    for (const GarageFuel& f : car.fuel) fuel.append(toJson(f));
    for (const GarageService& s : car.services) services.append(toJson(s));
    o["checks"] = checks;
    o["trips"] = trips;
    o["fuel"] = fuel;
    o["services"] = services;
    return o;
}

GarageCar carFromJson(const QJsonObject& o) {
    GarageCar car;
    car.vin = o["vin"].toString();
    car.name = o["name"].toString();
    car.description = o["description"].toString();
    car.bodyStyle = o["bodyStyle"].toInt();
    car.note = o["note"].toString();
    car.firstSeen = QDateTime::fromString(o["firstSeen"].toString(), Qt::ISODate);
    car.lastSeen = QDateTime::fromString(o["lastSeen"].toString(), Qt::ISODate);
    for (const QJsonValue& v : o["checks"].toArray()) car.checks.push_back(checkFromJson(v.toObject()));
    for (const QJsonValue& v : o["trips"].toArray()) car.trips.push_back(tripFromJson(v.toObject()));
    for (const QJsonValue& v : o["fuel"].toArray()) car.fuel.push_back(fuelFromJson(v.toObject()));
    for (const QJsonValue& v : o["services"].toArray()) car.services.push_back(serviceFromJson(v.toObject()));
    return car;
}

QByteArray serialise(const QVector<GarageCar>& cars) {
    QJsonArray list;
    for (const GarageCar& car : cars) list.append(toJson(car));
    QJsonObject root;
    root["version"] = 2;
    root["cars"] = list;
    return QJsonDocument(root).toJson(QJsonDocument::Indented);
}

QVector<GarageCar> parse(const QByteArray& data, QString* error) {
    QJsonParseError parseError;
    const QJsonDocument doc = QJsonDocument::fromJson(data, &parseError);
    QVector<GarageCar> cars;
    if (doc.isNull()) {
        if (error) *error = parseError.errorString();
        return cars;
    }
    for (const QJsonValue& value : doc.object()["cars"].toArray()) {
        GarageCar car = carFromJson(value.toObject());
        if (!car.vin.isEmpty()) cars.push_back(car);
    }
    return cars;
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
    cars_ = parse(file.readAll(), nullptr);
    return true;
}

bool Garage::save() const {
    QDir().mkpath(QFileInfo(path_).absolutePath());
    // QSaveFile writes to a temporary file first, so a crash mid-write never
    // leaves a broken garage behind.
    QSaveFile file(path_);
    if (!file.open(QIODevice::WriteOnly)) return false;
    file.write(serialise(cars_));
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
        // A new car starts with the items almost everybody needs.
        for (const GarageService& s : serviceTemplates()) {
            if (s.title == QCoreApplication::translate("caruspex::Garage", "Ölwechsel") || s.title == QCoreApplication::translate("caruspex::Garage", "Hauptuntersuchung (HU/AU)")) {
                fresh.services.push_back(s);
            }
        }
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

void Garage::addTrip(const QString& vin, const GarageTrip& trip) {
    if (GarageCar* car = findMutable(vin)) {
        car->trips.push_back(trip);
        save();
        emit changed();
    }
}

void Garage::addFuel(const QString& vin, const GarageFuel& fuel) {
    if (GarageCar* car = findMutable(vin)) {
        car->fuel.push_back(fuel);
        std::sort(car->fuel.begin(), car->fuel.end(), [](const GarageFuel& a, const GarageFuel& b) {
            if (a.date != b.date) return a.date < b.date;
            return a.odometerKm < b.odometerKm;
        });
        save();
        emit changed();
    }
}

void Garage::removeFuel(const QString& vin, int index) {
    GarageCar* car = findMutable(vin);
    if (!car || index < 0 || index >= car->fuel.size()) return;
    car->fuel.removeAt(index);
    save();
    emit changed();
}

void Garage::setService(const QString& vin, const GarageService& service) {
    GarageCar* car = findMutable(vin);
    if (!car) return;
    GarageService copy = service;
    if (copy.id.isEmpty()) copy.id = QUuid::createUuid().toString(QUuid::WithoutBraces);
    for (GarageService& s : car->services) {
        if (s.id == copy.id) {
            s = copy;
            save();
            emit changed();
            return;
        }
    }
    car->services.push_back(copy);
    save();
    emit changed();
}

void Garage::removeService(const QString& vin, const QString& id) {
    GarageCar* car = findMutable(vin);
    if (!car) return;
    for (int i = 0; i < car->services.size(); ++i) {
        if (car->services[i].id == id) {
            car->services.removeAt(i);
            save();
            emit changed();
            return;
        }
    }
}

void Garage::markServiceDone(const QString& vin, const QString& id, const QDate& date, double km) {
    GarageCar* car = findMutable(vin);
    if (!car) return;
    for (GarageService& s : car->services) {
        if (s.id == id) {
            s.lastDate = date;
            s.lastKm = km;
            save();
            emit changed();
            return;
        }
    }
}

double Garage::latestOdometer(const QString& vin) const {
    const GarageCar* car = find(vin);
    if (!car) return -1.0;
    double best = -1.0;
    for (const GarageCheck& c : car->checks) best = std::max(best, c.odometerKm);
    for (const GarageFuel& f : car->fuel) best = std::max(best, f.odometerKm);
    return best;
}

QVector<ServiceStatus> Garage::serviceStatus(const GarageCar& car, double currentKm, const QDate& today) {
    QVector<ServiceStatus> out;
    for (const GarageService& s : car.services) {
        ServiceStatus st;
        st.service = &s;
        bool haveKm = false;
        bool haveDays = false;
        if (s.intervalKm > 0 && s.lastKm >= 0 && currentKm >= 0) {
            st.kmLeft = static_cast<int>(std::lround(s.lastKm + s.intervalKm - currentKm));
            haveKm = true;
        }
        if (s.intervalMonths > 0 && s.lastDate.isValid()) {
            st.daysLeft = static_cast<int>(today.daysTo(s.lastDate.addMonths(s.intervalMonths)));
            haveDays = true;
        }
        if (!haveKm && !haveDays) {
            st.state = ServiceStatus::State::Unknown;
            st.text = s.lastDate.isValid() || s.lastKm >= 0
                          ? QCoreApplication::translate("caruspex::Garage", "Kilometerstand unbekannt, beim nächsten Verbinden berechnet")
                          : QCoreApplication::translate("caruspex::Garage", "Noch nicht eingetragen, wann es zuletzt gemacht wurde");
            out.push_back(st);
            continue;
        }
        // Whichever limit comes first decides. A kilometre is roughly worth
        // a day here; good enough to pick the closer one.
        st.byKm = haveKm && (!haveDays || st.kmLeft / 40 < st.daysLeft);
        const bool over = (haveKm && st.kmLeft < 0) || (haveDays && st.daysLeft < 0);
        const bool soon = (haveKm && st.kmLeft < 1500) || (haveDays && st.daysLeft < 30);
        st.state = over ? ServiceStatus::State::Due
                   : soon ? ServiceStatus::State::Soon
                          : ServiceStatus::State::Ok;
        if (st.byKm) {
            st.text = st.kmLeft >= 0 ? QCoreApplication::translate("caruspex::Garage", "noch %1 km").arg(st.kmLeft)
                                     : QCoreApplication::translate("caruspex::Garage", "seit %1 km überfällig").arg(-st.kmLeft);
        } else if (st.daysLeft >= 60) {
            const int months = st.daysLeft / 30;
            st.text = QCoreApplication::translate("caruspex::Garage", "noch etwa %n Monat(e)", nullptr,
                                                  months);
        } else if (st.daysLeft >= 0) {
            st.text = QCoreApplication::translate("caruspex::Garage", "noch %n Tag(e)", nullptr, st.daysLeft);
        } else {
            st.text = QCoreApplication::translate("caruspex::Garage", "seit %n Tag(en) überfällig", nullptr, -st.daysLeft);
        }
        out.push_back(st);
    }
    // Most urgent first.
    std::stable_sort(out.begin(), out.end(), [](const ServiceStatus& a, const ServiceStatus& b) {
        auto rank = [](ServiceStatus::State s) {
            switch (s) {
                case ServiceStatus::State::Due: return 0;
                case ServiceStatus::State::Soon: return 1;
                case ServiceStatus::State::Ok: return 2;
                case ServiceStatus::State::Unknown: return 3;
            }
            return 3;
        };
        return rank(a.state) < rank(b.state);
    });
    return out;
}

QVector<OdometerJump> Garage::odometerJumps(const GarageCar& car) {
    QVector<OdometerJump> jumps;
    const GarageCheck* previous = nullptr;
    for (const GarageCheck& c : car.checks) {
        if (c.odometerKm < 0) continue;
        // More than a kilometre back cannot be rounding.
        if (previous && c.odometerKm + 1.0 < previous->odometerKm) {
            jumps.push_back({previous->when, previous->odometerKm, c.when, c.odometerKm});
        }
        previous = &c;
    }
    return jumps;
}

double Garage::fuelLogConsumption(const GarageCar& car) {
    // Classic full-tank method: from the first full tank, add every litre
    // until the last full tank and divide by the distance between them.
    int first = -1;
    int last = -1;
    for (int i = 0; i < car.fuel.size(); ++i) {
        if (!car.fuel[i].full || car.fuel[i].odometerKm < 0) continue;
        if (first < 0) first = i;
        last = i;
    }
    if (first < 0 || last <= first) return -1.0;
    const double distance = car.fuel[last].odometerKm - car.fuel[first].odometerKm;
    if (distance < 50.0) return -1.0;
    double litres = 0.0;
    for (int i = first + 1; i <= last; ++i) litres += car.fuel[i].litres;
    return litres / distance * 100.0;
}

QVector<GarageService> Garage::serviceTemplates() {
    // Common intervals; the owner's manual knows the exact ones.
    auto make = [](const QString& title, int km, int months) {
        GarageService s;
        s.title = title;
        s.intervalKm = km;
        s.intervalMonths = months;
        return s;
    };
    return {
        make(QCoreApplication::translate("caruspex::Garage", "Ölwechsel"), 15000, 12),
        make(QCoreApplication::translate("caruspex::Garage", "Inspektion"), 30000, 24),
        make(QCoreApplication::translate("caruspex::Garage", "Hauptuntersuchung (HU/AU)"), 0, 24),
        make(QCoreApplication::translate("caruspex::Garage", "Zahnriemen"), 120000, 72),
        make(QCoreApplication::translate("caruspex::Garage", "Bremsflüssigkeit"), 0, 24),
        make(QCoreApplication::translate("caruspex::Garage", "Innenraumfilter"), 15000, 12),
        make(QCoreApplication::translate("caruspex::Garage", "Luftfilter"), 30000, 24),
        make(QCoreApplication::translate("caruspex::Garage", "Zündkerzen"), 60000, 48),
        make(QCoreApplication::translate("caruspex::Garage", "Kühlmittel"), 0, 60),
        make(QCoreApplication::translate("caruspex::Garage", "Reifen wechseln (Sommer/Winter)"), 0, 6),
    };
}

bool Garage::exportTo(const QString& path) const {
    QSaveFile file(path);
    if (!file.open(QIODevice::WriteOnly)) return false;
    file.write(serialise(cars_));
    return file.commit();
}

int Garage::importFrom(const QString& path, QString* error) {
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        if (error) *error = file.errorString();
        return -1;
    }
    QString parseError;
    const QVector<GarageCar> incoming = parse(file.readAll(), &parseError);
    if (!parseError.isEmpty()) {
        if (error) *error = parseError;
        return -1;
    }
    int count = 0;
    for (const GarageCar& car : incoming) {
        GarageCar* mine = findMutable(car.vin);
        if (!mine) {
            cars_.push_back(car);
            ++count;
            continue;
        }
        // Add what is not there yet, matched by time stamp.
        auto addChecks = [&] {
            for (const GarageCheck& c : car.checks) {
                const bool known = std::any_of(mine->checks.begin(), mine->checks.end(),
                                               [&](const GarageCheck& m) { return m.when == c.when; });
                if (!known) mine->checks.push_back(c);
            }
            std::sort(mine->checks.begin(), mine->checks.end(),
                      [](const GarageCheck& a, const GarageCheck& b) { return a.when < b.when; });
        };
        addChecks();
        for (const GarageTrip& t : car.trips) {
            const bool known = std::any_of(mine->trips.begin(), mine->trips.end(),
                                           [&](const GarageTrip& m) { return m.start == t.start; });
            if (!known) mine->trips.push_back(t);
        }
        for (const GarageFuel& f : car.fuel) {
            const bool known = std::any_of(mine->fuel.begin(), mine->fuel.end(), [&](const GarageFuel& m) {
                return m.date == f.date && std::abs(m.odometerKm - f.odometerKm) < 0.5;
            });
            if (!known) mine->fuel.push_back(f);
        }
        for (const GarageService& s : car.services) {
            const bool known = std::any_of(mine->services.begin(), mine->services.end(),
                                           [&](const GarageService& m) { return m.id == s.id; });
            if (!known) mine->services.push_back(s);
        }
        ++count;
    }
    save();
    emit changed();
    return count;
}

}  // namespace caruspex
