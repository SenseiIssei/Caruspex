#include "app/pages/DashboardPage.h"

#include <QFrame>
#include <QGridLayout>
#include <QGraphicsOpacityEffect>
#include <QHBoxLayout>
#include <QLabel>
#include <QPropertyAnimation>
#include <QPushButton>
#include <QVBoxLayout>

#include "app/GaugeWidget.h"
#include "app/Theme.h"

namespace caruspex {
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
    {0x05, QT_TRANSLATE_NOOP("caruspex::DashboardPage", "Kühlmittel"),
     QT_TRANSLATE_NOOP("caruspex::DashboardPage", "Grad C"), -40, 130, 0, true, 70, 105},
    {0x04, QT_TRANSLATE_NOOP("caruspex::DashboardPage", "Motorlast"), "%", 0, 100, 0, false, 0,
     0},
    {0x11, QT_TRANSLATE_NOOP("caruspex::DashboardPage", "Drosselklappe"), "%", 0, 100, 0, false,
     0, 0},
    {0x42, QT_TRANSLATE_NOOP("caruspex::DashboardPage", "Bordspannung"), "V", 8, 16, 1, true,
     11.8, 15.0},
};

QFrame* makeCard() {
    auto* frame = new QFrame;
    frame->setObjectName("Card");
    return frame;
}

}  // namespace

DashboardPage::DashboardPage(QWidget* parent) : QWidget(parent) {
    auto* root = new QVBoxLayout(this);
    root->setContentsMargins(24, 20, 24, 20);
    root->setSpacing(18);

    auto* title = new QLabel(tr("Übersicht"));
    title->setObjectName("PageTitle");
    root->addWidget(title);

    // Status card
    card_ = makeCard();
    auto* cardLayout = new QVBoxLayout(card_);
    cardLayout->setContentsMargins(22, 18, 22, 18);
    cardLayout->setSpacing(6);
    headline_ = new QLabel(tr("Nicht verbunden"));
    headline_->setObjectName("Headline");
    subline_ = new QLabel(tr("Links Fahrzeug und Adapter wählen, dann auf Verbinden drücken."));
    subline_->setObjectName("Subtle");
    subline_->setWordWrap(true);
    connectionInfo_ = new QLabel();
    connectionInfo_->setObjectName("Hint");
    cardLayout->addWidget(headline_);
    cardLayout->addWidget(subline_);
    cardLayout->addWidget(connectionInfo_);
    root->addWidget(card_);

    // Button row
    auto* buttons = new QHBoxLayout;
    buttons->setSpacing(10);
    readButton_ = new QPushButton(tr("Fehler auslesen"));
    readButton_->setObjectName("Primary");
    readButton_->setMinimumHeight(42);
    detailsButton_ = new QPushButton(tr("Details laden"));
    detailsButton_->setMinimumHeight(42);
    liveButton_ = new QPushButton(tr("Messung starten"));
    liveButton_->setMinimumHeight(42);
    for (QPushButton* b : {readButton_, detailsButton_, liveButton_}) {
        b->setEnabled(false);
        buttons->addWidget(b);
    }
    buttons->addStretch();
    root->addLayout(buttons);

    connect(readButton_, &QPushButton::clicked, this, &DashboardPage::readRequested);
    connect(detailsButton_, &QPushButton::clicked, this, &DashboardPage::detailsRequested);
    connect(liveButton_, &QPushButton::clicked, this, &DashboardPage::liveRequested);

    // Gauges
    auto* gaugeCard = makeCard();
    auto* grid = new QGridLayout(gaugeCard);
    grid->setContentsMargins(18, 18, 18, 18);
    grid->setSpacing(14);
    int column = 0;
    for (const GaugeSpec& spec : kGauges) {
        auto* g = new GaugeWidget;
        g->configure(tr(spec.title), tr(spec.unit), spec.minimum, spec.maximum,
                     spec.decimals);
        if (spec.hasWarn) g->setWarnRange(spec.warnFrom, spec.warnTo);
        grid->addWidget(g, 0, column++);
        gauges_.insert(spec.pid, g);
    }
    root->addWidget(gaugeCard);
    root->addStretch();
}

