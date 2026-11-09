#include "app/pages/KklToolsPage.h"

#include <QComboBox>
#include <QDateTime>
#include <QFrame>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QLabel>
#include <QMessageBox>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QScrollArea>
#include <QSpinBox>
#include <QTableWidget>
#include <QVBoxLayout>

#include "app/Theme.h"
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

KklToolsPage::KklToolsPage(QWidget* parent) : QWidget(parent) {
    auto* outer = new QVBoxLayout(this);
    outer->setContentsMargins(24, 20, 24, 20);
    outer->setSpacing(14);

    auto* title = new QLabel(tr("Anpassung und Stellglieder"));
    title->setObjectName("PageTitle");
    outer->addWidget(title);

    auto* scroll = new QScrollArea;
    scroll->setWidgetResizable(true);
    scroll->setFrameShape(QFrame::NoFrame);
    auto* content = new QWidget;
    auto* root = new QVBoxLayout(content);
    root->setContentsMargins(0, 0, 0, 0);
    root->setSpacing(14);

    // --- Connection ---------------------------------------------------------
    QVBoxLayout* body = nullptr;
    QFrame* connectCard = card(tr("Steuergerät verbinden (KKL-Kabel)"), body);
    auto* row = new QHBoxLayout;
    ecuBox_ = new QComboBox;
    for (Ecu ecu : knownEcus()) {
        ecuBox_->addItem(QString::fromStdString(ecuName(ecu)), static_cast<int>(ecu));
    }
    portBox_ = new QComboBox;
    portBox_->setMinimumWidth(220);
    rescanButton_ = new QPushButton(tr("Suchen"));
    connectButton_ = new QPushButton(tr("Verbinden"));
    connectButton_->setObjectName("Primary");
    connectButton_->setMinimumHeight(38);
    row->addWidget(new QLabel(tr("Steuergerät:")));
    row->addWidget(ecuBox_, 1);
    row->addWidget(new QLabel(tr("Anschluss:")));
    row->addWidget(portBox_, 1);
    row->addWidget(rescanButton_);
    row->addWidget(connectButton_);
    body->addLayout(row);

    state_ = new QLabel(tr("nicht verbunden"));
    state_->setStyleSheet(QStringLiteral("color: %1;").arg(theme::color::textMuted.name()));
    identification_ = new QLabel;
    identification_->setObjectName("Hint");
    identification_->setWordWrap(true);
    body->addWidget(state_);
    body->addWidget(identification_);
    root->addWidget(connectCard);

    connect(rescanButton_, &QPushButton::clicked, this, &KklToolsPage::refreshPorts);
    connect(connectButton_, &QPushButton::clicked, this, [this] {
        if (connected_) {
            emit disconnectRequested();
            return;
        }
        const QString port = portBox_->currentData().toString();
        if (port.isEmpty()) {
            QMessageBox::information(this, tr("Kein Anschluss"),
                                     tr("Erst ein KKL-Kabel anstecken und auf Suchen "
                                        "drücken."));
            return;
        }
        emit connectRequested(port, ecuBox_->currentData().toInt());
    });

    // --- Adaptation channels ------------------------------------------------
    QFrame* adaptCard = card(tr("Anpassungskanal lesen"), body);
    auto* adaptHint = new QLabel(
        tr("Anpassungskanäle sind die Stellschrauben des Steuergeräts, etwa der "
           "Leerlauf oder die Startmenge. Caruspex liest sie hier aus. Verstellt wird "
           "bewusst nichts: ein falscher Wert im falschen Kanal legt den Motor lahm, und "
           "manche Kanäle brauchen zusätzlich einen Zugriffscode."));
    adaptHint->setObjectName("Subtle");
    adaptHint->setWordWrap(true);
    body->addWidget(adaptHint);

    auto* adaptRow = new QHBoxLayout;
    channelBox_ = new QSpinBox;
    channelBox_->setRange(0, 255);
    channelBox_->setValue(1);
    readChannelButton_ = new QPushButton(tr("Kanal lesen"));
    readChannelButton_->setMinimumHeight(38);
    readChannelButton_->setEnabled(false);
    channelValue_ = new QLabel(tr("Wert: --"));
    QFont bigValue = channelValue_->font();
    bigValue.setPointSizeF(bigValue.pointSizeF() + 3.0);
    bigValue.setBold(true);
    channelValue_->setFont(bigValue);
    adaptRow->addWidget(new QLabel(tr("Kanal:")));
    adaptRow->addWidget(channelBox_);
    adaptRow->addWidget(readChannelButton_);
    adaptRow->addSpacing(16);
    adaptRow->addWidget(channelValue_);
    adaptRow->addStretch();
    body->addLayout(adaptRow);

    channelTable_ = new QTableWidget(0, 3);
    channelTable_->setHorizontalHeaderLabels({tr("Feld"), tr("Wert"), tr("Einheit")});
    channelTable_->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    channelTable_->verticalHeader()->setVisible(false);
    channelTable_->setEditTriggers(QAbstractItemView::NoEditTriggers);
    channelTable_->setMinimumHeight(160);
    body->addWidget(channelTable_);
    root->addWidget(adaptCard);

    connect(readChannelButton_, &QPushButton::clicked, this,
            [this] { emit adaptationRequested(channelBox_->value()); });

    // --- Output test ---------------------------------------------------------
    QFrame* actuatorCard = card(tr("Stellglieddiagnose"), body);
    auto* actuatorHint = new QLabel(
        tr("Das Steuergerät klappert seine Aktoren der Reihe nach durch und nennt jeweils "
           "das laufende Bauteil. Man hört und sieht dann, ob es wirklich anspricht. Jeder "
           "Druck auf den Knopf schaltet zum nächsten Stellglied weiter.\n\n"
           "Zündung an, Motor aus. Beim laufenden Motor verweigern die meisten "
           "Steuergeräte den Dienst."));
    actuatorHint->setObjectName("Subtle");
    actuatorHint->setWordWrap(true);
    body->addWidget(actuatorHint);

    auto* actuatorRow = new QHBoxLayout;
    actuatorButton_ = new QPushButton(tr("Nächstes Stellglied"));
    actuatorButton_->setMinimumHeight(38);
    actuatorButton_->setEnabled(false);
    actuatorLabel_ = new QLabel(QStringLiteral("--"));
    QFont bigActuator = actuatorLabel_->font();
    bigActuator.setPointSizeF(bigActuator.pointSizeF() + 3.0);
    bigActuator.setBold(true);
    actuatorLabel_->setFont(bigActuator);
    actuatorLabel_->setStyleSheet(
        QStringLiteral("color: %1;").arg(theme::color::accent.name()));
    actuatorRow->addWidget(actuatorButton_);
    actuatorRow->addSpacing(16);
    actuatorRow->addWidget(actuatorLabel_, 1);
    body->addLayout(actuatorRow);

    actuatorLog_ = new QPlainTextEdit;
    actuatorLog_->setReadOnly(true);
    actuatorLog_->setMinimumHeight(130);
    body->addWidget(actuatorLog_);
    root->addWidget(actuatorCard);

    connect(actuatorButton_, &QPushButton::clicked, this,
            [this] { emit actuatorTestRequested(); });

    message_ = new QLabel;
    message_->setWordWrap(true);
    message_->setObjectName("Subtle");
    root->addWidget(message_);
    root->addStretch();

    scroll->setWidget(content);
    outer->addWidget(scroll, 1);

    refreshPorts();
    updateAvailability();
}

