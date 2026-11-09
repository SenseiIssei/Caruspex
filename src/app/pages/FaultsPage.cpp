#include "app/pages/FaultsPage.h"

#include <QDateTime>
#include <QFileDialog>
#include <QFrame>
#include <QHBoxLayout>
#include <QLabel>
#include <QListWidget>
#include <QLocale>
#include <QMessageBox>
#include <QPainter>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QScrollArea>
#include <QStyledItemDelegate>
#include <QTextStream>
#include <QVBoxLayout>

#include "app/Icons.h"
#include "app/Theme.h"
#include "app/VehicleDiagram.h"
#include "app/Widgets.h"
#include "data/DtcDatabase.h"
#include "data/DtcLocation.h"

namespace caruspex {

using namespace theme;

QColor severityColour(int severity) {
    switch (static_cast<Severity>(severity)) {
        case Severity::Info: return color::info;
        case Severity::Low: return color::good;
        case Severity::Medium: return color::warn;
        case Severity::High: return color::bad;
        case Severity::Critical: return QColor("#ff4d6a");
    }
    return color::textMuted;
}

namespace {

enum Role { CodeRole = Qt::UserRole + 1, TextRole, MetaRole, SeverityRole, LabelRole };

// Paints one fault as a row: severity bar, code, text, a small pill.
class FaultDelegate : public QStyledItemDelegate {
public:
    using QStyledItemDelegate::QStyledItemDelegate;

    QSize sizeHint(const QStyleOptionViewItem& option, const QModelIndex&) const override {
        return {option.rect.width(), 78};
    }

