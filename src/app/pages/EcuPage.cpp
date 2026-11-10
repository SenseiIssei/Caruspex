#include "app/pages/EcuPage.h"

#include <QComboBox>
#include <QFrame>
#include <QHBoxLayout>
#include <QLabel>
#include <QListWidget>
#include <QMessageBox>
#include <QProgressBar>
#include <QPushButton>
#include <QScrollArea>
#include <QVBoxLayout>

#include "app/Icons.h"
#include "app/Theme.h"
#include "app/Widgets.h"
#include "app/pages/FaultsPage.h"
#include "data/DtcDatabase.h"
#include "data/EcuAddresses.h"

namespace caruspex {

using namespace theme;

namespace {

QString hexId(quint32 id, bool extended) {
    return QStringLiteral("%1").arg(id, extended ? 8 : 3, 16, QLatin1Char('0')).toUpper();
}

QString kindIcon(int kind) {
    switch (static_cast<EcuKind>(kind)) {
        case EcuKind::Engine: return QStringLiteral("engine");
        case EcuKind::Gearbox: return QStringLiteral("sliders");
        case EcuKind::Brakes: return QStringLiteral("stop");
        case EcuKind::Airbag: return QStringLiteral("shield");
        case EcuKind::Steering: return QStringLiteral("steering");
        case EcuKind::Instruments: return QStringLiteral("gauge");
        case EcuKind::Gateway: return QStringLiteral("cpu");
        case EcuKind::Climate: return QStringLiteral("thermometer");
        case EcuKind::Battery:
        case EcuKind::Charger:
        case EcuKind::Hybrid: return QStringLiteral("battery");
        case EcuKind::Lighting: return QStringLiteral("lightbulb");
        default: return QStringLiteral("cpu");
    }
}

QWidget* fact(const QString& key, const QString& value) {
    auto* row = new QWidget;
    auto* h = new QHBoxLayout(row);
    h->setContentsMargins(0, 1, 0, 1);
    auto* k = makeLabel(key, "Hint");
    k->setFixedWidth(130);
    auto* v = makeLabel(value, nullptr, true);
    styleFont(v, Family::Mono, 13, 500);
    v->setTextInteractionFlags(Qt::TextSelectableByMouse);
    h->addWidget(k);
    h->addWidget(v, 1);
    return row;
}

// The name from the address list, or, for the generic sweep where the list
// only knows the address, the name the unit reports itself (DID F197).
QString displayName(const UdsEcuRow& r) {
    if (r.kind == static_cast<int>(EcuKind::Other) && !r.systemName.isEmpty()) return r.systemName;
    return r.name;
}

void clearLayout(QVBoxLayout* layout) {
    while (QLayoutItem* item = layout->takeAt(0)) {
        if (item->widget()) item->widget()->deleteLater();
        delete item;
    }
}

}  // namespace

EcuPage::EcuPage(QWidget* parent) : QWidget(parent) {
    auto* root = new QVBoxLayout(this);
    root->setContentsMargins(32, 26, 32, 24);
    root->setSpacing(16);
    root->addWidget(makePageHeader(
        tr("Werkstatt"), tr("Alle Steuergeräte"),
        tr("Fragt über CAN jedes Steuergerät einzeln: Motor, Getriebe, ABS, Airbag, Kombi, "
           "Klima und mehr. Mit einem normalen ELM327, bei den meisten Autos ab etwa 2008. "
           "Ältere VW und Audi gehen über das KKL-Kabel.")));

    auto* bar = new QHBoxLayout;
    bar->setSpacing(10);
    family_ = new QComboBox;
    family_->setMinimumHeight(40);
    family_->addItem(tr("Alle üblichen Adressen durchsuchen (langsamer)"), QString());
    for (const std::string& f : ecuFamilies()) {
        family_->addItem(tr("Adressliste %1").arg(QString::fromStdString(brandFamilyLabel(f))),
                         QString::fromStdString(f));
        if (f == "BMW") {
            family_->addItem(tr("BMW und Mini: alle 128 Adressen (langsamer, findet alles)"),
                             QStringLiteral("BMW*"));
        }
    }
    start_ = new QPushButton(tr("Steuergeräte suchen"));
    start_->setObjectName("Primary");
    start_->setIcon(icons::icon(QStringLiteral("search"), Qt::white, Qt::white, 16));
    start_->setMinimumHeight(40);
    cancel_ = new QPushButton(tr("Abbrechen"));
    cancel_->setObjectName("Ghost");
    cancel_->setVisible(false);
    bar->addWidget(family_, 1);
    bar->addWidget(start_);
    bar->addWidget(cancel_);
    root->addLayout(bar);
    progress_ = new QProgressBar;
    progress_->setVisible(false);
    status_ = makeLabel(QString(), "Subtle", true);
    root->addWidget(progress_);
    root->addWidget(status_);

    connect(start_, &QPushButton::clicked, this, [this] {
        emit scanRequested(family_->currentData().toString());
    });
    connect(cancel_, &QPushButton::clicked, this, &EcuPage::cancelRequested);

    auto* columns = new QHBoxLayout;
    columns->setSpacing(20);
    QVBoxLayout* listLayout = nullptr;
    QFrame* listCard = makeCard(&listLayout, 12);
    list_ = new QListWidget;
    list_->setIconSize(QSize(18, 18));
    list_->setStyleSheet(QStringLiteral("QListWidget { background: transparent; border: none; }"
                                        "QListWidget::item { padding: 10px 8px; border-radius: 8px; }"));
    listLayout->addWidget(list_);
    columns->addWidget(listCard, 4);
    connect(list_, &QListWidget::currentItemChanged, this, [this](QListWidgetItem* item) {
        if (item) showEcu(item->data(Qt::UserRole).toUInt());
    });

    auto* scroll = new QScrollArea;
    scroll->setWidgetResizable(true);
    scroll->setFrameShape(QFrame::NoFrame);
    QVBoxLayout* detailLayout = nullptr;
    QFrame* detailCard = makeCard(&detailLayout, 24);
    scroll->setWidget(detailCard);
    detail_ = scroll;
    name_ = makeLabel(QString(), "PageTitle", true);
    address_ = makeLabel(QString(), "Hint");
    detailLayout->addWidget(name_);
    detailLayout->addWidget(address_);
    facts_ = new QVBoxLayout;
    facts_->setSpacing(0);
    detailLayout->addLayout(facts_);
    auto* divider = new QFrame;
    divider->setObjectName("Divider");
    detailLayout->addWidget(divider);
    detailLayout->addWidget(makeLabel(tr("Fehlerspeicher"), "SectionTitle"));
    faults_ = new QVBoxLayout;
    faults_->setSpacing(0);
    detailLayout->addLayout(faults_);
    clear_ = new QPushButton(tr("Fehler dieses Steuergeräts löschen"));
    clear_->setObjectName("Danger");
    clear_->setIcon(icons::icon(QStringLiteral("trash"), color::bad, color::bad, 16));
    clear_->setMaximumWidth(320);
    detailLayout->addWidget(clear_);
    detailLayout->addStretch();
    connect(clear_, &QPushButton::clicked, this, [this] {
        const auto it = rows_.find(selected_);
        if (it == rows_.end()) return;
        const auto answer = QMessageBox::question(
            this, tr("Fehler löschen"),
            tr("Den Fehlerspeicher von %1 löschen? Ein Fehler, dessen Ursache noch besteht, "
               "kommt wieder. Bei Airbag und ABS erst die Ursache beheben.")
                .arg(it->name));
        if (answer == QMessageBox::Yes) {
            emit clearRequested(it->request, it->response, it->extended29, it->name);
        }
    });

    QVBoxLayout* emptyLayout = nullptr;
    QFrame* emptyCard = makeCard(&emptyLayout, 32);
    empty_ = emptyCard;
    auto* emptyIcon = new QLabel;
    emptyIcon->setPixmap(icons::pixmap(QStringLiteral("cpu"), 46, color::textFaint, 1.4));
    emptyIcon->setAlignment(Qt::AlignCenter);
    emptyText_ = makeLabel(tr("Noch nicht gesucht. Verbinden, dann auf Steuergeräte suchen."),
                           "Subtle", true);
    emptyText_->setAlignment(Qt::AlignCenter);
    emptyLayout->addStretch();
    emptyLayout->addWidget(emptyIcon);
    emptyLayout->addWidget(emptyText_);
    emptyLayout->addStretch();

    auto* right = new QVBoxLayout;
    right->addWidget(scroll, 1);
    right->addWidget(emptyCard, 1);
    columns->addLayout(right, 6);
    root->addLayout(columns, 1);

    detail_->setVisible(false);
    setConnected(false);
}

void EcuPage::setBrand(const QString& brand) {
    brand_ = brand;
    const QString family = QString::fromStdString(brandFamily(brand.toStdString()));
    const int index = family_->findData(family);
    family_->setCurrentIndex(index >= 0 ? index : 0);
}

QVector<UdsEcuRow> EcuPage::results() const {
    QVector<UdsEcuRow> out;
    for (quint32 id : order_) out.push_back(rows_.value(id));
    return out;
}

void EcuPage::setConnected(bool connected) {
    connected_ = connected;
    start_->setEnabled(connected && !running_);
    if (!connected) {
        running_ = false;
        progress_->setVisible(false);
        cancel_->setVisible(false);
    }
}

void EcuPage::onScanStarted(int total, bool generic) {
    running_ = true;
    rows_.clear();
    order_.clear();
    list_->clear();
    detail_->setVisible(false);
    empty_->setVisible(true);
    emptyText_->setText(tr("Suche läuft ..."));
    start_->setEnabled(false);
    cancel_->setVisible(true);
    progress_->setRange(0, total);
    progress_->setValue(0);
    progress_->setVisible(true);
    status_->setText(generic ? tr("Durchsucht %1 Adressen. Das dauert ein, zwei Minuten.").arg(total)
                             : tr("Fragt %1 bekannte Steuergeräte.").arg(total));
}

void EcuPage::onScanProgress(const QString& name, int index, int total) {
    progress_->setValue(index);
    status_->setText(tr("Frage %1 (%2 von %3)").arg(name).arg(index + 1).arg(total));
}

void EcuPage::onEcuResult(const UdsEcuRow& row) {
    if (!rows_.contains(row.request)) order_.push_back(row.request);
    rows_.insert(row.request, row);
    refreshList();
    if (selected_ == row.request || list_->currentRow() < 0) {
        for (int i = 0; i < list_->count(); ++i) {
            if (list_->item(i)->data(Qt::UserRole).toUInt() == row.request) list_->setCurrentRow(i);
        }
        showEcu(row.request);
    }
}

void EcuPage::refreshList() {
    const quint32 keep = selected_;
    const QSignalBlocker block(list_);
    list_->clear();
    for (quint32 id : std::as_const(order_)) {
        const UdsEcuRow& r = rows_[id];
        const int count = r.dtcs.size();
        const QColor tone = count > 0 ? color::bad : color::good;
        const QString state = count > 0 ? tr("%n Fehler", nullptr, count) : tr("in Ordnung");
        auto* item = new QListWidgetItem(icons::icon(kindIcon(r.kind), tone, tone, 18),
                                         QStringLiteral("%1\n%2  ·  %3")
                                             .arg(displayName(r), hexId(r.request, r.extended29), state));
        item->setData(Qt::UserRole, r.request);
        list_->addItem(item);
        if (id == keep) list_->setCurrentItem(item);
    }
}

void EcuPage::showEcu(quint32 request) {
    const auto it = rows_.find(request);
    if (it == rows_.end()) return;
    selected_ = request;
    const UdsEcuRow& r = *it;
    detail_->setVisible(true);
    empty_->setVisible(false);
    name_->setText(displayName(r));
    address_->setText(tr("Anfrage %1, Antwort %2").arg(hexId(r.request, r.extended29),
                                                        hexId(r.response, r.extended29)));
    clearLayout(facts_);
    if (!r.systemName.isEmpty()) facts_->addWidget(fact(tr("Bezeichnung"), r.systemName));
    if (!r.partNumber.isEmpty()) facts_->addWidget(fact(tr("Teilenummer"), r.partNumber));
    if (!r.softwareVersion.isEmpty()) facts_->addWidget(fact(tr("Software"), r.softwareVersion));
    if (!r.hardwareVersion.isEmpty()) facts_->addWidget(fact(tr("Hardware"), r.hardwareVersion));
    if (!r.serial.isEmpty()) facts_->addWidget(fact(tr("Seriennummer"), r.serial));
    if (!r.note.isEmpty()) facts_->addWidget(makeLabel(r.note, "Hint", true));

    clearLayout(faults_);
    if (!r.dtcsRead) {
        faults_->addWidget(new FindingRow(QStringLiteral("info"), color::info,
                                          tr("Fehlerspeicher nicht lesbar"),
                                          tr("Das Steuergerät hat geantwortet, gibt seinen "
                                             "Fehlerspeicher aber nicht heraus.")));
    } else if (r.dtcs.isEmpty()) {
        faults_->addWidget(new FindingRow(QStringLiteral("check-circle"), color::good,
                                          tr("Kein Fehler gespeichert"), QString()));
    }
    for (const UdsDtcRow& d : r.dtcs) {
        const DtcInfo info = lookupDtc(d.code.toStdString(), brand_.toStdString());
        const QString text = info.known ? QString::fromStdString(info.text) : d.text;
        QStringList state;
        if (d.testFailed) state << tr("gerade aktiv");
        if (d.confirmed) state << tr("gespeichert");
        else if (d.pending) state << tr("vorläufig");
        if (d.warningLamp) state << tr("Warnlampe an");
        QString detail = state.join(QStringLiteral(" · "));
        if (!d.failureText.isEmpty()) detail = d.failureText + QStringLiteral(" · ") + detail;
        const QString advice = info.advice.empty() ? QString() : QString::fromStdString(info.advice);
        if (!advice.isEmpty()) detail += QStringLiteral("\n") + advice;
        faults_->addWidget(new FindingRow(QStringLiteral("alert"),
                                          severityColour(static_cast<int>(info.severity)),
                                          QStringLiteral("%1  %2").arg(d.code, text), detail,
                                          QString::fromUtf8(severityLabel(info.severity))));
    }
    clear_->setVisible(r.dtcsRead && !r.dtcs.isEmpty());
}

void EcuPage::onScanFinished(int present, int withFaults, bool cancelled, const QString& note) {
    running_ = false;
    progress_->setVisible(false);
    cancel_->setVisible(false);
    start_->setEnabled(connected_);
    if (!note.isEmpty()) {
        status_->setText(note);
        emptyText_->setText(note);
        return;
    }
    QString text = tr("%1 Steuergeräte gefunden, %2 davon mit Fehlern.").arg(present).arg(withFaults);
    if (cancelled) text += QStringLiteral(" ") + tr("Abgebrochen.");
    if (present == 0) {
        text = tr("Kein Steuergerät hat geantwortet. Manche Autos lassen über die Buchse nur "
                  "das Motorsteuergerät antworten, dann hilft nur die Werkstatt.");
        emptyText_->setText(text);
    }
    status_->setText(text);
    toast::show(this, text, withFaults > 0 ? toast::Kind::Warning : toast::Kind::Success);
}

void EcuPage::onClearResult(const QString&, bool ok, const QString& message) {
    toast::show(this, message, ok ? toast::Kind::Success : toast::Kind::Error);
}

}  // namespace caruspex
