#include "app/Report.h"

#include <QCoreApplication>
#include <QDateTime>
#include <QDir>
#include <QFileDialog>
#include <QLocale>

#include <algorithm>
#include <QPageLayout>
#include <QPageSize>
#include <QPdfWriter>
#include <QPrintDialog>
#include <QPrinter>
#include <QStandardPaths>
#include <QTextDocument>

#include "app/Garage.h"
#include "app/VehicleState.h"
#include "data/DtcLocation.h"

namespace caruspex {

namespace {

QString esc(const QString& s) { return s.toHtmlEscaped(); }

QString scoreColour(int score) {
    if (score < 0) return QStringLiteral("#6b7280");
    if (score >= 85) return QStringLiteral("#12875a");
    if (score >= 60) return QStringLiteral("#b97a06");
    return QStringLiteral("#c93636");
}

}  // namespace

QString reportHtml(const VehicleState& state, const ReportExtras& extras) {
    QString h;
    h += QStringLiteral(
        "<html><head><style>"
        "body { font-family: 'Inter', 'Segoe UI', sans-serif; color: #141821; font-size: 10pt; }"
        "h1 { font-size: 20pt; margin: 0; color: #141821; }"
        "h2 { font-size: 12.5pt; margin-top: 18px; margin-bottom: 6px; color: #3b3fb8; }"
        ".muted { color: #515b6b; }"
        ".code { font-family: 'JetBrains Mono', Consolas, monospace; font-weight: bold; }"
        "table { border-collapse: collapse; width: 100%; }"
        "th { text-align: left; background: #eef0f6; color: #515b6b; padding: 5px 6px; "
        "font-size: 9pt; }"
        "td { padding: 5px 6px; border-bottom: 1px solid #e2e5ee; vertical-align: top; }"
        "</style></head><body>");

    // Header
    h += QStringLiteral("<table><tr><td style='border:none'>");
    h += QStringLiteral("<div style='color:#5a54f0; font-weight:bold; letter-spacing:2px; "
                        "font-size:9pt'>CARUSPEX</div>");
    h += QStringLiteral("<h1>%1</h1>").arg(esc(QCoreApplication::translate("caruspex::Report", "Prüfbericht")));
    h += QStringLiteral("<div class='muted'>%1</div>")
             .arg(esc(QDateTime::currentDateTime().toString(QStringLiteral("dd.MM.yyyy, HH:mm"))));
    h += QStringLiteral("</td><td style='border:none; text-align:right; width:30%'>");
    if (extras.score >= 0) {
        h += QStringLiteral("<div style='font-size:30pt; font-weight:bold; color:%1'>%2</div>"
                            "<div class='muted'>%3</div>")
                 .arg(scoreColour(extras.score))
                 .arg(extras.score)
                 .arg(esc(extras.scoreCaption));
    }
    h += QStringLiteral("</td></tr></table>");

    // Vehicle
    h += QStringLiteral("<h2>%1</h2><table>").arg(esc(QCoreApplication::translate("caruspex::Report", "Fahrzeug")));
    auto row = [&h](const QString& k, const QString& v) {
        if (v.isEmpty()) return;
        h += QStringLiteral("<tr><td style='width:32%' class='muted'>%1</td><td>%2</td></tr>")
                 .arg(esc(k), esc(v));
    };
    row(QCoreApplication::translate("caruspex::Report", "Bezeichnung"), state.displayName());
    row(QCoreApplication::translate("caruspex::Report", "Beschreibung"), state.description());
    row(QCoreApplication::translate("caruspex::Report", "Fahrgestellnummer"), state.vin());
    row(QCoreApplication::translate("caruspex::Report", "Steuergerät"), state.ecuName());
    row(QCoreApplication::translate("caruspex::Report", "Kalibrierung"), state.calibration());
    if (state.hasLive(0xA6)) {
        row(QCoreApplication::translate("caruspex::Report", "Kilometerstand"), QStringLiteral("%1 km").arg(state.liveValue(0xA6), 0, 'f', 0));
    }
    row(QCoreApplication::translate("caruspex::Report", "Adapter"), state.adapter());
    row(QCoreApplication::translate("caruspex::Report", "Verbindung"), state.transport());
    row(QCoreApplication::translate("caruspex::Report", "Protokoll"), state.protocol());
    h += QStringLiteral("</table>");

    // Findings
    if (!extras.findings.isEmpty()) {
        h += QStringLiteral("<h2>%1</h2><table><tr><th>%2</th><th>%3</th></tr>")
                 .arg(esc(QCoreApplication::translate("caruspex::Report", "Befunde")), esc(QCoreApplication::translate("caruspex::Report", "Stufe")), esc(QCoreApplication::translate("caruspex::Report", "Befund")));
        for (const ReportFinding& f : extras.findings) {
            h += QStringLiteral("<tr><td style='width:16%'>%1</td><td><b>%2</b><br/>"
                                "<span class='muted'>%3</span></td></tr>")
                     .arg(esc(f.severity), esc(f.title), esc(f.text));
        }
        h += QStringLiteral("</table>");
    }

    // Faults
    h += QStringLiteral("<h2>%1</h2>").arg(esc(QCoreApplication::translate("caruspex::Report", "Fehlerspeicher")));
    if (!state.haveFaults()) {
        h += QStringLiteral("<p class='muted'>%1</p>").arg(esc(QCoreApplication::translate("caruspex::Report", "Nicht ausgelesen.")));
    } else if (state.faults().isEmpty()) {
        h += QStringLiteral("<p>%1</p>").arg(esc(QCoreApplication::translate("caruspex::Report", "Kein Fehler gespeichert.")));
    } else {
        h += QStringLiteral("<table><tr><th>%1</th><th>%2</th><th>%3</th><th>%4</th></tr>")
                 .arg(esc(QCoreApplication::translate("caruspex::Report", "Code")), esc(QCoreApplication::translate("caruspex::Report", "Art")), esc(QCoreApplication::translate("caruspex::Report", "Bedeutung")), esc(QCoreApplication::translate("caruspex::Report", "Wo")));
        for (const FaultRow& f : state.faults()) {
            const ZoneInfo where = locateDtc(f.code.toStdString());
            h += QStringLiteral("<tr><td class='code'>%1</td><td>%2</td><td>%3</td>"
                                "<td class='muted'>%4</td></tr>")
                     .arg(esc(f.code), esc(f.kind), esc(f.text),
                          esc(QString::fromStdString(where.name)));
        }
        h += QStringLiteral("</table>");
    }
    if (state.haveStatus()) {
        h += QStringLiteral("<p>%1: <b>%2</b></p>")
                 .arg(esc(QCoreApplication::translate("caruspex::Report", "Motorkontrollleuchte")), esc(state.mil() ? QCoreApplication::translate("caruspex::Report", "an") : QCoreApplication::translate("caruspex::Report", "aus")));
    }

    // All control units over UDS
    if (!extras.ecus.isEmpty()) {
        h += QStringLiteral("<h2>%1</h2><table><tr><th>%2</th><th>%3</th><th>%4</th></tr>")
                 .arg(esc(QCoreApplication::translate("caruspex::Report", "Alle Steuergeräte")),
                      esc(QCoreApplication::translate("caruspex::Report", "Steuergerät")),
                      esc(QCoreApplication::translate("caruspex::Report", "Teilenummer")),
                      esc(QCoreApplication::translate("caruspex::Report", "Fehler")));
        for (const UdsEcuRow& e : extras.ecus) {
            QStringList codes;
            for (const UdsDtcRow& d : e.dtcs) {
                codes << QStringLiteral("<span class='code'>%1</span> %2").arg(esc(d.code), esc(d.text));
            }
            h += QStringLiteral("<tr><td>%1</td><td class='muted'>%2</td><td>%3</td></tr>")
                     .arg(esc(e.name), esc(e.partNumber),
                          codes.isEmpty() ? esc(QCoreApplication::translate("caruspex::Report", "keine"))
                                          : codes.join(QStringLiteral("<br/>")));
        }
        h += QStringLiteral("</table>");
    }

    // Readiness
    const QVector<ReadinessRow> readiness = state.readiness();
    if (!readiness.isEmpty()) {
        h += QStringLiteral("<h2>%1</h2><table><tr><th>%2</th><th>%3</th></tr>")
                 .arg(esc(QCoreApplication::translate("caruspex::Report", "Bereitschaftstests")), esc(QCoreApplication::translate("caruspex::Report", "Test")), esc(QCoreApplication::translate("caruspex::Report", "Stand")));
        for (const ReadinessRow& r : readiness) {
            h += QStringLiteral("<tr><td>%1</td><td style='color:%2'>%3</td></tr>")
                     .arg(esc(r.name), r.ready ? QStringLiteral("#12875a") : QStringLiteral("#b97a06"),
                          esc(r.ready ? QCoreApplication::translate("caruspex::Report", "abgeschlossen") : QCoreApplication::translate("caruspex::Report", "offen")));
        }
        h += QStringLiteral("</table>");
    }

    // Freeze frame
    if (!state.freezeFrame().isEmpty()) {
        h += QStringLiteral("<h2>%1</h2>").arg(esc(QCoreApplication::translate("caruspex::Report", "Umgebungsdaten zum Fehler")));
        if (!state.freezeCode().isEmpty()) {
            h += QStringLiteral("<p class='muted'>%1 <span class='code'>%2</span></p>")
                     .arg(esc(QCoreApplication::translate("caruspex::Report", "Ausgelöst durch")), esc(state.freezeCode()));
        }
        h += QStringLiteral("<table>");
        for (const LiveRow& r : state.freezeFrame()) {
            const QString value = r.text.isEmpty()
                                      ? QStringLiteral("%1 %2").arg(r.value, 0, 'f', 1).arg(r.unit)
                                      : r.text;
            h += QStringLiteral("<tr><td style='width:50%'>%1</td><td>%2</td></tr>")
                     .arg(esc(r.name), esc(value));
        }
        h += QStringLiteral("</table>");
    }

    // Component monitoring
    if (!state.monitors().isEmpty()) {
        h += QStringLiteral("<h2>%1</h2><table><tr><th>%2</th><th>%3</th><th>%4</th><th>%5</th></tr>")
                 .arg(esc(QCoreApplication::translate("caruspex::Report", "Bauteilprüfung")), esc(QCoreApplication::translate("caruspex::Report", "Prüfung")), esc(QCoreApplication::translate("caruspex::Report", "Wert")),
                      esc(QCoreApplication::translate("caruspex::Report", "Grenzen")), esc(QCoreApplication::translate("caruspex::Report", "Ergebnis")));
        for (const MonitorRow& m : state.monitors()) {
            QString limits;
            if (m.hasMinimum) limits += QStringLiteral("≥ %1").arg(m.minimum, 0, 'g', 5);
            if (m.hasMaximum) {
                if (!limits.isEmpty()) limits += QStringLiteral("  ");
                limits += QStringLiteral("≤ %1").arg(m.maximum, 0, 'g', 5);
            }
            h += QStringLiteral("<tr><td>%1<br/><span class='muted'>%2</span></td><td>%3 %4</td>"
                                "<td>%5</td><td style='color:%6'>%7</td></tr>")
                     .arg(esc(m.name), esc(m.component))
                     .arg(m.value, 0, 'g', 5)
                     .arg(esc(m.unit), esc(limits),
                          m.passed ? QStringLiteral("#12875a") : QStringLiteral("#c93636"),
                          esc(m.passed ? QCoreApplication::translate("caruspex::Report", "bestanden") : QCoreApplication::translate("caruspex::Report", "nicht bestanden")));
        }
        h += QStringLiteral("</table>");
    }

    h += QStringLiteral("<p class='muted' style='margin-top:22px; font-size:8.5pt'>%1</p>")
             .arg(esc(QCoreApplication::translate("caruspex::Report", "Erstellt mit Caruspex, freie Fahrzeugdiagnose. Der Bericht gibt "
                         "wieder, was die Steuergeräte gemeldet haben. Er ersetzt keine "
                         "Prüfung in der Werkstatt.")));
    h += QStringLiteral("</body></html>");
    return h;
}

QString saveReportPdf(QWidget* parent, const VehicleState& state, const ReportExtras& extras) {
    const QString base = state.vin().isEmpty() ? QStringLiteral("Caruspex") : state.vin();
    const QString suggested = QDir(QStandardPaths::writableLocation(QStandardPaths::DocumentsLocation))
                                  .filePath(QStringLiteral("%1_%2.pdf")
                                                .arg(base, QDate::currentDate().toString(Qt::ISODate)));
    const QString path = QFileDialog::getSaveFileName(parent, QCoreApplication::translate("caruspex::Report", "Prüfbericht speichern"), suggested,
                                                      QCoreApplication::translate("caruspex::Report", "PDF-Dokument (*.pdf)"));
    if (path.isEmpty()) return {};

    QPdfWriter writer(path);
    writer.setPageSize(QPageSize(QPageSize::A4));
    writer.setPageMargins(QMarginsF(16, 16, 16, 16), QPageLayout::Millimeter);
    writer.setTitle(QCoreApplication::translate("caruspex::Report", "Prüfbericht"));
    writer.setCreator(QStringLiteral("Caruspex"));
    QTextDocument doc;
    doc.setHtml(reportHtml(state, extras));
    doc.print(&writer);
    return path;
}

QString garageHtml(const GarageCar& car) {
    const QLocale locale;
    QString h = QStringLiteral(
        "<html><head><style>"
        "body { font-family: 'Inter', 'Segoe UI', sans-serif; color: #141821; font-size: 10pt; }"
        "h1 { font-size: 20pt; margin: 0; } h2 { font-size: 12.5pt; margin-top: 18px; color: #3b3fb8; }"
        ".muted { color: #515b6b; } .code { font-family: 'JetBrains Mono', Consolas, monospace; font-weight: bold; }"
        "table { border-collapse: collapse; width: 100%; }"
        "th { text-align: left; background: #eef0f6; color: #515b6b; padding: 5px 6px; font-size: 9pt; }"
        "td { padding: 5px 6px; border-bottom: 1px solid #e2e5ee; vertical-align: top; }"
        "</style></head><body>");
    h += QStringLiteral("<div style='color:#5a54f0; font-weight:bold; letter-spacing:2px; font-size:9pt'>CARUSPEX</div>");
    h += QStringLiteral("<h1>%1</h1>").arg(esc(QCoreApplication::translate("caruspex::Report", "Fahrzeugakte")));
    h += QStringLiteral("<p><b>%1</b><br/><span class='muted'>%2<br/>VIN %3</span></p>")
             .arg(esc(car.name), esc(car.description), esc(car.vin));

    double km = -1.0;
    for (const GarageCheck& c : car.checks) km = std::max(km, c.odometerKm);
    for (const GarageFuel& f : car.fuel) km = std::max(km, f.odometerKm);
    const QVector<ServiceStatus> services = Garage::serviceStatus(car, km, QDate::currentDate());
    if (!services.isEmpty()) {
        h += QStringLiteral("<h2>%1</h2><table><tr><th>%2</th><th>%3</th><th>%4</th></tr>")
                 .arg(esc(QCoreApplication::translate("caruspex::Report", "Wartung")), esc(QCoreApplication::translate("caruspex::Report", "Was")), esc(QCoreApplication::translate("caruspex::Report", "Zuletzt")), esc(QCoreApplication::translate("caruspex::Report", "Stand")));
        for (const ServiceStatus& st : services) {
            QStringList last;
            if (st.service->lastDate.isValid()) last << locale.toString(st.service->lastDate, QLocale::ShortFormat);
            if (st.service->lastKm >= 0) {
                last << QStringLiteral("%1 km").arg(locale.toString(st.service->lastKm, 'f', 0));
            }
            h += QStringLiteral("<tr><td>%1</td><td class='muted'>%2</td><td>%3</td></tr>")
                     .arg(esc(st.service->title), esc(last.join(QStringLiteral(", "))), esc(st.text));
        }
        h += QStringLiteral("</table>");
    }

    for (const OdometerJump& j : Garage::odometerJumps(car)) {
        const QString detail = QStringLiteral("%1 km (%2), %3 km (%4)")
                                   .arg(locale.toString(j.earlierKm, 'f', 0),
                                        locale.toString(j.earlier.date(), QLocale::ShortFormat),
                                        locale.toString(j.laterKm, 'f', 0),
                                        locale.toString(j.later.date(), QLocale::ShortFormat));
        h += QStringLiteral("<p style='color:#c93636'><b>%1</b> %2</p>")
                 .arg(esc(QCoreApplication::translate("caruspex::Report", "Kilometerstand gesunken:")), esc(detail));
    }

    h += QStringLiteral("<h2>%1</h2>").arg(esc(QCoreApplication::translate("caruspex::Report", "Prüfungen")));
    if (car.checks.isEmpty()) {
        h += QStringLiteral("<p class='muted'>%1</p>").arg(esc(QCoreApplication::translate("caruspex::Report", "Noch keine.")));
    } else {
        h += QStringLiteral("<table><tr><th>%1</th><th>%2</th><th>%3</th><th>%4</th></tr>")
                 .arg(esc(QCoreApplication::translate("caruspex::Report", "Datum")), esc(QCoreApplication::translate("caruspex::Report", "Kilometer")), esc(QCoreApplication::translate("caruspex::Report", "Punkte")), esc(QCoreApplication::translate("caruspex::Report", "Fehler")));
        for (int i = car.checks.size() - 1; i >= 0; --i) {
            const GarageCheck& c = car.checks[i];
            QStringList codes;
            for (int k = 0; k < c.codes.size(); ++k) {
                codes << QStringLiteral("<span class='code'>%1</span> %2")
                             .arg(esc(c.codes[k]), k < c.texts.size() ? esc(c.texts[k]) : QString());
            }
            h += QStringLiteral("<tr><td>%1</td><td>%2</td><td>%3</td><td>%4</td></tr>")
                     .arg(esc(locale.toString(c.when, QLocale::ShortFormat)),
                          c.odometerKm >= 0 ? locale.toString(c.odometerKm, 'f', 0) : QStringLiteral("-"),
                          c.score >= 0 ? QString::number(c.score) : QStringLiteral("-"),
                          codes.isEmpty() ? esc(QCoreApplication::translate("caruspex::Report", "keine")) : codes.join(QStringLiteral("<br/>")));
        }
        h += QStringLiteral("</table>");
    }

    if (!car.trips.isEmpty()) {
        double distance = 0.0;
        double fuel = 0.0;
        double cost = 0.0;
        for (const GarageTrip& t : car.trips) {
            distance += t.distanceKm;
            fuel += t.fuelLitres;
            cost += t.cost;
        }
        const QString summary = QCoreApplication::translate("caruspex::Report", "%1 Fahrten, %2 km, im Schnitt %3 l/100 km, %4 €")
                                    .arg(car.trips.size())
                                    .arg(locale.toString(distance, 'f', 0))
                                    .arg(distance > 1 ? locale.toString(fuel / distance * 100, 'f', 1)
                                                      : QStringLiteral("-"))
                                    .arg(locale.toString(cost, 'f', 2));
        h += QStringLiteral("<h2>%1</h2><p>%2</p>").arg(esc(QCoreApplication::translate("caruspex::Report", "Fahrten")), esc(summary));
    }
    if (!car.fuel.isEmpty()) {
        const double logged = Garage::fuelLogConsumption(car);
        const QString summary = logged > 0 ? QCoreApplication::translate("caruspex::Report", "%1 Tankfüllungen, Verbrauch %2 l/100 km")
                                                 .arg(car.fuel.size())
                                                 .arg(locale.toString(logged, 'f', 1))
                                           : QCoreApplication::translate("caruspex::Report", "%1 Tankfüllungen").arg(car.fuel.size());
        h += QStringLiteral("<h2>%1</h2><p>%2</p>").arg(esc(QCoreApplication::translate("caruspex::Report", "Tankbuch")), esc(summary));
    }
    if (!car.note.isEmpty()) {
        QString note = esc(car.note);
        note.replace(QLatin1Char('\n'), QStringLiteral("<br/>"));
        h += QStringLiteral("<h2>%1</h2><p>%2</p>").arg(esc(QCoreApplication::translate("caruspex::Report", "Notiz")), note);
    }
    h += QStringLiteral("<p class='muted' style='margin-top:22px; font-size:8.5pt'>%1</p>")
             .arg(esc(QCoreApplication::translate("caruspex::Report", "Erstellt mit Caruspex aus der Garage auf diesem Rechner.")));
    h += QStringLiteral("</body></html>");
    return h;
}

QString saveGaragePdf(QWidget* parent, const GarageCar& car) {
    const QString suggested = QDir(QStandardPaths::writableLocation(QStandardPaths::DocumentsLocation))
                                  .filePath(QStringLiteral("%1_Akte_%2.pdf")
                                                .arg(car.vin, QDate::currentDate().toString(Qt::ISODate)));
    const QString path = QFileDialog::getSaveFileName(
        parent, QCoreApplication::translate("caruspex::Report", "Fahrzeugakte speichern"), suggested,
        QCoreApplication::translate("caruspex::Report", "PDF-Dokument (*.pdf)"));
    if (path.isEmpty()) return {};
    return writeGaragePdf(car, path) ? path : QString();
}

bool writeGaragePdf(const GarageCar& car, const QString& path) {
    QPdfWriter writer(path);
    writer.setPageSize(QPageSize(QPageSize::A4));
    writer.setPageMargins(QMarginsF(16, 16, 16, 16), QPageLayout::Millimeter);
    writer.setTitle(QCoreApplication::translate("caruspex::Report", "Fahrzeugakte"));
    writer.setCreator(QStringLiteral("Caruspex"));
    QTextDocument doc;
    doc.setHtml(garageHtml(car));
    doc.print(&writer);
    return true;
}

void printReport(QWidget* parent, const VehicleState& state, const ReportExtras& extras) {
    QPrinter printer(QPrinter::HighResolution);
    QPrintDialog dialog(&printer, parent);
    if (dialog.exec() != QDialog::Accepted) return;
    QTextDocument doc;
    doc.setHtml(reportHtml(state, extras));
    doc.print(&printer);
}

}  // namespace caruspex