GaugeWidget* DashboardPage::gauge(int pid) {
    const auto it = gauges_.find(pid);
    return it == gauges_.end() ? nullptr : it.value();
}

void DashboardPage::onConnected(const QString& adapter, const QString& transport,
                                const QString& protocol) {
    connected_ = true;
    for (QPushButton* b : {readButton_, detailsButton_, liveButton_}) b->setEnabled(true);
    connectionInfo_->setText(tr("Adapter: %1     Weg: %2     Protokoll: %3")
                                 .arg(adapter, transport, protocol));
    updateHeadline();
}

void DashboardPage::onDisconnected() {
    connected_ = false;
    haveRead_ = false;
    mil_ = false;
    faultCount_ = 0;
    for (QPushButton* b : {readButton_, detailsButton_, liveButton_}) b->setEnabled(false);
    connectionInfo_->clear();
    for (GaugeWidget* g : std::as_const(gauges_)) g->setActive(false);
    updateHeadline();
}

void DashboardPage::onStatus(const StatusRow& row) {
    if (!row.valid) return;
    mil_ = row.mil;
    updateHeadline();
}

void DashboardPage::onFaults(const QVector<FaultRow>& rows) {
    haveRead_ = true;
    faultCount_ = rows.size();
    updateHeadline();
}

void DashboardPage::onLiveValue(const LiveRow& row) {
    if (GaugeWidget* g = gauge(row.pid)) g->setValue(row.value);
}

void DashboardPage::updateHeadline() {
    QColor colour = theme::color::accent;
    if (!connected_) {
        headline_->setText(tr("Nicht verbunden"));
        subline_->setText(
            tr("Links Fahrzeug und Adapter wählen, dann auf Verbinden drücken."));
    } else if (!haveRead_) {
        headline_->setText(tr("Verbunden"));
        subline_->setText(tr("Auf Fehler auslesen drücken."));
    } else if (faultCount_ == 0 && !mil_) {
        colour = theme::color::good;
        headline_->setText(tr("Alles in Ordnung"));
        subline_->setText(tr("Kein Fehler gespeichert, Fehlerlampe ist aus."));
    } else {
        colour = theme::color::bad;
        headline_->setText(faultCount_ == 1 ? tr("1 Fehler gespeichert")
                                            : tr("%1 Fehler gespeichert").arg(faultCount_));
        subline_->setText(mil_ ? tr("Die Fehlerlampe im Tacho brennt.")
                               : tr("Die Fehlerlampe brennt nicht, der Speicher ist trotzdem "
                                    "nicht leer."));
    }

    headline_->setStyleSheet(QStringLiteral("color: %1; font-weight: 700;")
                                 .arg(colour.name()));
    theme::addGlow(headline_, colour, 1.0);
    card_->setStyleSheet(QStringLiteral("QFrame#Card { border: 1px solid %1; background: %2; }")
                             .arg(colour.darker(160).name(),
                                  QColor(colour.red() / 9 + 20, colour.green() / 9 + 22,
                                         colour.blue() / 9 + 28)
                                      .name()));

    // Short fade-in so a change gets noticed.
    auto* effect = new QGraphicsOpacityEffect(card_);
    card_->setGraphicsEffect(effect);
    auto* fade = new QPropertyAnimation(effect, "opacity", card_);
    fade->setDuration(320);
    fade->setStartValue(0.45);
    fade->setEndValue(1.0);
    connect(fade, &QPropertyAnimation::finished, card_, [this] { card_->setGraphicsEffect(nullptr); });
    fade->start(QAbstractAnimation::DeleteWhenStopped);
}

}  // namespace caruspex
