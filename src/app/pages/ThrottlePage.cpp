#include "app/pages/ThrottlePage.h"

#include <QComboBox>
#include <QFrame>
#include <QGridLayout>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QLabel>
#include <QMessageBox>
#include <QProgressBar>
#include <QPushButton>
#include <QScrollArea>
#include <QTableWidget>
#include <QVBoxLayout>

#include "app/GaugeWidget.h"
#include "app/Theme.h"
#include "app/Widgets.h"
#include "core/Kwp1281.h"
#include "core/Serial.h"

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

}  // namespace

ThrottlePage::ThrottlePage(QWidget* parent) : QWidget(parent) {
    auto* outer = new QVBoxLayout(this);
    outer->setContentsMargins(32, 26, 32, 24);
    outer->setSpacing(14);

    outer->addWidget(makeLabel(tr("WERKSTATT"), "Eyebrow"));

    auto* title = new QLabel(tr("Drosselklappe anlernen"));

    title->setObjectName("PageTitle");

    outer->addWidget(title);


    auto* scroll = new QScrollArea;
    scroll->setWidgetResizable(true);
    scroll->setFrameShape(QFrame::NoFrame);
    auto* content = new QWidget;
    auto* root = new QVBoxLayout(content);
    root->setContentsMargins(0, 0, 0, 0);
    root->setSpacing(14);

    // --- What works with what ----------------------------------------------
    QVBoxLayout* body = nullptr;
    QFrame* intro = card(tr("Zwei Wege, je nach Adapter"), body);
    auto* explain = new QLabel(
        tr("Die Grundeinstellung ist keine EOBD-Funktion. Über einen ELM327-Adapter kommt "
           "man nicht daran. Mit einem KKL-Kabel (VAG 409.1) geht es, Caruspex spricht die "
           "VAG-Protokolle selbst.\n\n"
           "Welches Protokoll das Steuergerät spricht, verrät es nach der Weckadresse "
           "selbst, und Caruspex richtet sich danach:\n"
           "  - KWP1281, das alte Blockprotokoll. Üblich beim 1.4 mit "
           "Saugrohreinspritzung. Dort gibt es einen eigenen Befehl für die "
           "Grundeinstellung.\n"
           "  - KWP2000, der Nachfolger. Üblich beim 1.6 FSI mit Bosch MED. Dort wird "
           "stattdessen eine andere Sitzungsart eröffnet.\n\n"
           "Der Messwertblock hängt am Motor: 060 bei Saugrohreinspritzung, 098 bei "
           "Direkteinspritzung. Meldet das Steuergerät, dass es den Block nicht kennt, "
           "nimm den anderen."));
    explain->setObjectName("Subtle");
    explain->setWordWrap(true);
    body->addWidget(explain);
    root->addWidget(intro);

    // --- KKL connection -----------------------------------------------------
    QFrame* kklCard = card(tr("KKL-Kabel verbinden"), body);
    auto* portRow = new QHBoxLayout;
    portBox_ = new QComboBox;
    portBox_->setMinimumWidth(260);
    rescanButton_ = new QPushButton(tr("Suchen"));
    connectButton_ = new QPushButton(tr("Verbinden"));
    connectButton_->setObjectName("Primary");
    connectButton_->setMinimumHeight(38);
    portRow->addWidget(new QLabel(tr("Anschluss:")));
    portRow->addWidget(portBox_, 1);
    portRow->addWidget(rescanButton_);
    portRow->addWidget(connectButton_);
    body->addLayout(portRow);

    kklState_ = new QLabel(tr("nicht verbunden"));
    kklState_->setStyleSheet(
        QStringLiteral("color: %1;").arg(theme::color::textMuted.name()));
    identification_ = new QLabel;
    identification_->setObjectName("Hint");
    identification_->setWordWrap(true);
    body->addWidget(kklState_);
    body->addWidget(identification_);
    root->addWidget(kklCard);

    connect(rescanButton_, &QPushButton::clicked, this, &ThrottlePage::refreshPorts);
    connect(connectButton_, &QPushButton::clicked, this, [this] {
        if (kklConnected_) {
            emit kklDisconnectRequested();
            return;
        }
        const QString port = portBox_->currentData().toString();
        if (port.isEmpty()) {
            QMessageBox::information(this, tr("Kein Anschluss"),
                                     tr("Erst ein KKL-Kabel anstecken und auf Suchen "
                                        "drücken."));
            return;
        }
        emit kklConnectRequested(port, static_cast<int>(Ecu::Engine));
    });

    // --- Prerequisites ------------------------------------------------------
    QFrame* checks = card(tr("Vorher prüfen"), body);
    checklistHint_ = new QLabel(
        tr("Damit das Anlernen sauber durchläuft, müssen diese Punkte stimmen:\n"
           "  - Motor betriebswarm, mindestens 80 Grad Kühlmitteltemperatur\n"
           "  - Alle Verbraucher aus: Licht, Gebläse, Klima, Heckscheibe\n"
           "  - Lenkrad geradeaus, Getriebe in Leerlauf, Handbremse angezogen\n"
           "  - Gaspedal nicht antippen, während die Grundeinstellung läuft\n"
           "  - Fehlerspeicher vorher löschen, sonst bricht das Steuergerät ab"));
    checklistHint_->setObjectName("Subtle");
    checklistHint_->setWordWrap(true);
    body->addWidget(checklistHint_);
    root->addWidget(checks);

    // --- Basic settings -----------------------------------------------------
    QFrame* runCard = card(tr("Grundeinstellung starten"), body);
    auto* runRow = new QHBoxLayout;
    groupBox_ = new QComboBox;
    groupBox_->addItem(tr("Messwertblock 060 (Saugrohreinspritzung, z.B. 1.4 MPI)"), 60);
    groupBox_->addItem(tr("Messwertblock 098 (Direkteinspritzung, z.B. 1.6 FSI)"), 98);
    groupBox_->addItem(tr("Messwertblock 001 (nur beobachten, verändert nichts)"), 1);
    runButton_ = new QPushButton(tr("Grundeinstellung starten"));
    runButton_->setObjectName("Primary");
    runButton_->setMinimumHeight(40);
    runButton_->setEnabled(false);
    runRow->addWidget(groupBox_, 1);
    runRow->addWidget(runButton_);
    body->addLayout(runRow);

    // Only visible with KWP2000: there is no dedicated service for basic
    // settings there; a different session type is opened instead.
    // Which one the control unit expects is not consistent.
    // The whole row is hidden, not just the combo box, otherwise the label
    // would remain without content.
    auto* sessionWidget = new QWidget;
    auto* sessionRow = new QHBoxLayout(sessionWidget);
    sessionRow->setContentsMargins(0, 0, 0, 0);
    sessionBox_ = new QComboBox;
    sessionBox_->addItem(tr("Sitzung 0x85 (Anpassung, üblich)"), 0x85);
    sessionBox_->addItem(tr("Sitzung 0x86"), 0x86);
    sessionBox_->addItem(tr("Sitzung 0x87"), 0x87);
    sessionBox_->addItem(tr("Sitzung 0x89 (Diagnose, nur lesen)"), 0x89);
    sessionRow->addWidget(new QLabel(tr("Sitzungsart:")));
    sessionRow->addWidget(sessionBox_, 1);
    sessionRow->addStretch();
    body->addWidget(sessionWidget);
    sessionRow_ = sessionWidget;
    connect(sessionBox_, &QComboBox::currentIndexChanged, this, [this](int) {
        emit basicSettingSessionChanged(sessionBox_->currentData().toInt());
    });

    sessionHint_ = new QLabel(
        tr("Steuergeräte mit KWP2000 kennen keinen eigenen Befehl für die "
           "Grundeinstellung. Stattdessen wird eine andere Sitzungsart eröffnet und "
           "derselbe Messwertblock gelesen, das Steuergerät regelt dann mit. Welche "
           "Sitzungsart es verlangt, steht in keinem öffentlichen Papier. Lehnt es die "
           "erste ab, nimm die nächste."));
    sessionHint_->setObjectName("Hint");
    sessionHint_->setWordWrap(true);
    body->addWidget(sessionHint_);

    activity_ = new QProgressBar;
    activity_->setRange(0, 0);
    activity_->setTextVisible(false);
    activity_->setVisible(false);
    body->addWidget(activity_);

    message_ = new QLabel;
    message_->setWordWrap(true);
    message_->setObjectName("Subtle");
    body->addWidget(message_);

    auto* gauges = new QHBoxLayout;
    angleGauge_ = new GaugeWidget;
    angleGauge_->configure(tr("Klappenwinkel"), tr("%"), 0, 100, 1);
    rpmGauge_ = new GaugeWidget;
    rpmGauge_->configure(tr("Leerlauf"), tr("1/min"), 0, 3000, 0);
    rpmGauge_->setWarnRange(600, 1100);
    gauges->addWidget(angleGauge_);
    gauges->addWidget(rpmGauge_);
    gauges->addStretch();
    body->addLayout(gauges);

    values_ = new QTableWidget(0, 3);
    values_->setHorizontalHeaderLabels({tr("Feld"), tr("Wert"), tr("Einheit")});
    values_->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    values_->verticalHeader()->setVisible(false);
    values_->setEditTriggers(QAbstractItemView::NoEditTriggers);
    values_->setMinimumHeight(170);
    body->addWidget(values_);
    root->addWidget(runCard);

    connect(runButton_, &QPushButton::clicked, this, [this] {
        if (running_) {
            emit basicSettingStopRequested();
            return;
        }
        emit basicSettingRequested(groupBox_->currentData().toInt());
    });

    // --- Without a scan tool ------------------------------------------------
    QFrame* manual = card(tr("Anlernversuch ohne Tester"), body);
    eobdHint_ = new QLabel(
        tr("Wenn kein KKL-Kabel da ist, klappt das bei vielen VAG-Motoren auch so. "
           "Garantiert ist es nicht.\n\n"
           "  1. Fehlerspeicher löschen\n"
           "  2. Zündung aus, Batterie 30 Minuten abklemmen, wieder anklemmen\n"
           "  3. Zündung an, 30 Sekunden warten, Motor NICHT starten. Die Klappe fährt "
           "dabei hörbar ihre Endanschläge ab\n"
           "  4. Motor starten, 3 bis 5 Minuten im Leerlauf warm laufen lassen, Gaspedal "
           "nicht antippen\n"
           "  5. Fehlerspeicher erneut auslesen"));
    eobdHint_->setObjectName("Subtle");
    eobdHint_->setWordWrap(true);
    body->addWidget(eobdHint_);
    root->addWidget(manual);
    root->addStretch();

    scroll->setWidget(content);
    outer->addWidget(scroll, 1);

    refreshPorts();
    sessionRow_->setVisible(false);
    sessionHint_->setVisible(false);
    updateAvailability();
}

