#include "app/pages/VehiclesPage.h"

#include <QDate>
#include <QFrame>
#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QPushButton>
#include <QScrollArea>
#include <QSpinBox>
#include <QVBoxLayout>

#include "app/Icons.h"
#include "app/Theme.h"
#include "app/VehicleDiagram.h"
#include "app/VehicleIdentity.h"
#include "app/Widgets.h"
#include "data/Vin.h"

namespace caruspex {

using namespace theme;

namespace {

QColor supportColour(ObdSupport s) {
    switch (s) {
        case ObdSupport::Full: return color::good;
        case ObdSupport::Partial: return color::warn;
        case ObdSupport::Kkl: return color::accent;
        case ObdSupport::Manufacturer: return color::warn;
        case ObdSupport::None: return color::bad;
    }
    return color::textMuted;
}

QWidget* factRow(const QString& key, const QString& value, bool monospace = false) {
    auto* row = new QWidget;
    auto* h = new QHBoxLayout(row);
    h->setContentsMargins(0, 2, 0, 2);
    h->setSpacing(12);
    auto* k = makeLabel(key, "Hint");
    k->setFixedWidth(150);
    auto* v = makeLabel(value, nullptr, true);
    v->setTextInteractionFlags(Qt::TextSelectableByMouse);
    if (monospace) styleFont(v, Family::Mono, 13, 600);
    h->addWidget(k, 0, Qt::AlignTop);
    h->addWidget(v, 1);
    return row;
}

}  // namespace

VehiclesPage::VehiclesPage(QWidget* parent) : QWidget(parent) {
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
    root->addWidget(makePageHeader(
        tr("Wissen"), tr("Fahrzeuge"),
        tr("Über %1 Modellgenerationen von %2 Marken, von 1980 bis heute. Zu jeder steht, "
           "welchen Diagnosestecker sie hat, wo er sitzt und was Caruspex damit kann.")
            .arg(catalog::size())
            .arg(catalog::brands().size())));

    // --- VIN check ------------------------------------------------------------------
    QVBoxLayout* vinLayout = nullptr;
    QFrame* vinCard = makeCard(&vinLayout, 22);
    vinLayout->addWidget(makeLabel(tr("Fahrgestellnummer prüfen"), "SectionTitle"));
    vinLayout->addWidget(makeLabel(tr("Die 17 Zeichen stehen im Fahrzeugschein unter E, an der "
                                      "Windschutzscheibe unten links und im Motorraum."),
                                   "Subtle", true));
    auto* vinRow = new QHBoxLayout;
    vinRow->setSpacing(10);
    vinEdit_ = new QLineEdit;
    vinEdit_->setPlaceholderText(QStringLiteral("WAUZZZ8Z12A123456"));
    vinEdit_->setMaxLength(24);
    styleFont(vinEdit_, Family::Mono, 15, 600);
    vinEdit_->setMinimumHeight(42);
    auto* check = new QPushButton(tr("Prüfen"));
    check->setObjectName("Primary");
    check->setIcon(icons::icon(QStringLiteral("search"), Qt::white, Qt::white, 16));
    check->setMinimumHeight(42);
    vinRow->addWidget(vinEdit_, 1);
    vinRow->addWidget(check);
    vinLayout->addLayout(vinRow);
    vinResult_ = new QVBoxLayout;
    vinResult_->setSpacing(0);
    vinLayout->addLayout(vinResult_);
    root->addWidget(vinCard);
    connect(check, &QPushButton::clicked, this, &VehiclesPage::checkVin);
    connect(vinEdit_, &QLineEdit::returnPressed, this, &VehiclesPage::checkVin);

    // --- Catalog ----------------------------------------------------------------------
    auto* columns = new QHBoxLayout;
    columns->setSpacing(20);

    QVBoxLayout* listLayout = nullptr;
    QFrame* listCard = makeCard(&listLayout, 18);
    search_ = new QLineEdit;
    search_->setPlaceholderText(tr("Golf 4, A2, E46, W203, Octavia 2008, Model 3 ..."));
    search_->setClearButtonEnabled(true);
    search_->setMinimumHeight(40);
    listLayout->addWidget(search_);
    count_ = makeLabel(QString(), "Hint");
    listLayout->addWidget(count_);
    results_ = new QListWidget;
    results_->setMinimumHeight(420);
    listLayout->addWidget(results_, 1);
    columns->addWidget(listCard, 2);
    connect(search_, &QLineEdit::textChanged, this, &VehiclesPage::runSearch);
    connect(results_, &QListWidget::currentItemChanged, this,
            [this](QListWidgetItem* item) {
                if (item) showGeneration(item->data(Qt::UserRole).value<const VehicleGeneration*>());
            });

    QVBoxLayout* detailLayout = nullptr;
    QFrame* detailCard = makeCard(&detailLayout, 24);
    detail_ = detailCard;
    picture_ = new VehicleDiagram;
    picture_->setMinimumHeight(200);
    detailLayout->addWidget(picture_);
    name_ = makeLabel(QString(), "PageTitle", true);
    years_ = makeLabel(QString(), "Subtle", true);
    detailLayout->addWidget(name_);
    detailLayout->addWidget(years_);
    auto* yearRow = new QHBoxLayout;
    yearRow->addWidget(makeLabel(tr("Baujahr"), "Hint"));
    year_ = new QSpinBox;
    year_->setRange(1980, QDate::currentDate().year() + 1);
    year_->setMinimumHeight(36);
    yearRow->addWidget(year_);
    support_ = new QLabel;
    yearRow->addWidget(support_);
    yearRow->addStretch();
    detailLayout->addLayout(yearRow);
    diagnosis_ = makeLabel(QString(), nullptr, true);
    detailLayout->addWidget(diagnosis_);
    connector_ = makeLabel(QString(), "Subtle", true);
    location_ = makeLabel(QString(), "Subtle", true);
    note_ = makeLabel(QString(), "Hint", true);
    detailLayout->addWidget(connector_);
    detailLayout->addWidget(location_);
    detailLayout->addWidget(note_);
    use_ = new QPushButton(tr("Mit diesem Auto verbinden"));
    use_->setObjectName("Primary");
    use_->setIcon(icons::icon(QStringLiteral("plug"), Qt::white, Qt::white, 16));
    use_->setMinimumHeight(42);
    use_->setMaximumWidth(280);
    detailLayout->addWidget(use_);
    detailLayout->addStretch();
    columns->addWidget(detailCard, 3);
    root->addLayout(columns);

    connect(year_, &QSpinBox::valueChanged, this, [this] { showGeneration(current_); });
    connect(use_, &QPushButton::clicked, this, [this] {
        if (current_) emit useForConnection(current_, year_->value());
    });

    detail_->setVisible(false);
    runSearch(QString());
}

void VehiclesPage::runSearch(const QString& text) {
    results_->clear();
    std::vector<const VehicleGeneration*> hits;
    if (text.trimmed().isEmpty()) {
        // Without a query: the newest generation of each popular brand would
        // be arbitrary, so show a few well-known ones to start from.
        for (const char* q : {"Golf", "A4", "3er", "C-Klasse", "Corsa", "Polo", "Octavia", "Model 3"}) {
            for (const VehicleGeneration* g : catalog::search(q, 3)) hits.push_back(g);
        }
    } else {
        hits = catalog::search(text.trimmed().toStdString(), 80);
    }
    for (const VehicleGeneration* g : hits) {
        auto* item = new QListWidgetItem(QStringLiteral("%1   %2")
                                             .arg(QString::fromStdString(catalog::displayName(*g)),
                                                  QString::fromStdString(catalog::yearsText(*g))));
        item->setData(Qt::UserRole, QVariant::fromValue(g));
        results_->addItem(item);
    }
    count_->setText(text.trimmed().isEmpty() ? tr("Beliebte Modelle, oben suchen für alle")
                                             : tr("%n Treffer", nullptr, static_cast<int>(hits.size())));
    if (results_->count() > 0) results_->setCurrentRow(0);
}

void VehiclesPage::showGeneration(const VehicleGeneration* g) {
    if (!g) return;
    const bool changed = g != current_;
    current_ = g;
    detail_->setVisible(true);
    if (changed) {
        const QSignalBlocker block(year_);
        const int last = g->yearTo > 0 ? g->yearTo : QDate::currentDate().year();
        year_->setRange(g->yearFrom, last);
        year_->setValue(last);
    }
    const int year = year_->value();
    const IdentifiedVehicle v = identifyGeneration(*g, year);
    picture_->setBodyStyle(v.body);
    name_->setText(v.title);
    years_->setText(v.subtitle);
    const ObdSupport support = catalog::supportFor(*g, year);
    const QColor tone = supportColour(support);
    support_->setText(QString::fromUtf8(supportName(support)));
    support_->setStyleSheet(QStringLiteral("color: %1; background: %2; border-radius: 11px; "
                                           "padding: 3px 12px; font-weight: 700;")
                                .arg(tone.lighter(110).name(), rgba(tone, 0.15)));
    diagnosis_->setText(QString::fromStdString(catalog::diagnosisText(*g, year)));
    connector_->setText(tr("Stecker: %1").arg(QString::fromUtf8(connectorName(catalog::connectorFor(*g, year)))));
    location_->setText(tr("Wo er sitzt: %1").arg(QString::fromStdString(locationText(*g, year))));
    note_->setText(QString::fromUtf8(g->note.get()));
    note_->setVisible(!note_->text().isEmpty());
    use_->setEnabled(support != ObdSupport::None);
}

void VehiclesPage::showVin(const QString& vin) {
    vinEdit_->setText(vin);
    checkVin();
}

void VehiclesPage::checkVin() {
    while (QLayoutItem* item = vinResult_->takeAt(0)) {
        if (item->widget()) item->widget()->deleteLater();
        delete item;
    }
    const QString text = vinEdit_->text().trimmed().toUpper();
    if (text.isEmpty()) return;
    const VinInfo info = decodeVin(text.toStdString(), QDate::currentDate().year());
    for (const std::string& p : info.problems) {
        vinResult_->addWidget(new FindingRow(QStringLiteral("alert"), color::warn,
                                             QString::fromStdString(p), QString()));
    }
    if (!info.valid) return;
    vinResult_->addWidget(factRow(tr("Hersteller"),
                                  info.manufacturer.empty() ? tr("unbekannt")
                                                            : QString::fromStdString(info.manufacturer)));
    vinResult_->addWidget(factRow(tr("Herkunft"), QStringLiteral("%1, %2").arg(
                                                     QString::fromStdString(info.country),
                                                     QString::fromStdString(info.region))));
    QStringList years;
    for (int y : info.modelYears) years << QString::number(y);
    vinResult_->addWidget(factRow(
        tr("Modelljahr"),
        years.isEmpty() ? tr("steht nicht in der Nummer")
                        : (info.modelYearCertain ? years.first()
                                                 : tr("%1 (nicht sicher)").arg(years.join(QStringLiteral(" / "))))));
    if (!info.modelCode.empty()) {
        vinResult_->addWidget(factRow(tr("Modellcode"), QString::fromStdString(info.modelCode), true));
    }
    vinResult_->addWidget(factRow(
        tr("Prüfziffer"),
        !info.checkDigitApplies ? tr("wird hier nicht verwendet")
        : info.checkDigitOk     ? tr("stimmt")
                                : tr("stimmt nicht, erwartet wäre %1").arg(QChar(info.expectedCheckDigit))));
    vinResult_->addWidget(factRow(tr("Seriennummer"), QString::fromStdString(info.serial), true));
    for (const std::string& n : info.notes) {
        vinResult_->addWidget(makeLabel(QString::fromStdString(n), "Hint", true));
    }

    const CatalogMatch match = catalog::matchVin(info);
    if (match.generation) {
        vinResult_->addWidget(new FindingRow(
            QStringLiteral("car"), color::accent,
            tr("Passt zu: %1").arg(QString::fromStdString(catalog::displayName(*match.generation))),
            tr("Sicherheit %1 von 100. %2").arg(match.confidence).arg(QString::fromStdString(match.reason))));
        // Show the match and its alternatives, without running a text search.
        {
            const QSignalBlocker block(search_);
            search_->setText(QString::fromUtf8(match.generation->model));
        }
        results_->clear();
        std::vector<const VehicleGeneration*> list{match.generation};
        for (const VehicleGeneration* g : match.alternatives) {
            if (g != match.generation) list.push_back(g);
        }
        for (const VehicleGeneration* g : list) {
            auto* item = new QListWidgetItem(QStringLiteral("%1   %2")
                                                 .arg(QString::fromStdString(catalog::displayName(*g)),
                                                      QString::fromStdString(catalog::yearsText(*g))));
            item->setData(Qt::UserRole, QVariant::fromValue(g));
            results_->addItem(item);
        }
        count_->setText(tr("Treffer zur Fahrgestellnummer"));
        results_->setCurrentRow(0);
        showGeneration(match.generation);
        if (match.year > 0) year_->setValue(match.year);
    }
}

}  // namespace caruspex
