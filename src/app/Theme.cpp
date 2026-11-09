#include "app/Theme.h"

#include <QApplication>
#include <QFont>
#include <QFontDatabase>
#include <QGraphicsDropShadowEffect>
#include <QPalette>
#include <QPropertyAnimation>
#include <QStyle>
#include <QWidget>

namespace caruspex::theme {
namespace {

struct Families {
    QString sans = QStringLiteral("Segoe UI");
    QString display = QStringLiteral("Segoe UI");
    QString mono = QStringLiteral("Consolas");
};

Families& families() {
    static Families f;
    return f;
}

// Loads a set of font files and returns the family name of the first one that
// loaded, otherwise the fallback.
QString load(std::initializer_list<const char*> files, const QString& fallback) {
    QString family;
    for (const char* file : files) {
        const int id = QFontDatabase::addApplicationFont(QString::fromLatin1(file));
        if (id < 0) continue;
        const QStringList names = QFontDatabase::applicationFontFamilies(id);
        if (!names.isEmpty() && family.isEmpty()) family = names.front();
    }
    return family.isEmpty() ? fallback : family;
}

void loadFonts() {
    Families& f = families();
    f.sans = load({":/fonts/Inter-Regular.ttf", ":/fonts/Inter-Medium.ttf",
                   ":/fonts/Inter-SemiBold.ttf", ":/fonts/Inter-Bold.ttf"},
                  f.sans);
    f.display = load({":/fonts/PlusJakartaSans-SemiBold.ttf", ":/fonts/PlusJakartaSans-Bold.ttf",
                      ":/fonts/PlusJakartaSans-ExtraBold.ttf"},
                     f.sans);
    f.mono = load({":/fonts/JetBrainsMono-Regular.ttf", ":/fonts/JetBrainsMono-Medium.ttf",
                   ":/fonts/JetBrainsMono-Bold.ttf"},
                  f.mono);
}

QFont make(const QString& family, int px, int weight) {
    QFont font(family);
    font.setPixelSize(px);
    font.setWeight(static_cast<QFont::Weight>(weight));
    font.setHintingPreference(QFont::PreferNoHinting);
    font.setStyleStrategy(QFont::PreferAntialias);
    return font;
}

}  // namespace

QString sansFamily() { return families().sans; }
QString displayFamily() { return families().display; }
QString monoFamily() { return families().mono; }

QFont sans(int px, int weight) { return make(families().sans, px, weight); }
QFont display(int px, int weight) { return make(families().display, px, weight); }
QFont mono(int px, int weight) { return make(families().mono, px, weight); }

QLinearGradient accentGradient(double x1, double y1, double x2, double y2) {
    QLinearGradient g(x1, y1, x2, y2);
    g.setColorAt(0.0, color::accent);
    g.setColorAt(1.0, color::sky);
    return g;
}

QColor mix(const QColor& a, const QColor& b, double t) {
    t = qBound(0.0, t, 1.0);
    return QColor::fromRgbF(static_cast<float>(a.redF() + (b.redF() - a.redF()) * t),
                            static_cast<float>(a.greenF() + (b.greenF() - a.greenF()) * t),
                            static_cast<float>(a.blueF() + (b.blueF() - a.blueF()) * t),
                            static_cast<float>(a.alphaF() + (b.alphaF() - a.alphaF()) * t));
}

QColor healthColor(double fraction) {
    fraction = qBound(0.0, fraction, 1.0);
    if (fraction < 0.5) return mix(color::bad, color::warn, fraction * 2.0);
    return mix(color::warn, color::good, (fraction - 0.5) * 2.0);
}

QString rgba(const QColor& c, double alpha) {
    return QStringLiteral("rgba(%1, %2, %3, %4)")
        .arg(c.red())
        .arg(c.green())
        .arg(c.blue())
        .arg(alpha, 0, 'f', 3);
}

QString stylesheet() {
    return QStringLiteral(R"(
QWidget {
    background: transparent;
    color: %TEXT%;
    font-family: "%SANS%";
    font-size: 14px;
}
QMainWindow, QDialog { background: %BG%; }
QStackedWidget > QWidget, QScrollArea > QWidget > QWidget { background: transparent; }

QFrame#Card, QWidget#Card {
    background: qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 #151924, stop:1 %SURFACE%);
    border: 1px solid %BORDER%;
    border-radius: 16px;
}
QFrame#CardFlat {
    background: %SURFACE%;
    border: 1px solid %BORDER%;
    border-radius: 12px;
}
QFrame#Sidebar {
    background: #0d1016;
    border: none;
    border-right: 1px solid %BORDER%;
}
QFrame#Divider { background: %BORDER%; max-height: 1px; min-height: 1px; border: none; }

