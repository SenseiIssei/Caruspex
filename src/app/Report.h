// Inspection report as a printable document: what the car is, what it said,
// what that means. Written as HTML for QTextDocument, saved as PDF or printed.
#pragma once

#include <QString>
#include <QVector>

#include "app/Session.h"

class QWidget;

namespace caruspex {

class VehicleState;

struct ReportFinding {
    QString severity;   // short label, e.g. "Hoch"
    QString title;
    QString text;
};

struct ReportExtras {
    int score = -1;
    QString scoreCaption;
    QVector<ReportFinding> findings;
    QVector<UdsEcuRow> ecus;   // from the scan of all control units, if done
};

QString reportHtml(const VehicleState& state, const ReportExtras& extras);

// Asks for a file name and writes the PDF. Returns the path, empty if the user
// cancelled or writing failed.
QString saveReportPdf(QWidget* parent, const VehicleState& state, const ReportExtras& extras);

// The file of one garage car: checks, service plan, trips and fuel log.
// Works without a connection.
QString garageHtml(const struct GarageCar& car);
QString saveGaragePdf(QWidget* parent, const struct GarageCar& car);
bool writeGaragePdf(const struct GarageCar& car, const QString& path);

// Opens the print dialog.
void printReport(QWidget* parent, const VehicleState& state, const ReportExtras& extras);

}  // namespace caruspex
