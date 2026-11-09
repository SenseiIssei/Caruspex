#include "app/pages/DashboardPage.h"

#include <QFrame>
#include <QGridLayout>
#include <QHBoxLayout>
#include <QLabel>
#include <QLocale>
#include <QMenu>
#include <QSettings>
#include <QPushButton>
#include <QScrollArea>
#include <QVBoxLayout>

#include "app/GaugeWidget.h"
#include "app/Icons.h"
#include "app/Theme.h"
#include "app/Widgets.h"
#include "app/pages/FaultsPage.h"
#include "data/DtcDatabase.h"
#include "data/DtcLocation.h"
#include "data/Pids.h"

namespace caruspex {

using namespace theme;

namespace {

// What can be put on a gauge, in menu order. Names, units and ranges come
// from the PID table, so they follow the language and the standard.
const int kGaugeChoices[] = {0x0C, 0x0D, 0x05, 0x04, 0x11, 0x42, 0x0F, 0x10, 0x0B, 0x2F,
                             0x5C, 0x5E, 0x0E, 0x06, 0x07, 0x3C, 0x46, 0x49, 0x70, 0xA4};
const QVector<int> kDefaultGauges = {0x0C, 0x0D, 0x05, 0x04, 0x11, 0x42};

QPushButton* makeButton(const QString& text, const QString& iconName, bool primary) {
    auto* b = new QPushButton(text);
    if (primary) b->setObjectName("Primary");
    b->setIcon(icons::icon(iconName, primary ? QColor(Qt::white) : color::textMuted,
                           primary ? QColor(Qt::white) : color::text, 18));
    b->setIconSize(QSize(18, 18));
    b->setMinimumHeight(44);
    b->setCursor(Qt::PointingHandCursor);
    return b;
}

}  // namespace

DashboardPage::DashboardPage(QWidget* parent) : QWidget(parent) {
    auto* outer = new QVBoxLayout(this);
    outer->setContentsMargins(0, 0, 0, 0);
    auto* scroll = new QScrollArea;
    scroll->setWidgetResizable(true);
    scroll->setFrameShape(QFrame::NoFrame);
    outer->addWidget(scroll);
    auto* content = new QWidget;
    scroll->setWidget(content);

    auto* root = new QVBoxLayout(content);
    root->setContentsMargins(32, 26, 32, 26);
    root->setSpacing(20);

    // --- Hero: the car ---------------------------------------------------------
    QVBoxLayout* heroLayout = nullptr;
    QFrame* hero = makeCard(&heroLayout, 28);
    auto* heroRow = new QHBoxLayout;
    heroRow->setSpacing(24);
    heroLayout->addLayout(heroRow);

    auto* text = new QVBoxLayout;
    text->setSpacing(8);
    eyebrow_ = makeLabel(tr("DEIN FAHRZEUG"), "Eyebrow");
    vehicleTitle_ = makeLabel(QString(), "Headline", true);
    vehicleSub_ = makeLabel(QString(), "Subtle", true);
    stateLine_ = new QLabel;
    stateLine_->setWordWrap(true);
    text->addWidget(eyebrow_);
    text->addWidget(vehicleTitle_);
    text->addWidget(vehicleSub_);
    text->addSpacing(6);
    text->addWidget(stateLine_);
    text->addSpacing(10);
    auto* buttons = new QHBoxLayout;
    buttons->setSpacing(10);
    primary_ = makeButton(tr("Verbinden"), QStringLiteral("plug"), true);
    secondary_ = makeButton(tr("Live-Werte"), QStringLiteral("activity"), false);
    report_ = makeButton(tr("Prüfbericht"), QStringLiteral("file"), false);
    report_->setToolTip(tr("Alles Gelesene als PDF speichern oder drucken"));
    buttons->addWidget(primary_);
    buttons->addWidget(secondary_);
    buttons->addWidget(report_);
    buttons->addStretch();
    text->addLayout(buttons);
    text->addStretch();
    heroRow->addLayout(text, 5);

    car_ = new VehicleDiagram;
    car_->setMinimumWidth(360);
    heroRow->addWidget(car_, 6);
    root->addWidget(hero);

    connect(primary_, &QPushButton::clicked, this, [this] {
        if (!connected_) {
            emit connectRequested();
            return;
        }
        emit readRequested();
        emit detailsRequested();
    });
    connect(secondary_, &QPushButton::clicked, this, &DashboardPage::liveRequested);
    connect(report_, &QPushButton::clicked, this, &DashboardPage::reportRequested);

    // --- Health and tiles --------------------------------------------------------
    auto* middle = new QHBoxLayout;
    middle->setSpacing(20);

    QVBoxLayout* healthLayout = nullptr;
    QFrame* health = makeCard(&healthLayout, 24);
    auto* healthTop = new QHBoxLayout;
    healthTop->setSpacing(24);
    ring_ = new ScoreRing;
    ring_->setFixedSize(220, 220);
    healthTop->addWidget(ring_, 0, Qt::AlignTop);
    auto* findings = new QVBoxLayout;
    findings->setSpacing(4);
    findingsTitle_ = makeLabel(tr("Was Caruspex gefunden hat"), "SectionTitle");
    findingsSub_ = makeLabel(tr("Jeder Punkt Abzug steht hier, mit Grund."), "Subtle", true);
    findings->addWidget(findingsTitle_);
    findings->addWidget(findingsSub_);
    findings->addSpacing(6);
    findingsList_ = new QVBoxLayout;
    findingsList_->setSpacing(0);
    findings->addLayout(findingsList_);
    findings->addStretch();
    healthTop->addLayout(findings, 1);
    healthLayout->addLayout(healthTop);
    middle->addWidget(health, 3);

    auto* tiles = new QGridLayout;
    tiles->setSpacing(14);
    battery_ = new StatTile(QStringLiteral("battery"), tr("Batterie"));
    coolant_ = new StatTile(QStringLiteral("thermometer"), tr("Kühlmittel"));
    odometer_ = new StatTile(QStringLiteral("route"), tr("Kilometerstand"));
    fuel_ = new StatTile(QStringLiteral("fuel"), tr("Tank"));
    tiles->addWidget(battery_, 0, 0);
    tiles->addWidget(coolant_, 0, 1);
    tiles->addWidget(odometer_, 1, 0);
    tiles->addWidget(fuel_, 1, 1);
    auto* tileBox = new QWidget;
    tileBox->setLayout(tiles);
    middle->addWidget(tileBox, 2, Qt::AlignTop);
    root->addLayout(middle);

    // --- Gauges ------------------------------------------------------------------------
    QVBoxLayout* gaugeLayout = nullptr;
    QFrame* gaugeCard = makeCard(&gaugeLayout, 22);
    auto* gaugeHead = new QHBoxLayout;
    gaugeHead->setSpacing(8);
    gaugeHead->addWidget(makeLabel(tr("Live"), "SectionTitle"));
    gaugeHead->addStretch();
    gaugeLive_ = new QPushButton(tr("Live starten"));
    gaugeLive_->setObjectName("Ghost");
    gaugeLive_->setIcon(icons::icon(QStringLiteral("play"), color::textMuted, color::text, 14));
    gaugeLive_->setEnabled(false);
    auto* choose = new QPushButton(tr("Anzeigen wählen"));
    choose->setObjectName("Ghost");
    choose->setIcon(icons::icon(QStringLiteral("sliders"), color::textMuted, color::text, 14));
    gaugeHead->addWidget(gaugeLive_);
    gaugeHead->addWidget(choose);
    gaugeLayout->addLayout(gaugeHead);
    gaugeGrid_ = new QGridLayout;
    gaugeGrid_->setSpacing(10);
    gaugeLayout->addLayout(gaugeGrid_);
    rebuildGauges();

    connect(choose, &QPushButton::clicked, this, [this, choose] {
        QMenu menu(this);
        const QVector<int> current = gaugePids_;
        for (int pid : kGaugeChoices) {
            const PidSpec* spec = findPid(pid);
            if (!spec) continue;
            QAction* action = menu.addAction(QString::fromUtf8(spec->name));
            action->setCheckable(true);
            action->setChecked(current.contains(pid));
            action->setData(pid);
            // At most eight, so the gauges stay readable.
            action->setEnabled(current.contains(pid) || current.size() < 8);
        }
        QAction* picked = menu.exec(choose->mapToGlobal(QPoint(0, choose->height())));
        if (!picked) return;
        const int pid = picked->data().toInt();
        if (picked->isChecked()) {
            if (!gaugePids_.contains(pid)) gaugePids_.push_back(pid);
        } else if (gaugePids_.size() > 1) {
            gaugePids_.removeAll(pid);
        }
        QStringList stored;
        for (int p : std::as_const(gaugePids_)) stored << QString::number(p);
        QSettings(QStringLiteral("Caruspex"), QStringLiteral("Caruspex"))
            .setValue(QStringLiteral("start/anzeigen"), stored);
        rebuildGauges();
        if (gaugesRunning_) emit gaugesLiveRequested(gaugePids_);
    });
    connect(gaugeLive_, &QPushButton::clicked, this, [this] {
        gaugesRunning_ = !gaugesRunning_;
        if (gaugesRunning_) {
            emit gaugesLiveRequested(gaugePids_);
        } else {
            emit gaugesLiveStopRequested();
        }
        gaugeLive_->setText(gaugesRunning_ ? tr("Live anhalten") : tr("Live starten"));
        gaugeLive_->setIcon(icons::icon(gaugesRunning_ ? QStringLiteral("pause") : QStringLiteral("play"),
                                        color::textMuted, color::text, 14));
    });
    root->addWidget(gaugeCard);
    root->addStretch();

    battery_->clear();
    coolant_->clear();
    odometer_->clear();
    fuel_->clear();
    updateHero();
    rebuildFindings();
}

void DashboardPage::startGaugesLive() {
    if (connected_ && !gaugesRunning_) gaugeLive_->click();
}

void DashboardPage::rebuildGauges() {
    if (gaugePids_.isEmpty()) {
        const QStringList stored = QSettings(QStringLiteral("Caruspex"), QStringLiteral("Caruspex"))
                                       .value(QStringLiteral("start/anzeigen"))
                                       .toStringList();
        for (const QString& p : stored) {
            if (findPid(p.toInt())) gaugePids_.push_back(p.toInt());
        }
        if (gaugePids_.isEmpty()) gaugePids_ = kDefaultGauges;
    }
    while (QLayoutItem* item = gaugeGrid_->takeAt(0)) {
        if (item->widget()) item->widget()->deleteLater();
        delete item;
    }
    gauges_.clear();
    const int perRow = gaugePids_.size() > 6 ? 4 : qMax(1, int(gaugePids_.size()));
    int index = 0;
    for (int pid : std::as_const(gaugePids_)) {
        const PidSpec* spec = findPid(pid);
        if (!spec) continue;
        auto* g = new GaugeWidget;
        double low = spec->displayMin;
        double high = spec->displayMax;
        if (high <= low) {
            low = 0.0;
            high = 100.0;
        }
        const int decimals = (high - low) <= 20.0 ? 1 : 0;
        g->configure(QString::fromUtf8(spec->name), QString::fromUtf8(spec->unit), low, high, decimals);
        if (spec->warnLow != 0.0 || spec->warnHigh != 0.0) g->setWarnRange(spec->warnLow, spec->warnHigh);
        gaugeGrid_->addWidget(g, index / perRow, index % perRow);
        gauges_.insert(pid, g);
        ++index;
    }
}

GaugeWidget* DashboardPage::gauge(int pid) {
    const auto it = gauges_.find(pid);
    return it == gauges_.end() ? nullptr : it.value();
}

void DashboardPage::setBrand(const QString& brand) {
    brand_ = brand;
    rebuildFindings();
}

void DashboardPage::setVehicle(const QString& title, const QString& subtitle, BodyStyle body) {
    vehicleName_ = title;
    car_->setBodyStyle(body);
    vehicleSub_->setText(subtitle);
    updateHero();
}

void DashboardPage::onConnected(const QString& adapter, const QString& transport,
                                const QString& protocol) {
    connected_ = true;
    gaugeLive_->setEnabled(true);
    if (vehicleName_.isEmpty()) {
        vehicleSub_->setText(tr("%1 über %2, Protokoll %3").arg(adapter, transport, protocol));
    }
    updateHero();
    rebuildFindings();
}

void DashboardPage::onDisconnected() {
    connected_ = false;
    haveRead_ = false;
    haveStatus_ = false;
    mil_ = false;
    incompleteMonitors_ = 0;
    faults_.clear();
    externalHealth_ = false;
    vehicleName_.clear();
    vehicleSub_->clear();
    car_->setBodyStyle(BodyStyle::Hatchback);
    for (GaugeWidget* g : std::as_const(gauges_)) g->setActive(false);
    gaugesRunning_ = false;
    gaugeLive_->setText(tr("Live starten"));
    gaugeLive_->setIcon(icons::icon(QStringLiteral("play"), color::textMuted, color::text, 14));
    gaugeLive_->setEnabled(false);
    battery_->clear();
    coolant_->clear();
    odometer_->clear();
    fuel_->clear();
    updateHero();
    rebuildFindings();
}

void DashboardPage::onBusy(bool busy) {
    busy_ = busy;
    ring_->setBusy(busy && connected_);
    car_->setScanning(busy && connected_);
}

void DashboardPage::onStatus(const StatusRow& row) {
    if (!row.valid) return;
    haveStatus_ = true;
    mil_ = row.mil;
    incompleteMonitors_ = 0;
    for (bool ready : row.monitorReady) {
        if (!ready) ++incompleteMonitors_;
    }
    rebuildFindings();
}

void DashboardPage::onFaults(const QVector<FaultRow>& rows) {
    haveRead_ = true;
    faults_ = rows;
    updateHero();
    rebuildFindings();
}

void DashboardPage::onLiveValue(const LiveRow& row) {
    if (GaugeWidget* g = gauge(row.pid)) g->setValue(row.value);
    switch (row.pid) {
        case 0x42: {
            const QColor state = row.value < 11.8 ? color::bad
                                 : row.value < 12.3 ? color::warn
                                                    : color::good;
            battery_->setValue(QString::number(row.value, 'f', 1), QStringLiteral("V"), state);
            break;
        }
        case 0x05: {
            const QColor state = row.value > 110 ? color::bad
                                 : row.value < 70 ? color::info
                                                  : color::good;
            coolant_->setValue(QString::number(row.value, 'f', 0), QStringLiteral("°C"), state);
            coolant_->setNote(row.value < 70 ? tr("noch nicht warm") : QString());
            break;
        }
        case 0xA6:
            odometer_->setValue(QLocale().toString(row.value, 'f', 0), QStringLiteral("km"));
            break;
        case 0x2F: {
            const QColor state = row.value < 12 ? color::warn : color::good;
            fuel_->setValue(QString::number(row.value, 'f', 0), QStringLiteral("%"), state);
            break;
        }
        default:
            break;
    }
}

void DashboardPage::setHealth(int score, const QString& caption, const QVector<Finding>& findings) {
    externalHealth_ = true;
    showFindings(score, caption, tr("Gesundheit"), findings, false);
}

void DashboardPage::showFindings(int score, const QString& caption, const QString& subcaption,
                                 const QVector<Finding>& findings, bool clickableFaults) {
    score_ = score;
    caption_ = caption;
    findings_ = findings;
    ring_->setScore(score, caption, subcaption);
    while (QLayoutItem* item = findingsList_->takeAt(0)) {
        if (item->widget()) item->widget()->deleteLater();
        delete item;
    }
    int delay = 0;
    for (const Finding& f : findings) {
        auto* row = new FindingRow(f.icon, f.tint, f.title, f.text, f.meta);
        if (clickableFaults && f.icon == QStringLiteral("alert")) {
            row->setClickable(true);
            connect(row, &FindingRow::clicked, this, &DashboardPage::faultsRequested);
        }
        findingsList_->addWidget(row);
        fadeIn(row, delay);
        delay += 60;
    }
    if (score >= 0) emit scoreChanged(score);
}

void DashboardPage::updateHero() {
    if (!connected_) {
        eyebrow_->setText(tr("WILLKOMMEN"));
        vehicleTitle_->setText(tr("Noch kein Auto verbunden"));
        vehicleSub_->setText(tr("Adapter einstecken, Zündung an, verbinden. Oder erst einmal "
                                "mit dem Demo-Auto ausprobieren."));
        stateLine_->setText(QString());
        primary_->setText(tr("Verbinden"));
        primary_->setIcon(icons::icon(QStringLiteral("plug"), Qt::white, Qt::white, 18));
        secondary_->setEnabled(false);
        report_->setEnabled(false);
        return;
    }
    eyebrow_->setText(tr("DEIN FAHRZEUG"));
    vehicleTitle_->setText(vehicleName_.isEmpty() ? tr("Verbundenes Fahrzeug") : vehicleName_);
    primary_->setText(tr("Auto prüfen"));
    primary_->setIcon(icons::icon(QStringLiteral("sparkles"), Qt::white, Qt::white, 18));
    secondary_->setEnabled(true);
    report_->setEnabled(true);

    QColor tone = color::accent;
    QString text = tr("Verbunden. Auto prüfen liest Fehler, Bereitschaft und Fahrzeugdaten.");
    if (haveRead_) {
        int stored = 0;
        for (const FaultRow& f : faults_) {
            if (f.stored) ++stored;
        }
        if (faults_.isEmpty()) {
            tone = color::good;
            text = tr("Kein Fehler gespeichert.");
        } else {
            tone = stored > 0 ? color::bad : color::warn;
            text = faults_.size() == 1 ? tr("1 Eintrag im Fehlerspeicher.")
                                       : tr("%1 Einträge im Fehlerspeicher.").arg(faults_.size());
        }
    }
    stateLine_->setText(QStringLiteral("<span style='color:%1; font-size:16px;'>●</span>&nbsp;&nbsp;%2")
                            .arg(tone.name(), text.toHtmlEscaped()));
}

void DashboardPage::rebuildFindings() {
    if (externalHealth_) return;

    if (!connected_) {
        showFindings(-1, tr("Noch offen"), tr("Gesundheit"),
                     {{QStringLiteral("info"), color::info, tr("Noch nichts geprüft"),
                       tr("Nach dem Verbinden steht hier, was mit dem Auto los ist, in einfachen "
                          "Worten."),
                       QString()}},
                     false);
        return;
    }
    if (!haveRead_) {
        showFindings(-1, tr("Bereit"), tr("Auto prüfen drücken"),
                     {{QStringLiteral("sparkles"), color::accent, tr("Bereit zur Prüfung"),
                       tr("Auto prüfen liest den Fehlerspeicher, die Bereitschaftstests und die "
                          "Fahrzeugdaten."),
                       QString()}},
                     false);
        return;
    }

    // Simple rating until the full health check reports: what is stored
    // weighs most, then the lamp, then the open readiness tests.
    int score = 100;
    QVector<Finding> findings;
    for (const FaultRow& f : faults_) {
        // Weight by how urgent the code is; a pending code counts half.
        const DtcInfo info = lookupDtc(f.code.toStdString(), brand_.toStdString());
        static const int weight[] = {3, 6, 12, 20, 30};
        int minus = weight[static_cast<int>(info.severity)];
        if (!f.stored) minus = (minus + 1) / 2;
        score -= minus;
        const ZoneInfo where = locateDtc(f.code.toStdString());
        findings.push_back({QStringLiteral("alert"), severityColour(static_cast<int>(info.severity)),
                            QStringLiteral("%1  %2").arg(f.code, info.known ? QString::fromStdString(info.text) : f.text),
                            QStringLiteral("%1 · %2 · %3").arg(QString::fromUtf8(severityLabel(info.severity)), f.kind,
                                                                QString::fromStdString(where.name)),
                            QStringLiteral("−%1").arg(minus)});
    }
    if (haveStatus_ && mil_) {
        score -= 12;
        findings.push_back({QStringLiteral("engine"), color::bad, tr("Motorkontrollleuchte an"),
                            tr("Das Steuergerät meldet einen abgasrelevanten Fehler. Bald in die "
                               "Werkstatt oder selbst nachsehen."),
                            QStringLiteral("−12")});
    }
    if (haveStatus_ && incompleteMonitors_ > 0) {
        const int minus = qMin(9, incompleteMonitors_ * 3);
        score -= minus;
        findings.push_back(
            {QStringLiteral("clock"), color::info,
             incompleteMonitors_ == 1 ? tr("1 Bereitschaftstest noch offen")
                                      : tr("%1 Bereitschaftstests noch offen").arg(incompleteMonitors_),
             tr("Nach dem Löschen oder einem Batteriewechsel normal. Für die AU sollten sie "
                "abgeschlossen sein."),
             QStringLiteral("−%1").arg(minus)});
    }
    if (findings.isEmpty()) {
        findings.push_back({QStringLiteral("check-circle"), color::good, tr("Alles in Ordnung"),
                            tr("Kein Fehler gespeichert, die Lampe ist aus."), QString()});
    }
    score = qBound(0, score, 100);
    const QString caption = score >= 85   ? tr("Gut in Schuss")
                            : score >= 60 ? tr("Braucht Pflege")
                                          : tr("Bitte ansehen");
    showFindings(score, caption, tr("Gesundheit"), findings, true);
}

}  // namespace caruspex
