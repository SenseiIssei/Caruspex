#include "app/pages/BatteryPage.h"

#include <QCheckBox>
#include <QComboBox>
#include <QFrame>
#include <QGridLayout>
#include <QHBoxLayout>
#include <QLabel>
#include <QLocale>
#include <QPushButton>
#include <QScrollArea>
#include <QVBoxLayout>

#include <algorithm>

#include "app/GaugeWidget.h"
#include "app/Icons.h"
#include "app/Session.h"
#include "app/Theme.h"
#include "app/Widgets.h"
#include "app/pages/HealthPage.h"

namespace caruspex {

using namespace theme;

BatteryPage::BatteryPage(QWidget* parent) : QWidget(parent) {
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
    root->addWidget(makePageHeader(tr("Messen"), tr("Batterie und Lichtmaschine"),
                                   tr("Vier kurze Messungen, Schritt für Schritt. Die Spannung "
                                      "misst der Adapter direkt an der Buchse, das klappt bei "
                                      "jedem Auto.")));

    auto* columns = new QHBoxLayout;
    columns->setSpacing(20);

    // Left: gauge and battery details
    QVBoxLayout* gaugeLayout = nullptr;
    QFrame* gaugeCard = makeCard(&gaugeLayout, 22);
    gaugeLayout->addWidget(makeLabel(tr("Spannung jetzt"), "SectionTitle"));
    gauge_ = new GaugeWidget;
    gauge_->configure(tr("Bordnetz"), QStringLiteral("V"), 6, 16, 2);
    gauge_->setWarnRange(12.2, 14.9);
    gauge_->setMinimumSize(240, 240);
    gaugeLayout->addWidget(gauge_, 0, Qt::AlignHCenter);
    type_ = new QComboBox;
    type_->addItem(tr("Batterieart unbekannt"), static_cast<int>(analysis::BatteryType::Unknown));
    type_->addItem(tr("Nassbatterie (Standard)"), static_cast<int>(analysis::BatteryType::Flooded));
    type_->addItem(tr("EFB (Start-Stopp)"), static_cast<int>(analysis::BatteryType::Efb));
    type_->addItem(tr("AGM (Start-Stopp, Vlies)"), static_cast<int>(analysis::BatteryType::Agm));
    startStop_ = new QCheckBox(tr("Auto hat Start-Stopp"));
    gaugeLayout->addWidget(type_);
    gaugeLayout->addWidget(startStop_);
    gaugeLayout->addStretch();
    columns->addWidget(gaugeCard, 2);

    // Right: the four steps
    QVBoxLayout* stepsLayout = nullptr;
    QFrame* stepsCard = makeCard(&stepsLayout, 22);
    stepsLayout->addWidget(makeLabel(tr("So geht's"), "SectionTitle"));
    const auto& steps = analysis::batteryTestSteps();
    const Recording order[] = {Recording::Resting, Recording::Cranking, Recording::Idle,
                               Recording::Loaded};
    const int durations[] = {4, 12, 6, 6};
    const QString titles[] = {tr("Ruhespannung"), tr("Beim Anlassen"), tr("Laden im Leerlauf"),
                              tr("Laden unter Last")};
    for (int i = 0; i < 4 && i < static_cast<int>(steps.size()); ++i) {
        auto* row = new QHBoxLayout;
        row->setSpacing(12);
        auto* texts = new QVBoxLayout;
        texts->setSpacing(2);
        auto* title = makeLabel(QStringLiteral("%1. %2").arg(i + 1).arg(titles[i]));
        title->setStyleSheet(QStringLiteral("font-weight: 600; font-size: 15px;"));
        texts->addWidget(title);
        texts->addWidget(makeLabel(QString::fromUtf8(steps[i].instruction.get()), "Subtle", true));
        auto* value = makeLabel(QString(), "Hint");
        texts->addWidget(value);
        row->addLayout(texts, 1);
        auto* button = new QPushButton(i == 1 ? tr("Aufnehmen, dann starten") : tr("Messen"));
        button->setMinimumHeight(38);
        button->setIcon(icons::icon(QStringLiteral("bolt-circle"), color::textMuted, color::text, 16));
        row->addWidget(button, 0, Qt::AlignTop);
        stepsLayout->addLayout(row);
        if (i < 3) {
            auto* divider = new QFrame;
            divider->setObjectName("Divider");
            stepsLayout->addWidget(divider);
        }
        stepButtons_ << button;
        stepValues_ << value;
        const Recording what = order[i];
        const int seconds = durations[i];
        connect(button, &QPushButton::clicked, this, [this, what, seconds] { record(what, seconds); });
    }
    columns->addWidget(stepsCard, 3);
    root->addLayout(columns);

    QVBoxLayout* resultLayout = nullptr;
    QFrame* resultCard = makeCard(&resultLayout, 22);
    resultLayout->addWidget(makeLabel(tr("Ergebnis"), "SectionTitle"));
    summary_ = makeLabel(tr("Nach der ersten Messung steht hier, wie es um Batterie und "
                            "Lichtmaschine steht."),
                         "Subtle", true);
    resultLayout->addWidget(summary_);
    results_ = new QVBoxLayout;
    results_->setSpacing(0);
    resultLayout->addLayout(results_);
    root->addWidget(resultCard);
    root->addStretch();

    connect(type_, &QComboBox::currentIndexChanged, this, &BatteryPage::evaluate);
    connect(startStop_, &QCheckBox::toggled, this, &BatteryPage::evaluate);
    setConnected(false);
}

void BatteryPage::setConnected(bool connected) {
    connected_ = connected;
    for (QPushButton* b : std::as_const(stepButtons_)) b->setEnabled(connected);
    if (!connected) {
        recording_ = Recording::None;
        gauge_->setActive(false);
    }
}

void BatteryPage::record(Recording what, int seconds) {
    recording_ = what;
    buffer_.clear();
    recordUntil_ = appSeconds() + seconds;
    for (QPushButton* b : std::as_const(stepButtons_)) b->setEnabled(false);
    const int index = static_cast<int>(what) - 1;
    stepValues_[index]->setText(what == Recording::Cranking
                                    ? tr("Nimmt %1 s auf. Jetzt den Motor starten.").arg(seconds)
                                    : tr("Misst %1 s ...").arg(seconds));
    // Fast polling while cranking catches the lowest point.
    emit pollRequested(what == Recording::Cranking ? 40 : 200);
}

void BatteryPage::onVoltage(double volts, double time) {
    gauge_->setValue(volts);
    if (recording_ == Recording::None) return;
    buffer_.push_back({time, volts});
    if (time >= recordUntil_) finishRecording();
}

void BatteryPage::finishRecording() {
    const Recording what = recording_;
    recording_ = Recording::None;
    // Keep watching slowly, so the gauge stays alive.
    emit pollRequested(400);
    for (QPushButton* b : std::as_const(stepButtons_)) b->setEnabled(connected_);
    if (buffer_.empty()) return;

    double sum = 0.0;
    double lowest = buffer_.front().volts;
    for (const auto& s : buffer_) {
        sum += s.volts;
        lowest = std::min(lowest, s.volts);
    }
    const double mean = sum / buffer_.size();
    const QLocale locale;
    const int index = static_cast<int>(what) - 1;
    switch (what) {
        case Recording::Resting:
            input_.restingVolts = mean;
            stepValues_[index]->setText(tr("gemessen: %1 V").arg(locale.toString(mean, 'f', 2)));
            break;
        case Recording::Cranking:
            input_.cranking = buffer_;
            input_.crankingMinimumVolts = lowest;
            stepValues_[index]->setText(tr("tiefster Wert: %1 V").arg(locale.toString(lowest, 'f', 2)));
            break;
        case Recording::Idle:
            input_.idleVolts = mean;
            stepValues_[index]->setText(tr("gemessen: %1 V").arg(locale.toString(mean, 'f', 2)));
            break;
        case Recording::Loaded:
            input_.loadedVolts = mean;
            stepValues_[index]->setText(tr("gemessen: %1 V").arg(locale.toString(mean, 'f', 2)));
            break;
        case Recording::None:
            break;
    }
    evaluate();
}

void BatteryPage::evaluate() {
    if (!input_.restingVolts && !input_.crankingMinimumVolts && !input_.idleVolts &&
        !input_.loadedVolts) {
        return;
    }
    input_.type = static_cast<analysis::BatteryType>(type_->currentData().toInt());
    input_.startStop = startStop_->isChecked();
    const analysis::BatteryReport report = analysis::evaluateBattery(input_);
    QString summary = QString::fromStdString(report.summary.text());
    if (report.stateOfChargePercent) {
        summary += QStringLiteral(" ") + tr("Ladezustand etwa %1 %.").arg(*report.stateOfChargePercent);
    }
    summary_->setText(summary);
    while (QLayoutItem* item = results_->takeAt(0)) {
        if (item->widget()) item->widget()->deleteLater();
        delete item;
    }
    for (const analysis::BatteryStepResult& step : report.steps) {
        const int level = static_cast<int>(step.level);
        QString text = QString::fromStdString(step.explanation.text());
        const QString advice = QString::fromStdString(step.advice.text());
        if (!advice.isEmpty()) text += QStringLiteral("\n") + tr("Was tun: %1").arg(advice);
        results_->addWidget(new FindingRow(levelIcon(level), levelColour(level),
                                           QString::fromStdString(step.title.text()), text,
                                           QStringLiteral("%1 V").arg(QLocale().toString(step.volts, 'f', 2))));
    }
}

}  // namespace caruspex
