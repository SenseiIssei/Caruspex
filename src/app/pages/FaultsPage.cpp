#include "app/pages/FaultsPage.h"

#include <QDateTime>
#include <QFileDialog>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QLabel>
#include <QMessageBox>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QSplitter>
#include <QTableWidget>
#include <QTextStream>
#include <QVBoxLayout>

#include "app/Theme.h"
#include "app/VehicleDiagram.h"
#include "data/DtcLocation.h"

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
    clearButton_ = new QPushButton(tr("Fehler löschen"));
    clearButton_->setObjectName("Danger");
    clearButton_->setMinimumHeight(40);
    bruteButton_ = new QPushButton(tr("Hartnäckig löschen"));
    bruteButton_->setObjectName("Danger");
    bruteButton_->setMinimumHeight(40);
    exportButton_ = new QPushButton(tr("Als CSV speichern"));
    exportButton_->setMinimumHeight(40);
    for (QPushButton* b : {readButton_, clearButton_, bruteButton_, exportButton_}) {
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
    connect(bruteButton_, &QPushButton::clicked, this, &FaultsPage::askAndBruteForce);
    connect(exportButton_, &QPushButton::clicked, this, &FaultsPage::exportCsv);

    clearLog_ = new QPlainTextEdit;
    clearLog_->setReadOnly(true);
    clearLog_->setMaximumHeight(120);
    clearLog_->setVisible(false);
    root->addWidget(clearLog_);

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
    table_->setColumnWidth(0, 84);
    table_->setColumnWidth(1, 106);
    table_->verticalHeader()->setVisible(false);
    table_->setSelectionBehavior(QAbstractItemView::SelectRows);
    table_->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table_->setWordWrap(true);
    table_->setAlternatingRowColors(true);
    connect(table_, &QTableWidget::currentCellChanged, this,
            [this](int row, int, int, int) { showSelected(row); });

    // To the right, the car that shows where the fault comes from.
    auto* right = new QWidget;
    auto* rightLayout = new QVBoxLayout(right);
    rightLayout->setContentsMargins(0, 0, 0, 0);
    rightLayout->setSpacing(8);
    auto* diagramTitle = new QLabel(tr("Wo kommt das her?"));
    QFont bold = diagramTitle->font();
    bold.setBold(true);
    diagramTitle->setFont(bold);
    rightLayout->addWidget(diagramTitle);
    diagram_ = new VehicleDiagram;
    rightLayout->addWidget(diagram_, 1);
    whereLabel_ = new QLabel(tr("Einen Fehler in der Liste anklicken."));
    whereLabel_->setObjectName("Subtle");
    whereLabel_->setWordWrap(true);
    whereLabel_->setMinimumHeight(52);
    rightLayout->addWidget(whereLabel_);

    auto* split = new QSplitter(Qt::Horizontal);
    split->addWidget(table_);
    split->addWidget(right);
    split->setStretchFactor(0, 5);
    split->setStretchFactor(1, 4);
    split->setSizes({560, 460});
    root->addWidget(split, 1);

    auto* note = new QLabel(
        tr("Löschen setzt auch die Bereitschaftstests zurück. Direkt danach fällt eine "
           "Abgasuntersuchung durch, bis das Auto wieder ein paar vollständige Fahrzyklen "
           "gelaufen ist. Ein Fehler, dessen Ursache noch da ist, kommt nach kurzer Fahrt "
           "zurück."));
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
        code->setForeground(f.stored ? theme::color::bad : theme::color::warn);
        table_->setItem(i, 0, code);
        table_->setItem(i, 1, new QTableWidgetItem(f.kind));
        table_->setItem(i, 2, new QTableWidgetItem(f.text));
        throttle = throttle || f.throttleRelated;
    }
    table_->resizeRowsToContents();

    summary_->setText(rows.isEmpty() ? tr("Kein Eintrag im Speicher")
                                     : tr("%1 Einträge").arg(rows.size()));
    throttleHint_->setVisible(throttle);
    if (throttle) {
        throttleHint_->setText(
            tr("Mindestens ein Code betrifft die Drosselklappe. Löschen allein reicht da "
               "erfahrungsgemäß nicht, die Klappe muss angelernt werden."));
    }
    exportButton_->setEnabled(!rows.isEmpty());

    // Mark all affected spots on the car.
    QSet<int> zones;
    for (const FaultRow& f : rows) {
        const ZoneInfo info = locateDtc(f.code.toStdString());
        if (info.zone != CarZone::Unknown) zones.insert(static_cast<int>(info.zone));
    }
    diagram_->setAffected(zones);
    if (!rows.isEmpty()) {
        table_->setCurrentCell(0, 0);
        showSelected(0);
    } else {
        diagram_->clear();
        whereLabel_->setText(tr("Kein Fehler, nichts zu zeigen."));
    }
}

