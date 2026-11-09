#include "app/pages/LexiconPage.h"

#include <QComboBox>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QLabel>
#include <QLineEdit>
#include <QTableWidget>
#include <QVBoxLayout>

#include <vector>

#include "app/Theme.h"
#include "app/Widgets.h"
#include "app/pages/FaultsPage.h"
#include "data/DtcDatabase.h"

namespace caruspex {
namespace {

struct Entry {
    QString code;
    QString family;
    QString familyLabel;
    QString text;
    int severity;
    QString system;
};

// Built once in the language that is active at startup.
const std::vector<Entry>& allEntries() {
    static const std::vector<Entry> entries = [] {
        std::vector<Entry> list;
        for (const DtcLexiconEntry& e : dtcLexicon()) {
            list.push_back({QString::fromStdString(e.code), QString::fromStdString(e.family),
                            QString::fromStdString(e.familyLabel), QString::fromStdString(e.text),
                            static_cast<int>(e.severity),
                            QString::fromUtf8(dtcSystemName(e.system))});
        }
        return list;
    }();
    return entries;
}

}  // namespace

LexiconPage::LexiconPage(QWidget* parent) : QWidget(parent) {
    auto* root = new QVBoxLayout(this);
    root->setContentsMargins(32, 26, 32, 24);
    root->setSpacing(14);
    root->addWidget(makePageHeader(
        tr("Wissen"), tr("Fehlercode-Lexikon"),
        tr("Nachschlagewerk für unterwegs, funktioniert auch ohne Auto. Suche nach Code oder "
           "Stichwort, zum Beispiel Drosselklappe oder Lambdasonde. Herstellercodes bedeuten je "
           "Marke etwas anderes, deshalb lässt sich nach Marke filtern.")));

    auto* row = new QHBoxLayout;
    row->setSpacing(10);
    search_ = new QLineEdit;
    search_->setPlaceholderText(tr("P0301, Lambdasonde, Zylinder 3 ..."));
    search_->setClearButtonEnabled(true);
    search_->setMinimumHeight(40);
    family_ = new QComboBox;
    family_->setMinimumHeight(40);
    family_->addItem(tr("Alle Marken"), QStringLiteral("*"));
    family_->addItem(QString::fromStdString(brandFamilyLabel("")), QString());
    for (const std::string& f : brandFamilies()) {
        if (dtcCount(f) == 0) continue;
        family_->addItem(QString::fromStdString(brandFamilyLabel(f)), QString::fromStdString(f));
    }
    count_ = new QLabel;
    count_->setObjectName("Subtle");
    row->addWidget(search_, 1);
    row->addWidget(family_);
    row->addWidget(count_);
    root->addLayout(row);

    table_ = new QTableWidget(0, 5);
    table_->setHorizontalHeaderLabels(
        {tr("Code"), tr("Bedeutung"), tr("Bereich"), tr("Dringlichkeit"), tr("Gilt für")});
    table_->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
    table_->setColumnWidth(0, 96);
    table_->setColumnWidth(2, 190);
    table_->setColumnWidth(3, 110);
    table_->setColumnWidth(4, 170);
    table_->verticalHeader()->setVisible(false);
    table_->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table_->setAlternatingRowColors(true);
    table_->setSelectionBehavior(QAbstractItemView::SelectRows);
    table_->setWordWrap(false);
    root->addWidget(table_, 1);

    connect(search_, &QLineEdit::textChanged, this, &LexiconPage::applyFilter);
    connect(family_, &QComboBox::currentIndexChanged, this,
            [this] { applyFilter(search_->text()); });
    applyFilter(QString());
}

void LexiconPage::applyFilter(const QString& needle) {
    const QString query = needle.trimmed();
    const QString family = family_->currentData().toString();
    std::vector<const Entry*> hits;
    for (const Entry& e : allEntries()) {
        if (family != QStringLiteral("*") && e.family != family) continue;
        if (query.isEmpty() || e.code.contains(query, Qt::CaseInsensitive) ||
            e.text.contains(query, Qt::CaseInsensitive) ||
            e.system.contains(query, Qt::CaseInsensitive)) {
            hits.push_back(&e);
        }
    }

    // Filling thousands of rows at once would stall the window; the first
    // few thousand are plenty for a search result.
    const int shown = static_cast<int>(std::min<size_t>(hits.size(), 3000));
    table_->setUpdatesEnabled(false);
    table_->setRowCount(shown);
    const QFont codeFont = theme::mono(13, 700);
    for (int i = 0; i < shown; ++i) {
        const Entry* e = hits[i];
        const QColor tone = severityColour(e->severity);
        auto* code = new QTableWidgetItem(e->code);
        code->setFont(codeFont);
        code->setForeground(tone);
        table_->setItem(i, 0, code);
        table_->setItem(i, 1, new QTableWidgetItem(e->text));
        auto* system = new QTableWidgetItem(e->system);
        system->setForeground(theme::color::textMuted);
        table_->setItem(i, 2, system);
        auto* severity = new QTableWidgetItem(
            QString::fromUtf8(severityLabel(static_cast<Severity>(e->severity))));
        severity->setForeground(tone);
        table_->setItem(i, 3, severity);
        auto* scope = new QTableWidgetItem(e->familyLabel);
        scope->setForeground(theme::color::textFaint);
        table_->setItem(i, 4, scope);
    }
    table_->setUpdatesEnabled(true);
    count_->setText(hits.size() > static_cast<size_t>(shown)
                        ? tr("%1 Treffer, die ersten %2 gezeigt").arg(hits.size()).arg(shown)
                        : tr("%1 von %2 Codes").arg(hits.size()).arg(allEntries().size()));
}

}  // namespace caruspex