QLabel#Brand {
    font-family: "%DISPLAY%";
    font-size: 17px;
    font-weight: 800;
    color: %TEXT%;
}
QLabel#Eyebrow {
    color: %ACCENT%;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 1.6px;
}
QLabel#NavSection {
    color: %FAINT%;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 1.4px;
    padding: 8px 12px 2px 12px;
}
QLabel#Subtle { color: %DIM%; }
QLabel#Hint   { color: %FAINT%; font-size: 12px; }
QLabel#PageTitle {
    font-family: "%DISPLAY%";
    font-size: 26px;
    font-weight: 700;
    color: %TEXT%;
}
QLabel#SectionTitle {
    font-family: "%DISPLAY%";
    font-size: 17px;
    font-weight: 700;
    color: %TEXT%;
}
QLabel#Headline {
    font-family: "%DISPLAY%";
    font-size: 30px;
    font-weight: 800;
}
QLabel#Code { font-family: "%MONO%"; }

QPushButton {
    background: %SURFACE2%;
    border: 1px solid %BORDER_BRIGHT%;
    border-radius: 10px;
    padding: 9px 18px;
    color: %TEXT%;
    font-weight: 600;
}
QPushButton:hover {
    background: %SURFACE_HOVER%;
    border-color: #454e66;
}
QPushButton:pressed { background: #151924; }
QPushButton:disabled { color: %FAINT%; background: #12151c; border-color: %BORDER%; }

QPushButton#Primary {
    background: qlineargradient(x1:0, y1:0, x2:1, y2:0, stop:0 %ACCENT%, stop:1 %SKY%);
    border: none;
    color: #ffffff;
    font-weight: 700;
    padding: 10px 20px;
}
QPushButton#Primary:hover {
    background: qlineargradient(x1:0, y1:0, x2:1, y2:0, stop:0 #a9a6ff, stop:1 #7cc8ff);
}
QPushButton#Primary:pressed {
    background: qlineargradient(x1:0, y1:0, x2:1, y2:0, stop:0 #7a77ef, stop:1 #4aa6ec);
}
QPushButton#Primary:disabled {
    background: #1a1e28; color: %FAINT%; border: 1px solid %BORDER%;
}

QPushButton#Danger {
    background: %DANGER_FILL%;
    border: 1px solid %DANGER_LINE%;
    color: %DANGER%;
}
QPushButton#Danger:hover { background: %DANGER_FILL_HOVER%; border-color: %DANGER%; }

QPushButton#Ghost {
    background: transparent;
    border: 1px solid transparent;
    color: %DIM%;
    padding: 7px 12px;
}
QPushButton#Ghost:hover { background: %SURFACE2%; color: %TEXT%; }

QPushButton#Chip {
    background: %SURFACE2%;
    border: 1px solid %BORDER_BRIGHT%;
    border-radius: 14px;
    padding: 5px 12px;
    font-weight: 500;
    font-size: 13px;
    color: %DIM%;
}
QPushButton#Chip:checked {
    background: %ACCENT_FILL%;
    border-color: %ACCENT_LINE%;
    color: %ACCENT_HOVER%;
}

