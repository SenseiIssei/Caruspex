#include "app/pages/EvPage.h"

#include <QComboBox>

#include <cmath>
#include <QFrame>
#include <QGridLayout>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QLabel>
#include <QLocale>
#include <QPushButton>
#include <QScrollArea>
#include <QTableWidget>
#include <QVBoxLayout>

#include "app/Icons.h"
#include "app/Theme.h"
#include "app/Widgets.h"
#include "data/PidPacks.h"

namespace caruspex {

using namespace theme;

namespace {

QString num(double v, int decimals) { return QLocale().toString(v, 'f', decimals); }

}  // namespace

EvPage::EvPage(QWidget* parent) : QWidget(parent) {
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
    root->addWidget(makePageHeader(
        tr("Messen"), tr("Akku von E-Auto und Hybrid"),
        tr("Akkugesundheit, Ladestand, Zellen und Temperaturen direkt aus dem Batteriesteuergerät. "
           "Die Werte kommen aus belegten Herstellerlisten, die Quelle steht bei jedem Wert.")));

    auto* bar = new QHBoxLayout;
    bar->setSpacing(10);
    pack_ = new QComboBox;
    pack_->setMinimumHeight(40);
    for (const std::string& name : packNames()) {
        if (packCategory(name) != PidCategory::HybridEv) continue;
        pack_->addItem(QString::fromStdString(packTitle(name)), QString::fromStdString(name));
    }
    read_ = new QPushButton(tr("Akku lesen"));
    read_->setObjectName("Primary");
    read_->setIcon(icons::icon(QStringLiteral("battery"), Qt::white, Qt::white, 16));
    read_->setMinimumHeight(40);
    live_ = new QPushButton(tr("Laufend lesen"));
    live_->setIcon(icons::icon(QStringLiteral("play"), color::textMuted, color::text, 14));
    live_->setMinimumHeight(40);
    bar->addWidget(pack_, 1);
    bar->addWidget(read_);
    bar->addWidget(live_);
    root->addLayout(bar);
    status_ = makeLabel(tr("Für Hyundai Kona und Ioniq, Kia e-Niro, Renault Zoe und Nissan Leaf. Andere "
                           "E-Autos lassen sich über eigene Werte auf der Seite Live-Werte ergänzen."),
                        "Hint", true);
    root->addWidget(status_);

    // Hero: health ring and state of charge
    QVBoxLayout* heroLayout = nullptr;
    QFrame* hero = makeCard(&heroLayout, 26);
    auto* heroRow = new QHBoxLayout;
    heroRow->setSpacing(32);
    health_ = new ScoreRing;
    health_->setFixedSize(220, 220);
    health_->setScore(-1, tr("Akkugesundheit"), tr("SOH"));
    heroRow->addWidget(health_);
    auto* chargeBox = new QVBoxLayout;
    chargeBox->addWidget(makeLabel(tr("LADESTAND"), "Eyebrow"));
    charge_ = new QLabel(QStringLiteral("--"));
    styleFont(charge_, Family::Display, 72, 800);
    chargeBox->addWidget(charge_);
    chargeNote_ = makeLabel(QString(), "Subtle", true);
    chargeBox->addWidget(chargeNote_);
    advice_ = makeLabel(QString(), nullptr, true);
    chargeBox->addWidget(advice_);
    chargeBox->addStretch();
    heroRow->addLayout(chargeBox, 1);
    heroLayout->addLayout(heroRow);
    root->addWidget(hero);

    auto* grid = new QGridLayout;
    grid->setSpacing(14);
    voltage_ = new StatTile(QStringLiteral("zap"), tr("Spannung"));
    current_ = new StatTile(QStringLiteral("activity"), tr("Strom"));
    power_ = new StatTile(QStringLiteral("bolt-circle"), tr("Leistung"));
    cells_ = new StatTile(QStringLiteral("battery"), tr("Zellen"));
    spread_ = new StatTile(QStringLiteral("sliders"), tr("Zellspreizung"));
    temperature_ = new StatTile(QStringLiteral("thermometer"), tr("Akkutemperatur"));
    aux_ = new StatTile(QStringLiteral("battery"), tr("12-Volt-Batterie"));
    const QList<StatTile*> tiles = {voltage_, current_, power_, cells_, spread_, temperature_, aux_};
    for (int i = 0; i < tiles.size(); ++i) {
        tiles[i]->clear();
        grid->addWidget(tiles[i], i / 4, i % 4);
    }
    root->addLayout(grid);

    QVBoxLayout* tableLayout = nullptr;
    QFrame* tableCard = makeCard(&tableLayout, 18);
    tableLayout->addWidget(makeLabel(tr("Alle Werte"), "SectionTitle"));
    table_ = new QTableWidget(0, 3);
    table_->setHorizontalHeaderLabels({tr("Wert"), tr("Messung"), tr("Quelle")});
    table_->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    table_->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Stretch);
    table_->setColumnWidth(1, 150);
    table_->verticalHeader()->setVisible(false);
    table_->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table_->setMinimumHeight(320);
    tableLayout->addWidget(table_);
    root->addWidget(tableCard);
    root->addStretch();

