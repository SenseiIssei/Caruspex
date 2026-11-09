#include "app/pages/TripPage.h"

#include <QComboBox>
#include <QDoubleSpinBox>
#include <QFrame>
#include <QGridLayout>
#include <QHBoxLayout>
#include <QLabel>
#include <QLocale>
#include <QPushButton>
#include <QScrollArea>
#include <QSettings>
#include <QTimer>
#include <QVBoxLayout>

#include "app/Icons.h"
#include "app/Theme.h"
#include "app/Widgets.h"
#include "core/analysis/TripComputer.h"

namespace caruspex {

using namespace theme;

namespace {

// What the trip computer can use. The worker skips what the car lacks.
const QVector<int> kTripPids = {0x0D, 0x5E, 0x10, 0x0C, 0x0B, 0x0F, 0x04, 0x2F};

QString num(double v, int decimals) { return QLocale().toString(v, 'f', decimals); }

}  // namespace

TripPage::TripPage(QWidget* parent) : QWidget(parent) {
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
    root->addWidget(makePageHeader(tr("Messen"), tr("Bordcomputer"),
                                   tr("Verbrauch, Kosten und Fahrstil, auch bei Autos, die "
                                      "selbst keinen Bordcomputer haben. Am genauesten mit dem "
                                      "Kraftstoffdurchfluss, sonst aus der Luftmasse gerechnet.")));

    // --- Hero: live consumption and eco ring -------------------------------------
    QVBoxLayout* heroLayout = nullptr;
    QFrame* hero = makeCard(&heroLayout, 26);
    auto* heroRow = new QHBoxLayout;
    heroRow->setSpacing(28);
    auto* live = new QVBoxLayout;
    live->setSpacing(4);
    liveCaption_ = makeLabel(tr("VERBRAUCH JETZT"), "Eyebrow");
    live->addWidget(liveCaption_);
    auto* valueRow = new QHBoxLayout;
    valueRow->setSpacing(10);
    liveValue_ = new QLabel(QStringLiteral("--"));
    liveValue_->setFont(display(64, 800));
    liveUnit_ = new QLabel(QStringLiteral("l/100 km"));
    liveUnit_->setStyleSheet(QStringLiteral("color: %1; font-size: 18px; font-weight: 600;")
                                 .arg(color::textMuted.name()));
    valueRow->addWidget(liveValue_, 0, Qt::AlignBottom);
    valueRow->addWidget(liveUnit_, 0, Qt::AlignBottom);
    valueRow->addStretch();
    live->addLayout(valueRow);
    fuelNote_ = makeLabel(QString(), "Hint", true);
    live->addWidget(fuelNote_);
    live->addSpacing(10);

    auto* controls = new QHBoxLayout;
    controls->setSpacing(10);
    start_ = new QPushButton(tr("Fahrt starten"));
    start_->setObjectName("Primary");
    start_->setIcon(icons::icon(QStringLiteral("play"), Qt::white, Qt::white, 16));
    start_->setMinimumHeight(44);
    reset_ = new QPushButton(tr("Zurücksetzen"));
    reset_->setIcon(icons::icon(QStringLiteral("refresh"), color::textMuted, color::text, 16));
    reset_->setMinimumHeight(44);
    controls->addWidget(start_);
    controls->addWidget(reset_);
    controls->addStretch();
    live->addLayout(controls);

    auto* setup = new QHBoxLayout;
    setup->setSpacing(10);
    fuelType_ = new QComboBox;
    fuelType_->addItem(tr("Benzin"), static_cast<int>(analysis::FuelType::Petrol));
    fuelType_->addItem(tr("Diesel"), static_cast<int>(analysis::FuelType::Diesel));
    price_ = new QDoubleSpinBox;
    price_->setRange(0.0, 5.0);
    price_->setDecimals(3);
    price_->setSingleStep(0.01);
    price_->setSuffix(tr(" €/l"));
    displacement_ = new QDoubleSpinBox;
    displacement_->setRange(0.0, 8.0);
    displacement_->setDecimals(1);
    displacement_->setSingleStep(0.1);
    displacement_->setSuffix(tr(" l Hubraum"));
    displacement_->setToolTip(tr("Nur nötig, wenn das Auto weder Durchfluss noch Luftmasse meldet."));
    QSettings store(QStringLiteral("Caruspex"), QStringLiteral("Caruspex"));
    price_->setValue(store.value(QStringLiteral("bordcomputer/preis"), 1.799).toDouble());
    displacement_->setValue(store.value(QStringLiteral("bordcomputer/hubraum"), 1.4).toDouble());
    fuelType_->setCurrentIndex(store.value(QStringLiteral("bordcomputer/kraftstoff"), 0).toInt());
    setup->addWidget(fuelType_);
    setup->addWidget(price_);
    setup->addWidget(displacement_);
    setup->addStretch();
    live->addLayout(setup);
    heroRow->addLayout(live, 3);

    auto* ecoBox = new QVBoxLayout;
    ecoBox->setSpacing(6);
    eco_ = new ScoreRing;
    eco_->setFixedSize(190, 190);
    eco_->setScore(-1, tr("Fahrstil"), tr("noch keine Fahrt"));
    ecoBox->addWidget(eco_, 0, Qt::AlignHCenter);
    ecoText_ = makeLabel(QString(), "Hint", true);
    ecoText_->setMaximumWidth(320);
    ecoBox->addWidget(ecoText_);
    heroRow->addLayout(ecoBox, 2);
    heroLayout->addLayout(heroRow);
    root->addWidget(hero);

    // --- Tiles -----------------------------------------------------------------------
    auto* grid = new QGridLayout;
    grid->setSpacing(14);
    distance_ = new StatTile(QStringLiteral("route"), tr("Strecke"));
    average_ = new StatTile(QStringLiteral("fuel"), tr("Verbrauch im Schnitt"));
    fuel_ = new StatTile(QStringLiteral("fuel"), tr("Kraftstoff verbraucht"));
    cost_ = new StatTile(QStringLiteral("file"), tr("Kosten"));
    co2_ = new StatTile(QStringLiteral("leaf"), tr("CO₂"));
    speed_ = new StatTile(QStringLiteral("gauge"), tr("Tempo im Schnitt"));
    maxSpeed_ = new StatTile(QStringLiteral("zap"), tr("Höchsttempo"));
    idle_ = new StatTile(QStringLiteral("clock"), tr("Leerlauf"));
    const QList<StatTile*> tiles = {distance_, average_, fuel_, cost_, co2_, speed_, maxSpeed_, idle_};
    for (int i = 0; i < tiles.size(); ++i) {
        tiles[i]->clear();
        grid->addWidget(tiles[i], i / 4, i % 4);
    }
    root->addLayout(grid);
    root->addStretch();

    refresh_ = new QTimer(this);
    refresh_->setInterval(500);
    connect(refresh_, &QTimer::timeout, this, &TripPage::refresh);
    connect(start_, &QPushButton::clicked, this, &TripPage::toggle);
    connect(reset_, &QPushButton::clicked, this, &TripPage::resetTrip);
    auto remember = [this] {
        QSettings s(QStringLiteral("Caruspex"), QStringLiteral("Caruspex"));
        s.setValue(QStringLiteral("bordcomputer/preis"), price_->value());
        s.setValue(QStringLiteral("bordcomputer/hubraum"), displacement_->value());
        s.setValue(QStringLiteral("bordcomputer/kraftstoff"), fuelType_->currentIndex());
        if (!running_) rebuildComputer();
    };
    connect(price_, &QDoubleSpinBox::valueChanged, this, remember);
    connect(displacement_, &QDoubleSpinBox::valueChanged, this, remember);
    connect(fuelType_, &QComboBox::currentIndexChanged, this, remember);

    rebuildComputer();
    setConnected(false);
}

TripPage::~TripPage() = default;

void TripPage::rebuildComputer() {
    analysis::TripConfig config;
    config.fuel = static_cast<analysis::FuelType>(fuelType_->currentData().toInt());
    config.fuelPricePerLitre = price_->value();
    config.displacementLitres = displacement_->value();
    trip_ = std::make_unique<analysis::TripComputer>(config);
}

void TripPage::handOverTrip() {
    if (!trip_) return;
    const analysis::TripSummary s = trip_->summary();
    // Short hops are not worth a line in the garage.
    if (s.distanceKm < 0.3) return;
    GarageTrip t;
    t.start = tripStart_.isValid() ? tripStart_ : QDateTime::currentDateTime();
    t.end = QDateTime::currentDateTime();
    t.distanceKm = s.distanceKm;
    t.fuelLitres = s.fuelLitres;
    t.averageL100 = s.averageLitresPer100Km;
    t.cost = s.cost;
    t.co2Kg = s.co2Kg;
    t.maxSpeedKmh = s.maxSpeedKmh;
    t.ecoScore = s.distanceKm > 0.3 ? s.eco.total : -1;
    emit tripCompleted(t);
    tripStart_ = QDateTime();
}

void TripPage::setConnected(bool connected) {
    if (!connected && connected_) handOverTrip();
    connected_ = connected;
    if (!connected && running_) {
        running_ = false;
        refresh_->stop();
        start_->setText(tr("Fahrt starten"));
        start_->setIcon(icons::icon(QStringLiteral("play"), Qt::white, Qt::white, 16));
    }
    start_->setEnabled(connected);
}

void TripPage::onSupportedPids(const QVector<int>& pids) {
    supported_ = QSet<int>(pids.begin(), pids.end());
}

void TripPage::toggle() {
    running_ = !running_;
    if (running_) {
        if (!tripStart_.isValid()) tripStart_ = QDateTime::currentDateTime();
        QVector<int> pids;
        for (int pid : kTripPids) {
            if (supported_.isEmpty() || supported_.contains(pid)) pids << pid;
        }
        emit startRequested(pids);
        refresh_->start();
        start_->setText(tr("Fahrt anhalten"));
        start_->setIcon(icons::icon(QStringLiteral("pause"), Qt::white, Qt::white, 16));
        fuelType_->setEnabled(false);
        displacement_->setEnabled(false);
    } else {
        emit stopRequested();
        refresh_->stop();
        refresh();
        start_->setText(tr("Weiterfahren"));
        start_->setIcon(icons::icon(QStringLiteral("play"), Qt::white, Qt::white, 16));
    }
}

void TripPage::resetTrip() {
    handOverTrip();
    rebuildComputer();
    fuelType_->setEnabled(true);
    displacement_->setEnabled(true);
    if (!running_) start_->setText(tr("Fahrt starten"));
    liveValue_->setText(QStringLiteral("--"));
    eco_->setScore(-1, tr("Fahrstil"), tr("noch keine Fahrt"));
    ecoText_->clear();
    fuelNote_->clear();
    for (StatTile* t : {distance_, average_, fuel_, cost_, co2_, speed_, maxSpeed_, idle_}) t->clear();
}

void TripPage::onLiveValue(const LiveRow& row) {
    if (!running_ || !trip_) return;
    trip_->addSample(row.pid, row.value, row.time);
}

void TripPage::refresh() {
    if (!trip_) return;
    const analysis::TripSummary s = trip_->summary();
    const bool standing = s.liveLitresPer100Km <= 0.0 && s.liveLitresPerHour > 0.0;
    if (standing) {
        liveValue_->setText(num(s.liveLitresPerHour, 1));
        liveUnit_->setText(QStringLiteral("l/h"));
        liveCaption_->setText(tr("VERBRAUCH IM STAND"));
    } else {
        liveValue_->setText(s.liveLitresPer100Km > 0 ? num(s.liveLitresPer100Km, 1)
                                                     : QStringLiteral("--"));
        liveUnit_->setText(QStringLiteral("l/100 km"));
        liveCaption_->setText(tr("VERBRAUCH JETZT"));
    }
    fuelNote_->setText(QString::fromStdString(s.fuelNote.text()));

    distance_->setValue(num(s.distanceKm, s.distanceKm < 10 ? 2 : 1), QStringLiteral("km"));
    average_->setValue(s.averageLitresPer100Km > 0 ? num(s.averageLitresPer100Km, 1)
                                                   : QStringLiteral("--"),
                       QStringLiteral("l/100 km"));
    fuel_->setValue(num(s.fuelLitres, 2), QStringLiteral("l"));
    cost_->setValue(price_->value() > 0 ? num(s.cost, 2) : QStringLiteral("--"), QStringLiteral("€"));
    co2_->setValue(s.co2GramsPerKm > 0 ? num(s.co2GramsPerKm, 0) : QStringLiteral("--"),
                   QStringLiteral("g/km"));
    co2_->setNote(tr("%1 kg gesamt").arg(num(s.co2Kg, 2)));
    speed_->setValue(num(s.averageSpeedKmh, 0), QStringLiteral("km/h"));
    maxSpeed_->setValue(num(s.maxSpeedKmh, 0), QStringLiteral("km/h"));
    idle_->setValue(num(s.idleShare * 100.0, 0), QStringLiteral("%"),
                    s.idleShare > 0.25 ? color::warn : color::good);
    if (s.idleLitresPerHour > 0) idle_->setNote(tr("%1 l/h im Stand").arg(num(s.idleLitresPerHour, 2)));

    if (s.distanceKm > 0.3) {
        eco_->setScore(s.eco.total, tr("Fahrstil"),
                       tr("%1 starke Beschleunigungen").arg(s.eco.harshAccelerations));
        ecoText_->setText(QString::fromStdString(s.eco.explanation.text()));
    }
}

}  // namespace caruspex