QComboBox, QLineEdit, QSpinBox, QDoubleSpinBox, QPlainTextEdit, QTextEdit {
    background: #0e1118;
    border: 1px solid %BORDER_BRIGHT%;
    border-radius: 10px;
    padding: 8px 12px;
    color: %TEXT%;
    selection-background-color: %ACCENT_DIM%;
    selection-color: #ffffff;
}
QComboBox:hover, QLineEdit:hover, QSpinBox:hover, QDoubleSpinBox:hover { border-color: #454e66; }
QComboBox:focus, QLineEdit:focus, QSpinBox:focus, QDoubleSpinBox:focus,
QPlainTextEdit:focus, QTextEdit:focus { border-color: %ACCENT%; }
QComboBox::drop-down { border: none; width: 26px; }
QComboBox::down-arrow { image: url(:/ui/chevron-down.svg); width: 12px; height: 12px; }
QComboBox QAbstractItemView {
    background: %SURFACE2%;
    border: 1px solid %BORDER_BRIGHT%;
    border-radius: 10px;
    padding: 4px;
    selection-background-color: %ACCENT_FILL%;
    selection-color: %TEXT%;
    outline: none;
}
QSpinBox::up-button, QSpinBox::down-button,
QDoubleSpinBox::up-button, QDoubleSpinBox::down-button { width: 0; border: none; }

QTableWidget, QTableView, QTreeWidget, QTreeView, QListWidget {
    background: %SURFACE%;
    alternate-background-color: #141823;
    border: 1px solid %BORDER%;
    border-radius: 12px;
    gridline-color: transparent;
    selection-background-color: %ACCENT_FILL%;
    selection-color: %TEXT%;
    outline: none;
}
QTableWidget::item, QTreeWidget::item, QListWidget::item {
    padding: 6px 8px;
    border-bottom: 1px solid #1a1e29;
}
QTableWidget::item:hover, QTreeWidget::item:hover, QListWidget::item:hover {
    background: %SURFACE2%;
}
QHeaderView { background: transparent; }
QHeaderView::section {
    background: transparent;
    color: %FAINT%;
    border: none;
    border-bottom: 1px solid %BORDER_BRIGHT%;
    padding: 10px 8px;
    font-weight: 600;
    font-size: 12px;
}
QTableCornerButton::section { background: transparent; border: none; }

QScrollBar:vertical   { background: transparent; width: 10px; margin: 2px; }
QScrollBar::handle:vertical {
    background: #2a3040; border-radius: 4px; min-height: 36px;
}
QScrollBar::handle:vertical:hover { background: #3a4256; }
QScrollBar:horizontal { background: transparent; height: 10px; margin: 2px; }
QScrollBar::handle:horizontal {
    background: #2a3040; border-radius: 4px; min-width: 36px;
}
QScrollBar::add-line, QScrollBar::sub-line { height: 0; width: 0; }
QScrollBar::add-page, QScrollBar::sub-page { background: transparent; }

QProgressBar {
    background: #1a1e28;
    border: none;
    border-radius: 3px;
    max-height: 6px;
    min-height: 6px;
    text-align: center;
    color: transparent;
}
QProgressBar::chunk {
    background: qlineargradient(x1:0, y1:0, x2:1, y2:0, stop:0 %ACCENT%, stop:1 %SKY%);
    border-radius: 3px;
}

QCheckBox { spacing: 10px; }
QCheckBox::indicator { width: 18px; height: 18px; }
QCheckBox::indicator:unchecked {
    border: 1px solid #454e66; border-radius: 5px; background: #0e1118;
}
QCheckBox::indicator:unchecked:hover { border-color: %ACCENT%; }
QCheckBox::indicator:checked {
    border: none; border-radius: 5px; image: url(:/ui/check-box.svg);
}
QRadioButton::indicator { width: 16px; height: 16px; }

QToolTip {
    background: %SURFACE2%; color: %TEXT%;
    border: 1px solid %BORDER_BRIGHT%; border-radius: 8px; padding: 6px 8px;
}
QSplitter::handle { background: transparent; }
QScrollArea { border: none; background: transparent; }
QTabWidget::pane { border: none; }
QTabBar::tab {
    background: transparent; color: %DIM%; padding: 8px 14px; border: none;
    border-bottom: 2px solid transparent; font-weight: 600;
}
QTabBar::tab:selected { color: %TEXT%; border-bottom: 2px solid %ACCENT%; }
QTabBar::tab:hover { color: %TEXT%; }
QGroupBox {
    border: 1px solid %BORDER%; border-radius: 12px; margin-top: 18px; padding: 14px;
    font-weight: 600;
}
QGroupBox::title { subcontrol-origin: margin; left: 14px; padding: 0 4px; color: %DIM%; }
QMenu {
    background: %SURFACE2%; border: 1px solid %BORDER_BRIGHT%; border-radius: 10px; padding: 4px;
}
QMenu::item { padding: 7px 18px; border-radius: 6px; }
QMenu::item:selected { background: %ACCENT_FILL%; }
)")
        .replace("%SANS%", families().sans)
        .replace("%DISPLAY%", families().display)
        .replace("%MONO%", families().mono)
        .replace("%BG%", color::background.name())
        .replace("%SURFACE_HOVER%", color::surfaceHover.name())
        .replace("%SURFACE2%", color::surfaceHigh.name())
        .replace("%SURFACE%", color::surface.name())
        .replace("%BORDER_BRIGHT%", color::borderBright.name())
        .replace("%BORDER%", color::border.name())
        .replace("%TEXT%", color::text.name())
        .replace("%DIM%", color::textMuted.name())
        .replace("%FAINT%", color::textFaint.name())
        .replace("%ACCENT_FILL%", rgba(color::accent, 0.14))
        .replace("%ACCENT_LINE%", rgba(color::accent, 0.45))
        .replace("%ACCENT_HOVER%", color::accentHover.name())
        .replace("%ACCENT_DIM%", color::accentDim.name())
        .replace("%ACCENT%", color::accent.name())
        .replace("%SKY%", color::sky.name())
        .replace("%DANGER_FILL_HOVER%", rgba(color::bad, 0.20))
        .replace("%DANGER_FILL%", rgba(color::bad, 0.10))
        .replace("%DANGER_LINE%", rgba(color::bad, 0.45))
        .replace("%DANGER%", color::bad.name());
}

