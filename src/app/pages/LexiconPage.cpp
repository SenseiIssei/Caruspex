#include "app/pages/LexiconPage.h"

#include <QHBoxLayout>
#include <QHeaderView>
#include <QLabel>
#include <QLineEdit>
#include <QTableWidget>
#include <QVBoxLayout>

#include <algorithm>
#include <vector>

#include "app/Theme.h"
#include "data/DtcDatabase.h"

namespace caruspex {
namespace {

struct Entry {
    QString code;
    QString text;
    bool throttle;
};

const std::vector<Entry>& allEntries() {
    static const std::vector<Entry> entries = [] {
        std::vector<Entry> list;
        list.reserve(dtcDescriptions().size());
        // Only the codes come from the table. The text is fetched through
        // describeDtc(), which returns it in the active language.
        for (const auto& entry : dtcDescriptions()) {
            const std::string& code = entry.first;
            list.push_back({QString::fromStdString(code),
                            QString::fromStdString(describeDtc(code)), isThrottleDtc(code)});
        }
        std::sort(list.begin(), list.end(),
                  [](const Entry& a, const Entry& b) { return a.code < b.code; });
        return list;
    }();
    return entries;
}

}  // namespace

LexiconPage::LexiconPage(QWidget* parent) : QWidget(parent) {
    auto* root = new QVBoxLayout(this);
    root->setContentsMargins(32, 26, 32, 24);
    root->setSpacing(14);

    auto* title = new QLabel(tr("Fehlercode-Lexikon"));
    title->setObjectName("PageTitle");
    root->addWidget(title);

    auto* hint = new QLabel(
        tr("Nachschlagewerk für unterwegs, funktioniert auch ohne Auto. Suche nach Code "
           "oder nach Stichwort, zum Beispiel Drosselklappe oder Lambdasonde."));
    hint->setObjectName("Hint");
    hint->setWordWrap(true);
    root->addWidget(hint);

    auto* row = new QHBoxLayout;
    search_ = new QLineEdit;
    search_->setPlaceholderText(tr("P0301, Lambdasonde, Zylinder 3 ..."));
    search_->setClearButtonEnabled(true);
    count_ = new QLabel;
    count_->setObjectName("Subtle");
    row->addWidget(search_, 1);
    row->addWidget(count_);
    root->addLayout(row);

    table_ = new QTableWidget(0, 2);
    table_->setHorizontalHeaderLabels({tr("Code"), tr("Bedeutung")});
    table_->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Fixed);
    table_->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Stretch);
    table_->setColumnWidth(0, 110);
    table_->verticalHeader()->setVisible(false);
    table_->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table_->setAlternatingRowColors(true);
    table_->setSelectionBehavior(QAbstractItemView::SelectRows);
    root->addWidget(table_, 1);

    connect(search_, &QLineEdit::textChanged, this, &LexiconPage::applyFilter);
    applyFilter(QString());
}

void LexiconPage::applyFilter(const QString& needle) {
    const QString query = needle.trimmed();
    std::vector<const Entry*> hits;
    for (const Entry& e : allEntries()) {
        if (query.isEmpty() || e.code.contains(query, Qt::CaseInsensitive) ||
            e.text.contains(query, Qt::CaseInsensitive)) {
            hits.push_back(&e);
        }
    }

    table_->setRowCount(static_cast<int>(hits.size()));
    for (int i = 0; i < static_cast<int>(hits.size()); ++i) {
        const Entry* e = hits[i];
        auto* code = new QTableWidgetItem(e->code);
        QFont bold = code->font();
        bold.setBold(true);
        code->setFont(bold);
        if (e->throttle) code->setForeground(theme::color::warn);
        table_->setItem(i, 0, code);
        table_->setItem(i, 1, new QTableWidgetItem(e->text));
    }
    count_->setText(tr("%1 von %2 Codes")
                        .arg(hits.size())
                        .arg(allEntries().size()));
}

}  // namespace caruspex
