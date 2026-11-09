#include "app/pages/GaragePage.h"

#include <QCheckBox>
#include <QComboBox>
#include <QDateEdit>
#include <QDialog>
#include <QDialogButtonBox>
#include <QDir>
#include <QDoubleSpinBox>
#include <QFileDialog>
#include <QFormLayout>
#include <QFrame>
#include <QLineEdit>
#include <QSpinBox>
#include <QStandardPaths>
#include <QTabWidget>
#include <algorithm>
#include <functional>
#include <QHBoxLayout>
#include <QLabel>
#include <QLocale>
#include <QMessageBox>
#include <QMouseEvent>
#include <QPainter>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QScrollArea>
#include <QTimer>
#include <QVBoxLayout>

#include "app/Garage.h"
#include "app/Icons.h"
#include "app/Theme.h"
#include "app/VehicleDiagram.h"
#include "app/Widgets.h"

namespace caruspex {

using namespace theme;

namespace {

// Clickable card for one car in the list.
class CarCard : public QFrame {
public:
    CarCard(const GarageCar& car, bool selected, bool current) {
        setCursor(Qt::PointingHandCursor);
        setObjectName("CarCard");
        setStyleSheet(QStringLiteral("QFrame#CarCard { background: %1; border: 1px solid %2; "
                                     "border-radius: 14px; }"
                                     "QFrame#CarCard:hover { border-color: %3; }")
                          .arg(selected ? rgba(color::accent, 0.10) : color::surface.name(),
                               selected ? rgba(color::accent, 0.55) : color::border.name(),
                               rgba(color::accent, 0.45)));
        auto* layout = new QHBoxLayout(this);
        layout->setContentsMargins(10, 8, 14, 8);
        layout->setSpacing(12);
        auto* picture = new VehicleDiagram;
        picture->setBodyStyle(static_cast<BodyStyle>(car.bodyStyle));
        picture->setFixedSize(132, 58);
        picture->setAttribute(Qt::WA_TransparentForMouseEvents);
        layout->addWidget(picture);
        auto* texts = new QVBoxLayout;
        texts->setSpacing(2);
        auto* name = new QLabel(car.name.isEmpty() ? QObject::tr("Unbekanntes Fahrzeug") : car.name);
        name->setStyleSheet(QStringLiteral("font-weight: 700; font-size: 14px;"));
        texts->addWidget(name);
        auto* vin = new QLabel(car.vin);
        vin->setFont(mono(12));
        vin->setStyleSheet(QStringLiteral("color: %1;").arg(color::textFaint.name()));
        texts->addWidget(vin);
        int faults = car.checks.isEmpty() ? 0 : car.checks.back().codes.size();
        QString state = car.checks.isEmpty()
                            ? QObject::tr("noch keine Prüfung")
                            : (faults == 0 ? QObject::tr("zuletzt ohne Fehler")
                                           : QObject::tr("zuletzt %n Fehler", nullptr, faults));
        auto* line = makeStatusLine(state, car.checks.isEmpty() ? color::textFaint
                                           : faults == 0              ? color::good
                                                                      : color::bad);
        texts->addWidget(line);
        layout->addLayout(texts, 1);
        if (current) {
            auto* live = new QLabel(QObject::tr("verbunden"));
            live->setStyleSheet(QStringLiteral("color: %1; background: %2; border-radius: 9px; "
                                               "padding: 2px 8px; font-size: 11px; font-weight: 700;")
                                    .arg(color::good.name(), rgba(color::good, 0.14)));
            layout->addWidget(live, 0, Qt::AlignTop);
        }
    }
    std::function<void()> onClick;

protected:
    void mouseReleaseEvent(QMouseEvent* event) override {
        if (event->button() == Qt::LeftButton && onClick) onClick();
    }
};

}  // namespace

GaragePage::GaragePage(Garage* garage, QWidget* parent) : QWidget(parent), garage_(garage) {
    auto* root = new QVBoxLayout(this);
    root->setContentsMargins(32, 26, 32, 24);
    root->setSpacing(18);
    root->addWidget(makePageHeader(tr("Garage"), tr("Deine Fahrzeuge"),
                                   tr("Jedes Auto, das Caruspex einmal gesehen hat, erkannt an "
                                      "seiner Fahrgestellnummer. Mit allen Prüfungen und "
                                      "Kilometerständen. Bleibt auf diesem Rechner.")));

    auto* tools = new QHBoxLayout;
    tools->setSpacing(8);
    tools->addStretch();
    auto* exportButton = new QPushButton(tr("Garage sichern"));
    exportButton->setObjectName("Ghost");
    exportButton->setIcon(icons::icon(QStringLiteral("download"), color::textMuted, color::text, 16));
    auto* importButton = new QPushButton(tr("Sicherung laden"));
    importButton->setObjectName("Ghost");
    importButton->setIcon(icons::icon(QStringLiteral("refresh"), color::textMuted, color::text, 16));
    tools->addWidget(exportButton);
    tools->addWidget(importButton);
    root->addLayout(tools);
    connect(exportButton, &QPushButton::clicked, this, [this] {
        const QString path = QFileDialog::getSaveFileName(
            this, tr("Garage sichern"),
            QDir(QStandardPaths::writableLocation(QStandardPaths::DocumentsLocation))
                .filePath(QStringLiteral("caruspex-garage-%1.json").arg(QDate::currentDate().toString(Qt::ISODate))),
            tr("Garage (*.json)"));
        if (path.isEmpty()) return;
        if (garage_->exportTo(path)) {
            toast::show(this, tr("Garage gesichert: %1").arg(path), toast::Kind::Success);
        } else {
            toast::show(this, tr("Sichern hat nicht geklappt."), toast::Kind::Error);
        }
    });
    connect(importButton, &QPushButton::clicked, this, [this] {
        const QString path = QFileDialog::getOpenFileName(this, tr("Sicherung laden"), QString(),
                                                          tr("Garage (*.json)"));
        if (path.isEmpty()) return;
        QString error;
        const int count = garage_->importFrom(path, &error);
        if (count < 0) {
            toast::show(this, tr("Laden hat nicht geklappt: %1").arg(error), toast::Kind::Error);
        } else {
            toast::show(this, tr("%n Auto(s) übernommen, nichts wurde gelöscht.", nullptr, count),
                        toast::Kind::Success);
        }
    });

    auto* columns = new QHBoxLayout;
    columns->setSpacing(20);

    // List
    auto* scroll = new QScrollArea;
    scroll->setWidgetResizable(true);
    scroll->setFrameShape(QFrame::NoFrame);
    scroll->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    scroll->setMinimumWidth(420);
    scroll->setMaximumWidth(460);
    auto* listHost = new QWidget;
    list_ = new QVBoxLayout(listHost);
    list_->setContentsMargins(0, 0, 6, 0);
    list_->setSpacing(10);
    scroll->setWidget(listHost);
    columns->addWidget(scroll);

    // Detail
    auto* right = new QVBoxLayout;
    right->setSpacing(0);
    QVBoxLayout* detailLayout = nullptr;
    QFrame* detailCard = makeCard(&detailLayout, 24);
    detail_ = detailCard;
    auto* head = new QHBoxLayout;
    head->setSpacing(18);
    picture_ = new VehicleDiagram;
    picture_->setFixedSize(300, 126);
    head->addWidget(picture_);
    auto* headText = new QVBoxLayout;
    headText->setSpacing(4);
    name_ = makeLabel(QString(), "PageTitle", true);
    meta_ = makeLabel(QString(), "Subtle", true);
    meta_->setTextInteractionFlags(Qt::TextSelectableByMouse);
    headText->addStretch();
    headText->addWidget(name_);
    headText->addWidget(meta_);
    auto* actions = new QHBoxLayout;
    actions->setSpacing(8);
    report_ = new QPushButton(tr("Prüfbericht"));
    report_->setIcon(icons::icon(QStringLiteral("file"), color::textMuted, color::text, 16));
    remove_ = new QPushButton(tr("Entfernen"));
    remove_->setToolTip(tr("Aus der Garage entfernen"));
    remove_->setObjectName("Ghost");
    remove_->setIcon(icons::icon(QStringLiteral("trash"), color::textMuted, color::bad, 16));
    actions->addWidget(report_);
    actions->addWidget(remove_);
    actions->addStretch();
    headText->addSpacing(6);
    headText->addLayout(actions);
    headText->addStretch();
    head->addLayout(headText, 1);
    detailLayout->addLayout(head);

    tabs_ = new QTabWidget;
    tabs_->setDocumentMode(true);
    auto scrolledTab = [this](const QString& title, QVBoxLayout** body, QWidget* top = nullptr) {
        auto* scroll = new QScrollArea;
        scroll->setWidgetResizable(true);
        scroll->setFrameShape(QFrame::NoFrame);
        auto* host = new QWidget;
        auto* outer = new QVBoxLayout(host);
        outer->setContentsMargins(0, 10, 0, 0);
        outer->setSpacing(10);
        if (top) outer->addWidget(top);
        *body = new QVBoxLayout;
        (*body)->setSpacing(0);
        outer->addLayout(*body);
        outer->addStretch();
        scroll->setWidget(host);
        tabs_->addTab(scroll, title);
    };
    scrolledTab(tr("Verlauf"), &history_);

    // Service plan with its own add button.
    auto* serviceBar = new QWidget;
    auto* serviceRow = new QHBoxLayout(serviceBar);
    serviceRow->setContentsMargins(0, 0, 0, 0);
    serviceRow->addWidget(makeLabel(tr("Was wann fällig ist, nach Kilometer oder Zeit, je "
                                       "nachdem was zuerst kommt."),
                                    "Subtle", true),
                          1);
    auto* addService = new QPushButton(tr("Wartung hinzufügen"));
    addService->setIcon(icons::icon(QStringLiteral("wrench"), color::textMuted, color::text, 16));
    serviceRow->addWidget(addService);
    scrolledTab(tr("Wartung"), &services_, serviceBar);
    connect(addService, &QPushButton::clicked, this, &GaragePage::addService);

    // Trips with a small summary.
    auto* tripBar = new QWidget;
    auto* tripRow = new QHBoxLayout(tripBar);
    tripRow->setContentsMargins(0, 0, 0, 0);
    tripRow->setSpacing(10);
    tripDistance_ = new StatTile(QStringLiteral("route"), tr("Strecke gesamt"));
    tripConsumption_ = new StatTile(QStringLiteral("fuel"), tr("Verbrauch im Schnitt"));
    tripCost_ = new StatTile(QStringLiteral("file"), tr("Kosten gesamt"));
    tripRow->addWidget(tripDistance_);
    tripRow->addWidget(tripConsumption_);
    tripRow->addWidget(tripCost_);
    scrolledTab(tr("Fahrten"), &trips_, tripBar);

    // Fuel log
    auto* fuelBar = new QWidget;
    auto* fuelRow = new QHBoxLayout(fuelBar);
    fuelRow->setContentsMargins(0, 0, 0, 0);
    fuelRow->setSpacing(10);
    fuelConsumption_ = new StatTile(QStringLiteral("fuel"), tr("Verbrauch laut Tankbuch"));
    fuelCost_ = new StatTile(QStringLiteral("file"), tr("Getankt gesamt"));
    fuelRow->addWidget(fuelConsumption_);
    fuelRow->addWidget(fuelCost_);
    auto* addFuel = new QPushButton(tr("Tankfüllung eintragen"));
    addFuel->setIcon(icons::icon(QStringLiteral("fuel"), color::textMuted, color::text, 16));
    fuelRow->addWidget(addFuel, 0, Qt::AlignTop);
    scrolledTab(tr("Tankbuch"), &fuel_, fuelBar);
    connect(addFuel, &QPushButton::clicked, this, &GaragePage::addFuel);

    // Note
    auto* noteHost = new QWidget;
    auto* noteLayout = new QVBoxLayout(noteHost);
    noteLayout->setContentsMargins(0, 10, 0, 0);
    note_ = new QPlainTextEdit;
    note_->setPlaceholderText(tr("z.B. Zahnriemen bei 180 000 km gewechselt"));
    noteLayout->addWidget(note_);
    tabs_->addTab(noteHost, tr("Notiz"));
    detailLayout->addWidget(tabs_, 1);
    right->addWidget(detailCard, 1);

    QVBoxLayout* emptyLayout = nullptr;
    QFrame* emptyCard = makeCard(&emptyLayout, 32);
    empty_ = emptyCard;
    auto* emptyIcon = new QLabel;
    emptyIcon->setPixmap(icons::pixmap(QStringLiteral("garage"), 48, color::textFaint, 1.4));
    emptyIcon->setAlignment(Qt::AlignCenter);
    emptyLayout->addStretch();
    emptyLayout->addWidget(emptyIcon);
    auto* emptyTitle = makeLabel(tr("Die Garage ist noch leer"), "SectionTitle");
    emptyTitle->setAlignment(Qt::AlignCenter);
    emptyLayout->addWidget(emptyTitle);
    auto* emptyText = makeLabel(tr("Sobald ein Auto verbunden ist und seine Fahrgestellnummer "
                                   "verrät, steht es hier. Ältere Autos ohne VIN im Steuergerät "
                                   "lassen sich leider nicht wiedererkennen."),
                                "Subtle", true);
    emptyText->setAlignment(Qt::AlignCenter);
    emptyLayout->addWidget(emptyText);
    emptyLayout->addStretch();
    right->addWidget(emptyCard, 1);
    columns->addLayout(right, 1);
    root->addLayout(columns, 1);

    connect(garage_, &Garage::changed, this, [this] {
        rebuildList();
        if (!selected_.isEmpty()) showCar(selected_);
    });
    connect(remove_, &QPushButton::clicked, this, [this] {
        if (selected_.isEmpty()) return;
        const auto answer = QMessageBox::question(
            this, tr("Entfernen"),
            tr("Das Auto und seinen ganzen Verlauf aus der Garage entfernen? Das lässt sich "
               "nicht rückgängig machen."));
        if (answer != QMessageBox::Yes) return;
        const QString vin = selected_;
        selected_.clear();
        garage_->remove(vin);
    });
    connect(report_, &QPushButton::clicked, this, &GaragePage::reportRequested);
    auto* noteTimer = new QTimer(this);
    noteTimer->setSingleShot(true);
    noteTimer->setInterval(700);
    connect(note_, &QPlainTextEdit::textChanged, noteTimer, qOverload<>(&QTimer::start));
    connect(noteTimer, &QTimer::timeout, this, [this] {
        if (!selected_.isEmpty()) {
            const GarageCar* car = garage_->find(selected_);
            if (car && car->note != note_->toPlainText()) {
                garage_->setNote(selected_, note_->toPlainText());
            }
        }
    });

    rebuildList();
}

void GaragePage::setCurrentVin(const QString& vin) {
    current_ = vin;
    if (!vin.isEmpty()) selected_ = vin;
    rebuildList();
    if (!selected_.isEmpty()) showCar(selected_);
}

void GaragePage::rebuildList() {
    while (QLayoutItem* item = list_->takeAt(0)) {
        if (item->widget()) item->widget()->deleteLater();
        delete item;
    }
    const QVector<GarageCar>& cars = garage_->cars();
    if (selected_.isEmpty() && !cars.isEmpty()) selected_ = cars.back().vin;
    // Most recently seen first.
    QVector<const GarageCar*> sorted;
    for (const GarageCar& c : cars) sorted.push_back(&c);
    std::sort(sorted.begin(), sorted.end(),
              [](const GarageCar* a, const GarageCar* b) { return a->lastSeen > b->lastSeen; });
    for (const GarageCar* car : sorted) {
        auto* card = new CarCard(*car, car->vin == selected_, car->vin == current_);
        const QString vin = car->vin;
        card->onClick = [this, vin] {
            selected_ = vin;
            rebuildList();
            showCar(vin);
        };
        list_->addWidget(card);
    }
    list_->addStretch();
    const bool any = !cars.isEmpty();
    detail_->setVisible(any);
    empty_->setVisible(!any);
    remove_->setEnabled(any);
    if (any && !selected_.isEmpty()) showCar(selected_);
}

void GaragePage::showCar(const QString& vin) {
    const GarageCar* car = garage_->find(vin);
    if (!car) return;
    picture_->setBodyStyle(static_cast<BodyStyle>(car->bodyStyle));
    name_->setText(car->name.isEmpty() ? tr("Unbekanntes Fahrzeug") : car->name);
    const QLocale locale;
    QStringList meta;
    if (!car->description.isEmpty()) meta << car->description;
    meta << tr("VIN %1").arg(car->vin);
    meta << tr("zuerst gesehen %1").arg(locale.toString(car->firstSeen.date(), QLocale::ShortFormat));
    meta_->setText(meta.join(QStringLiteral(" · ")));
    report_->setEnabled(vin == current_);
    report_->setToolTip(vin == current_ ? QString()
                                        : tr("Der Bericht braucht das verbundene Auto."));
    if (note_->toPlainText() != car->note) {
        const QSignalBlocker block(note_);
        note_->setPlainText(car->note);
    }

    fillHistory(*car);
    fillServices(*car);
    fillTrips(*car);
    fillFuel(*car);
}

namespace {

void clearBox(QVBoxLayout* layout) {
    while (QLayoutItem* item = layout->takeAt(0)) {
        if (item->widget()) item->widget()->deleteLater();
        delete item;
    }
}

}  // namespace

void GaragePage::showTab(int index) { tabs_->setCurrentIndex(index); }

void GaragePage::setCurrentOdometer(double km) {
    currentKm_ = km;
    if (!selected_.isEmpty() && selected_ == current_) {
        if (const GarageCar* car = garage_->find(selected_)) fillServices(*car);
    }
}

void GaragePage::fillHistory(const GarageCar& car) {
    clearBox(history_);
    const QLocale locale;
    // A falling odometer between two checks is worth a word, especially when
    // buying a used car. It is a hint: a swapped instrument cluster or a
    // mistake in reading can explain it too.
    for (const OdometerJump& j : Garage::odometerJumps(car)) {
        history_->addWidget(new FindingRow(
            QStringLiteral("alert"), color::bad, tr("Kilometerstand ist gesunken"),
            tr("Am %1 waren es %2 km, am %3 nur noch %4 km. Das kann ein getauschtes Kombiinstrument "
               "sein, aber auch ein zurückgedrehter Tacho. Beim Gebrauchtwagenkauf nachfragen.")
                .arg(locale.toString(j.earlier.date(), QLocale::ShortFormat))
                .arg(locale.toString(j.earlierKm, 'f', 0))
                .arg(locale.toString(j.later.date(), QLocale::ShortFormat))
                .arg(locale.toString(j.laterKm, 'f', 0))));
    }
    if (car.checks.isEmpty()) {
        history_->addWidget(new FindingRow(QStringLiteral("info"), color::info,
                                           tr("Noch keine Prüfung gespeichert"),
                                           tr("Auto prüfen auf der Startseite legt einen "
                                              "Eintrag an.")));
    }
    for (int i = car.checks.size() - 1; i >= 0; --i) {
        const GarageCheck& c = car.checks[i];
        QString title = locale.toString(c.when, QLocale::ShortFormat);
        if (c.odometerKm >= 0) {
            title += QStringLiteral("  ·  %1 km").arg(locale.toString(c.odometerKm, 'f', 0));
        }
        QStringList parts;
        for (int k = 0; k < c.codes.size(); ++k) {
            parts << (k < c.texts.size() ? QStringLiteral("%1 %2").arg(c.codes[k], c.texts[k])
                                         : c.codes[k]);
        }
        QString text = parts.isEmpty() ? tr("Kein Fehler gespeichert.") : parts.join(QStringLiteral("\n"));
        if (c.mil) text += QStringLiteral("\n") + tr("Motorkontrollleuchte an.");
        if (c.openMonitors > 0) {
            text += QStringLiteral("\n") + tr("%n Bereitschaftstest(s) offen.", nullptr, c.openMonitors);
        }
        const QColor tint = c.codes.isEmpty() ? color::good : color::bad;
        const QString meta = c.score >= 0 ? tr("%1 Punkte").arg(c.score) : QString();
        history_->addWidget(new FindingRow(c.codes.isEmpty() ? QStringLiteral("check-circle")
                                                             : QStringLiteral("alert"),
                                           tint, title, text, meta));
    }
}

void GaragePage::fillServices(const GarageCar& car) {
    clearBox(services_);
    const double km = car.vin == current_ && currentKm_ >= 0 ? currentKm_
                                                             : garage_->latestOdometer(car.vin);
    const QLocale locale;
    const QString vin = car.vin;
    for (const ServiceStatus& st : Garage::serviceStatus(car, km, QDate::currentDate())) {
        const GarageService& s = *st.service;
        QColor tint = color::textFaint;
        QString icon = QStringLiteral("clock");
        switch (st.state) {
            case ServiceStatus::State::Due: tint = color::bad; icon = QStringLiteral("alert"); break;
            case ServiceStatus::State::Soon: tint = color::warn; break;
            case ServiceStatus::State::Ok: tint = color::good; icon = QStringLiteral("check-circle"); break;
            case ServiceStatus::State::Unknown: break;
        }
        QStringList rule;
        if (s.intervalKm > 0) rule << tr("alle %1 km").arg(locale.toString(s.intervalKm));
        if (s.intervalMonths > 0) rule << tr("alle %n Monat(e)", nullptr, s.intervalMonths);
        QString text = rule.join(tr(" oder "));
        if (s.lastDate.isValid() || s.lastKm >= 0) {
            QStringList last;
            if (s.lastDate.isValid()) last << locale.toString(s.lastDate, QLocale::ShortFormat);
            if (s.lastKm >= 0) last << QStringLiteral("%1 km").arg(locale.toString(s.lastKm, 'f', 0));
            text += QStringLiteral("\n") + tr("Zuletzt: %1").arg(last.join(QStringLiteral(", ")));
        }

        auto* row = new QWidget;
        auto* h = new QHBoxLayout(row);
        h->setContentsMargins(0, 0, 0, 0);
        h->setSpacing(6);
        h->addWidget(new FindingRow(icon, tint, s.title, text, st.text), 1);
        auto* done = new QPushButton(tr("Erledigt"));
        done->setObjectName("Ghost");
        done->setIcon(icons::icon(QStringLiteral("check"), color::textMuted, color::good, 14));
        auto* drop = new QPushButton;
        drop->setObjectName("Ghost");
        drop->setIcon(icons::icon(QStringLiteral("trash"), color::textFaint, color::bad, 14));
        drop->setToolTip(tr("Entfernen"));
        h->addWidget(done);
        h->addWidget(drop);
        services_->addWidget(row);
        const QString id = s.id;
        const QString title = s.title;
        connect(done, &QPushButton::clicked, this, [this, vin, id, title, km] {
            QDialog dialog(this);
            dialog.setWindowTitle(tr("%1 erledigt").arg(title));
            auto* form = new QFormLayout(&dialog);
            auto* date = new QDateEdit(QDate::currentDate());
            date->setCalendarPopup(true);
            auto* odo = new QDoubleSpinBox;
            odo->setRange(0, 2000000);
            odo->setDecimals(0);
            odo->setSuffix(QStringLiteral(" km"));
            odo->setValue(km >= 0 ? km : 0);
            form->addRow(tr("Datum"), date);
            form->addRow(tr("Kilometerstand"), odo);
            auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel);
            form->addRow(buttons);
            connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
            connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
            if (dialog.exec() == QDialog::Accepted) {
                garage_->markServiceDone(vin, id, date->date(), odo->value() > 0 ? odo->value() : -1.0);
            }
        });
        connect(drop, &QPushButton::clicked, this, [this, vin, id] { garage_->removeService(vin, id); });
    }
    if (car.services.isEmpty()) {
        services_->addWidget(new FindingRow(QStringLiteral("wrench"), color::info,
                                            tr("Noch keine Wartung eingetragen"),
                                            tr("Ölwechsel, Inspektion, HU und mehr: einmal "
                                               "eintragen, Caruspex erinnert beim Verbinden.")));
    }
}

