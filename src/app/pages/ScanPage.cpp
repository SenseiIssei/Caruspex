#include "app/pages/ScanPage.h"

#include <QComboBox>
#include <QDateTime>
#include <QFileDialog>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QLabel>
#include <QMessageBox>
#include <QPlainTextEdit>
#include <QProgressBar>
#include <QPushButton>
#include <QTextStream>
#include <QTreeWidget>
#include <QVBoxLayout>

#include "app/Theme.h"
#include "core/Serial.h"

namespace caruspex {

ScanPage::ScanPage(QWidget* parent) : QWidget(parent) {
    auto* root = new QVBoxLayout(this);
    root->setContentsMargins(32, 26, 32, 24);
    root->setSpacing(14);

    auto* title = new QLabel(tr("Alle Steuergeräte durchsuchen"));
    title->setObjectName("PageTitle");
    root->addWidget(title);

    auto* hint = new QLabel(
        tr("Klappert der Reihe nach jedes bekannte Steuergerät ab und zeigt, welche das "
           "Fahrzeug hat und wo etwas im Speicher steht. Jeder Versuch braucht wegen der "
           "5-Baud-Weckadresse ein paar Sekunden, der ganze Durchlauf also rund eine Minute. "
           "Dafür wird ein KKL-Kabel gebraucht, ein ELM327 kommt an die Steuergeräte "
           "außerhalb des Motors nicht heran."));
    hint->setObjectName("Hint");
    hint->setWordWrap(true);
    root->addWidget(hint);

    auto* bar = new QHBoxLayout;
    portBox_ = new QComboBox;
    portBox_->setMinimumWidth(250);
    rescanButton_ = new QPushButton(tr("Suchen"));
    startButton_ = new QPushButton(tr("Durchsuchen"));
    startButton_->setObjectName("Primary");
    startButton_->setMinimumHeight(40);
    reportButton_ = new QPushButton(tr("Bericht speichern"));
    reportButton_->setMinimumHeight(40);
    reportButton_->setEnabled(false);
    bar->addWidget(new QLabel(tr("Anschluss:")));
    bar->addWidget(portBox_, 1);
    bar->addWidget(rescanButton_);
    bar->addWidget(startButton_);
    clearAllButton_ = new QPushButton(tr("Überall löschen"));
    clearAllButton_->setObjectName("Danger");
    clearAllButton_->setMinimumHeight(40);
    bar->addWidget(clearAllButton_);
    bar->addWidget(reportButton_);
    root->addLayout(bar);

    connect(rescanButton_, &QPushButton::clicked, this, &ScanPage::refreshPorts);
    connect(reportButton_, &QPushButton::clicked, this, &ScanPage::exportReport);
    connect(startButton_, &QPushButton::clicked, this, [this] {
        if (running_) {
            emit cancelRequested();
            status_->setText(tr("Abbruch angefordert, das laufende Steuergerät wird noch "
                                "zu Ende gefragt ..."));
            return;
        }
        const QString device = portBox_->currentData().toString();
        if (device.isEmpty()) {
            QMessageBox::information(this, tr("Kein Anschluss"),
                                     tr("Erst ein KKL-Kabel anstecken und auf Suchen "
                                        "drücken."));
            return;
        }
        emit scanRequested(device, 0);
    });

    connect(clearAllButton_, &QPushButton::clicked, this, [this] {
        const QString device = portBox_->currentData().toString();
        if (device.isEmpty()) {
            QMessageBox::information(this, tr("Kein Anschluss"),
                                     tr("Erst ein KKL-Kabel anstecken und auf Suchen "
                                        "drücken."));
            return;
        }
        QMessageBox box(this);
        box.setWindowTitle(tr("Überall löschen"));
        box.setIcon(QMessageBox::Warning);
        box.setText(tr("In jedem gefundenen Steuergerät den Fehlerspeicher löschen?"));
        box.setInformativeText(
            tr("Jedes Steuergerät wird angefunkt, ausgelesen und gelöscht. Klappt das "
               "Löschen nicht, wird es bis zu %1-mal wiederholt und jedes Mal "
               "nachgeprüft.\n\n"
               "Was das nicht kann: einen Fehler beseitigen, dessen Ursache noch besteht. "
               "Der steht sofort wieder drin. Löschen ersetzt keine Reparatur.\n\n"
               "Zündung an, Motor aus. Der Durchlauf dauert einige Minuten.")
                .arg(clearAttempts_));
        QPushButton* yes = box.addButton(tr("Ja, überall löschen"), QMessageBox::AcceptRole);
        box.addButton(tr("Abbrechen"), QMessageBox::RejectRole);
        box.exec();
        if (box.clickedButton() == yes) {
            clearLog_->clear();
            clearLog_->setVisible(true);
            emit scanRequested(device, clearAttempts_);
        }
    });

    progress_ = new QProgressBar;
    progress_->setVisible(false);
    root->addWidget(progress_);

    status_ = new QLabel(tr("Noch nicht durchsucht."));
    status_->setObjectName("Subtle");
    status_->setWordWrap(true);
    root->addWidget(status_);

    clearLog_ = new QPlainTextEdit;
    clearLog_->setReadOnly(true);
    clearLog_->setMaximumHeight(130);
    clearLog_->setVisible(false);
    root->addWidget(clearLog_);

    tree_ = new QTreeWidget;
    tree_->setColumnCount(3);
    tree_->setHeaderLabels({tr("Steuergerät"), tr("Zustand"), tr("Kennung")});
    tree_->header()->setSectionResizeMode(0, QHeaderView::Fixed);
    tree_->header()->setSectionResizeMode(1, QHeaderView::Fixed);
    tree_->header()->setSectionResizeMode(2, QHeaderView::Stretch);
    tree_->setColumnWidth(0, 320);
    tree_->setColumnWidth(1, 170);
    tree_->setAlternatingRowColors(true);
    root->addWidget(tree_, 1);

    refreshPorts();
}

void ScanPage::refreshPorts() {
    portBox_->clear();
    for (const PortInfo& p : enumeratePorts()) {
        QString label = QString::fromStdString(p.device);
        if (!p.chipGuess.empty()) label += "  [" + QString::fromStdString(p.chipGuess) + "]";
        portBox_->addItem(label, QString::fromStdString(p.device));
    }
    if (portBox_->count() == 0) portBox_->addItem(tr("kein Anschluss gefunden"), QString());
}

void ScanPage::onScanStarted(int total) {
    running_ = true;
    results_.clear();
    tree_->clear();
    reportButton_->setEnabled(false);
    startButton_->setText(tr("Abbrechen"));
    progress_->setVisible(true);
    progress_->setRange(0, total);
    progress_->setValue(0);
}

void ScanPage::onClearProgress(const QString& line) {
    clearLog_->setVisible(true);
    clearLog_->appendPlainText(line);
}

void ScanPage::onScanProgress(const QString& ecuName, int index, int total) {
    progress_->setValue(index - 1);
    status_->setText(tr("Frage %1 an (%2 von %3) ...").arg(ecuName).arg(index).arg(total));
}

void ScanPage::onEcuResult(const EcuScanRow& row) {
    results_.push_back(row);
    progress_->setValue(progress_->value() + 1);

    auto* item = new QTreeWidgetItem(tree_);
    item->setText(0, row.name);
    item->setText(2, row.identification);

    if (!row.present) {
        item->setText(1, tr("nicht verbaut"));
        item->setForeground(0, theme::color::textMuted);
        item->setForeground(1, theme::color::textMuted);
        return;
    }

    if (row.faults.isEmpty()) {
        item->setText(1, tr("in Ordnung"));
        item->setForeground(1, theme::color::good);
    } else {
        item->setText(1, row.faults.size() == 1 ? tr("1 Fehler")
                                                : tr("%1 Fehler").arg(row.faults.size()));
        item->setForeground(1, theme::color::bad);
        for (const FaultRow& fault : row.faults) {
            auto* child = new QTreeWidgetItem(item);
            child->setText(0, fault.code);
            child->setText(1, fault.kind);
            child->setText(2, fault.text);
            child->setForeground(0, theme::color::bad);
        }
        item->setExpanded(true);
    }
    QFont bold = item->font(0);
    bold.setBold(true);
    item->setFont(0, bold);
}

void ScanPage::onScanFinished(int present, int withFaults, bool cancelled) {
    running_ = false;
    startButton_->setText(tr("Durchsuchen"));
    progress_->setVisible(false);
    reportButton_->setEnabled(!results_.isEmpty());

    if (cancelled) {
        status_->setText(tr("Abgebrochen. %1 Steuergeräte geprüft.").arg(results_.size()));
        return;
    }
    status_->setText(
        withFaults == 0
            ? tr("Fertig. %1 Steuergeräte gefunden, keines meldet einen Fehler.").arg(present)
            : tr("Fertig. %1 Steuergeräte gefunden, %2 davon mit Einträgen im Speicher.")
                  .arg(present)
                  .arg(withFaults));
}

void ScanPage::exportReport() {
    if (results_.isEmpty()) return;
    const QString stamp = QDateTime::currentDateTime().toString("yyyy-MM-dd_HH-mm");
    const QString path = QFileDialog::getSaveFileName(
        this, tr("Bericht speichern"), tr("prüfbericht_%1.txt").arg(stamp),
        tr("Textdatei (*.txt)"));
    if (path.isEmpty()) return;

    QFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text)) {
        QMessageBox::warning(this, tr("Fehler"), tr("Datei lässt sich nicht schreiben."));
        return;
    }
    QTextStream out(&file);
    out.setEncoding(QStringConverter::Utf8);
    out << tr("Prüfbericht Caruspex") << "\n";
    out << tr("Erstellt am %1")
               .arg(QDateTime::currentDateTime().toString(tr("dd.MM.yyyy HH:mm")))
        << "\n";
    out << QString(60, '=') << "\n\n";

    int totalFaults = 0;
    for (const EcuScanRow& row : std::as_const(results_)) {
        out << row.name << "\n";
        if (!row.present) {
            out << "   " << tr("nicht verbaut oder meldet sich nicht") << "\n\n";
            continue;
        }
        if (!row.identification.isEmpty()) {
            out << "   " << tr("Kennung: %1").arg(row.identification) << "\n";
        }
        if (row.faults.isEmpty()) {
            out << "   " << tr("kein Eintrag im Fehlerspeicher") << "\n";
        } else {
            for (const FaultRow& fault : row.faults) {
                out << "   " << fault.code << "  [" << fault.kind << "]  " << fault.text
                    << "\n";
                ++totalFaults;
            }
        }
        out << "\n";
    }
    out << QString(60, '-') << "\n";
    out << tr("Insgesamt %1 Einträge.").arg(totalFaults) << "\n";
}

}  // namespace caruspex
