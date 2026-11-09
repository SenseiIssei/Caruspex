#include "app/pages/AdapterPage.h"

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

namespace caruspex {

using namespace theme;

AdapterPage::AdapterPage(QWidget* parent) : QWidget(parent) {
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
    root->addWidget(makePageHeader(tr("Werkzeug"), tr("Adaptertest"),
                                   tr("Viele ELM327 sind Nachbauten, die eine höhere Version "
                                      "behaupten, als sie können. Der Test fragt nach, misst die "
                                      "Antwortzeit und sagt ehrlich, was er daraus schließt. "
                                      "Er schreibt nichts in den Adapter.")));

    QVBoxLayout* heroLayout = nullptr;
    QFrame* hero = makeCard(&heroLayout, 24);
    auto* top = new QHBoxLayout;
    top->setSpacing(12);
    start_ = new QPushButton(tr("Adapter testen"));
    start_->setObjectName("Primary");
    start_->setIcon(icons::icon(QStringLiteral("cpu"), Qt::white, Qt::white, 18));
    start_->setMinimumHeight(44);
    spinner_ = new Spinner(20);
    top->addWidget(start_);
    top->addWidget(spinner_);
    top->addStretch();
    heroLayout->addLayout(top);
    verdict_ = makeLabel(tr("Erst verbinden, dann testen. Dauert etwa zehn Sekunden."), "Subtle", true);
    verdict_->setStyleSheet(QStringLiteral("font-size: 16px;"));
    heroLayout->addWidget(verdict_);
    root->addWidget(hero);

    auto* tiles = new QGridLayout;
    tiles->setSpacing(14);
    version_ = new StatTile(QStringLiteral("info"), tr("Meldet sich als"));
    chip_ = new StatTile(QStringLiteral("cpu"), tr("Verhält sich wie"));
    speed_ = new StatTile(QStringLiteral("timer"), tr("Antwortzeit"));
    voltage_ = new StatTile(QStringLiteral("battery"), tr("Spannung an der Buchse"));
    int i = 0;
    for (StatTile* t : {version_, chip_, speed_, voltage_}) {
        t->clear();
        tiles->addWidget(t, 0, i++);
    }
    root->addLayout(tiles);

    auto* columns = new QHBoxLayout;
    columns->setSpacing(20);
    QVBoxLayout* factsLayout = nullptr;
    QFrame* factsCard = makeCard(&factsLayout, 22);
    factsLayout->addWidget(makeLabel(tr("Was der Test ergeben hat"), "SectionTitle"));
    facts_ = new QVBoxLayout;
    facts_->setSpacing(0);
    factsLayout->addLayout(facts_);
    factsLayout->addStretch();
    columns->addWidget(factsCard, 1);

    QVBoxLayout* checksLayout = nullptr;
    QFrame* checksCard = makeCard(&checksLayout, 22);
    checksLayout->addWidget(makeLabel(tr("Befehle im Einzelnen"), "SectionTitle"));
    checks_ = new QTableWidget(0, 4);
    checks_->setHorizontalHeaderLabels({tr("Befehl"), tr("Seit"), tr("Kann er"), tr("Wofür")});
    checks_->horizontalHeader()->setSectionResizeMode(3, QHeaderView::Stretch);
    checks_->verticalHeader()->setVisible(false);
    checks_->setEditTriggers(QAbstractItemView::NoEditTriggers);
    checks_->setSelectionMode(QAbstractItemView::NoSelection);
    checks_->setMinimumHeight(360);
    checksLayout->addWidget(checks_);
    columns->addWidget(checksCard, 1);
    root->addLayout(columns);
    root->addStretch();

    connect(start_, &QPushButton::clicked, this, [this] {
        spinner_->setRunning(true);
        start_->setEnabled(false);
        verdict_->setText(tr("Teste ..."));
        emit probeRequested();
    });
    setConnected(false);
}

void AdapterPage::setConnected(bool connected) {
    start_->setEnabled(connected);
    if (!connected) spinner_->setRunning(false);
}

void AdapterPage::onReport(const AdapterRow& row) {
    spinner_->setRunning(false);
    start_->setEnabled(true);
    verdict_->setText(row.verdict);
    const QLocale locale;
    version_->setValue(row.version.isEmpty() ? QStringLiteral("?") : row.version);
    version_->setNote(row.description);
    chip_->setValue(row.isStn ? QStringLiteral("STN") : (row.behavesLike.isEmpty() ? QStringLiteral("?")
                                                                                     : row.behavesLike));
    chip_->setNote(row.isStn ? row.stnId : QString());
    speed_->setValue(locale.toString(row.roundTripMs, 'f', 0), QStringLiteral("ms"),
                     row.roundTripMs < 60 ? color::good : row.roundTripMs < 150 ? color::warn : color::bad);
    if (row.roundTripWithHintMs > 0) {
        speed_->setNote(tr("mit Antwortzahl %1 ms").arg(locale.toString(row.roundTripWithHintMs, 'f', 0)));
    }
    if (row.hasVoltage) voltage_->setValue(locale.toString(row.voltage, 'f', 1), QStringLiteral("V"));

    while (QLayoutItem* item = facts_->takeAt(0)) {
        if (item->widget()) item->widget()->deleteLater();
        delete item;
    }
    for (const QString& f : row.facts) {
        facts_->addWidget(new FindingRow(QStringLiteral("info"), color::info, f, QString()));
    }
    for (const QString& t : row.speedTips) {
        facts_->addWidget(new FindingRow(QStringLiteral("zap"), color::accent, t, QString()));
    }

    checks_->setRowCount(row.checks.size());
    for (int r = 0; r < row.checks.size(); ++r) {
        const AdapterCheckRow& c = row.checks[r];
        auto* command = new QTableWidgetItem(c.command);
        command->setFont(mono(13, 600));
        checks_->setItem(r, 0, command);
        checks_->setItem(r, 1, new QTableWidgetItem(c.version));
        auto* ok = new QTableWidgetItem(c.supported ? tr("ja") : tr("nein"));
        ok->setForeground(c.supported ? color::good : color::bad);
        checks_->setItem(r, 2, ok);
        checks_->setItem(r, 3, new QTableWidgetItem(c.purpose));
    }
    checks_->resizeColumnsToContents();
    checks_->horizontalHeader()->setSectionResizeMode(3, QHeaderView::Stretch);
}

}  // namespace caruspex