void GaragePage::fillTrips(const GarageCar& car) {
    clearBox(trips_);
    const QLocale locale;
    double distance = 0.0;
    double fuel = 0.0;
    double cost = 0.0;
    for (int i = car.trips.size() - 1; i >= 0; --i) {
        const GarageTrip& t = car.trips[i];
        distance += t.distanceKm;
        fuel += t.fuelLitres;
        cost += t.cost;
        QString text = tr("%1 km, %2 l/100 km")
                           .arg(locale.toString(t.distanceKm, 'f', 1))
                           .arg(t.averageL100 > 0 ? locale.toString(t.averageL100, 'f', 1)
                                                  : QStringLiteral("--"));
        if (t.cost > 0) text += tr(", %1 €").arg(locale.toString(t.cost, 'f', 2));
        if (t.maxSpeedKmh > 0) text += tr(", höchstens %1 km/h").arg(locale.toString(t.maxSpeedKmh, 'f', 0));
        const QString meta = t.ecoScore >= 0 ? tr("Fahrstil %1").arg(t.ecoScore) : QString();
        trips_->addWidget(new FindingRow(QStringLiteral("route"), color::accent,
                                         locale.toString(t.start, QLocale::ShortFormat), text, meta));
    }
    if (car.trips.isEmpty()) {
        trips_->addWidget(new FindingRow(QStringLiteral("route"), color::info,
                                         tr("Noch keine Fahrt gespeichert"),
                                         tr("Der Bordcomputer legt hier jede Fahrt ab, sobald sie "
                                            "zurückgesetzt oder die Verbindung getrennt wird.")));
    }
    tripDistance_->setValue(locale.toString(distance, 'f', 0), QStringLiteral("km"));
    tripConsumption_->setValue(distance > 1 ? locale.toString(fuel / distance * 100.0, 'f', 1)
                                            : QStringLiteral("--"),
                               QStringLiteral("l/100 km"));
    tripCost_->setValue(cost > 0 ? locale.toString(cost, 'f', 2) : QStringLiteral("--"), QStringLiteral("€"));
}