void ThrottlePage::refreshPorts() {
    portBox_->clear();
    for (const PortInfo& p : enumeratePorts()) {
        QString label = QString::fromStdString(p.device);
        if (!p.chipGuess.empty()) {
            label += "  [" + QString::fromStdString(p.chipGuess) + "]";
        } else if (!p.description.empty()) {
            label += "  " + QString::fromStdString(p.description);
        }
        portBox_->addItem(label, QString::fromStdString(p.device));
    }
    if (portBox_->count() == 0) portBox_->addItem(tr("kein Anschluss gefunden"), QString());
}

void ThrottlePage::setEobdConnected(bool connected) {
    eobdConnected_ = connected;
    updateAvailability();
}

void ThrottlePage::setKklConnected(bool connected, const QString& identification) {
    kklConnected_ = connected;
    if (!connected) running_ = false;
    connectButton_->setText(connected ? tr("Trennen") : tr("Verbinden"));
    kklState_->setText(connected ? tr("verbunden mit dem Motorsteuergerät")
                                 : tr("nicht verbunden"));
    kklState_->setStyleSheet(QStringLiteral("color: %1;")
                                 .arg(connected ? theme::color::good.name()
                                                : theme::color::textMuted.name()));
    identification_->setText(identification);
    updateAvailability();
}

