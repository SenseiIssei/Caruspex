#include "app/pages/GaragePage.h"

#include <QFrame>
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
    remove_ = new QPushButton(tr("Aus der Garage entfernen"));
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

    auto* divider = new QFrame;
    divider->setObjectName("Divider");
    detailLayout->addWidget(divider);
    detailLayout->addWidget(makeLabel(tr("Verlauf"), "SectionTitle"));
    auto* historyScroll = new QScrollArea;
    historyScroll->setWidgetResizable(true);
    historyScroll->setFrameShape(QFrame::NoFrame);
    auto* historyHost = new QWidget;
    history_ = new QVBoxLayout(historyHost);
    history_->setContentsMargins(0, 0, 0, 0);
    history_->setSpacing(0);
    historyScroll->setWidget(historyHost);
    detailLayout->addWidget(historyScroll, 1);

    detailLayout->addWidget(makeLabel(tr("Notiz"), "SectionTitle"));
    note_ = new QPlainTextEdit;
    note_->setPlaceholderText(tr("z.B. Zahnriemen bei 180 000 km gewechselt"));
    note_->setMaximumHeight(90);
    detailLayout->addWidget(note_);
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

    while (QLayoutItem* item = history_->takeAt(0)) {
        if (item->widget()) item->widget()->deleteLater();
        delete item;
    }
    if (car->checks.isEmpty()) {
        history_->addWidget(new FindingRow(QStringLiteral("info"), color::info,
                                           tr("Noch keine Prüfung gespeichert"),
                                           tr("Auto prüfen auf der Startseite legt einen "
                                              "Eintrag an.")));
    }
    for (int i = car->checks.size() - 1; i >= 0; --i) {
        const GarageCheck& c = car->checks[i];
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
    history_->addStretch();
}

}  // namespace caruspex
