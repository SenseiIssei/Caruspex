#include "app/pages/HealthPage.h"

#include <QComboBox>
#include <QFrame>
#include <QHBoxLayout>
#include <QLabel>
#include <QProgressBar>
#include <QPushButton>
#include <QScrollArea>
#include <QVBoxLayout>

#include "app/Icons.h"
#include "app/Theme.h"
#include "app/Widgets.h"

namespace caruspex {

using namespace theme;

QString levelIcon(int level) {
    switch (level) {
        case 0: return QStringLiteral("check-circle");
        case 1: return QStringLiteral("info");
        case 2: return QStringLiteral("alert");
        default: return QStringLiteral("x-circle");
    }
}

QColor levelColour(int level) {
    switch (level) {
        case 0: return color::good;
        case 1: return color::info;
        case 2: return color::warn;
        default: return color::bad;
    }
}

QString levelLabel(int level) {
    switch (level) {
        case 0: return HealthPage::tr("In Ordnung");
        case 1: return HealthPage::tr("Hinweis");
        case 2: return HealthPage::tr("Bald ansehen");
        default: return HealthPage::tr("Dringend");
    }
}

HealthPage::HealthPage(QWidget* parent) : QWidget(parent) {
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
    root->addWidget(makePageHeader(tr("Diagnose"), tr("Gesundheitscheck"),
                                   tr("Liest Fehlerspeicher, Bereitschaftstests und die "
                                      "Bauteilprüfung, beobachtet dann eine halbe Minute lang "
                                      "Gemisch, Temperatur, Lambdasonden und Ladespannung. "
                                      "Heraus kommt eine Punktzahl, und jeder Abzug wird erklärt.")));

    // --- Hero ----------------------------------------------------------------
    QVBoxLayout* heroLayout = nullptr;
    QFrame* hero = makeCard(&heroLayout, 26);
    auto* heroRow = new QHBoxLayout;
    heroRow->setSpacing(28);
    ring_ = new ScoreRing;
    ring_->setFixedSize(230, 230);
    ring_->setScore(-1, tr("Noch offen"), tr("Gesundheit"));
    heroRow->addWidget(ring_, 0, Qt::AlignTop);
    auto* right = new QVBoxLayout;
    right->setSpacing(10);
    headline_ = makeLabel(tr("Bereit für den Check"), "Headline", true);
    summary_ = makeLabel(tr("Am besten mit warmem Motor im Stand. Während der Messung einmal "
                            "für ein paar Sekunden 2500 1/min halten, dann sieht Caruspex auch, "
                            "wie das Gemisch unter Last geregelt wird."),
                         "Subtle", true);
    right->addWidget(headline_);
    right->addWidget(summary_);
    right->addSpacing(6);
    auto* controls = new QHBoxLayout;
    controls->setSpacing(10);
    start_ = new QPushButton(tr("Check starten"));
    start_->setObjectName("Primary");
    start_->setIcon(icons::icon(QStringLiteral("sparkles"), Qt::white, Qt::white, 18));
    start_->setIconSize(QSize(18, 18));
    start_->setMinimumHeight(44);
    start_->setCursor(Qt::PointingHandCursor);
    duration_ = new QComboBox;
    duration_->addItem(tr("Kurz, 15 Sekunden"), 15);
    duration_->addItem(tr("Normal, 30 Sekunden"), 30);
    duration_->addItem(tr("Gründlich, 90 Sekunden"), 90);
    duration_->setCurrentIndex(1);
    duration_->setMinimumHeight(44);
    cancel_ = new QPushButton(tr("Abbrechen"));
    cancel_->setObjectName("Ghost");
    cancel_->setVisible(false);
    controls->addWidget(start_);
    controls->addWidget(duration_);
    controls->addWidget(cancel_);
    controls->addStretch();
    right->addLayout(controls);
    progress_ = new QProgressBar;
    progress_->setRange(0, 100);
    progress_->setVisible(false);
    step_ = makeLabel(QString(), "Hint");
    right->addWidget(progress_);
    right->addWidget(step_);
    right->addStretch();
    heroRow->addLayout(right, 1);
    heroLayout->addLayout(heroRow);
    root->addWidget(hero);

    connect(start_, &QPushButton::clicked, this, [this] {
        setRunning(true);
        emit startRequested(duration_->currentData().toInt());
    });
    connect(cancel_, &QPushButton::clicked, this, &HealthPage::cancelRequested);

    // --- Findings and readiness --------------------------------------------------
    auto* columns = new QHBoxLayout;
    columns->setSpacing(20);

    QVBoxLayout* findingsLayout = nullptr;
    QFrame* findingsCard = makeCard(&findingsLayout, 22);
    findingsLayout->addWidget(makeLabel(tr("Befunde"), "SectionTitle"));
    findingsLayout->addWidget(makeLabel(tr("Das Wichtigste zuerst. Jeder Punkt sagt, was er "
                                           "bedeutet und was jetzt zu tun ist."),
                                        "Subtle", true));
    findings_ = new QVBoxLayout;
    findings_->setSpacing(0);
    findingsLayout->addLayout(findings_);
    findingsLayout->addStretch();
    columns->addWidget(findingsCard, 3);

    auto* side = new QVBoxLayout;
    side->setSpacing(20);
    QVBoxLayout* monitorLayout = nullptr;
    QFrame* monitorCard = makeCard(&monitorLayout, 22);
    monitorLayout->addWidget(makeLabel(tr("Bereit für die AU?"), "SectionTitle"));
    monitorSummary_ = makeLabel(tr("Die Bereitschaftstests zeigen, ob das Auto seine "
                                   "Selbstprüfungen seit dem letzten Löschen abgeschlossen hat."),
                                "Subtle", true);
    monitorLayout->addWidget(monitorSummary_);
    monitors_ = new QVBoxLayout;
    monitors_->setSpacing(0);
    monitorLayout->addLayout(monitors_);
    side->addWidget(monitorCard);

    QVBoxLayout* skippedLayout = nullptr;
    QFrame* skippedCard = makeCard(&skippedLayout, 22);
    skippedCard_ = skippedCard;
    skippedLayout->addWidget(makeLabel(tr("Nicht geprüft"), "SectionTitle"));
    skippedLayout->addWidget(makeLabel(tr("Was dieses Auto nicht meldet oder was die Messung "
                                          "nicht hergab."),
                                       "Subtle", true));
    skipped_ = new QVBoxLayout;
    skipped_->setSpacing(4);
    skippedLayout->addLayout(skipped_);
    skippedCard->setVisible(false);
    side->addWidget(skippedCard);
    side->addStretch();
    columns->addLayout(side, 2);
    root->addLayout(columns);
    root->addStretch();

    findings_->addWidget(new FindingRow(QStringLiteral("info"), color::info,
                                        tr("Noch kein Ergebnis"),
                                        tr("Nach dem Check steht hier jeder Befund.")));
    setConnected(false);
}

void HealthPage::clearList(QVBoxLayout* list) {
    while (QLayoutItem* item = list->takeAt(0)) {
        if (item->widget()) item->widget()->deleteLater();
        delete item;
    }
}

void HealthPage::setConnected(bool connected) {
    connected_ = connected;
    start_->setEnabled(connected && !running_);
    start_->setToolTip(connected ? QString() : tr("Erst verbinden."));
    if (!connected && running_) setRunning(false);
}

void HealthPage::setRunning(bool running) {
    running_ = running;
    ring_->setBusy(running);
    start_->setEnabled(!running && connected_);
    duration_->setEnabled(!running);
    cancel_->setVisible(running);
    progress_->setVisible(running);
    if (running) {
        progress_->setValue(0);
        headline_->setText(tr("Prüfe ..."));
    }
}

void HealthPage::onProgress(const QString& step, int percent) {
    if (!running_) setRunning(true);
    progress_->setValue(percent);
    step_->setText(step);
}

void HealthPage::onResult(const HealthRow& row) {
    setRunning(false);
    step_->setText(tr("%n Prüfungen ausgeführt.", nullptr, row.checksRun));
    const QString caption = row.grade == 0   ? tr("Gut in Schuss")
                            : row.grade == 1 ? tr("Braucht Pflege")
                                             : tr("Bitte ansehen");
    ring_->setScore(row.score, caption, tr("Gesundheit"));
    headline_->setText(caption);
    summary_->setText(row.summary);

    clearList(findings_);
    int delay = 0;
    for (const HealthFindingRow& f : row.findings) {
        QString text = f.explanation;
        if (!f.advice.isEmpty()) text += QStringLiteral("\n") + tr("Was tun: %1").arg(f.advice);
        auto* finding = new FindingRow(levelIcon(f.level), levelColour(f.level), f.title, text,
                                       levelLabel(f.level));
        findings_->addWidget(finding);
        fadeIn(finding, delay);
        delay += 50;
    }
    if (row.findings.isEmpty()) {
        findings_->addWidget(new FindingRow(QStringLiteral("check-circle"), color::good,
                                            tr("Nichts gefunden"),
                                            tr("Alle Prüfungen, die dieses Auto zulässt, sind "
                                               "unauffällig.")));
    }

    clearList(monitors_);
    int open = 0;
    for (const MonitorGuideRow& m : row.monitors) {
        if (!m.complete) ++open;
    }
    if (row.monitors.isEmpty()) {
        monitorSummary_->setText(tr("Das Auto hat keine Bereitschaftstests gemeldet."));
    } else if (open == 0) {
        monitorSummary_->setText(tr("Alle Tests abgeschlossen. Von dieser Seite steht der AU "
                                    "nichts im Weg."));
    } else {
        monitorSummary_->setText(
            row.diesel ? tr("%n Test(s) noch offen. Beim Diesel sollten für die AU möglichst "
                            "alle abgeschlossen sein. Unten steht, wie man sie fährt.",
                            nullptr, open)
                       : tr("%n Test(s) noch offen. Beim Benziner wird ein offener Test oft "
                            "akzeptiert, sicherer ist es, vorher die Prüfstelle zu fragen. "
                            "Unten steht, wie man sie fährt.",
                            nullptr, open));
    }
    // Open ones first, with their drive pattern.
    for (int pass = 0; pass < 2; ++pass) {
        for (const MonitorGuideRow& m : row.monitors) {
            if ((pass == 0) == m.complete) continue;
            if (m.complete) {
                monitors_->addWidget(new FindingRow(QStringLiteral("check-circle"), color::good,
                                                    m.name, QString(), tr("fertig")));
            } else {
                QString text = m.explanation + QStringLiteral("\n") +
                               tr("So fahren: %1").arg(m.drivePattern);
                if (!m.typicalTime.isEmpty()) text += QStringLiteral(" (") + m.typicalTime + ')';
                if (m.coldStart) text += QStringLiteral("\n") + tr("Braucht einen Kaltstart, also "
                                                                    "erst nach einigen Stunden "
                                                                    "Standzeit beginnen.");
                monitors_->addWidget(new FindingRow(QStringLiteral("clock"), color::warn, m.name,
                                                    text, tr("offen")));
            }
        }
    }

    clearList(skipped_);
    for (const QString& s : row.notChecked) {
        auto* label = makeLabel(QStringLiteral("·  ") + s, "Hint", true);
        skipped_->addWidget(label);
    }
    skippedCard_->setVisible(!row.notChecked.isEmpty());
}

}  // namespace caruspex
