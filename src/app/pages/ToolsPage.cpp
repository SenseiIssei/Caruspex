#include "app/pages/ToolsPage.h"

#include <QFrame>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QLabel>
#include <QLineEdit>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QScrollArea>
#include <QTableWidget>
#include <QVBoxLayout>

#include "app/Theme.h"

namespace caruspex {
namespace {

QFrame* card(const QString& heading, QVBoxLayout*& body) {
    auto* frame = new QFrame;
    frame->setObjectName("Card");
    auto* layout = new QVBoxLayout(frame);
    layout->setContentsMargins(18, 16, 18, 16);
    layout->setSpacing(10);
    auto* label = new QLabel(heading);
    QFont f = label->font();
    f.setBold(true);
    f.setPointSizeF(f.pointSizeF() + 1.0);
    label->setFont(f);
    layout->addWidget(label);
    body = layout;
    return frame;
}

QTableWidget* makeTable(const QStringList& headers) {
    auto* table = new QTableWidget(0, headers.size());
    table->setHorizontalHeaderLabels(headers);
    table->verticalHeader()->setVisible(false);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->setAlternatingRowColors(true);
    table->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    for (int i = 1; i < headers.size(); ++i) {
        table->horizontalHeader()->setSectionResizeMode(i, QHeaderView::ResizeToContents);
    }
    return table;
}

}  // namespace

ToolsPage::ToolsPage(QWidget* parent) : QWidget(parent) {
    auto* outer = new QVBoxLayout(this);
    outer->setContentsMargins(24, 20, 24, 20);
    outer->setSpacing(14);

    auto* title = new QLabel(tr("Pruefen und Werkzeuge"));
    title->setObjectName("PageTitle");
    outer->addWidget(title);

    loadButton_ = new QPushButton(tr("Daten vom Fahrzeug holen"));
    loadButton_->setObjectName("Primary");
    loadButton_->setMinimumHeight(40);
    loadButton_->setEnabled(false);
    auto* topRow = new QHBoxLayout;
    topRow->addWidget(loadButton_);
    topRow->addStretch();
    outer->addLayout(topRow);
    connect(loadButton_, &QPushButton::clicked, this, &ToolsPage::detailsRequested);

    auto* scroll = new QScrollArea;
    scroll->setWidgetResizable(true);
    scroll->setFrameShape(QFrame::NoFrame);
    auto* content = new QWidget;
    auto* root = new QVBoxLayout(content);
    root->setContentsMargins(0, 0, 0, 0);
    root->setSpacing(14);

    // Bereitschaftstests
    QVBoxLayout* body = nullptr;
    QFrame* readinessCard = card(tr("Bereitschaftstests (Abgasuntersuchung)"), body);
    readinessSummary_ = new QLabel(tr("Noch nichts gelesen."));
    readinessSummary_->setObjectName("Subtle");
    readinessSummary_->setWordWrap(true);
    body->addWidget(readinessSummary_);
    readiness_ = makeTable({tr("Test"), tr("Zustand")});
    readiness_->setMinimumHeight(190);
    body->addWidget(readiness_);
    root->addWidget(readinessCard);

    // Fahrzeugkennung und Umgebungsdaten
    QFrame* infoCard = card(tr("Fahrzeug und Umgebungsdaten beim Fehler"), body);
    vin_ = new QLabel(tr("Fahrgestellnummer: noch nicht gelesen"));
    body->addWidget(vin_);
    freezeTitle_ = new QLabel;
    freezeTitle_->setObjectName("Subtle");
    body->addWidget(freezeTitle_);
    freeze_ = makeTable({tr("Messwert"), tr("Wert"), tr("Einheit")});
    freeze_->setMinimumHeight(190);
    body->addWidget(freeze_);
    root->addWidget(infoCard);

    // Rohbefehle
    QFrame* rawCard = card(tr("Adapter direkt ansprechen"), body);
    auto* hint = new QLabel(
        tr("Fuer Fortgeschrittene: ELM327-Befehle wie ATI, ATDP oder 0100 direkt senden."));
    hint->setObjectName("Hint");
    hint->setWordWrap(true);
    body->addWidget(hint);
    auto* row = new QHBoxLayout;
    command_ = new QLineEdit;
    command_->setPlaceholderText(QStringLiteral("ATI"));
    sendButton_ = new QPushButton(tr("Senden"));
    sendButton_->setEnabled(false);
    row->addWidget(command_, 1);
    row->addWidget(sendButton_);
    body->addLayout(row);
    replies_ = new QPlainTextEdit;
    replies_->setReadOnly(true);
    replies_->setMinimumHeight(150);
    QFont mono(QStringLiteral("Consolas"));
    mono.setStyleHint(QFont::Monospace);
    replies_->setFont(mono);
    body->addWidget(replies_);
    root->addWidget(rawCard);
    root->addStretch();

    connect(sendButton_, &QPushButton::clicked, this, &ToolsPage::sendRaw);
    connect(command_, &QLineEdit::returnPressed, this, &ToolsPage::sendRaw);

    scroll->setWidget(content);
    outer->addWidget(scroll, 1);
}

void ToolsPage::onStatus(const StatusRow& row) {
    readiness_->setRowCount(row.monitorNames.size());
    int ready = 0;
    for (int i = 0; i < row.monitorNames.size(); ++i) {
        readiness_->setItem(i, 0, new QTableWidgetItem(row.monitorNames.at(i)));
        const bool ok = row.monitorReady.value(i);
        auto* state = new QTableWidgetItem(ok ? tr("bereit") : tr("noch nicht bereit"));
        state->setForeground(ok ? theme::color::good : theme::color::warn);
        readiness_->setItem(i, 1, state);
        if (ok) ++ready;
    }
    if (row.monitorNames.isEmpty()) {
        readinessSummary_->setText(tr("Das Steuergeraet meldet keine Bereitschaftstests."));
        return;
    }
    const int total = row.monitorNames.size();
    readinessSummary_->setText(
        ready == total
            ? tr("Alle %1 Tests sind bereit. Fuer die Abgasuntersuchung passt das.").arg(total)
            : tr("%1 von %2 Tests sind bereit. Fuer die Abgasuntersuchung muessen alle "
                 "durchlaufen sein, dafuer braucht das Auto ein paar vollstaendige "
                 "Fahrzyklen.")
                  .arg(ready)
                  .arg(total));
}

void ToolsPage::onVehicleInfo(const QString& vin, const QString& freezeCode,
                              const QVector<LiveRow>& freezeRows) {
    vin_->setText(vin.isEmpty()
                      ? tr("Fahrgestellnummer: liefert das Steuergeraet nicht "
                           "(bei Fahrzeugen vor etwa 2003 normal)")
                      : tr("Fahrgestellnummer: %1").arg(vin));

    freezeTitle_->setText(freezeCode.isEmpty()
                              ? tr("Kein Umgebungsdatensatz gespeichert.")
                              : tr("Aufgezeichnet beim Auftreten von %1:").arg(freezeCode));

    freeze_->setRowCount(freezeRows.size());
    for (int i = 0; i < freezeRows.size(); ++i) {
        const LiveRow& v = freezeRows.at(i);
        freeze_->setItem(i, 0, new QTableWidgetItem(v.name));
        freeze_->setItem(i, 1,
                         new QTableWidgetItem(v.text.isEmpty()
                                                  ? QString::number(v.value, 'f', 2)
                                                  : v.text));
        freeze_->setItem(i, 2, new QTableWidgetItem(v.unit));
    }
}

void ToolsPage::onRawReply(const QString& reply) {
    replies_->appendPlainText(reply);
}

void ToolsPage::setConnected(bool connected) {
    loadButton_->setEnabled(connected);
    sendButton_->setEnabled(connected);
    if (!connected) {
        readiness_->setRowCount(0);
        freeze_->setRowCount(0);
        readinessSummary_->setText(tr("Noch nichts gelesen."));
        vin_->setText(tr("Fahrgestellnummer: noch nicht gelesen"));
        freezeTitle_->clear();
    }
}

void ToolsPage::sendRaw() {
    const QString text = command_->text().trimmed();
    if (text.isEmpty()) return;
    replies_->appendPlainText(QStringLiteral("> ") + text);
    emit rawRequested(text);
    command_->clear();
}

}  // namespace caruspex
