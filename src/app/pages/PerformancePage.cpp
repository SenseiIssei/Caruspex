#include "app/pages/PerformancePage.h"

#include <QFrame>
#include <QGridLayout>
#include <QHBoxLayout>
#include <QLabel>
#include <QLocale>
#include <QPushButton>
#include <QScrollArea>
#include <QTimer>
#include <QVBoxLayout>

#include "app/Icons.h"
#include "app/Theme.h"
#include "app/Widgets.h"

namespace caruspex {

using namespace theme;

PerformancePage::PerformancePage(QWidget* parent) : QWidget(parent) {
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
    root->addWidget(makePageHeader(tr("Messen"), tr("Leistungsmessung"),
                                   tr("Stoppuhr aus dem Tachosignal: 0 bis 100, 80 bis 120, "
                                      "Viertelmeile und Bremsweg. Während der Messung fragt "
                                      "Caruspex nur das Tempo ab, so oft der Adapter kann.")));

    // Safety first, and plainly.
    auto* warning = new QFrame;
    warning->setStyleSheet(QStringLiteral("QFrame { background: %1; border: 1px solid %2; "
                                          "border-radius: 12px; } QLabel { border: none; }")
                               .arg(rgba(color::warn, 0.08), rgba(color::warn, 0.35)));
    auto* warningLayout = new QHBoxLayout(warning);
    warningLayout->setContentsMargins(16, 12, 16, 12);
    warningLayout->setSpacing(12);
    auto* warningIcon = new QLabel;
    warningIcon->setPixmap(icons::pixmap(QStringLiteral("alert"), 20, color::warn));
    warningLayout->addWidget(warningIcon, 0, Qt::AlignTop);
    warningLayout->addWidget(
        makeLabel(tr("Nur auf abgesperrter Strecke oder Rennstrecke messen, nie im "
                     "Straßenverkehr. Den Laptop bedient der Beifahrer, nicht wer fährt."),
                  nullptr, true),
        1);
    root->addWidget(warning);

    QVBoxLayout* heroLayout = nullptr;
    QFrame* hero = makeCard(&heroLayout, 26);
    auto* heroRow = new QHBoxLayout;
    heroRow->setSpacing(28);
    auto* left = new QVBoxLayout;
    left->setSpacing(4);
    left->addWidget(makeLabel(tr("TEMPO"), "Eyebrow"));
    auto* speedRow = new QHBoxLayout;
    speed_ = new QLabel(QStringLiteral("--"));
    styleFont(speed_, Family::Display, 84, 800);
    auto* unit = new QLabel(QStringLiteral("km/h"));
    unit->setStyleSheet(QStringLiteral("color: %1; font-size: 20px; font-weight: 600;")
                            .arg(color::textMuted.name()));
    speedRow->addWidget(speed_, 0, Qt::AlignBottom);
    speedRow->addWidget(unit, 0, Qt::AlignBottom);
    speedRow->addStretch();
    left->addLayout(speedRow);
    state_ = makeLabel(tr("Bereit machen: anhalten, dann auf Messen drücken."), "Subtle", true);
    left->addWidget(state_);
    accuracy_ = makeLabel(QString(), "Hint", true);
    left->addWidget(accuracy_);
    left->addSpacing(8);
    start_ = new QPushButton(tr("Messen"));
    start_->setObjectName("Primary");
    start_->setIcon(icons::icon(QStringLiteral("timer"), Qt::white, Qt::white, 18));
    start_->setMinimumHeight(46);
    start_->setMaximumWidth(220);
    left->addWidget(start_);
    heroRow->addLayout(left, 1);
    heroLayout->addLayout(heroRow);
    root->addWidget(hero);

    auto* grid = new QGridLayout;
    grid->setSpacing(14);
    zeroHundred_ = new StatTile(QStringLiteral("timer"), tr("0 bis 100 km/h"));
    eightyTwenty_ = new StatTile(QStringLiteral("timer"), tr("80 bis 120 km/h"));
    zeroSixty_ = new StatTile(QStringLiteral("timer"), tr("0 bis 60 mph"));
    quarter_ = new StatTile(QStringLiteral("route"), tr("Viertelmeile (402 m)"));
    braking_ = new StatTile(QStringLiteral("stop"), tr("Bremsen 100 bis 0"));
    const QList<StatTile*> tiles = {zeroHundred_, eightyTwenty_, zeroSixty_, quarter_, braking_};
    for (int i = 0; i < tiles.size(); ++i) {
        tiles[i]->clear();
        grid->addWidget(tiles[i], 0, i);
    }
    root->addLayout(grid);
    root->addStretch();

    evaluateTimer_ = new QTimer(this);
    evaluateTimer_->setInterval(700);
    connect(evaluateTimer_, &QTimer::timeout, this, &PerformancePage::evaluate);
    connect(start_, &QPushButton::clicked, this, &PerformancePage::toggle);
    setConnected(false);
}

void PerformancePage::setConnected(bool connected) {
    connected_ = connected;
    start_->setEnabled(connected);
    if (!connected && running_) toggle();
}

void PerformancePage::toggle() {
    running_ = !running_;
    if (running_) {
        samples_.clear();
        for (StatTile* t : {zeroHundred_, eightyTwenty_, zeroSixty_, quarter_, braking_}) t->clear();
        emit startRequested({0x0D});
        evaluateTimer_->start();
        start_->setText(tr("Messung beenden"));
        start_->setIcon(icons::icon(QStringLiteral("stop"), Qt::white, Qt::white, 18));
        state_->setText(tr("Misst. Aus dem Stand losfahren oder bei 80 km/h durchbeschleunigen."));
    } else {
        emit stopRequested();
        evaluateTimer_->stop();
        evaluate();
        start_->setText(tr("Messen"));
        start_->setIcon(icons::icon(QStringLiteral("timer"), Qt::white, Qt::white, 18));
        state_->setText(tr("Messung beendet. Die Zeiten bleiben stehen, bis neu gemessen wird."));
    }
}

void PerformancePage::onLiveValue(const LiveRow& row) {
    if (row.pid != 0x0D) return;
    speed_->setText(QString::number(row.value, 'f', 0));
    if (running_) samples_.push_back({row.time, row.value});
}

void PerformancePage::showRun(StatTile* tile, const analysis::TimedRun& run, bool distance) {
    if (!run.found) return;
    const QLocale locale;
    if (distance && run.trapSpeedKmh <= 0.0) {
        tile->setValue(locale.toString(run.distanceMeters, 'f', 1), QStringLiteral("m"),
                       run.reliable ? color::good : color::warn);
        tile->setNote(tr("in %1 s").arg(locale.toString(run.seconds, 'f', 2)));
    } else {
        tile->setValue(locale.toString(run.seconds, 'f', 2), QStringLiteral("s"),
                       run.reliable ? color::good : color::warn);
        if (run.trapSpeedKmh > 0.0) {
            tile->setNote(tr("Ziel mit %1 km/h").arg(locale.toString(run.trapSpeedKmh, 'f', 0)));
        } else if (!run.reliable) {
            tile->setNote(tr("Lücken in der Messung, ungenau"));
        }
    }
}

void PerformancePage::evaluate() {
    if (samples_.size() < 3) return;
    const analysis::PerformanceReport report = analysis::analyzePerformance(samples_);
    showRun(zeroHundred_, report.zeroTo100, false);
    showRun(eightyTwenty_, report.eightyTo120, false);
    showRun(zeroSixty_, report.zeroTo60mph, false);
    showRun(quarter_, report.quarterMile, false);
    showRun(braking_, report.brake100To0, true);
    accuracy_->setText(tr("%1 Messungen pro Sekunde, Genauigkeit etwa ±%2 s. %3")
                           .arg(QLocale().toString(report.sampleRateHz, 'f', 1))
                           .arg(QLocale().toString(report.accuracySeconds, 'f', 2))
                           .arg(QString::fromStdString(report.accuracyHint.text())));
}

}  // namespace caruspex