void GaragePage::fillFuel(const GarageCar& car) {
    clearBox(fuel_);
    const QLocale locale;
    double total = 0.0;
    const QString vin = car.vin;
    for (int i = car.fuel.size() - 1; i >= 0; --i) {
        const GarageFuel& f = car.fuel[i];
        const double price = f.litres * f.pricePerLitre;
        total += price;
        QString text = tr("%1 l zu %2 €/l").arg(locale.toString(f.litres, 'f', 2),
                                                locale.toString(f.pricePerLitre, 'f', 3));
        if (f.odometerKm >= 0) text += tr(", bei %1 km").arg(locale.toString(f.odometerKm, 'f', 0));
        if (!f.full) text += tr(", nicht voll getankt");
        auto* row = new QWidget;
        auto* h = new QHBoxLayout(row);
        h->setContentsMargins(0, 0, 0, 0);
        h->addWidget(new FindingRow(QStringLiteral("fuel"), color::accent,
                                    locale.toString(f.date, QLocale::ShortFormat), text,
                                    price > 0 ? QStringLiteral("%1 €").arg(locale.toString(price, 'f', 2))
                                              : QString()),
                     1);
        auto* drop = new QPushButton;
        drop->setObjectName("Ghost");
        drop->setIcon(icons::icon(QStringLiteral("trash"), color::textFaint, color::bad, 14));
        drop->setToolTip(tr("Entfernen"));
        h->addWidget(drop);
        fuel_->addWidget(row);
        connect(drop, &QPushButton::clicked, this, [this, vin, i] { garage_->removeFuel(vin, i); });
    }
    if (car.fuel.isEmpty()) {
        fuel_->addWidget(new FindingRow(QStringLiteral("fuel"), color::info,
                                        tr("Noch nichts eingetragen"),
                                        tr("Mit zwei vollen Tankfüllungen samt Kilometerstand rechnet "
                                           "Caruspex den echten Verbrauch aus.")));
    }
    const double consumption = Garage::fuelLogConsumption(car);
    fuelConsumption_->setValue(consumption > 0 ? locale.toString(consumption, 'f', 1) : QStringLiteral("--"),
                               QStringLiteral("l/100 km"));
    fuelConsumption_->setNote(consumption > 0 ? QString() : tr("braucht zwei volle Tankfüllungen"));
    fuelCost_->setValue(total > 0 ? locale.toString(total, 'f', 2) : QStringLiteral("--"), QStringLiteral("€"));
}