    connect(pack_, &QComboBox::currentIndexChanged, this, [this] {
        values_.clear();
        rebuildTable();
        refreshTiles();
        if (running_) emit pollRequested(packKeys());
    });
    connect(read_, &QPushButton::clicked, this, [this] {
        status_->setText(tr("Lese ..."));
        emit readRequested(packKeys());
    });
    connect(live_, &QPushButton::clicked, this, [this] {
        running_ = !running_;
        if (running_) {
            emit pollRequested(packKeys());
        } else {
            emit stopRequested();
        }
        live_->setText(running_ ? tr("Anhalten") : tr("Laufend lesen"));
    });
    rebuildTable();
    setConnected(false);
}

void EvPage::setVehicle(const QString& brand, const QString& title) {
    // Pick the pack whose name mentions the model, else the brand.
    const QString hay = (brand + QLatin1Char(' ') + title).toLower();
    int best = -1;
    for (int i = 0; i < pack_->count(); ++i) {
        const QString text = pack_->itemText(i).toLower();
        for (const QString& word : {QStringLiteral("ioniq"), QStringLiteral("kona"), QStringLiteral("niro"),
                                    QStringLiteral("zoe"), QStringLiteral("leaf")}) {
            if (hay.contains(word) && text.contains(word)) best = i;
        }
        if (best < 0 && !brand.isEmpty() && text.contains(brand.toLower())) best = i;
    }
    if (best >= 0) pack_->setCurrentIndex(best);
}

void EvPage::readNow() {
    if (connected_) read_->click();
}

QVector<int> EvPage::packKeys() const {
    QVector<int> keys;
    for (const CustomPid& p : packPids(pack_->currentData().toString().toStdString())) keys << p.key;
    return keys;
}

void EvPage::rebuildTable() {
    shortNames_.clear();
    rowOf_.clear();
    const std::vector<CustomPid> list = packPids(pack_->currentData().toString().toStdString());
    table_->setRowCount(static_cast<int>(list.size()));
    int r = 0;
    for (const CustomPid& p : list) {
        shortNames_.insert(p.key, QString::fromStdString(p.shortName));
        rowOf_.insert(p.key, r);
        table_->setItem(r, 0, new QTableWidgetItem(QString::fromStdString(p.name)));
        table_->setItem(r, 1, new QTableWidgetItem(QStringLiteral("--")));
        auto* source = new QTableWidgetItem(QString::fromStdString(p.source));
        source->setForeground(color::textFaint);
        table_->setItem(r, 2, source);
        ++r;
    }
}

void EvPage::setConnected(bool connected) {
    connected_ = connected;
    read_->setEnabled(connected);
    live_->setEnabled(connected);
    if (!connected) {
        running_ = false;
        live_->setText(tr("Laufend lesen"));
    }
}

void EvPage::onLiveValue(const LiveRow& row) {
    const auto it = shortNames_.find(row.pid);
    if (it == shortNames_.end()) return;
    values_.insert(it.value(), row.value);
    const int r = rowOf_.value(row.pid, -1);
    if (r >= 0) {
        table_->item(r, 1)->setText(row.text.isEmpty()
                                        ? QStringLiteral("%1 %2").arg(num(row.value, 2), row.unit)
                                        : row.text);
    }
    refreshTiles();
}

void EvPage::onCustomRead(int ok, int failed, const QString& firstError) {
    if (ok == 0 && failed > 0) {
        status_->setText(tr("Keine Antwort vom Batteriesteuergerät: %1. Passt das Paket zum Auto, "
                            "und ist die Zündung an?").arg(firstError));
    } else if (failed > 0) {
        status_->setText(tr("%1 Werte gelesen, %2 ohne Antwort.").arg(ok).arg(failed));
    } else {
        status_->setText(tr("%n Wert(e) gelesen.", nullptr, ok));
    }
}

