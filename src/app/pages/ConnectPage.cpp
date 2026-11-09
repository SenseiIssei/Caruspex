#include "app/pages/ConnectPage.h"

#include <QButtonGroup>
#include <QComboBox>
#include <QCompleter>
#include <QDate>
#include <QStringListModel>
#include <QGridLayout>
#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QPainter>
#include <QPushButton>
#include <QScrollArea>
#include <QSpinBox>
#include <QStackedWidget>
#include <QVBoxLayout>

#include "app/Icons.h"
#include "app/Theme.h"
#include "app/Widgets.h"
#include "core/Elm327.h"
#include "core/Serial.h"
#include "data/VehicleCatalog.h"

namespace caruspex {

using namespace theme;

namespace {

// Numbered circle with the gradient for the step list.
class StepNumber : public QWidget {
public:
    explicit StepNumber(int n) : n_(n) { setFixedSize(30, 30); }

protected:
    void paintEvent(QPaintEvent*) override {
        QPainter p(this);
        p.setRenderHint(QPainter::Antialiasing);
        p.setPen(Qt::NoPen);
        QColor fill = color::accent;
        fill.setAlphaF(0.16f);
        p.setBrush(fill);
        p.drawEllipse(rect().adjusted(0, 0, -1, -1));
        p.setPen(color::accentHover);
        p.setFont(display(14, 800));
        p.drawText(rect(), Qt::AlignCenter, QString::number(n_));
    }

private:
    int n_;
};

QWidget* makeStep(int n, const QString& iconName, const QString& title, const QString& text,
                  QLabel** textOut = nullptr) {
    auto* row = new QWidget;
    auto* layout = new QHBoxLayout(row);
    layout->setContentsMargins(0, 6, 0, 6);
    layout->setSpacing(14);
    layout->addWidget(new StepNumber(n), 0, Qt::AlignTop);
    auto* texts = new QVBoxLayout;
    texts->setSpacing(3);
    auto* head = new QHBoxLayout;
    head->setSpacing(8);
    auto* icon = new QLabel;
    icon->setPixmap(icons::pixmap(iconName, 16, color::textMuted));
    auto* titleLabel = new QLabel(title);
    titleLabel->setStyleSheet(QStringLiteral("font-weight: 600; font-size: 15px;"));
    head->addWidget(icon);
    head->addWidget(titleLabel, 1);
    texts->addLayout(head);
    auto* body = makeLabel(text, "Subtle", true);
    texts->addWidget(body);
    if (textOut) *textOut = body;
    layout->addLayout(texts, 1);
    return row;
}

QPushButton* makeModeChip(const QString& iconName, const QString& text) {
    auto* chip = new QPushButton(text);
    chip->setObjectName("Chip");
    chip->setCheckable(true);
    chip->setCursor(Qt::PointingHandCursor);
    chip->setIcon(icons::icon(iconName, color::textMuted, color::accentHover, 16));
    chip->setIconSize(QSize(16, 16));
    chip->setMinimumHeight(34);
    return chip;
}

}  // namespace

ConnectPage::ConnectPage(QWidget* parent) : QWidget(parent) {
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
    root->addWidget(makePageHeader(tr("Verbindung"), tr("Mit dem Auto verbinden"),
                                   tr("Ein Adapter in der OBD-Buchse, die Zündung an, ein Klick. "
                                      "Welches Protokoll das Auto spricht, findet Caruspex "
                                      "selbst heraus.")));

    auto* columns = new QHBoxLayout;
    columns->setSpacing(20);
    columns->addWidget(buildSteps(), 5);
    columns->addWidget(buildAdapter(), 6);
    root->addLayout(columns);
    root->addStretch();

    updateProfileNote();
}

QWidget* ConnectPage::buildSteps() {
    QVBoxLayout* layout = nullptr;
    QFrame* card = makeCard(&layout);
    layout->addWidget(makeLabel(tr("So geht's"), "SectionTitle"));
    layout->addWidget(makeStep(1, QStringLiteral("map-pin"), tr("Buchse finden"),
                               tr("Die OBD-Buchse sitzt fast immer im Fußraum auf der Fahrerseite, "
                                  "unter dem Lenkrad oder hinter einer kleinen Klappe."),
                               &socketHint_));
    layout->addWidget(makeStep(2, QStringLiteral("cable"), tr("Adapter einstecken"),
                               tr("ELM327 per USB, Bluetooth oder WLAN. Für Airbag, ABS und "
                                  "Grundeinstellungen bei VAG bis etwa 2004 ein KKL-Kabel.")));
    layout->addWidget(makeStep(3, QStringLiteral("key"), tr("Zündung an"),
                               tr("Zündung einschalten. Der Motor darf aus sein, für Live-Werte "
                                  "darf er laufen, aber nur im Freien.")));
    layout->addWidget(makeStep(4, QStringLiteral("plug"), tr("Verbinden"),
                               tr("Rechts auf Verbinden drücken. Beim ersten Mal dauert es ein "
                                  "paar Sekunden, bis das Protokoll gefunden ist.")));
    layout->addStretch();
    return card;
}

QWidget* ConnectPage::buildAdapter() {
    QVBoxLayout* layout = nullptr;
    QFrame* card = makeCard(&layout);
    layout->setSpacing(14);
    layout->addWidget(makeLabel(tr("Adapter"), "SectionTitle"));

    auto* chips = new QHBoxLayout;
    chips->setSpacing(8);
    modes_ = new QButtonGroup(this);
    modes_->setExclusive(true);
    QPushButton* cable = makeModeChip(QStringLiteral("usb"), tr("USB oder Bluetooth"));
    QPushButton* wifi = makeModeChip(QStringLiteral("wifi"), tr("WLAN"));
    QPushButton* demo = makeModeChip(QStringLiteral("sparkles"), tr("Demo-Auto"));
    modes_->addButton(cable, int(Mode::Cable));
    modes_->addButton(wifi, int(Mode::Wifi));
    modes_->addButton(demo, int(Mode::Demo));
    cable->setChecked(true);
    chips->addWidget(cable);
    chips->addWidget(wifi);
    chips->addWidget(demo);
    chips->addStretch();
    layout->addLayout(chips);

    modeStack_ = new QStackedWidget;
    // Cable / Bluetooth
    {
        auto* page = new QWidget;
        auto* v = new QVBoxLayout(page);
        v->setContentsMargins(0, 0, 0, 0);
        v->setSpacing(8);
        auto* row = new QHBoxLayout;
        row->setSpacing(8);
        portBox_ = new QComboBox;
        portBox_->setMinimumHeight(38);
        auto* rescan = new QPushButton;
        rescan->setIcon(icons::icon(QStringLiteral("refresh"), color::textMuted, color::text, 18));
        rescan->setToolTip(tr("Anschlüsse neu suchen"));
        rescan->setFixedSize(40, 38);
        rescan->setStyleSheet(QStringLiteral("QPushButton { padding: 0; }"));
        row->addWidget(portBox_, 1);
        row->addWidget(rescan);
        v->addLayout(row);
        v->addWidget(makeLabel(tr("Bluetooth-Adapter erscheinen nach dem Koppeln in Windows als "
                                  "eigener COM-Anschluss. Bei Automatisch probiert Caruspex alle "
                                  "Anschlüsse der Reihe nach."),
                               "Hint", true));
        connect(rescan, &QPushButton::clicked, this, [this] { refreshPorts(); });
        modeStack_->addWidget(page);
    }
    // Wi-Fi
    {
        auto* page = new QWidget;
        auto* v = new QVBoxLayout(page);
        v->setContentsMargins(0, 0, 0, 0);
        v->setSpacing(8);
        addressEdit_ = new QLineEdit(QStringLiteral("192.168.0.10:35000"));
        addressEdit_->setMinimumHeight(38);
        v->addWidget(addressEdit_);
        v->addWidget(makeLabel(tr("Erst mit dem WLAN des Adapters verbinden. Die meisten nutzen "
                                  "192.168.0.10, Port 35000."),
                               "Hint", true));
        modeStack_->addWidget(page);
    }
    // Demo
    {
        auto* page = new QWidget;
        auto* v = new QVBoxLayout(page);
        v->setContentsMargins(0, 0, 0, 0);
        v->addWidget(makeLabel(tr("Ein nachgebautes Auto im Programm: es fährt, wärmt sich auf, "
                                  "hat ein paar Fehler gespeichert. Ideal zum Ausprobieren, ganz "
                                  "ohne Adapter."),
                               "Subtle", true));
        modeStack_->addWidget(page);
    }
    layout->addWidget(modeStack_);
    connect(modes_, &QButtonGroup::idClicked, this, [this](int id) {
        modeStack_->setCurrentIndex(id);
    });

    // Vehicle, optional
    auto* divider = new QFrame;
    divider->setObjectName("Divider");
    layout->addWidget(divider);
    auto* vehicleTitle = new QHBoxLayout;
    vehicleTitle->addWidget(makeLabel(tr("Fahrzeug"), "SectionTitle"));
    auto* optional = makeLabel(tr("freiwillig"), "Hint");
    vehicleTitle->addWidget(optional);
    vehicleTitle->addStretch();
    layout->addLayout(vehicleTitle);

    auto* vehicleRow = new QHBoxLayout;
    vehicleRow->setSpacing(8);
    vehicleEdit_ = new QLineEdit;
    vehicleEdit_->setPlaceholderText(tr("Automatisch erkennen, oder suchen: Golf 4, A2, E46 ..."));
    vehicleEdit_->setClearButtonEnabled(true);
    vehicleEdit_->setMinimumHeight(38);
    suggestions_ = new QStringListModel(this);
    completer_ = new QCompleter(suggestions_, this);
    completer_->setCaseSensitivity(Qt::CaseInsensitive);
    completer_->setCompletionMode(QCompleter::UnfilteredPopupCompletion);
    completer_->setMaxVisibleItems(12);
    vehicleEdit_->setCompleter(completer_);
    yearBox_ = new QSpinBox;
    yearBox_->setRange(1980, QDate::currentDate().year() + 1);
    yearBox_->setValue(2010);
    yearBox_->setMinimumHeight(38);
    yearBox_->setMinimumWidth(110);
    yearBox_->setPrefix(tr("Baujahr "));
    vehicleRow->addWidget(vehicleEdit_, 1);
    vehicleRow->addWidget(yearBox_);
    layout->addLayout(vehicleRow);
    profileNote_ = makeLabel(QString(), "Hint", true);
    layout->addWidget(profileNote_);
    connect(vehicleEdit_, &QLineEdit::textEdited, this, [this](const QString& text) {
        QStringList items;
        suggestionMap_.clear();
        if (text.trimmed().size() >= 2) {
            for (const VehicleGeneration* g : catalog::search(text.trimmed().toStdString(), 15)) {
                const QString label = QStringLiteral("%1  (%2)").arg(
                    QString::fromStdString(catalog::displayName(*g)),
                    QString::fromStdString(catalog::yearsText(*g)));
                items << label;
                suggestionMap_.insert(label, g);
            }
        }
        suggestions_->setStringList(items);
        if (text.trimmed().isEmpty()) setGeneration(nullptr, yearBox_->value());
    });
    connect(completer_, qOverload<const QString&>(&QCompleter::activated), this,
            [this](const QString& label) {
                if (const VehicleGeneration* g = suggestionMap_.value(label)) {
                    const int last = g->yearTo > 0 ? g->yearTo : QDate::currentDate().year();
                    const int year = qBound(g->yearFrom, yearBox_->value(), last);
                    setGeneration(g, year);
                }
            });
    connect(yearBox_, &QSpinBox::valueChanged, this, [this] {
        updateProfileNote();
        emit vehicleChoiceChanged();
    });

    // Connect button and state
    auto* divider2 = new QFrame;
    divider2->setObjectName("Divider");
    layout->addWidget(divider2);
    connectButton_ = new QPushButton(tr("Verbinden"));
    connectButton_->setObjectName("Primary");
    connectButton_->setIcon(icons::icon(QStringLiteral("plug"), Qt::white, Qt::white, 18));
    connectButton_->setIconSize(QSize(18, 18));
    connectButton_->setMinimumHeight(48);
    connectButton_->setCursor(Qt::PointingHandCursor);
    pulseGlow(connectButton_, color::accent);
    layout->addWidget(connectButton_);
    connect(connectButton_, &QPushButton::clicked, this, &ConnectPage::startConnect);

    auto* state = new QHBoxLayout;
    state->setSpacing(8);
    spinner_ = new Spinner(16);
    stateLabel_ = new QLabel(tr("Nicht verbunden"));
    stateLabel_->setStyleSheet(QStringLiteral("font-weight: 600; color: %1;").arg(color::textMuted.name()));
    state->addWidget(spinner_);
    state->addWidget(stateLabel_, 1);
    layout->addLayout(state);
    details_ = makeLabel(QString(), "Hint", true);
    details_->setTextInteractionFlags(Qt::TextSelectableByMouse);
    layout->addWidget(details_);

    errorCard_ = new QFrame;
    errorCard_->setStyleSheet(QStringLiteral("QFrame { background: %1; border: 1px solid %2; "
                                             "border-radius: 12px; } QLabel { border: none; }")
                                  .arg(rgba(color::bad, 0.08), rgba(color::bad, 0.35)));
    auto* errorLayout = new QVBoxLayout(errorCard_);
    errorLayout->setContentsMargins(14, 12, 14, 12);
    errorLayout->setSpacing(6);
    auto* errorHead = new QLabel(tr("Das hat nicht geklappt"));
    errorHead->setStyleSheet(QStringLiteral("font-weight: 700; color: %1;").arg(color::bad.name()));
    errorText_ = makeLabel(QString(), nullptr, true);
    errorText_->setTextInteractionFlags(Qt::TextSelectableByMouse);
    auto* help = makeLabel(tr("Häufige Gründe: Zündung aus, Adapter nicht ganz eingesteckt, falscher "
                              "Anschluss gewählt, oder ein anderes Programm hält den Anschluss "
                              "offen. Bei Bluetooth hilft oft, den Adapter neu zu koppeln."),
                           "Subtle", true);
    errorLayout->addWidget(errorHead);
    errorLayout->addWidget(errorText_);
    errorLayout->addWidget(help);
    errorCard_->setVisible(false);
    layout->addWidget(errorCard_);
    layout->addStretch();
    return card;
}

void ConnectPage::setSettings(const AppSettings& settings) { settings_ = settings; }

int ConnectPage::refreshPorts() {
    const QString previous = portBox_->currentData().toString();
    portBox_->clear();
    portBox_->addItem(icons::icon(QStringLiteral("sparkles"), color::accent),
                      tr("Automatisch suchen"), QString());
    const std::vector<PortInfo> ports = enumeratePorts();
    for (const PortInfo& p : ports) {
        QString label = QString::fromStdString(p.device);
        if (!p.description.empty()) label += QStringLiteral("  ·  ") + QString::fromStdString(p.description);
        if (!p.chipGuess.empty()) label += QStringLiteral("  [") + QString::fromStdString(p.chipGuess) + ']';
        portBox_->addItem(icons::icon(QStringLiteral("usb"), color::textMuted), label,
                          QString::fromStdString(p.device));
    }
    if (!previous.isEmpty()) selectPort(previous);
    return static_cast<int>(ports.size());
}

bool ConnectPage::selectPort(const QString& device) {
    const int index = portBox_->findData(device);
    if (index < 0) return false;
    portBox_->setCurrentIndex(index);
    return true;
}

void ConnectPage::setWifiAddress(const QString& address) { addressEdit_->setText(address); }

void ConnectPage::setMode(Mode mode) {
    if (QAbstractButton* b = modes_->button(int(mode))) b->setChecked(true);
    modeStack_->setCurrentIndex(int(mode));
}

void ConnectPage::setSocketHint(const QString& text) {
    socketHint_->setText(text.isEmpty()
                             ? tr("Die OBD-Buchse sitzt fast immer im Fußraum auf der Fahrerseite, "
                                  "unter dem Lenkrad oder hinter einer kleinen Klappe.")
                             : text);
}

QString ConnectPage::protocolOrder() const {
    if (generation_) {
        const std::string order = catalog::protocolOrderFor(*generation_);
        if (!order.empty()) return QString::fromStdString(order);
    }
    return QString::fromStdString(settings_.protocolOrder == QStringLiteral("can")
                                      ? kProtocolOrderCanFirst
                                      : kProtocolOrderKLineFirst);
}

void ConnectPage::setGeneration(const VehicleGeneration* generation, int year) {
    generation_ = generation;
    if (generation) {
        const QSignalBlocker block(yearBox_);
        const int last = generation->yearTo > 0 ? generation->yearTo : QDate::currentDate().year();
        yearBox_->setRange(generation->yearFrom, last);
        yearBox_->setValue(qBound(generation->yearFrom, year, last));
        vehicleEdit_->setText(QString::fromStdString(catalog::displayName(*generation)));
    } else {
        const QSignalBlocker block(yearBox_);
        yearBox_->setRange(1980, QDate::currentDate().year() + 1);
    }
    updateProfileNote();
    emit vehicleChoiceChanged();
}

void ConnectPage::updateProfileNote() {
    if (!generation_) {
        profileNote_->setText(tr("Ohne Auswahl probiert Caruspex die Protokolle der Reihe nach "
                                 "durch und erkennt das Auto danach an der Fahrgestellnummer."));
        setSocketHint(QString());
        return;
    }
    const int year = yearBox_->value();
    profileNote_->setText(QString::fromStdString(catalog::diagnosisText(*generation_, year)));
    setSocketHint(tr("Bei diesem Auto: %1")
                      .arg(QString::fromStdString(locationText(*generation_, year))));
}

void ConnectPage::startConnect() {
    if (connected_) {
        emit disconnectRequested();
        return;
    }
    if (connecting_) return;
    errorCard_->setVisible(false);
    const QString order = protocolOrder();
    switch (Mode(modes_->checkedId())) {
        case Mode::Cable:
            emit serialRequested(portBox_->currentData().toString(), order);
            break;
        case Mode::Wifi: {
            const QString address = addressEdit_->text().trimmed();
            const int colon = address.indexOf(':');
            const QString host = colon < 0 ? address : address.left(colon);
            const int port = colon < 0 ? 35000 : address.mid(colon + 1).toInt();
            emit tcpRequested(host, port, order);
            break;
        }
        case Mode::Demo:
            emit demoRequested(order);
            break;
    }
    onConnecting();
}

void ConnectPage::onConnecting() {
    connecting_ = true;
    connectButton_->setEnabled(false);
    spinner_->setRunning(true);
    stateLabel_->setText(tr("Verbinde ..."));
    stateLabel_->setStyleSheet(QStringLiteral("font-weight: 600; color: %1;").arg(color::warn.name()));
}

void ConnectPage::onConnected(const QString& adapter, const QString& transport,
                              const QString& protocol) {
    connected_ = true;
    connecting_ = false;
    spinner_->setRunning(false);
    connectButton_->setEnabled(true);
    connectButton_->setText(tr("Trennen"));
    connectButton_->setIcon(icons::icon(QStringLiteral("unplug"), color::bad, color::bad, 18));
    connectButton_->setObjectName("Danger");
    removeGlow(connectButton_);
    repolish(connectButton_);
    stateLabel_->setText(tr("Verbunden"));
    stateLabel_->setStyleSheet(QStringLiteral("font-weight: 700; color: %1;").arg(color::good.name()));
    details_->setText(tr("Adapter: %1\nWeg: %2\nProtokoll: %3").arg(adapter, transport, protocol));
    errorCard_->setVisible(false);
}

void ConnectPage::showError(const QString& error) {
    errorText_->setText(error);
    errorCard_->setVisible(true);
    fadeIn(errorCard_);
}

void ConnectPage::onFailed(const QString& error) {
    connecting_ = false;
    spinner_->setRunning(false);
    connectButton_->setEnabled(true);
    if (!connected_) {
        stateLabel_->setText(tr("Nicht verbunden"));
        stateLabel_->setStyleSheet(QStringLiteral("font-weight: 600; color: %1;").arg(color::bad.name()));
    }
    showError(error);
}

void ConnectPage::onClosed() {
    connected_ = false;
    connecting_ = false;
    spinner_->setRunning(false);
    connectButton_->setEnabled(true);
    connectButton_->setText(tr("Verbinden"));
    connectButton_->setIcon(icons::icon(QStringLiteral("plug"), Qt::white, Qt::white, 18));
    connectButton_->setObjectName("Primary");
    repolish(connectButton_);
    pulseGlow(connectButton_, color::accent);
    stateLabel_->setText(tr("Nicht verbunden"));
    stateLabel_->setStyleSheet(QStringLiteral("font-weight: 600; color: %1;").arg(color::textMuted.name()));
    details_->clear();
}

}  // namespace caruspex