void GaragePage::addService() {
    if (selected_.isEmpty()) return;
    QDialog dialog(this);
    dialog.setWindowTitle(tr("Wartung hinzufügen"));
    auto* form = new QFormLayout(&dialog);
    auto* kind = new QComboBox;
    const QVector<GarageService> templates = Garage::serviceTemplates();
    for (const GarageService& t : templates) kind->addItem(t.title);
    kind->addItem(tr("Eigener Eintrag"));
    auto* title = new QLineEdit;
    auto* km = new QSpinBox;
    km->setRange(0, 1000000);
    km->setSingleStep(1000);
    km->setSuffix(QStringLiteral(" km"));
    km->setSpecialValueText(tr("nicht nach km"));
    auto* months = new QSpinBox;
    months->setRange(0, 240);
    months->setSuffix(tr(" Monate"));
    months->setSpecialValueText(tr("nicht nach Zeit"));
    auto* lastDate = new QDateEdit(QDate::currentDate());
    lastDate->setCalendarPopup(true);
    auto* known = new QCheckBox(tr("Zuletzt erledigt am"));
    auto* lastKm = new QDoubleSpinBox;
    lastKm->setRange(0, 2000000);
    lastKm->setDecimals(0);
    lastKm->setSuffix(QStringLiteral(" km"));
    lastKm->setSpecialValueText(tr("unbekannt"));
    const double odo = selected_ == current_ && currentKm_ >= 0 ? currentKm_ : garage_->latestOdometer(selected_);
    lastKm->setValue(odo > 0 ? odo : 0);
    auto apply = [&](int index) {
        const bool custom = index >= templates.size();
        title->setEnabled(custom);
        title->setText(custom ? QString() : templates[index].title);
        if (!custom) {
            km->setValue(templates[index].intervalKm);
            months->setValue(templates[index].intervalMonths);
        }
    };
    apply(0);
    connect(kind, &QComboBox::currentIndexChanged, &dialog, apply);
    form->addRow(tr("Art"), kind);
    form->addRow(tr("Bezeichnung"), title);
    form->addRow(tr("Alle"), km);
    form->addRow(tr("oder alle"), months);
    form->addRow(known, lastDate);
    form->addRow(tr("bei Kilometerstand"), lastKm);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel);
    form->addRow(buttons);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    if (dialog.exec() != QDialog::Accepted || title->text().trimmed().isEmpty()) return;
    GarageService s;
    s.title = title->text().trimmed();
    s.intervalKm = km->value();
    s.intervalMonths = months->value();
    if (known->isChecked()) {
        s.lastDate = lastDate->date();
        s.lastKm = lastKm->value() > 0 ? lastKm->value() : -1.0;
    }
    garage_->setService(selected_, s);
}