    void paint(QPainter* p, const QStyleOptionViewItem& option,
               const QModelIndex& index) const override {
        p->save();
        p->setRenderHint(QPainter::Antialiasing);
        const QRectF r = QRectF(option.rect).adjusted(4, 4, -4, -4);
        const bool selected = option.state & QStyle::State_Selected;
        const bool hover = option.state & QStyle::State_MouseOver;
        const QColor tone = severityColour(index.data(SeverityRole).toInt());

        QColor fill = selected ? rgbaColour(color::accent, 0.12)
                               : hover ? rgbaColour(color::text, 0.04) : QColor(0, 0, 0, 0);
        p->setPen(selected ? QPen(rgbaColour(color::accent, 0.5), 1.0) : Qt::NoPen);
        p->setBrush(fill);
        p->drawRoundedRect(r, 12, 12);

        // Severity bar
        p->setPen(Qt::NoPen);
        p->setBrush(tone);
        p->drawRoundedRect(QRectF(r.left() + 8, r.top() + 12, 4, r.height() - 24), 2, 2);

        // Code
        p->setFont(mono(16, 700));
        p->setPen(tone);
        const QRectF codeBox(r.left() + 24, r.top() + 10, 80, 26);
        p->drawText(codeBox, Qt::AlignLeft | Qt::AlignVCenter, index.data(CodeRole).toString());

        // Severity pill on the right
        const QString label = index.data(LabelRole).toString();
        QFont pillFont = sans(11, 700);
        const double pw = QFontMetricsF(pillFont).horizontalAdvance(label) + 18;
        const QRectF pill(r.right() - pw - 10, r.top() + 13, pw, 20);
        p->setBrush(rgbaColour(tone, 0.15));
        p->drawRoundedRect(pill, 10, 10);
        p->setFont(pillFont);
        p->setPen(tone.lighter(110));
        p->drawText(pill, Qt::AlignCenter, label);

        // Text and meta
        const QRectF textBox(r.left() + 108, r.top() + 10, pill.left() - r.left() - 118, 26);
        p->setFont(sans(14, 600));
        p->setPen(color::text);
        p->drawText(textBox, Qt::AlignLeft | Qt::AlignVCenter,
                    QFontMetrics(p->font()).elidedText(index.data(TextRole).toString(),
                                                       Qt::ElideRight, int(textBox.width())));
        p->setFont(sans(12));
        p->setPen(color::textFaint);
        const QRectF metaBox(r.left() + 24, r.top() + 40, r.width() - 40, 22);
        p->drawText(metaBox, Qt::AlignLeft | Qt::AlignVCenter,
                    QFontMetrics(p->font()).elidedText(index.data(MetaRole).toString(),
                                                       Qt::ElideRight, int(metaBox.width())));
        p->restore();
    }

private:
    static QColor rgbaColour(const QColor& c, double alpha) {
        QColor out = c;
        out.setAlphaF(static_cast<float>(alpha));
        return out;
    }
};

QWidget* titled(const QString& title, QVBoxLayout** body) {
    auto* box = new QWidget;
    auto* layout = new QVBoxLayout(box);
    layout->setContentsMargins(0, 6, 0, 0);
    layout->setSpacing(6);
    auto* head = makeLabel(title.toUpper(), "Eyebrow");
    layout->addWidget(head);
    *body = new QVBoxLayout;
    (*body)->setSpacing(4);
    layout->addLayout(*body);
    return box;
}

void clearLayout(QVBoxLayout* layout) {
    while (QLayoutItem* item = layout->takeAt(0)) {
        if (item->widget()) item->widget()->deleteLater();
        delete item;
    }
}

}  // namespace

FaultsPage::FaultsPage(QWidget* parent) : QWidget(parent) {
    auto* root = new QVBoxLayout(this);
    root->setContentsMargins(32, 26, 32, 24);
    root->setSpacing(16);
    root->addWidget(makePageHeader(tr("Diagnose"), tr("Fehlerspeicher"),
                                   tr("Jeder Eintrag mit Dringlichkeit. Rechts steht, was er "
                                      "bedeutet, ob das Auto noch fahren darf und wo man "
                                      "nachsehen kann.")));

    auto* buttons = new QHBoxLayout;
    buttons->setSpacing(10);
    readButton_ = new QPushButton(tr("Neu einlesen"));
    readButton_->setObjectName("Primary");
    readButton_->setIcon(icons::icon(QStringLiteral("refresh"), Qt::white, Qt::white, 16));
    clearButton_ = new QPushButton(tr("Fehler löschen"));
    clearButton_->setObjectName("Danger");
    clearButton_->setIcon(icons::icon(QStringLiteral("trash"), color::bad, color::bad, 16));
    bruteButton_ = new QPushButton(tr("Hartnäckig löschen"));
    bruteButton_->setObjectName("Danger");
    exportButton_ = new QPushButton(tr("Als CSV speichern"));
    exportButton_->setIcon(icons::icon(QStringLiteral("download"), color::textMuted, color::text, 16));
    for (QPushButton* b : {readButton_, clearButton_, bruteButton_, exportButton_}) {
        b->setEnabled(false);
        b->setMinimumHeight(40);
        buttons->addWidget(b);
    }
    buttons->addStretch();
    summary_ = makeLabel(QString(), "Subtle");
    buttons->addWidget(summary_);
    root->addLayout(buttons);

    connect(readButton_, &QPushButton::clicked, this, &FaultsPage::readRequested);
    connect(clearButton_, &QPushButton::clicked, this, &FaultsPage::askAndClear);
    connect(bruteButton_, &QPushButton::clicked, this, &FaultsPage::askAndBruteForce);
    connect(exportButton_, &QPushButton::clicked, this, &FaultsPage::exportCsv);

    throttleHint_ = new QLabel;
    throttleHint_->setWordWrap(true);
    throttleHint_->setVisible(false);
    throttleHint_->setStyleSheet(QStringLiteral("color: %1;").arg(color::warn.name()));
    root->addWidget(throttleHint_);

    clearLog_ = new QPlainTextEdit;
    clearLog_->setReadOnly(true);
    clearLog_->setMaximumHeight(110);
    clearLog_->setFont(mono(12));
    clearLog_->setVisible(false);
    root->addWidget(clearLog_);

    auto* columns = new QHBoxLayout;
    columns->setSpacing(20);

    // --- List ---------------------------------------------------------------------
    QVBoxLayout* listLayout = nullptr;
    QFrame* listCard = makeCard(&listLayout, 12);
    list_ = new QListWidget;
    list_->setItemDelegate(new FaultDelegate(list_));
    list_->setMouseTracking(true);
    list_->setStyleSheet(QStringLiteral("QListWidget { background: transparent; border: none; }"
                                        "QListWidget::item { border: none; padding: 0; }"));
    list_->setVerticalScrollMode(QAbstractItemView::ScrollPerPixel);
    listLayout->addWidget(list_);
    columns->addWidget(listCard, 5);
    connect(list_, &QListWidget::currentRowChanged, this, &FaultsPage::showSelected);

    // --- Detail ---------------------------------------------------------------------
    auto* detailScroll = new QScrollArea;
    detailScroll->setWidgetResizable(true);
    detailScroll->setFrameShape(QFrame::NoFrame);
    QVBoxLayout* detailLayout = nullptr;
    QFrame* detailCard = makeCard(&detailLayout, 24);
    detailScroll->setWidget(detailCard);
    detail_ = detailCard;

    auto* head = new QHBoxLayout;
    head->setSpacing(12);
    code_ = new QLabel;
    code_->setFont(mono(26, 700));
    severity_ = new QLabel;
    head->addWidget(code_);
    head->addWidget(severity_, 0, Qt::AlignVCenter);
    head->addStretch();
    detailLayout->addLayout(head);
    title_ = makeLabel(QString(), "SectionTitle", true);
    meta_ = makeLabel(QString(), "Hint", true);
    detailLayout->addWidget(title_);
    detailLayout->addWidget(meta_);

    advice_ = makeLabel(QString(), nullptr, true);
    advice_->setContentsMargins(14, 10, 14, 10);
    detailLayout->addWidget(advice_);

    diagram_ = new VehicleDiagram;
    diagram_->setAnatomyVisible(true);
    diagram_->setMinimumHeight(190);
    detailLayout->addWidget(diagram_);
    where_ = makeLabel(QString(), "Subtle", true);
    detailLayout->addWidget(where_);

    causesBox_ = titled(tr("Häufige Ursachen"), &causes_);
    detailLayout->addWidget(causesBox_);
    checksBox_ = titled(tr("Selbst nachsehen"), &checks_);
    detailLayout->addWidget(checksBox_);
    frameBox_ = titled(tr("Was im Moment des Fehlers los war"), &frame_);
    detailLayout->addWidget(frameBox_);
    frameBox_->setVisible(false);
    cost_ = makeLabel(QString(), "Hint", true);
    detailLayout->addWidget(cost_);
    detailLayout->addStretch();

    QVBoxLayout* emptyLayout = nullptr;
    QFrame* emptyCard = makeCard(&emptyLayout, 32);
    empty_ = emptyCard;
    auto* emptyIcon = new QLabel;
    emptyIcon->setPixmap(icons::pixmap(QStringLiteral("check-circle"), 46, color::textFaint, 1.4));
    emptyIcon->setAlignment(Qt::AlignCenter);
    emptyText_ = makeLabel(tr("Noch nichts gelesen. Nach dem Verbinden stehen hier die "
                                   "Einträge, mit Erklärung."),
                                "Subtle", true);
    emptyText_->setAlignment(Qt::AlignCenter);
    emptyLayout->addStretch();
    emptyLayout->addWidget(emptyIcon);
    emptyLayout->addWidget(emptyText_);
    emptyLayout->addStretch();

    auto* right = new QVBoxLayout;
    right->addWidget(detailScroll, 1);
    right->addWidget(emptyCard, 1);
    columns->addLayout(right, 6);
    root->addLayout(columns, 1);

    root->addWidget(makeLabel(tr("Löschen setzt auch die Bereitschaftstests zurück. Direkt danach "
                                 "fällt eine Abgasuntersuchung durch, bis das Auto wieder ein paar "
                                 "vollständige Fahrzyklen gelaufen ist. Ein Fehler, dessen Ursache "
                                 "noch da ist, kommt nach kurzer Fahrt zurück."),
                              "Hint", true));
    // The scroll area is what shows or hides, not the card inside it.
    detail_ = detailScroll;
    detailScroll->setVisible(false);
}

void FaultsPage::onFreezeFrames(const QVector<FreezeFrameRow>& frames) {
    frames_ = frames;
    if (list_->currentRow() >= 0) showSelected(list_->currentRow());
}

void FaultsPage::setBrand(const QString& brand) {
    brand_ = brand;
    if (!rows_.isEmpty()) onFaults(rows_);
}

void FaultsPage::onFaults(const QVector<FaultRow>& rows) {
    rows_ = rows;
    list_->clear();
    bool throttle = false;
    QSet<int> zones;
    for (const FaultRow& f : rows) {
        const DtcInfo info = lookupDtc(f.code.toStdString(), brand_.toStdString());
        auto* item = new QListWidgetItem;
        item->setData(CodeRole, f.code);
        // The text the control unit's code means for this brand, else what the
        // worker sent (which also covers KKL texts).
        item->setData(TextRole, info.known ? QString::fromStdString(info.text) : f.text);
        QStringList meta;
        meta << f.kind;
        if (!f.ecu.isEmpty()) meta << f.ecu;
        if (!info.systemName.empty()) meta << QString::fromStdString(info.systemName);
        item->setData(MetaRole, meta.join(QStringLiteral("  ·  ")));
        item->setData(SeverityRole, static_cast<int>(info.severity));
        item->setData(LabelRole, QString::fromUtf8(severityLabel(info.severity)));
        list_->addItem(item);
        throttle = throttle || f.throttleRelated || info.throttleRelated;
        const ZoneInfo where = locateDtc(f.code.toStdString());
        if (where.zone != CarZone::Unknown) zones.insert(static_cast<int>(where.zone));
    }

    summary_->setText(rows.isEmpty() ? tr("Kein Eintrag im Speicher")
                                     : tr("%n Einträge", nullptr, rows.size()));
    throttleHint_->setVisible(throttle);
    if (throttle) {
        throttleHint_->setText(
            tr("Mindestens ein Code betrifft die Drosselklappe. Löschen allein reicht da "
               "erfahrungsgemäß nicht, die Klappe muss angelernt werden."));
    }
    exportButton_->setEnabled(!rows.isEmpty());
    diagram_->setAffected(zones);
    if (!rows.isEmpty()) {
        list_->setCurrentRow(0);
        showSelected(0);
    } else {
        clearDetail();
        emptyText_->setText(tr("Kein Fehler gespeichert. Gut so."));
    }
}

void FaultsPage::clearDetail() {
    detail_->setVisible(false);
    empty_->setVisible(true);
    diagram_->clear();
}

void FaultsPage::showSelected(int row) {
    if (row < 0 || row >= rows_.size()) return;
    const FaultRow& fault = rows_.at(row);
    const DtcInfo info = lookupDtc(fault.code.toStdString(), brand_.toStdString());
    const ZoneInfo where = locateDtc(fault.code.toStdString());
    detail_->setVisible(true);
    empty_->setVisible(false);

    const QColor tone = severityColour(static_cast<int>(info.severity));
    code_->setText(fault.code);
    code_->setStyleSheet(QStringLiteral("color: %1; font-family: \"%2\"; font-size: 26px; "
                                        "font-weight: 700;")
                             .arg(tone.name(), monoFamily()));
    severity_->setText(QString::fromUtf8(severityLabel(info.severity)));
    severity_->setStyleSheet(QStringLiteral("color: %1; background: %2; border-radius: 11px; "
                                            "padding: 3px 12px; font-weight: 700; font-size: 12px;")
                                 .arg(tone.lighter(110).name(), rgba(tone, 0.16)));
    title_->setText(info.known ? QString::fromStdString(info.text) : fault.text);
    QStringList meta;
    meta << fault.kind;
    if (!fault.ecu.isEmpty()) meta << fault.ecu;
    if (!info.systemName.empty()) meta << QString::fromStdString(info.systemName);
    if (info.manufacturerSpecific && !info.familyLabel.empty()) {
        meta << tr("Herstellercode %1").arg(QString::fromStdString(info.familyLabel));
    }
    if (!info.known) meta << tr("genaue Bedeutung unbekannt, das ist die Gruppe");
    meta_->setText(meta.join(QStringLiteral("  ·  ")));

    const QString advice = info.advice.empty() ? QString::fromUtf8(severityAdvice(info.severity))
                                               : QString::fromStdString(info.advice);
    advice_->setText(QStringLiteral("<b>%1</b><br/>%2")
                         .arg(tr("Darf ich weiterfahren?").toHtmlEscaped(), advice.toHtmlEscaped()));
    advice_->setStyleSheet(QStringLiteral("background: %1; border: 1px solid %2; border-radius: 12px;")
                               .arg(rgba(tone, 0.08), rgba(tone, 0.35)));

    diagram_->setFocus(where.zone);
    where_->setText(where.zone == CarZone::Unknown
                        ? QString()
                        : tr("Wo nachsehen: %1. %2")
                              .arg(QString::fromStdString(where.name),
                                   QString::fromStdString(where.hint)));

    clearLayout(causes_);
    int n = 1;
    for (const std::string& c : info.causes) {
        auto* label = makeLabel(QStringLiteral("%1.  %2").arg(n++).arg(QString::fromStdString(c)),
                                nullptr, true);
        causes_->addWidget(label);
    }
    causesBox_->setVisible(!info.causes.empty());

    clearLayout(checks_);
    for (const std::string& c : info.checks) {
        auto* row = new QWidget;
        auto* h = new QHBoxLayout(row);
        h->setContentsMargins(0, 0, 0, 0);
        h->setSpacing(8);
        auto* icon = new QLabel;
        icon->setPixmap(icons::pixmap(QStringLiteral("check"), 14, color::accent));
        h->addWidget(icon, 0, Qt::AlignTop);
        h->addWidget(makeLabel(QString::fromStdString(c), nullptr, true), 1);
        checks_->addWidget(row);
    }
    checksBox_->setVisible(!info.checks.empty());

    // The freeze frame the control unit stored together with this code.
    clearLayout(frame_);
    const FreezeFrameRow* match = nullptr;
    for (const FreezeFrameRow& f : std::as_const(frames_)) {
        if (f.dtc == fault.code) {
            match = &f;
            break;
        }
    }
    if (match) {
        for (const LiveRow& v : match->values) {
            if (v.pid == 0x02) continue;   // the code itself
            const QString value = v.text.isEmpty()
                                      ? QStringLiteral("%1 %2").arg(QLocale().toString(v.value, 'f', 1), v.unit)
                                      : v.text;
            auto* row = new QWidget;
            auto* h = new QHBoxLayout(row);
            h->setContentsMargins(0, 0, 0, 0);
            h->addWidget(makeLabel(v.name, "Subtle"), 1);
            auto* number = makeLabel(value);
            number->setStyleSheet(QStringLiteral("font-weight: 600;"));
            h->addWidget(number);
            frame_->addWidget(row);
        }
    }
    frameBox_->setVisible(match != nullptr && !match->values.isEmpty());

    cost_->setVisible(info.costClass != CostClass::Unknown);
    if (info.costClass != CostClass::Unknown) {
        cost_->setText(tr("Kosten in der Werkstatt: %1. %2")
                           .arg(QString::fromUtf8(costClassLabel(info.costClass)),
                                QString::fromUtf8(costClassHint(info.costClass))));
    }
}

void FaultsPage::setConnected(bool connected) {
    readButton_->setEnabled(connected);
    clearButton_->setEnabled(connected);
    bruteButton_->setEnabled(connected);
    if (!connected) {
        list_->clear();
        rows_.clear();
        frames_.clear();
        summary_->clear();
        throttleHint_->setVisible(false);
        exportButton_->setEnabled(false);
        clearDetail();
        emptyText_->setText(tr("Noch nichts gelesen. Nach dem Verbinden stehen hier die "
                               "Einträge, mit Erklärung."));
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