void FaultsPage::showSelected(int row) {
    if (row < 0 || row >= rows_.size()) return;
    const FaultRow& fault = rows_.at(row);
    const ZoneInfo info = locateDtc(fault.code.toStdString());
    diagram_->setFocus(info.zone);
    whereLabel_->setText(tr("%1 zeigt auf: %2\n%3")
                             .arg(fault.code,
                                  QString::fromStdString(info.name),
                                  QString::fromStdString(info.hint)));
}

void FaultsPage::setConnected(bool connected) {
    readButton_->setEnabled(connected);
    clearButton_->setEnabled(connected);
    bruteButton_->setEnabled(connected);
    if (!connected) {
        table_->setRowCount(0);
        rows_.clear();
        summary_->clear();
        throttleHint_->setVisible(false);
        exportButton_->setEnabled(false);
        diagram_->clear();
        whereLabel_->setText(tr("Einen Fehler in der Liste anklicken."));
    }
}

void FaultsPage::setClearOptions(int attempts, bool confirm) {
    clearAttempts_ = attempts > 0 ? attempts : 1;
    confirmClear_ = confirm;
}

void FaultsPage::askAndClear() {
    if (!confirmClear_) {
        emit clearRequested();
        return;
    }

    QMessageBox box(this);
    box.setWindowTitle(tr("Fehlerspeicher löschen"));
    box.setIcon(QMessageBox::Warning);
    box.setText(tr("Fehlerspeicher wirklich löschen?"));
    box.setInformativeText(
        tr("Die Fehlerlampe geht aus und die Bereitschaftstests werden zurückgesetzt. "
           "Eine Abgasuntersuchung fällt direkt danach durch, bis das Auto wieder ein paar "
           "Fahrzyklen gelaufen ist.\n\nLöschen ersetzt keine Reparatur."));
    QPushButton* yes = box.addButton(tr("Ja, löschen"), QMessageBox::AcceptRole);
    box.addButton(tr("Abbrechen"), QMessageBox::RejectRole);
    box.exec();
    if (box.clickedButton() == yes) emit clearRequested();
}

void FaultsPage::askAndBruteForce() {
    if (!confirmClear_) {
        clearLog_->clear();
        clearLog_->setVisible(true);
        emit bruteForceRequested(clearAttempts_);
        return;
    }

    QMessageBox box(this);
    box.setWindowTitle(tr("Hartnäckig löschen"));
    box.setIcon(QMessageBox::Warning);
    box.setText(tr("Mehrfach löschen und jedes Mal nachprüfen?"));
    box.setInformativeText(
        tr("Es wird bis zu %1-mal gelöscht. Nach jedem Versuch wird nachgelesen, ob der "
           "Speicher wirklich leer ist. Bleibt etwas stehen, wird zwischendurch die "
           "Verbindung neu aufgebaut, weil manche Steuergeräte den Löschbefehl nur "
           "direkt nach dem Wecken annehmen.\n\n"
           "Was das nicht kann: einen Fehler beseitigen, dessen Ursache noch besteht. Der "
           "steht sofort wieder drin, egal wie oft gelöscht wird. Löschen ersetzt keine "
           "Reparatur.\n\n"
           "Die Bereitschaftstests werden dabei ebenfalls zurückgesetzt.")
            .arg(clearAttempts_));
    QPushButton* yes = box.addButton(tr("Ja, hartnäckig löschen"), QMessageBox::AcceptRole);
    box.addButton(tr("Abbrechen"), QMessageBox::RejectRole);
    box.exec();
    if (box.clickedButton() == yes) {
        clearLog_->clear();
        clearLog_->setVisible(true);
        emit bruteForceRequested(clearAttempts_);
    }
}

void FaultsPage::onClearProgress(const QString& line) {
    clearLog_->setVisible(true);
    clearLog_->appendPlainText(line);
}

void FaultsPage::onClearFinished(bool everythingGone, int remaining) {
    if (everythingGone) {
        clearLog_->appendPlainText(tr("Fertig: der Speicher ist leer."));
        return;
    }
    clearLog_->appendPlainText(
        tr("Fertig: %1 Einträge bleiben stehen. Das heißt, die Ursache besteht noch. "
           "Der Reiter Fehlerspeicher zeigt oben rechts am Fahrzeug, wo zu suchen ist.")
            .arg(remaining));
}

void FaultsPage::exportCsv() {
    if (rows_.isEmpty()) return;
    const QString suggestion =
        tr("fehlerspeicher_%1.csv")
            .arg(QDateTime::currentDateTime().toString("yyyy-MM-dd_HH-mm"));
    const QString path =
        QFileDialog::getSaveFileName(this, tr("Speichern"), suggestion, tr("CSV (*.csv)"));
    if (path.isEmpty()) return;

    QFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text)) {
        QMessageBox::warning(this, tr("Fehler"), tr("Datei lässt sich nicht schreiben."));
        return;
    }
    QTextStream out(&file);
    out.setEncoding(QStringConverter::Utf8);
    out << tr("Code;Art;Bedeutung") << '\n';
    for (const FaultRow& f : std::as_const(rows_)) {
        QString text = f.text;
        out << f.code << ';' << f.kind << ';' << text.replace(';', ',') << '\n';
    }
}

}  // namespace caruspex