void apply(QApplication& app) {
    app.setStyle(QStringLiteral("Fusion"));
    loadFonts();

    QPalette palette;
    palette.setColor(QPalette::Window, color::background);
    palette.setColor(QPalette::WindowText, color::text);
    palette.setColor(QPalette::Base, color::surface);
    palette.setColor(QPalette::AlternateBase, color::surfaceHigh);
    palette.setColor(QPalette::Text, color::text);
    palette.setColor(QPalette::Button, color::surfaceHigh);
    palette.setColor(QPalette::ButtonText, color::text);
    palette.setColor(QPalette::Highlight, color::accentDim);
    palette.setColor(QPalette::HighlightedText, Qt::white);
    palette.setColor(QPalette::ToolTipBase, color::surfaceHigh);
    palette.setColor(QPalette::ToolTipText, color::text);
    palette.setColor(QPalette::PlaceholderText, color::textFaint);
    palette.setColor(QPalette::Link, color::accent);
    app.setPalette(palette);

    app.setFont(sans(14));
    app.setStyleSheet(stylesheet());
}

void addShadow(QWidget* widget, int blur, int offsetY, double alpha) {
    if (!widget) return;
    auto* shadow = new QGraphicsDropShadowEffect(widget);
    shadow->setOffset(0, offsetY);
    shadow->setBlurRadius(blur);
    shadow->setColor(QColor(0, 0, 0, static_cast<int>(255 * alpha)));
    widget->setGraphicsEffect(shadow);
}

void addGlow(QWidget* widget, const QColor& colour, double strength) {
    if (!widget) return;
    auto* glow = new QGraphicsDropShadowEffect(widget);
    glow->setOffset(0, 0);
    glow->setBlurRadius(28.0 * strength);
    QColor lit = colour;
    lit.setAlphaF(static_cast<float>(qBound(0.0, 0.35 * strength, 1.0)));
    glow->setColor(lit);
    widget->setGraphicsEffect(glow);
}

void removeGlow(QWidget* widget) {
    if (widget) widget->setGraphicsEffect(nullptr);
}

void pulseGlow(QWidget* widget, const QColor& colour) {
    if (!widget) return;
    auto* glow = new QGraphicsDropShadowEffect(widget);
    glow->setOffset(0, 6);
    QColor lit = colour;
    lit.setAlphaF(0.40f);
    glow->setColor(lit);
    glow->setBlurRadius(18.0);
    widget->setGraphicsEffect(glow);

    auto* animation = new QPropertyAnimation(glow, "blurRadius", widget);
    animation->setDuration(2400);
    animation->setStartValue(16.0);
    animation->setKeyValueAt(0.5, 34.0);
    animation->setEndValue(16.0);
    animation->setEasingCurve(QEasingCurve::InOutSine);
    animation->setLoopCount(-1);
    animation->start(QAbstractAnimation::DeleteWhenStopped);
}

void repolish(QWidget* widget) {
    if (!widget) return;
    widget->style()->unpolish(widget);
    widget->style()->polish(widget);
    widget->update();
}

}  // namespace caruspex::theme
