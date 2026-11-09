#include "app/VehicleIdentity.h"

#include <QCoreApplication>
#include <QDate>
#include <QStringList>

#include "core/DemoTransport.h"
#include "data/Vin.h"

namespace caruspex {

BodyStyle bodyStyleFor(BodyType type) {
    switch (type) {
        case BodyType::Hatchback: return BodyStyle::Hatchback;
        case BodyType::Sedan: return BodyStyle::Sedan;
        case BodyType::Estate: return BodyStyle::Estate;
        case BodyType::Suv: return BodyStyle::Suv;
        case BodyType::Van:
        case BodyType::Mpv: return BodyStyle::Van;
        case BodyType::Coupe:
        case BodyType::Convertible:
        case BodyType::Roadster: return BodyStyle::Coupe;
        case BodyType::Pickup: return BodyStyle::Pickup;
        case BodyType::Microcar: return BodyStyle::Microcar;
    }
    return BodyStyle::Hatchback;
}

IdentifiedVehicle identifyGeneration(const VehicleGeneration& g, int year) {
    IdentifiedVehicle v;
    v.known = true;
    v.generation = &g;
    v.year = year;
    v.confidence = 100;
    v.brand = QString::fromUtf8(g.brand);
    v.title = QString::fromStdString(catalog::displayName(g));
    v.body = bodyStyleFor(g.body);
    QStringList parts;
    if (year > 0) parts << QCoreApplication::translate("caruspex::VehicleIdentity", "Baujahr %1").arg(year);
    parts << QString::fromStdString(catalog::yearsText(g));
    const QString power = QString::fromStdString(powertrainText(g.powertrains));
    if (!power.isEmpty()) parts << power;
    parts << QString::fromUtf8(bodyName(g.body));
    v.subtitle = parts.join(QStringLiteral(" · "));
    v.socketHint = QString::fromStdString(locationText(g, year > 0 ? year : g.yearFrom));
    return v;
}

IdentifiedVehicle identifyVin(const QString& vinText) {
    const QString vin = vinText.trimmed().toUpper();
    IdentifiedVehicle v;
    if (vin.isEmpty()) return v;

    // The built-in demo car has a made-up maker code; give it a proper name.
    if (vin == QString::fromLatin1(DemoTransport::vin())) {
        v.known = true;
        v.title = QCoreApplication::translate("caruspex::VehicleIdentity", "Caruspex Demo-Auto");
        v.subtitle = QCoreApplication::translate("caruspex::VehicleIdentity", "Baujahr 2021 · 1.4 Benziner · Automatik · simuliert");
        v.year = 2021;
        v.confidence = 100;
        v.body = BodyStyle::Hatchback;
        return v;
    }

    const VinInfo info = decodeVin(vin.toStdString(), QDate::currentDate().year());
    const CatalogMatch match = catalog::matchVin(info);
    if (match.generation) {
        v = identifyGeneration(*match.generation, match.year);
        v.confidence = match.confidence;
        if (match.confidence < 60) {
            v.subtitle += QStringLiteral(" · ") + QCoreApplication::translate("caruspex::VehicleIdentity", "vermutet");
        }
        return v;
    }
    const QString brand = QString::fromStdString(!match.brand.empty() ? match.brand : info.brand);
    if (!brand.isEmpty()) {
        v.known = true;
        v.brand = brand;
        v.title = brand;
        v.year = match.year > 0 ? match.year : info.modelYear;
        QStringList parts;
        if (v.year > 0) parts << QCoreApplication::translate("caruspex::VehicleIdentity", "Baujahr etwa %1").arg(v.year);
        if (!info.country.empty()) parts << QString::fromStdString(info.country);
        parts << QCoreApplication::translate("caruspex::VehicleIdentity", "Modell nicht eindeutig");
        v.subtitle = parts.join(QStringLiteral(" · "));
        return v;
    }
    v.title = QCoreApplication::translate("caruspex::VehicleIdentity", "Unbekannter Hersteller");
    v.subtitle = QCoreApplication::translate("caruspex::VehicleIdentity", "Die Fahrgestellnummer passt zu keinem bekannten Hersteller.");
    return v;
}

}  // namespace caruspex