void GaragePage::addFuel() {
    if (selected_.isEmpty()) return;
    QDialog dialog(this);
    dialog.setWindowTitle(tr("Tankfüllung eintragen"));
    auto* form = new QFormLayout(&dialog);
    auto* date = new QDateEdit(QDate::currentDate());
    date->setCalendarPopup(true);
    auto* odo = new QDoubleSpinBox;
    odo->setRange(0, 2000000);
    odo->setDecimals(0);
    odo->setSuffix(QStringLiteral(" km"));
    const double km = selected_ == current_ && currentKm_ >= 0 ? currentKm_ : garage_->latestOdometer(selected_);
    odo->setValue(km > 0 ? km : 0);
    auto* litres = new QDoubleSpinBox;
    litres->setRange(0, 500);
    litres->setDecimals(2);
    litres->setSuffix(QStringLiteral(" l"));
    auto* price = new QDoubleSpinBox;
    price->setRange(0, 10);
    price->setDecimals(3);
    price->setSuffix(QStringLiteral(" €/l"));
    price->setValue(1.799);
    auto* full = new QCheckBox(tr("voll getankt"));
    full->setChecked(true);
    form->addRow(tr("Datum"), date);
    form->addRow(tr("Kilometerstand"), odo);
    form->addRow(tr("Menge"), litres);
    form->addRow(tr("Preis"), price);
    form->addRow(full);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel);
    form->addRow(buttons);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    if (dialog.exec() != QDialog::Accepted || litres->value() <= 0) return;
    GarageFuel f;
    f.date = date->date();
    f.odometerKm = odo->value() > 0 ? odo->value() : -1.0;
    f.litres = litres->value();
    f.pricePerLitre = price->value();
    f.full = full->isChecked();
    garage_->addFuel(selected_, f);
}

}  // namespace caruspex
