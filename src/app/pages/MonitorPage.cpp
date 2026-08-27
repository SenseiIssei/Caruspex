#include "app/pages/MonitorPage.h"

#include <QDateTime>
#include <QFileDialog>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QLabel>
#include <QMessageBox>
#include <QPushButton>
#include <QTextStream>
#include <QTreeWidget>
#include <QVBoxLayout>

#include "app/Theme.h"

namespace caruspex {

MonitorPage::MonitorPage(QWidget* parent) : QWidget(parent) {
    auto* root = new QVBoxLayout(this);
    root->setContentsMargins(24, 20, 24, 20);
    root->setSpacing(14);

    auto* title = new QLabel(tr("Bauteilueberwachung"));
    title->setObjectName("PageTitle");
    root->addWidget(title);

    auto* hint = new QLabel(
        tr("Das Steuergeraet prueft im Betrieb laufend seine eigenen Bauteile. Hier stehen "
           "die Ergebnisse: nicht nur bestanden oder nicht, sondern der Messwert samt "
           "erlaubter Unter- und Obergrenze. Damit sieht man, wie knapp ein Bauteil noch "
           "im Rahmen liegt, lange bevor die Fehlerlampe angeht.\n\n"
           "Ein Katalysator, dessen Wirkungsgrad hart an der Grenze kratzt, faellt hier "
           "auf. Im Fehlerspeicher steht dazu noch nichts."));
    hint->setObjectName("Hint");
    hint->setWordWrap(true);
    root->addWidget(hint);

    auto* bar = new QHBoxLayout;
    readButton_ = new QPushButton(tr("Auslesen"));
    readButton_->setObjectName("Primary");
    readButton_->setMinimumHeight(40);
    readButton_->setEnabled(false);
    exportButton_ = new QPushButton(tr("Als Bericht speichern"));
    exportButton_->setMinimumHeight(40);
    exportButton_->setEnabled(false);
    bar->addWidget(readButton_);
    bar->addWidget(exportButton_);
    bar->addStretch();
    status_ = new QLabel(tr("Noch nichts ausgelesen."));
    status_->setObjectName("Subtle");
    bar->addWidget(status_);
    root->addLayout(bar);

    connect(readButton_, &QPushButton::clicked, this, &MonitorPage::readRequested);
    connect(exportButton_, &QPushButton::clicked, this, &MonitorPage::exportReport);

    identity_ = new QLabel;
    identity_->setObjectName("Subtle");
    identity_->setWordWrap(true);
    root->addWidget(identity_);

    tree_ = new QTreeWidget;
    tree_->setColumnCount(6);
    tree_->setHeaderLabels({tr("Test"), tr("Baugruppe"), tr("Messwert"), tr("Untergrenze"),
                            tr("Obergrenze"), tr("Bewertung")});
    tree_->header()->setSectionResizeMode(0, QHeaderView::Stretch);
    for (int i = 1; i < 6; ++i) {
        tree_->header()->setSectionResizeMode(i, QHeaderView::ResizeToContents);
    }
    tree_->setAlternatingRowColors(true);
    root->addWidget(tree_, 1);
}

void MonitorPage::setConnected(bool connected) {
    readButton_->setEnabled(connected);
    if (!connected) {
        tree_->clear();
        monitors_.clear();
        oxygen_.clear();
        identity_->clear();
        exportButton_->setEnabled(false);
        status_->setText(tr("Noch nichts ausgelesen."));
    }
}

void MonitorPage::onIdentity(const QString& vin, const QString& calibration,
                             const QString& checksum, const QString& ecuName) {
    QStringList parts;
    if (!vin.isEmpty()) parts << tr("Fahrgestellnummer: %1").arg(vin);
    if (!calibration.isEmpty()) parts << tr("Kalibrierstand: %1").arg(calibration);
    if (!checksum.isEmpty()) parts << tr("Pruefsumme: %1").arg(checksum);
    if (!ecuName.isEmpty()) parts << tr("Steuergeraet: %1").arg(ecuName);
    identity_->setText(parts.isEmpty()
                           ? tr("Das Steuergeraet nennt keine Kennungen. Bei Fahrzeugen "
                                "vor etwa 2003 ist das normal.")
                           : parts.join(QStringLiteral("     ")));
}

void MonitorPage::onMonitorResults(const QVector<MonitorRow>& rows, const QString& note) {
    monitors_ = rows;
    tree_->clear();

    if (rows.isEmpty()) {
        status_->setText(note.isEmpty() ? tr("Keine Tests gemeldet.") : note);
    } else {
        int failed = 0;
        for (const MonitorRow& r : rows) {
            if (!r.passed) ++failed;
        }
        status_->setText(failed == 0
                             ? tr("%1 Tests, alle im Rahmen.").arg(rows.size())
                             : tr("%1 Tests, %2 ausserhalb der Grenzen.")
                                   .arg(rows.size())
                                   .arg(failed));
    }

    auto* group = new QTreeWidgetItem(tree_);
    group->setText(0, tr("Bauteilueberwachung (Modus 06)"));
    QFont bold = group->font(0);
    bold.setBold(true);
    group->setFont(0, bold);
    group->setForeground(0, theme::color::accent);

    for (const MonitorRow& r : rows) {
        auto* item = new QTreeWidgetItem(group);
        item->setText(0, r.name);
        item->setText(1, r.component);
        item->setText(2, QStringLiteral("%1 %2").arg(r.value, 0, 'f', 3).arg(r.unit));
        item->setText(3, r.hasMinimum ? QString::number(r.minimum, 'f', 3)
                                      : QStringLiteral("-"));
        item->setText(4, r.hasMaximum ? QString::number(r.maximum, 'f', 3)
                                      : QStringLiteral("-"));
        item->setText(5, r.passed ? tr("im Rahmen") : tr("ausserhalb"));
        item->setForeground(5, r.passed ? theme::color::good : theme::color::bad);
    }
    group->setExpanded(true);
    exportButton_->setEnabled(!rows.isEmpty() || !oxygen_.isEmpty());
}

void MonitorPage::onOxygenTests(const QVector<OxygenRow>& rows, const QString& note) {
    oxygen_ = rows;
    if (rows.isEmpty()) {
        if (monitors_.isEmpty() && !note.isEmpty()) status_->setText(note);
        return;
    }

    auto* group = new QTreeWidgetItem(tree_);
    group->setText(0, tr("Lambdasondenpruefung (Modus 05)"));
    QFont bold = group->font(0);
    bold.setBold(true);
    group->setFont(0, bold);
    group->setForeground(0, theme::color::accent);

    for (const OxygenRow& r : rows) {
        auto* item = new QTreeWidgetItem(group);
        item->setText(0, r.name);
        item->setText(1, tr("Sonde %1").arg(r.sensor));
        item->setText(2, QStringLiteral("%1 %2").arg(r.value, 0, 'f', 3).arg(r.unit));
    }
    group->setExpanded(true);
    exportButton_->setEnabled(true);
}

void MonitorPage::exportReport() {
    if (monitors_.isEmpty() && oxygen_.isEmpty()) return;
    const QString stamp = QDateTime::currentDateTime().toString("yyyy-MM-dd_HH-mm");
    const QString path = QFileDialog::getSaveFileName(
        this, tr("Bericht speichern"), QStringLiteral("bauteilpruefung_%1.txt").arg(stamp),
        tr("Textdatei (*.txt)"));
    if (path.isEmpty()) return;

    QFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text)) {
        QMessageBox::warning(this, tr("Fehler"), tr("Datei laesst sich nicht schreiben."));
        return;
    }
    QTextStream out(&file);
    out.setEncoding(QStringConverter::Utf8);
    out << "Bauteilueberwachung, ausgelesen mit Caruspex\n";
    out << QDateTime::currentDateTime().toString("dd.MM.yyyy HH:mm") << "\n";
    out << identity_->text() << "\n";
    out << QString(78, '=') << "\n\n";

    if (!monitors_.isEmpty()) {
        out << "Bauteilueberwachung (Modus 06)\n\n";
        for (const MonitorRow& r : std::as_const(monitors_)) {
            out << (r.passed ? "  [ok]   " : "  [!!]   ") << r.name << "\n";
            out << "          " << r.component << "\n";
            out << "          Messwert " << QString::number(r.value, 'f', 3) << " " << r.unit;
            if (r.hasMinimum) out << ", Untergrenze " << QString::number(r.minimum, 'f', 3);
            if (r.hasMaximum) out << ", Obergrenze " << QString::number(r.maximum, 'f', 3);
            out << "\n\n";
        }
    }
    if (!oxygen_.isEmpty()) {
        out << "Lambdasondenpruefung (Modus 05)\n\n";
        for (const OxygenRow& r : std::as_const(oxygen_)) {
            out << "  Sonde " << r.sensor << "  " << r.name << ": "
                << QString::number(r.value, 'f', 3) << " " << r.unit << "\n";
        }
    }
}

}  // namespace caruspex