void ThrottlePage::setProtocol(const QString& protocolName, bool isKwp2000) {
    isKwp2000_ = isKwp2000;
    // The session type only matters for KWP2000.
    sessionRow_->setVisible(isKwp2000);
    sessionHint_->setVisible(isKwp2000);
    identification_->setText(QStringLiteral("%1\n%2").arg(protocolName, identification_->text()));
}

void ThrottlePage::updateAvailability() {
    runButton_->setEnabled(kklConnected_);
    runButton_->setText(running_ ? tr("Grundeinstellung beenden")
                                 : tr("Grundeinstellung starten"));
    activity_->setVisible(running_);
    if (!kklConnected_) {
        message_->setText(
            tr("Für die echte Grundeinstellung wird ein KKL-Kabel gebraucht. Ohne das Kabel "
               "bleibt nur der Anlernversuch weiter unten."));
    }
}

void ThrottlePage::onLiveValue(const LiveRow& row) {
    // As long as only EOBD is connected, the throttle can at least be
    // observed.
    if (row.pid == 0x11 || row.pid == 0x45) angleGauge_->setValue(row.value);
    if (row.pid == 0x0C) rpmGauge_->setValue(row.value);
}

void ThrottlePage::onBasicSettingValues(const QVector<LiveRow>& rows) {
    running_ = true;
    updateAvailability();

    values_->setRowCount(rows.size());
    for (int i = 0; i < rows.size(); ++i) {
        const LiveRow& v = rows.at(i);
        values_->setItem(i, 0, new QTableWidgetItem(v.name));
        values_->setItem(i, 1, new QTableWidgetItem(QString::number(v.value, 'f', 2)));
        values_->setItem(i, 2, new QTableWidgetItem(v.unit));

        if (v.unit == QLatin1String("1/min")) rpmGauge_->setValue(v.value);
        // The core may name the angle unit in either language.
        if (v.unit == QLatin1String("%") || v.unit == QLatin1String("Grad") ||
            v.unit == QLatin1String("deg") || v.unit == QStringLiteral("°")) {
            angleGauge_->setValue(v.value);
        }
    }
}

void ThrottlePage::onKklMessage(const QString& message, bool isError) {
    message_->setText(message);
    message_->setStyleSheet(QStringLiteral("color: %1;")
                                .arg(isError ? theme::color::bad.name()
                                             : theme::color::textMuted.name()));
    if (isError) {
        running_ = false;
        updateAvailability();
    }
}

}  // namespace caruspex
