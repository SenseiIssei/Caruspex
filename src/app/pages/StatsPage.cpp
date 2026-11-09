#include "app/pages/StatsPage.h"

#include <QDateTime>
#include <QFileDialog>
#include <QFrame>
#include <QGridLayout>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QLabel>
#include <QMessageBox>
#include <QPushButton>
#include <QTableWidget>
#include <QTextStream>
#include <QVBoxLayout>

#include "app/Theme.h"

namespace caruspex {
namespace {

// Petrol: air-fuel ratio 14.7 to 1, density about 745 grams per litre.
constexpr double kAirFuelRatio = 14.7;
constexpr double kFuelDensityGramPerLitre = 745.0;

QFrame* statCard(const QString& caption, QLabel*& valueLabel) {
    auto* frame = new QFrame;
    frame->setObjectName("Card");
    auto* layout = new QVBoxLayout(frame);
    layout->setContentsMargins(16, 12, 16, 12);
    layout->setSpacing(2);

    valueLabel = new QLabel(QStringLiteral("--"));
    QFont big = valueLabel->font();
    big.setPointSizeF(big.pointSizeF() + 7.0);
    big.setBold(true);
    valueLabel->setFont(big);
    valueLabel->setStyleSheet(QStringLiteral("color: %1;").arg(theme::color::accent.name()));

    auto* captionLabel = new QLabel(caption);
    captionLabel->setObjectName("Subtle");
    captionLabel->setWordWrap(true);

    layout->addWidget(valueLabel);
    layout->addWidget(captionLabel);
    return frame;
}

QString formatDuration(qint64 milliseconds) {
    const qint64 seconds = milliseconds / 1000;
    return QStringLiteral("%1:%2")
        .arg(seconds / 60, 2, 10, QChar('0'))
        .arg(seconds % 60, 2, 10, QChar('0'));
}

}  // namespace

StatsPage::StatsPage(QWidget* parent) : QWidget(parent) {
    auto* root = new QVBoxLayout(this);
    root->setContentsMargins(32, 26, 32, 24);
    root->setSpacing(14);

    auto* title = new QLabel(tr("Statistik"));
    title->setObjectName("PageTitle");
    root->addWidget(title);

    auto* bar = new QHBoxLayout;
    resetButton_ = new QPushButton(tr("Zurücksetzen"));
    resetButton_->setMinimumHeight(38);
    exportButton_ = new QPushButton(tr("Als CSV speichern"));
    exportButton_->setMinimumHeight(38);
    exportButton_->setEnabled(false);
    bar->addWidget(resetButton_);
    bar->addWidget(exportButton_);
    bar->addStretch();
    root->addLayout(bar);
    connect(resetButton_, &QPushButton::clicked, this, &StatsPage::reset);
    connect(exportButton_, &QPushButton::clicked, this, &StatsPage::exportCsv);

    // Key figures as cards
    auto* cards = new QGridLayout;
    cards->setSpacing(12);
    cards->addWidget(statCard(tr("Messdauer"), durationValue_), 0, 0);
    cards->addWidget(statCard(tr("Messpunkte"), samplesValue_), 0, 1);
    cards->addWidget(statCard(tr("Höchste Drehzahl"), maxRpmValue_), 0, 2);
    cards->addWidget(statCard(tr("Höchste Geschwindigkeit"), maxSpeedValue_), 0, 3);
    cards->addWidget(statCard(tr("Verbrauch geschätzt"), consumptionValue_), 0, 4);
    cards->addWidget(statCard(tr("Strecke seit Start"), distanceValue_), 0, 5);
    root->addLayout(cards);

    table_ = new QTableWidget(0, 6);
    table_->setHorizontalHeaderLabels({tr("Messwert"), tr("Kleinster"), tr("Mittel"),
                                       tr("Größter"), tr("Aktuell"), tr("Einheit")});
    table_->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    for (int i = 1; i < 6; ++i) {
        table_->horizontalHeader()->setSectionResizeMode(i, QHeaderView::Fixed);
        table_->setColumnWidth(i, 110);
    }
    table_->verticalHeader()->setVisible(false);
    table_->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table_->setAlternatingRowColors(true);
    root->addWidget(table_, 1);

    note_ = new QLabel(
        tr("Der Verbrauch wird aus der Luftmasse gerechnet (Luftverhältnis 14,7 zu 1, "
           "Dichte 745 Gramm je Liter) und gilt so nur für Benziner. Die Strecke ergibt "
           "sich aus dem Geschwindigkeitssignal. Beides sind Schätzungen aus den "
           "Messwerten, keine Werte vom Bordcomputer."));
    note_->setObjectName("Hint");
    note_->setWordWrap(true);
    root->addWidget(note_);
}

void StatsPage::onLiveValue(const LiveRow& row) {
    if (!row.text.isEmpty()) return;   // status values cannot be averaged
    if (!clock_.isValid()) clock_.start();

    Series& s = series_[row.pid];
    if (s.count == 0) {
        s.name = row.name;
        s.unit = row.unit;
        s.minimum = row.value;
        s.maximum = row.value;
        s.row = table_->rowCount();
        table_->insertRow(s.row);
        table_->setItem(s.row, 0, new QTableWidgetItem(row.name));
        for (int column = 1; column <= 4; ++column) {
            table_->setItem(s.row, column, new QTableWidgetItem());
        }
        table_->setItem(s.row, 5, new QTableWidgetItem(row.unit));
    }
    s.minimum = qMin(s.minimum, row.value);
    s.maximum = qMax(s.maximum, row.value);
    s.sum += row.value;
    s.last = row.value;
    ++s.count;

    const double average = s.sum / static_cast<double>(s.count);
    table_->item(s.row, 1)->setText(QString::number(s.minimum, 'f', 2));
    table_->item(s.row, 2)->setText(QString::number(average, 'f', 2));
    table_->item(s.row, 3)->setText(QString::number(s.maximum, 'f', 2));
    table_->item(s.row, 4)->setText(QString::number(s.last, 'f', 2));

    const qint64 now = clock_.elapsed();

    // Integrating the air mass gives the amount of fuel burned.
    if (row.pid == 0x10) {
        sawMaf_ = true;
        if (lastMafMs_ >= 0) {
            const double seconds = (now - lastMafMs_) / 1000.0;
            const double airGrams = 0.5 * (lastMaf_ + row.value) * seconds;
            fuelLitres_ += airGrams / kAirFuelRatio / kFuelDensityGramPerLitre;
        }
        lastMaf_ = row.value;
        lastMafMs_ = now;
    }
    // Integrating the speed gives the distance.
    if (row.pid == 0x0D) {
        if (lastSpeedMs_ >= 0) {
            const double hours = (now - lastSpeedMs_) / 3600000.0;
            distanceKm_ += 0.5 * (lastSpeed_ + row.value) * hours;
        }
        lastSpeed_ = row.value;
        lastSpeedMs_ = now;
    }

    refreshCards();
    exportButton_->setEnabled(true);
}

void StatsPage::refreshCards() {
    durationValue_->setText(formatDuration(clock_.isValid() ? clock_.elapsed() : 0));

    long long samples = 0;
    for (const Series& s : std::as_const(series_)) samples += s.count;
    samplesValue_->setText(QString::number(samples));

    const auto rpm = series_.constFind(0x0C);
    maxRpmValue_->setText(rpm == series_.constEnd()
                              ? QStringLiteral("--")
                              : QString::number(rpm->maximum, 'f', 0));

    const auto speed = series_.constFind(0x0D);
    maxSpeedValue_->setText(speed == series_.constEnd()
                                ? QStringLiteral("--")
                                : QString::number(speed->maximum, 'f', 0));

    distanceValue_->setText(distanceKm_ > 0.0 ? QString::number(distanceKm_, 'f', 2)
                                              : QStringLiteral("--"));

    if (!sawMaf_ || fuelLitres_ <= 0.0) {
        consumptionValue_->setText(QStringLiteral("--"));
    } else if (distanceKm_ > 0.1) {
        consumptionValue_->setText(
            QStringLiteral("%1 l/100km").arg(fuelLitres_ / distanceKm_ * 100.0, 0, 'f', 1));
    } else {
        // When stationary there is no distance, so the flow rate is what matters.
        const double hours = clock_.elapsed() / 3600000.0;
        consumptionValue_->setText(hours > 0.0
                                       ? QStringLiteral("%1 l/h").arg(fuelLitres_ / hours, 0,
                                                                      'f', 2)
                                       : QStringLiteral("--"));
    }
}

void StatsPage::setConnected(bool connected) {
    if (!connected) reset();
}

void StatsPage::reset() {
    series_.clear();
    table_->setRowCount(0);
    clock_.invalidate();
    fuelLitres_ = 0.0;
    distanceKm_ = 0.0;
    lastMaf_ = 0.0;
    lastSpeed_ = 0.0;
    lastMafMs_ = -1;
    lastSpeedMs_ = -1;
    sawMaf_ = false;
    exportButton_->setEnabled(false);
    for (QLabel* label : {durationValue_, samplesValue_, maxRpmValue_, maxSpeedValue_,
                          consumptionValue_, distanceValue_}) {
        label->setText(QStringLiteral("--"));
    }
}

void StatsPage::exportCsv() {
    if (series_.isEmpty()) return;
    const QString suggestion =
        tr("statistik_%1.csv")
            .arg(QDateTime::currentDateTime().toString("yyyy-MM-dd_HH-mm"));
    const QString path =
        QFileDialog::getSaveFileName(this, tr("Speichern"), suggestion, tr("CSV (*.csv)"));
    if (path.isEmpty()) return;

    QFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text)) {
        QMessageBox::warning(this, tr("Fehler"), tr("Datei lässt sich nicht schreiben."));
        return;
    }
    QTextStream out(&file);
    out.setEncoding(QStringConverter::Utf8);
    out << tr("Messwert;Kleinster;Mittel;Größter;Aktuell;Einheit;Messpunkte") << '\n';
    for (const Series& s : std::as_const(series_)) {
        out << s.name << ';' << QString::number(s.minimum, 'f', 3) << ';'
            << QString::number(s.sum / static_cast<double>(s.count), 'f', 3) << ';'
            << QString::number(s.maximum, 'f', 3) << ';' << QString::number(s.last, 'f', 3)
            << ';' << s.unit << ';' << s.count << '\n';
    }
}

}  // namespace caruspex
