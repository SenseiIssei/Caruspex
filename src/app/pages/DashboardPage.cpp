#include "app/pages/DashboardPage.h"

#include <QFrame>
#include <QGridLayout>
#include <QHBoxLayout>
#include <QLabel>
#include <QLocale>
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

namespace caruspex {

using namespace theme;

namespace {

// Titles and units are untranslated source texts (QT_TRANSLATE_NOOP) and get
// translated when the gauges are built.
struct GaugeSpec {
    int pid;
    const char* title;
    const char* unit;
    double minimum;
    double maximum;
    int decimals;
    bool hasWarn;
    double warnFrom;
    double warnTo;
};

const GaugeSpec kGauges[] = {
    {0x0C, QT_TRANSLATE_NOOP("caruspex::DashboardPage", "Drehzahl"),
     QT_TRANSLATE_NOOP("caruspex::DashboardPage", "1/min"), 0, 7000, 0, false, 0, 0},
    {0x0D, QT_TRANSLATE_NOOP("caruspex::DashboardPage", "Geschwindigkeit"), "km/h", 0, 220, 0,
     false, 0, 0},
    {0x05, QT_TRANSLATE_NOOP("caruspex::DashboardPage", "Kühlmittel"), "°C", -40, 130, 0, true,
     70, 105},
    {0x04, QT_TRANSLATE_NOOP("caruspex::DashboardPage", "Motorlast"), "%", 0, 100, 0, false, 0,
     0},
    {0x11, QT_TRANSLATE_NOOP("caruspex::DashboardPage", "Drosselklappe"), "%", 0, 100, 0, false,
     0, 0},
    {0x42, QT_TRANSLATE_NOOP("caruspex::DashboardPage", "Bordspannung"), "V", 8, 16, 1, true,
     11.8, 15.0},
};

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
    gaugeHead->addWidget(makeLabel(tr("Live"), "SectionTitle"));
    gaugeHead->addStretch();
    auto* gaugeHint = makeLabel(tr("läuft, sobald eine Messung gestartet ist"), "Hint");
    gaugeHead->addWidget(gaugeHint);
    gaugeLayout->addLayout(gaugeHead);
    auto* grid = new QGridLayout;
    grid->setSpacing(10);
    int column = 0;
    for (const GaugeSpec& spec : kGauges) {
        auto* g = new GaugeWidget;
        g->configure(tr(spec.title), tr(spec.unit), spec.minimum, spec.maximum, spec.decimals);
        if (spec.hasWarn) g->setWarnRange(spec.warnFrom, spec.warnTo);
        grid->addWidget(g, 0, column++);
        gauges_.insert(spec.pid, g);
    }
    gaugeLayout->addLayout(grid);
    root->addWidget(gaugeCard);
    root->addStretch();

    battery_->clear();
    coolant_->clear();
    odometer_->clear();
    fuel_->clear();
    updateHero();
    rebuildFindings();
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