void EvPage::refreshTiles() {
    auto has = [this](const char* key) { return values_.contains(QString::fromLatin1(key)); };
    auto get = [this](const char* key) { return values_.value(QString::fromLatin1(key)); };
    const char* sohKey = has("SOH") ? "SOH" : "SOH LBC";
    if (has(sohKey)) {
        const double soh = get(sohKey);
        health_->setScore(qBound(0, int(std::lround(soh)), 100),
                          soh >= 90 ? tr("Wie neu") : soh >= 80 ? tr("Gut") : soh >= 70 ? tr("Gebraucht")
                                                                               : tr("Deutlich gealtert"),
                          tr("Akkugesundheit"));
    } else {
        health_->setScore(-1, tr("Akkugesundheit"), tr("noch nicht gelesen"));
    }
    const char* socKey = has("SOC display") ? "SOC display" : has("SOC") ? "SOC" : "SOC BMS";
    charge_->setText(has(socKey) ? QStringLiteral("%1 %").arg(num(get(socKey), 0)) : QStringLiteral("--"));
    chargeNote_->setText(has("SOC BMS") && has("SOC display")
                             ? tr("Anzeige %1 %, Batteriesteuergerät intern %2 %")
                                   .arg(num(get("SOC display"), 1), num(get("SOC BMS"), 1))
                             : QString());

    voltage_->setValue(has("Batt V") ? num(get("Batt V"), 1) : QStringLiteral("--"), QStringLiteral("V"));
    current_->setValue(has("Batt A") ? num(get("Batt A"), 1) : QStringLiteral("--"), QStringLiteral("A"));
    power_->setValue(has("Batt kW") ? num(get("Batt kW"), 1) : QStringLiteral("--"), QStringLiteral("kW"));
    if (has("Cell V max") && has("Cell V min")) {
        cells_->setValue(QStringLiteral("%1 / %2").arg(num(get("Cell V min"), 2), num(get("Cell V max"), 2)),
                         QStringLiteral("V"));
        const double spreadMv = has("Cell V diff") ? get("Cell V diff") * 1000.0
                                                   : (get("Cell V max") - get("Cell V min")) * 1000.0;
        // A healthy pack keeps its cells within a few hundredths of a volt.
        const QColor tone = spreadMv < 30 ? color::good : spreadMv < 80 ? color::warn : color::bad;
        spread_->setValue(num(spreadMv, 0), QStringLiteral("mV"), tone);
        spread_->setNote(spreadMv < 30 ? tr("gleichmäßig") : spreadMv < 80 ? tr("etwas ungleich")
                                                                            : tr("deutlich ungleich"));
    }
    const char* tKey = has("Batt T max") ? "Batt T max" : has("Batt T") ? "Batt T" : "Batt T avg";
    if (has(tKey)) {
        const double t = get(tKey);
        temperature_->setValue(num(t, 0), QStringLiteral("°C"),
                               t > 45 ? color::bad : t < 0 ? color::info : color::good);
        if (has("Batt T min")) temperature_->setNote(tr("kälteste Stelle %1 °C").arg(num(get("Batt T min"), 0)));
    }
    if (has("12V")) {
        const double v = get("12V");
        aux_->setValue(num(v, 1), QStringLiteral("V"), v < 12.0 ? color::warn : color::good);
    }

    // One plain sentence on what the numbers mean.
    QStringList advice;
    if (has(sohKey) && get(sohKey) < 70) {
        advice << tr("Der Akku hat deutlich an Kapazität verloren. Reichweite und Ladeleistung sind "
                     "spürbar geringer; Garantiebedingungen prüfen, viele Hersteller garantieren 70 %.");
    }
    if (has("Cell V max") && has("Cell V min") && (get("Cell V max") - get("Cell V min")) > 0.08) {
        advice << tr("Die Zellen sind ungleich geladen. Einmal langsam auf 100 % laden und einige "
                     "Stunden stehen lassen hilft dem Ausgleich; bleibt es so, in die Werkstatt.");
    }
    advice_->setText(advice.join(QStringLiteral("\n")));
    advice_->setStyleSheet(QStringLiteral("color: %1;").arg(color::warn.name()));
}

}  // namespace caruspex
