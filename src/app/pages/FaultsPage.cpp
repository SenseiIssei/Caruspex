#include "app/pages/FaultsPage.h"

#include <QDateTime>
#include <QFileDialog>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QLabel>
#include <QMessageBox>
#include <QPushButton>
#include <QTableWidget>
#include <QTextStream>
#include <QVBoxLayout>

#include "app/Theme.h"

namespace caruspex {

FaultsPage::FaultsPage(QWidget* parent) : QWidget(parent) {
    auto* root = new QVBoxLayout(this);
    root->setContentsMargins(24, 20, 24, 20);
    root->setSpacing(14);

    auto* title = new QLabel(tr("Fehlerspeicher"));
    title->setObjectName("PageTitle");
    root->addWidget(title);

    auto* buttons = new QHBoxLayout;
    buttons->setSpacing(10);
    readButton_ = new QPushButton(tr("Neu einlesen"));
    readButton_->setObjectName("Primary");
    readButton_->setMinimumHeight(40);
    clearButton_ = new QPushButton(tr("Fehler loeschen"));
    clearButton_->setObjectName("Danger");
    clearButton_->setMinimumHeight(40);
    exportButton_ = new QPushButton(tr("Als CSV speichern"));
    exportButton_->setMinimumHeight(40);
    for (QPushButton* b : {readButton_, clearButton_, exportButton_}) {
        b->setEnabled(false);
        buttons->addWidget(b);
    }
    buttons->addStretch();
    summary_ = new QLabel;
    summary_->setObjectName("Subtle");
    buttons->addWidget(summary_);
    root->addLayout(buttons);

    connect(readButton_, &QPushButton::clicked, this, &FaultsPage::readRequested);
    connect(clearButton_, &QPushButton::clicked, this, &FaultsPage::askAndClear);
    connect(exportButton_, &QPushButton::clicked, this, &FaultsPage::exportCsv);

    throttleHint_ = new QLabel;
    throttleHint_->setWordWrap(true);
    throttleHint_->setVisible(false);
    throttleHint_->setStyleSheet(
        QStringLiteral("color: %1;").arg(theme::color::warn.name()));
    root->addWidget(throttleHint_);

    table_ = new QTableWidget(0, 3);
    table_->setHorizontalHeaderLabels({tr("Code"), tr("Art"), tr("Bedeutung")});
    table_->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Fixed);
    table_->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Fixed);
    table_->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Stretch);
    table_->setColumnWidth(0, 110);
    table_->setColumnWidth(1, 140);
    table_->verticalHeader()->setVisible(false);
    table_->setSelectionBehavior(QAbstractItemView::SelectRows);
    table_->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table_->setWordWrap(true);
    table_->setAlternatingRowColors(true);
    root->addWidget(table_, 1);

    auto* note = new QLabel(
        tr("Loeschen setzt auch die Bereitschaftstests zurueck. Direkt danach faellt eine "
           "Abgasuntersuchung durch, bis das Auto wieder ein paar vollstaendige Fahrzyklen "
           "gelaufen ist. Ein Fehler, dessen Ursache noch da ist, kommt nach kurzer Fahrt "
           "zurueck."));
    note->setObjectName("Hint");
    note->setWordWrap(true);
    root->addWidget(note);
}

void FaultsPage::onFaults(const QVector<FaultRow>& rows) {
    rows_ = rows;
    table_->setRowCount(rows.size());
    bool throttle = false;

    for (int i = 0; i < rows.size(); ++i) {
        const FaultRow& f = rows[i];
        auto* code = new QTableWidgetItem(f.code);
        QFont bold = code->font();
        bold.setBold(true);
        code->setFont(bold);
        code->setForeground(f.kind == QLatin1String("gespeichert") ? theme::color::bad
                                                                   : theme::color::warn);
        table_->setItem(i, 0, code);
        table_->setItem(i, 1, new QTableWidgetItem(f.kind));
        table_->setItem(i, 2, new QTableWidgetItem(f.text));
        throttle = throttle || f.throttleRelated;
    }
    table_->resizeRowsToContents();

    summary_->setText(rows.isEmpty() ? tr("Kein Eintrag im Speicher")
                                     : tr("%1 Eintraege").arg(rows.size()));
    throttleHint_->setVisible(throttle);
    if (throttle) {
        throttleHint_->setText(
            tr("Mindestens ein Code betrifft die Drosselklappe. Loeschen allein reicht da "
               "erfahrungsgemaess nicht, die Klappe muss angelernt werden."));
    }
    exportButton_->setEnabled(!rows.isEmpty());
}

void FaultsPage::setConnected(bool connected) {
    readButton_->setEnabled(connected);
    clearButton_->setEnabled(connected);
    if (!connected) {
        table_->setRowCount(0);
        rows_.clear();
        summary_->clear();
        throttleHint_->setVisible(false);
        exportButton_->setEnabled(false);
    }
}

void FaultsPage::askAndClear() {
    QMessageBox box(this);
    box.setWindowTitle(tr("Fehlerspeicher loeschen"));
    box.setIcon(QMessageBox::Warning);
    box.setText(tr("Fehlerspeicher wirklich loeschen?"));
    box.setInformativeText(
        tr("Die Fehlerlampe geht aus und die Bereitschaftstests werden zurueckgesetzt. "
           "Eine Abgasuntersuchung faellt direkt danach durch, bis das Auto wieder ein paar "
           "Fahrzyklen gelaufen ist.\n\nLoeschen ersetzt keine Reparatur."));
    QPushButton* yes = box.addButton(tr("Ja, loeschen"), QMessageBox::AcceptRole);
    box.addButton(tr("Abbrechen"), QMessageBox::RejectRole);
    box.exec();
    if (box.clickedButton() == yes) emit clearRequested();
}

void FaultsPage::exportCsv() {
    if (rows_.isEmpty()) return;
    const QString suggestion =
        QStringLiteral("fehlerspeicher_%1.csv")
            .arg(QDateTime::currentDateTime().toString("yyyy-MM-dd_HH-mm"));
    const QString path =
        QFileDialog::getSaveFileName(this, tr("Speichern"), suggestion, tr("CSV (*.csv)"));
    if (path.isEmpty()) return;

    QFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text)) {
        QMessageBox::warning(this, tr("Fehler"), tr("Datei laesst sich nicht schreiben."));
        return;
    }
    QTextStream out(&file);
    out.setEncoding(QStringConverter::Utf8);
    out << "Code;Art;Bedeutung\n";
    for (const FaultRow& f : std::as_const(rows_)) {
        QString text = f.text;
        out << f.code << ';' << f.kind << ';' << text.replace(';', ',') << '\n';
    }
}

}  // namespace caruspex
