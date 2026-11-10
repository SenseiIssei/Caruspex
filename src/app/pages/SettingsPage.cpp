#include "app/pages/SettingsPage.h"

#include "core/Lang.h"

#include <QCheckBox>
#include <QComboBox>
#include <QDesktopServices>
#include <QFileDialog>
#include <QFormLayout>
#include <QGroupBox>
#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QPushButton>
#include <QScrollArea>
#include <QSpinBox>
#include <QStandardPaths>
#include <QUrl>
#include <QVBoxLayout>

#include "app/Theme.h"
#include "app/Widgets.h"

namespace caruspex {
namespace {

QGroupBox* makeGroup(const QString& title, QFormLayout*& form) {
    auto* box = new QGroupBox(title);
    form = new QFormLayout(box);
    form->setContentsMargins(18, 18, 18, 18);
    form->setSpacing(12);
    form->setLabelAlignment(Qt::AlignLeft | Qt::AlignVCenter);
    form->setFieldGrowthPolicy(QFormLayout::ExpandingFieldsGrow);
    return box;
}

QLabel* makeNote(const QString& text) {
    auto* note = new QLabel(text);
    note->setObjectName("Hint");
    note->setWordWrap(true);
    return note;
}

}  // namespace

SettingsPage::SettingsPage(QWidget* parent) : QWidget(parent) {
    auto* outer = new QVBoxLayout(this);
    outer->setContentsMargins(32, 26, 32, 24);
    outer->setSpacing(14);

    outer->addWidget(makeLabel(tr("PROGRAMM"), "Eyebrow"));

    auto* title = new QLabel(tr("Einstellungen"));

    title->setObjectName("PageTitle");

    outer->addWidget(title);


    outer->addWidget(makeNote(
        tr("Alles wird sofort übernommen und beim nächsten Start wieder so geladen. "
           "Änderungen wirken auch während einer laufenden Messung.")));

    auto* scroll = new QScrollArea;
    scroll->setWidgetResizable(true);
    scroll->setFrameShape(QFrame::NoFrame);
    auto* content = new QWidget;
    auto* root = new QVBoxLayout(content);
    root->setContentsMargins(0, 0, 12, 0);
    root->setSpacing(16);
    scroll->setWidget(content);
    outer->addWidget(scroll, 1);

    // --- Connection ---------------------------------------------------------
    QFormLayout* form = nullptr;
    auto* connectionBox = makeGroup(tr("Verbindung"), form);

    baud_ = new QComboBox;
    baud_->addItem(tr("Automatisch durchprobieren"), 0);
    for (int b : {9600, 19200, 38400, 57600, 115200, 230400, 500000}) {
        baud_->addItem(tr("%1 Baud").arg(b), b);
    }
    form->addRow(tr("Baudrate des Adapters"), baud_);

    protocolOrder_ = new QComboBox;
    protocolOrder_->addItem(tr("K-Line zuerst (A2 und ältere)"),
                            QStringLiteral("kline"));
    protocolOrder_->addItem(tr("CAN zuerst (A4 B7 und neuere)"),
                            QStringLiteral("can"));
    form->addRow(tr("Reihenfolge der Protokolle"), protocolOrder_);

    timeout_ = new QSpinBox;
    timeout_->setRange(25, 500);
    timeout_->setSingleStep(25);
    timeout_->setSuffix(tr(" %"));
    form->addRow(tr("Geduld mit dem Adapter"), timeout_);
    timing_ = new QComboBox;
    timing_->addItem(tr("Standard (ATAT1)"), 0);
    timing_->addItem(tr("Schneller bei OBDLink und anderen STN-Adaptern"), 1);
    timing_->addItem(tr("Immer schneller (ATAT2)"), 2);
    form->addRow(tr("Taktung"), timing_);
    form->addRow(makeNote(tr(
        "Streckt oder kürzt jede Wartezeit. Ein träger Klon braucht mehr, ein "
        "gutes FTDI-Kabel kommt mit weniger aus. Die Abstufung bleibt erhalten: "
        "Löschen darf länger dauern als eine einzelne Abfrage.")));

    autoConnect_ = new QCheckBox(tr("Beim Start gleich mit dem zuletzt benutzten "
                                    "Anschluss verbinden"));
    form->addRow(autoConnect_);
    root->addWidget(connectionBox);

    // --- Live values --------------------------------------------------------
    auto* liveBox = makeGroup(tr("Messwerte"), form);

    pollInterval_ = new QSpinBox;
    pollInterval_->setRange(0, 5000);
    pollInterval_->setSingleStep(10);
    pollInterval_->setSuffix(tr(" ms"));
    form->addRow(tr("Abstand zwischen zwei Abfragen"), pollInterval_);
    pollHint_ = makeNote(QString());
    form->addRow(pollHint_);

    chartWindow_ = new QSpinBox;
    chartWindow_->setRange(10, 3600);
    chartWindow_->setSingleStep(10);
    chartWindow_->setSuffix(tr(" s"));
    form->addRow(tr("Zeitfenster der Diagramme"), chartWindow_);
    root->addWidget(liveBox);

    // --- Clearing -----------------------------------------------------------
    auto* clearBox = makeGroup(tr("Fehler löschen"), form);

    clearAttempts_ = new QSpinBox;
    clearAttempts_->setRange(1, 20);
    form->addRow(tr("Versuche beim hartnäckigen Löschen"), clearAttempts_);
    form->addRow(makeNote(tr(
        "Nach jedem Versuch wird nachgelesen. Bleibt ein Fehler stehen, liegt er "
        "meist noch an: dann hilft kein weiterer Versuch, sondern nur die "
        "Reparatur.")));

    confirmClear_ = new QCheckBox(tr("Vor dem Löschen nachfragen"));
    form->addRow(confirmClear_);
    root->addWidget(clearBox);

    // --- K-Line -------------------------------------------------------------
    auto* kklBox = makeGroup(tr("K-Line und KKL-Kabel"), form);

    kklEcu_ = new QComboBox;
    kklEcu_->addItem(tr("01  Motor"), 0x01);
    kklEcu_->addItem(tr("02  Getriebe"), 0x02);
    kklEcu_->addItem(tr("03  Bremsenelektronik (ABS)"), 0x03);
    kklEcu_->addItem(tr("08  Klimaanlage"), 0x08);
    kklEcu_->addItem(tr("09  Zentralelektrik"), 0x09);
    kklEcu_->addItem(tr("15  Airbag"), 0x15);
    kklEcu_->addItem(tr("16  Lenkradelektronik"), 0x16);
    kklEcu_->addItem(tr("17  Kombiinstrument"), 0x17);
    kklEcu_->addItem(tr("25  Wegfahrsperre"), 0x25);
    kklEcu_->addItem(tr("46  Komfortsystem"), 0x46);
    kklEcu_->addItem(tr("56  Radio"), 0x56);
    form->addRow(tr("Voreingestelltes Steuergerät"), kklEcu_);

    basicSession_ = new QComboBox;
    basicSession_->addItem(tr("0x85  Grundeinstellung (üblich)"), 0x85);
    basicSession_->addItem(tr("0x86  Anpassung"), 0x86);
    basicSession_->addItem(tr("0x87  Erweiterte Diagnose"), 0x87);
    basicSession_->addItem(tr("0x89  Werkseinstellung"), 0x89);
    form->addRow(tr("Sitzungsart bei KWP2000"), basicSession_);
    form->addRow(makeNote(tr(
        "Welche Sitzungsart der 1.6 FSI für die Grundeinstellung verlangt, ist "
        "nicht öffentlich dokumentiert. Deshalb steht sie hier zur Wahl. "
        "Lehnt das Steuergerät ab, ist die nächste einen Versuch wert. Nur den "
        "KWP1281-Weg betrifft das nicht, dort gibt es diese Wahl nicht.")));

    basicInterval_ = new QSpinBox;
    basicInterval_->setRange(50, 2000);
    basicInterval_->setSingleStep(10);
    basicInterval_->setSuffix(tr(" ms"));
    form->addRow(tr("Nachfragetakt in der Grundeinstellung"), basicInterval_);
    form->addRow(makeNote(tr(
        "Das Steuergerät fällt aus der Grundeinstellung, wenn länger als etwa "
        "eine Sekunde nichts kommt. Deutlich höher als 500 ms zu gehen ist "
        "riskant, während die Drosselklappe lernt.")));
    root->addWidget(kklBox);

    // --- Log ----------------------------------------------------------------
    auto* logBox = makeGroup(tr("Protokoll"), form);

    logRaw_ = new QCheckBox(tr("Rohkommunikation mitschreiben (jede Zeile hin und "
                               "zurück)"));
    form->addRow(logRaw_);

    logToFile_ = new QCheckBox(tr("Zusätzlich in eine Datei schreiben"));
    form->addRow(logToFile_);

    auto* pathRow = new QHBoxLayout;
    logPath_ = new QLineEdit;
    logPath_->setPlaceholderText(tr("noch keine Datei gewählt"));
    logBrowse_ = new QPushButton(tr("Wählen ..."));
    pathRow->addWidget(logPath_, 1);
    pathRow->addWidget(logBrowse_);
    auto* pathWidget = new QWidget;
    pathWidget->setLayout(pathRow);
    pathRow->setContentsMargins(0, 0, 0, 0);
    form->addRow(tr("Datei"), pathWidget);
    root->addWidget(logBox);

    // --- Window -------------------------------------------------------------
    auto* windowBox = makeGroup(tr("Fenster und Sprache"), form);
    rememberGeometry_ = new QCheckBox(tr("Größe und Position beim Beenden merken"));
    form->addRow(rememberGeometry_);
    checkUpdates_ = new QCheckBox(tr("Beim Start nach einer neuen Version suchen (fragt nur GitHub, sonst nichts)"));
    form->addRow(checkUpdates_);

    language_ = new QComboBox;
    language_->addItem(tr("Wie das System"), QStringLiteral("auto"));
    // Every language under its own name, so anyone finds theirs.
    for (const LanguageInfo& info : languages())
        language_->addItem(QString::fromUtf8(info.nativeName), QString::fromLatin1(info.code));
    form->addRow(tr("Sprache"), language_);
    appearance_ = new QComboBox;
    appearance_->addItem(tr("Wie das System"), QStringLiteral("auto"));
    appearance_->addItem(tr("Dunkel"), QStringLiteral("dark"));
    appearance_->addItem(tr("Hell"), QStringLiteral("light"));
    form->addRow(tr("Erscheinungsbild"), appearance_);
    form->addRow(makeNote(tr("Sprache und Erscheinungsbild wechseln beim nächsten Start.")));
    root->addWidget(windowBox);

    // --- Footer -------------------------------------------------------------
    auto* footer = new QHBoxLayout;
    auto* reset = new QPushButton(tr("Auf Vorgabe zurücksetzen"));
    auto* openFolder = new QPushButton(tr("Ablageort öffnen"));
    footer->addWidget(reset);
    footer->addWidget(openFolder);
    footer->addStretch();
    root->addLayout(footer);
    root->addStretch();

    theme::addGlow(connectionBox, theme::color::accent, 0.35);

    // --- Wiring -------------------------------------------------------------
    connect(reset, &QPushButton::clicked, this, &SettingsPage::resetToDefaults);
    connect(openFolder, &QPushButton::clicked, this, &SettingsPage::openConfigLocation);
    connect(logBrowse_, &QPushButton::clicked, this, &SettingsPage::chooseLogPath);

    const auto onCombo = [this](int) { emitChange(); };
    connect(baud_, &QComboBox::currentIndexChanged, this, onCombo);
    connect(protocolOrder_, &QComboBox::currentIndexChanged, this, onCombo);
    connect(basicSession_, &QComboBox::currentIndexChanged, this, onCombo);
    connect(kklEcu_, &QComboBox::currentIndexChanged, this, onCombo);
    connect(language_, &QComboBox::currentIndexChanged, this, onCombo);
    connect(appearance_, &QComboBox::currentIndexChanged, this, onCombo);

    const auto onSpin = [this](int) { emitChange(); };
    connect(timeout_, &QSpinBox::valueChanged, this, onSpin);
    connect(timing_, &QComboBox::currentIndexChanged, this, onCombo);
    connect(pollInterval_, &QSpinBox::valueChanged, this, onSpin);
    connect(chartWindow_, &QSpinBox::valueChanged, this, onSpin);
    connect(clearAttempts_, &QSpinBox::valueChanged, this, onSpin);
    connect(basicInterval_, &QSpinBox::valueChanged, this, onSpin);

    const auto onCheck = [this](bool) { emitChange(); };
    connect(autoConnect_, &QCheckBox::toggled, this, onCheck);
    connect(confirmClear_, &QCheckBox::toggled, this, onCheck);
    connect(logRaw_, &QCheckBox::toggled, this, onCheck);
    connect(logToFile_, &QCheckBox::toggled, this, onCheck);
    connect(rememberGeometry_, &QCheckBox::toggled, this, onCheck);
    connect(checkUpdates_, &QCheckBox::toggled, this, onCheck);
    connect(logPath_, &QLineEdit::editingFinished, this, [this] { emitChange(); });

    showValues(AppSettings{});
}

void SettingsPage::showValues(const AppSettings& v) {
    loading_ = true;

    const int baudIndex = baud_->findData(v.baudOverride);
    baud_->setCurrentIndex(baudIndex >= 0 ? baudIndex : 0);
    const int orderIndex = protocolOrder_->findData(v.protocolOrder);
    protocolOrder_->setCurrentIndex(orderIndex >= 0 ? orderIndex : 0);
    timeout_->setValue(v.timeoutPercent);
    timing_->setCurrentIndex(qBound(0, v.adaptiveTiming, 2));
    autoConnect_->setChecked(v.autoConnect);

    pollInterval_->setValue(v.pollIntervalMs);
    chartWindow_->setValue(v.chartWindowSeconds);

    clearAttempts_->setValue(v.clearAttempts);
    confirmClear_->setChecked(v.confirmClear);

    const int sessionIndex = basicSession_->findData(v.basicSettingSession);
    basicSession_->setCurrentIndex(sessionIndex >= 0 ? sessionIndex : 0);
    basicInterval_->setValue(v.basicSettingIntervalMs);
    const int ecuIndex = kklEcu_->findData(v.kklEcuAddress);
    kklEcu_->setCurrentIndex(ecuIndex >= 0 ? ecuIndex : 0);

    logRaw_->setChecked(v.logRaw);
    logToFile_->setChecked(v.logToFile);
    logPath_->setText(v.logPath);

    rememberGeometry_->setChecked(v.rememberGeometry);
    checkUpdates_->setChecked(v.checkUpdates);
    const int languageIndex = language_->findData(v.language);
    language_->setCurrentIndex(languageIndex >= 0 ? languageIndex : 0);
    const int appearanceIndex = appearance_->findData(v.appearance);
    appearance_->setCurrentIndex(appearanceIndex >= 0 ? appearanceIndex : 0);

    loading_ = false;
    emitChange();
}

AppSettings SettingsPage::values() const {
    AppSettings v;
    v.baudOverride = baud_->currentData().toInt();
    v.protocolOrder = protocolOrder_->currentData().toString();
    v.timeoutPercent = timeout_->value();
    v.adaptiveTiming = timing_->currentData().toInt();
    v.autoConnect = autoConnect_->isChecked();

    v.pollIntervalMs = pollInterval_->value();
    v.chartWindowSeconds = chartWindow_->value();

    v.clearAttempts = clearAttempts_->value();
    v.confirmClear = confirmClear_->isChecked();

    v.basicSettingSession = basicSession_->currentData().toInt();
    v.basicSettingIntervalMs = basicInterval_->value();
    v.kklEcuAddress = kklEcu_->currentData().toInt();

    v.logRaw = logRaw_->isChecked();
    v.logToFile = logToFile_->isChecked();
    v.logPath = logPath_->text().trimmed();

    v.rememberGeometry = rememberGeometry_->isChecked();
    v.checkUpdates = checkUpdates_->isChecked();
    v.language = language_->currentData().toString();
    v.appearance = appearance_->currentData().toString();
    return v;
}

void SettingsPage::emitChange() {
    // The hint about the polling interval depends on the value, so update it here.
    const int interval = pollInterval_->value();
    pollHint_->setText(
        interval == 0
            ? tr("Null heißt: so schnell der Adapter mitkommt. Auf der K-Line ist "
                 "das ohnehin durch das Protokoll begrenzt.")
            : tr("Etwa %1 Abfragen je Sekunde. Ein sehr kleiner Wert schüttet die "
                 "Oberfläche mit Signalen zu, ein großer lässt die Diagramme "
                 "stocken.")
                  .arg(1000 / interval));

    // The file path only matters when logging to a file is enabled.
    logPath_->setEnabled(logToFile_->isChecked());
    logBrowse_->setEnabled(logToFile_->isChecked());

    if (loading_) return;
    emit changed(values());
}

void SettingsPage::resetToDefaults() { showValues(defaultSettings()); }

void SettingsPage::chooseLogPath() {
    const QString base =
        QStandardPaths::writableLocation(QStandardPaths::DocumentsLocation);
    const QString path = QFileDialog::getSaveFileName(
        this, tr("Protokolldatei wählen"),
        logPath_->text().isEmpty() ? base + QStringLiteral("/caruspex.log")
                                   : logPath_->text(),
        tr("Protokolldatei (*.log *.txt)"));
    if (path.isEmpty()) return;
    logPath_->setText(path);
    emitChange();
}

void SettingsPage::openConfigLocation() {
    const QString path =
        QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation);
    QDesktopServices::openUrl(QUrl::fromLocalFile(path));
}

}  // namespace caruspex