void KklToolsPage::refreshPorts() {
    portBox_->clear();
    for (const PortInfo& p : enumeratePorts()) {
        QString label = QString::fromStdString(p.device);
        if (!p.chipGuess.empty()) label += "  [" + QString::fromStdString(p.chipGuess) + "]";
        portBox_->addItem(label, QString::fromStdString(p.device));
    }
    if (portBox_->count() == 0) portBox_->addItem(tr("kein Anschluss gefunden"), QString());
}

void KklToolsPage::setKklConnected(bool connected, const QString& identification) {
    connected_ = connected;
    connectButton_->setText(connected ? tr("Trennen") : tr("Verbinden"));
    state_->setText(connected ? tr("verbunden") : tr("nicht verbunden"));
    state_->setStyleSheet(QStringLiteral("color: %1;")
                              .arg(connected ? theme::color::good.name()
                                             : theme::color::textMuted.name()));
    identification_->setText(identification);
    updateAvailability();
}

void KklToolsPage::updateAvailability() {
    readChannelButton_->setEnabled(connected_);
    actuatorButton_->setEnabled(connected_);
    ecuBox_->setEnabled(!connected_);
    if (!connected_) {
        message_->setText(tr("Beides sind herstellereigene Dienste. Über einen "
                             "ELM327-Adapter sind sie nicht erreichbar, dafür wird das "
                             "KKL-Kabel gebraucht."));
    }
}

void KklToolsPage::onAdaptationResult(int channel, int value, const QVector<LiveRow>& shown) {
    channelValue_->setText(tr("Kanal %1: Wert %2").arg(channel).arg(value));
    channelTable_->setRowCount(shown.size());
    for (int i = 0; i < shown.size(); ++i) {
        const LiveRow& v = shown.at(i);
        channelTable_->setItem(i, 0, new QTableWidgetItem(v.name));
        channelTable_->setItem(i, 1,
                               new QTableWidgetItem(QString::number(v.value, 'f', 2)));
        channelTable_->setItem(i, 2, new QTableWidgetItem(v.unit));
    }
}

void KklToolsPage::onActuatorResult(const QString& actuator) {
    actuatorLabel_->setText(actuator);
    actuatorLog_->appendPlainText(
        QDateTime::currentDateTime().toString("HH:mm:ss") + "  " + actuator);
}

void KklToolsPage::onMessage(const QString& message, bool isError) {
    message_->setText(message);
    message_->setStyleSheet(QStringLiteral("color: %1;")
                                .arg(isError ? theme::color::bad.name()
                                             : theme::color::textMuted.name()));
}

}  // namespace caruspex
