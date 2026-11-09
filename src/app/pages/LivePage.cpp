#include "app/pages/LivePage.h"

#include <QCheckBox>
#include <QChart>
#include <QChartView>
#include <QDateTime>
#include <QFileDialog>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QLabel>
#include <QLineEdit>
#include <QLineSeries>
#include <QListWidget>
#include <QMessageBox>
#include <QPushButton>
#include <QSplitter>
#include <QTableWidget>
#include <QTextStream>
#include <QVBoxLayout>
#include <QValueAxis>

#include <limits>

#include "app/Theme.h"
#include "app/Widgets.h"
#include "data/Pids.h"

namespace caruspex {
namespace {


const QColor kSeriesColours[] = {
    QColor("#4aa3ff"), QColor("#3ac47d"), QColor("#f2b13c"), QColor("#ef5350"),
    QColor("#b47cff"), QColor("#2fd1c5"), QColor("#ff8a5c"), QColor("#9ccc65"),
};

}  // namespace

LivePage::LivePage(QWidget* parent) : QWidget(parent) {
    auto* root = new QVBoxLayout(this);
    root->setContentsMargins(32, 26, 32, 24);
    root->setSpacing(14);

    root->addWidget(makePageHeader(tr("Messen"), tr("Live-Werte"),
                                   tr("Alles, was das Steuergerät laufend misst, nach Bereichen "
                                      "sortiert. Links ankreuzen, oben starten.")));

    auto* bar = new QHBoxLayout;
    bar->setSpacing(10);
    runButton_ = new QPushButton(tr("Messung starten"));
    runButton_->setObjectName("Primary");
    runButton_->setMinimumHeight(40);
    runButton_->setEnabled(false);
    exportButton_ = new QPushButton(tr("Aufzeichnung speichern"));
    exportButton_->setMinimumHeight(40);
    exportButton_->setEnabled(false);
    recordBox_ = new QCheckBox(tr("mitschreiben"));
    recordBox_->setChecked(true);
    bar->addWidget(runButton_);
    bar->addWidget(exportButton_);
    bar->addWidget(recordBox_);
    bar->addStretch();
    hint_ = new QLabel(tr("Die Werte laufen einzeln über den Bus, daher aktualisiert es "
                          "auf der K-Line gemächlich."));
    hint_->setObjectName("Hint");
    bar->addWidget(hint_);
    root->addLayout(bar);

    connect(runButton_, &QPushButton::clicked, this, &LivePage::toggleRun);
    connect(exportButton_, &QPushButton::clicked, this, &LivePage::exportCsv);

    // Chart
    chart_ = new QChart;
    chart_->setBackgroundBrush(QBrush(theme::color::surface));
    chart_->setPlotAreaBackgroundBrush(QBrush(theme::color::surface));
    chart_->setPlotAreaBackgroundVisible(true);
    chart_->legend()->setLabelColor(theme::color::text);
    chart_->legend()->setAlignment(Qt::AlignBottom);
    chart_->setMargins(QMargins(6, 6, 6, 6));
    chart_->setAnimationOptions(QChart::SeriesAnimations);

    axisX_ = new QValueAxis;
    axisX_->setTitleText(tr("Sekunden"));
    axisX_->setRange(0, windowSeconds_);
    axisY_ = new QValueAxis;
    axisY_->setRange(0, 100);
    for (QValueAxis* axis : {axisX_, axisY_}) {
        axis->setLabelsColor(theme::color::textMuted);
        axis->setTitleBrush(QBrush(theme::color::textMuted));
        axis->setGridLineColor(theme::color::border);
        axis->setLinePenColor(theme::color::border);
    }
    chart_->addAxis(axisX_, Qt::AlignBottom);
    chart_->addAxis(axisY_, Qt::AlignLeft);

    chartView_ = new QChartView(chart_);
    chartView_->setRenderHint(QPainter::Antialiasing);
    chartView_->setMinimumHeight(240);

    // Value list and selection
    table_ = new QTableWidget(0, 3);
    table_->setHorizontalHeaderLabels({tr("Messwert"), tr("Wert"), tr("Einheit")});
    table_->horizontalHeader()->setSectionResizeMode(0, QHeaderView::Stretch);
    table_->horizontalHeader()->setSectionResizeMode(1, QHeaderView::Fixed);
    table_->horizontalHeader()->setSectionResizeMode(2, QHeaderView::Fixed);
    table_->setColumnWidth(1, 130);
    table_->setColumnWidth(2, 90);
    table_->verticalHeader()->setVisible(false);
    table_->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table_->setAlternatingRowColors(true);

    pidList_ = new QListWidget;
    buildPidList();
    search_ = new QLineEdit;
    search_->setPlaceholderText(tr("Messwert suchen, z.B. Öl oder Lambda"));
    search_->setClearButtonEnabled(true);
    connect(search_, &QLineEdit::textChanged, this, [this](const QString& text) {
        const QString needle = text.trimmed();
        for (int i = 0; i < pidList_->count(); ++i) {
            QListWidgetItem* item = pidList_->item(i);
            const bool header = item->data(Qt::UserRole).toInt() < 0;
            item->setHidden(!needle.isEmpty() &&
                            (header || !item->text().contains(needle, Qt::CaseInsensitive)));
        }
    });
    auto* listBox = new QWidget;
    listBox->setMaximumWidth(340);
    auto* listLayout = new QVBoxLayout(listBox);
    listLayout->setContentsMargins(0, 0, 0, 0);
    listLayout->setSpacing(8);
    listLayout->addWidget(search_);
    listLayout->addWidget(pidList_, 1);

    auto* bottom = new QSplitter(Qt::Horizontal);
    bottom->addWidget(listBox);
    bottom->addWidget(table_);
    bottom->setStretchFactor(1, 1);

    auto* split = new QSplitter(Qt::Vertical);
    split->addWidget(chartView_);
    split->addWidget(bottom);
    split->setStretchFactor(0, 1);
    split->setStretchFactor(1, 1);
    root->addWidget(split, 1);
}

void LivePage::buildPidList() {
    pidList_->clear();
    for (PidCategory category : pidCategories()) {
      bool any = false;
      for (const PidSpec& spec : pidTable()) {
        if (spec.category != category || spec.pid == 0x00) continue;
        if (!any) {
            // Section heading, not selectable.
            auto* head = new QListWidgetItem(QString::fromUtf8(pidCategoryName(category)).toUpper());
            head->setFlags(Qt::NoItemFlags);
            head->setData(Qt::UserRole, -1);
            head->setForeground(theme::color::accent);
            QFont f = theme::sans(11, 700);
            f.setLetterSpacing(QFont::AbsoluteSpacing, 1.2);
            head->setFont(f);
            pidList_->addItem(head);
            any = true;
        }
        const char* label = (spec.title && *spec.title) ? spec.title : spec.name;
        auto* item = new QListWidgetItem(QString::fromUtf8(label));
        if (spec.values.size() > 1) {
            item->setToolTip(tr("%n Werte", nullptr, static_cast<int>(spec.values.size())));
        }
        item->setFlags(item->flags() | Qt::ItemIsUserCheckable);
        item->setData(Qt::UserRole, spec.pid);
        const bool standard = defaultLivePids().end() !=
                              std::find(defaultLivePids().begin(), defaultLivePids().end(),
                                        spec.pid);
        item->setCheckState(standard ? Qt::Checked : Qt::Unchecked);
        pidList_->addItem(item);
      }
    }
}

void LivePage::onSupportedPids(const QVector<int>& pids) {
    if (pids.isEmpty()) return;
    // Whatever the vehicle does not support is greyed out and deselected.
    for (int i = 0; i < pidList_->count(); ++i) {
        QListWidgetItem* item = pidList_->item(i);
        const int pid = item->data(Qt::UserRole).toInt();
        if (pid < 0) continue;
        const bool supported = pids.contains(pid);
        item->setForeground(supported ? theme::color::text : theme::color::textMuted);
        if (!supported) {
            item->setCheckState(Qt::Unchecked);
            item->setFlags(item->flags() & ~Qt::ItemIsEnabled);
            item->setToolTip(tr("Dieses Fahrzeug liefert den Wert nicht."));
        }
    }
}

QVector<int> LivePage::selectedPids() const {
    QVector<int> pids;
    for (int i = 0; i < pidList_->count(); ++i) {
        const QListWidgetItem* item = pidList_->item(i);
        if (item->data(Qt::UserRole).toInt() >= 0 && item->checkState() == Qt::Checked) {
            pids.push_back(item->data(Qt::UserRole).toInt());
        }
    }
    return pids;
}

void LivePage::setConnected(bool connected) {
    connected_ = connected;
    runButton_->setEnabled(connected);
    if (!connected) {
        setRunning(false);
        table_->setRowCount(0);
        tableRow_.clear();
    }
}

void LivePage::setRunning(bool running) {
    running_ = running;
    runButton_->setText(running ? tr("Messung stoppen") : tr("Messung starten"));
    runButton_->setObjectName(running ? "Danger" : "Primary");
    runButton_->style()->unpolish(runButton_);
    runButton_->style()->polish(runButton_);
}

void LivePage::toggleFromOutside() {
    if (connected_) toggleRun();
}

void LivePage::toggleRun() {
    if (running_) {
        emit stopRequested();
        setRunning(false);
        return;
    }
    const QVector<int> pids = selectedPids();
    if (pids.isEmpty()) {
        QMessageBox::information(this, tr("Nichts ausgewählt"),
                                 tr("Links mindestens einen Messwert ankreuzen."));
        return;
    }
    resetChart();
    clock_.restart();
    recorded_.clear();
    emit startRequested(pids);
    setRunning(true);
}

void LivePage::resetChart() {
    for (QLineSeries* s : std::as_const(series_)) {
        chart_->removeSeries(s);
        delete s;
    }
    series_.clear();
    axisX_->setRange(0, windowSeconds_);
    axisY_->setRange(0, 100);
    yLow_ = std::numeric_limits<double>::max();
    yHigh_ = std::numeric_limits<double>::lowest();
    rescaleClock_.invalidate();
}

void LivePage::setWindowSeconds(int seconds) {
    if (seconds < 5) seconds = 5;
    windowSeconds_ = static_cast<double>(seconds);
    // Adjust the axis right away, otherwise the change only takes effect with
    // the next value.
    const double now = clock_.isValid() ? clock_.elapsed() / 1000.0 : 0.0;
    if (now > windowSeconds_) {
        axisX_->setRange(now - windowSeconds_, now);
    } else {
        axisX_->setRange(0, windowSeconds_);
    }
}

void LivePage::onLiveValue(const LiveRow& row) {
    // Table
    if (!tableRow_.contains(row.pid)) {
        const int r = table_->rowCount();
        table_->insertRow(r);
        table_->setItem(r, 0, new QTableWidgetItem(row.name));
        table_->setItem(r, 1, new QTableWidgetItem());
        table_->setItem(r, 2, new QTableWidgetItem(row.unit));
        tableRow_.insert(row.pid, r);
    }
    const int r = tableRow_.value(row.pid);
    QTableWidgetItem* cell = table_->item(r, 1);
    cell->setText(row.text.isEmpty() ? QString::number(row.value, 'f', 2) : row.text);
    cell->setForeground(row.warn ? theme::color::warn : theme::color::text);

    if (!row.text.isEmpty()) return;   // status values do not belong in the chart

    const double seconds = clock_.isValid() ? clock_.elapsed() / 1000.0 : 0.0;

    if (recordBox_->isChecked()) {
        recorded_.push_back({seconds, row.pid, row.name, row.value, row.unit});
        exportButton_->setEnabled(true);
    }

    QLineSeries* line = series_.value(row.pid, nullptr);
    if (!line) {
        line = new QLineSeries;
        line->setName(QStringLiteral("%1 [%2]").arg(row.name, row.unit));
        QPen pen(kSeriesColours[series_.size() % 8]);
        pen.setWidthF(2.2);
        line->setPen(pen);
        chart_->addSeries(line);
        line->attachAxis(axisX_);
        line->attachAxis(axisY_);
        series_.insert(row.pid, line);
    }
    line->append(seconds, row.value);

    // Drop points outside the time window
    while (line->count() > 2 && line->at(0).x() < seconds - windowSeconds_) line->remove(0);

    // Move the axes along
    if (seconds > windowSeconds_) axisX_->setRange(seconds - windowSeconds_, seconds);

    // Running bounds. Iterating over all points of all curves would be
    // quadratic and would bog down the UI with fast adapters.
    yLow_ = qMin(yLow_, row.value);
    yHigh_ = qMax(yHigh_, row.value);

    // Recompute properly every two seconds so the axis narrows again once
    // old outliers have left the time window.
    if (!rescaleClock_.isValid() || rescaleClock_.elapsed() > 2000) {
        rescaleClock_.restart();
        double lo = std::numeric_limits<double>::max();
        double hi = std::numeric_limits<double>::lowest();
        for (QLineSeries* line : std::as_const(series_)) {
            for (const QPointF& point : line->points()) {
                lo = qMin(lo, point.y());
                hi = qMax(hi, point.y());
            }
        }
        if (lo <= hi) {
            yLow_ = lo;
            yHigh_ = hi;
        }
    }

    if (yLow_ <= yHigh_) {
        const double pad = qMax(1.0, (yHigh_ - yLow_) * 0.12);
        axisY_->setRange(yLow_ - pad, yHigh_ + pad);
    }
}

void LivePage::exportCsv() {
    if (recorded_.isEmpty()) return;
    const QString suggestion =
        tr("messung_%1.csv")
            .arg(QDateTime::currentDateTime().toString("yyyy-MM-dd_HH-mm"));
    const QString path =
        QFileDialog::getSaveFileName(this, tr("Aufzeichnung speichern"), suggestion,
                                     tr("CSV (*.csv)"));
    if (path.isEmpty()) return;

    QFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text)) {
        QMessageBox::warning(this, tr("Fehler"), tr("Datei lässt sich nicht schreiben."));
        return;
    }
    QTextStream out(&file);
    out.setEncoding(QStringConverter::Utf8);
    out << tr("Sekunde;PID;Messwert;Wert;Einheit") << '\n';
    for (const Sample& s : std::as_const(recorded_)) {
        out << QString::number(s.seconds, 'f', 2) << ';'
            << QStringLiteral("0x%1").arg(s.pid, 2, 16, QChar('0')).toUpper() << ';' << s.name
            << ';' << QString::number(s.value, 'f', 3) << ';' << s.unit << '\n';
    }
}

}  // namespace caruspex
