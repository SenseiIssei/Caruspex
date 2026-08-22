#include "app/pages/LivePage.h"

#include <QCheckBox>
#include <QChart>
#include <QChartView>
#include <QDateTime>
#include <QFileDialog>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QLabel>
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
#include "data/Pids.h"

namespace caruspex {
namespace {

constexpr double kWindowSeconds = 90.0;

const QColor kSeriesColours[] = {
    QColor("#4aa3ff"), QColor("#3ac47d"), QColor("#f2b13c"), QColor("#ef5350"),
    QColor("#b47cff"), QColor("#2fd1c5"), QColor("#ff8a5c"), QColor("#9ccc65"),
};

}  // namespace

LivePage::LivePage(QWidget* parent) : QWidget(parent) {
    auto* root = new QVBoxLayout(this);
    root->setContentsMargins(24, 20, 24, 20);
    root->setSpacing(14);

    auto* title = new QLabel(tr("Live-Werte"));
    title->setObjectName("PageTitle");
    root->addWidget(title);

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
    hint_ = new QLabel(tr("Die Werte laufen einzeln ueber den Bus, daher aktualisiert es "
                          "auf der K-Line gemaechlich."));
    hint_->setObjectName("Hint");
    bar->addWidget(hint_);
    root->addLayout(bar);

    connect(runButton_, &QPushButton::clicked, this, &LivePage::toggleRun);
    connect(exportButton_, &QPushButton::clicked, this, &LivePage::exportCsv);

    // Diagramm
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
    axisX_->setRange(0, kWindowSeconds);
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

    // Werteliste und Auswahl
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
    pidList_->setMaximumWidth(300);
    buildPidList();

    auto* bottom = new QSplitter(Qt::Horizontal);
    bottom->addWidget(pidList_);
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
    for (const PidSpec& spec : pidTable()) {
        auto* item = new QListWidgetItem(QString::fromUtf8(spec.name));
        item->setFlags(item->flags() | Qt::ItemIsUserCheckable);
        item->setData(Qt::UserRole, spec.pid);
        const bool standard = defaultLivePids().end() !=
                              std::find(defaultLivePids().begin(), defaultLivePids().end(),
                                        spec.pid);
        item->setCheckState(standard ? Qt::Checked : Qt::Unchecked);
        pidList_->addItem(item);
    }
}

void LivePage::onSupportedPids(const QVector<int>& pids) {
    if (pids.isEmpty()) return;
    // Was das Fahrzeug nicht kann, wird ausgegraut und abgewaehlt.
    for (int i = 0; i < pidList_->count(); ++i) {
        QListWidgetItem* item = pidList_->item(i);
        const int pid = item->data(Qt::UserRole).toInt();
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
        if (item->checkState() == Qt::Checked) pids.push_back(item->data(Qt::UserRole).toInt());
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
        QMessageBox::information(this, tr("Nichts ausgewaehlt"),
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
    axisX_->setRange(0, kWindowSeconds);
    axisY_->setRange(0, 100);
    yLow_ = std::numeric_limits<double>::max();
    yHigh_ = std::numeric_limits<double>::lowest();
    rescaleClock_.invalidate();
}

void LivePage::onLiveValue(const LiveRow& row) {
    // Tabelle
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

    if (!row.text.isEmpty()) return;   // Statuswerte gehoeren nicht ins Diagramm

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

    // Punkte ausserhalb des Zeitfensters wegwerfen
    while (line->count() > 2 && line->at(0).x() < seconds - kWindowSeconds) line->remove(0);

    // Achsen mitziehen
    if (seconds > kWindowSeconds) axisX_->setRange(seconds - kWindowSeconds, seconds);

    // Laufend mitgefuehrte Grenzen. Ueber alle Punkte aller Kurven zu laufen
    // waere quadratisch und legt die Oberflaeche bei schnellen Adaptern lahm.
    yLow_ = qMin(yLow_, row.value);
    yHigh_ = qMax(yHigh_, row.value);

    // Alle zwei Sekunden einmal sauber nachrechnen, damit die Achse wieder
    // enger wird, wenn alte Ausreisser aus dem Zeitfenster gelaufen sind.
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
        QStringLiteral("messung_%1.csv")
            .arg(QDateTime::currentDateTime().toString("yyyy-MM-dd_HH-mm"));
    const QString path =
        QFileDialog::getSaveFileName(this, tr("Aufzeichnung speichern"), suggestion,
                                     tr("CSV (*.csv)"));
    if (path.isEmpty()) return;

    QFile file(path);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text)) {
        QMessageBox::warning(this, tr("Fehler"), tr("Datei laesst sich nicht schreiben."));
        return;
    }
    QTextStream out(&file);
    out.setEncoding(QStringConverter::Utf8);
    out << "Sekunde;PID;Messwert;Wert;Einheit\n";
    for (const Sample& s : std::as_const(recorded_)) {
        out << QString::number(s.seconds, 'f', 2) << ';'
            << QStringLiteral("0x%1").arg(s.pid, 2, 16, QChar('0')).toUpper() << ';' << s.name
            << ';' << QString::number(s.value, 'f', 3) << ';' << s.unit << '\n';
    }
}

}  // namespace caruspex
