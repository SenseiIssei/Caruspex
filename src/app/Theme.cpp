#include "app/Theme.h"

#include <QApplication>
#include <QFont>
#include <QFontDatabase>
#include <QGraphicsDropShadowEffect>
#include <QPalette>
#include <QPropertyAnimation>
#include <QSequentialAnimationGroup>
#include <QWidget>

namespace caruspex::theme {
namespace {

QString loadFonts() {
    const char* files[] = {":/fonts/JetBrainsMono-Regular.ttf",
                           ":/fonts/JetBrainsMono-Medium.ttf",
                           ":/fonts/JetBrainsMono-Bold.ttf"};
    QString family;
    for (const char* file : files) {
        const int id = QFontDatabase::addApplicationFont(QString::fromLatin1(file));
        if (id < 0) continue;
        const QStringList families = QFontDatabase::applicationFontFamilies(id);
        if (!families.isEmpty() && family.isEmpty()) family = families.front();
    }
    // Falls back to what is available if the font is missing.
    return family.isEmpty() ? QStringLiteral("Consolas") : family;
}

QString rgba(const QColor& c, double alpha) {
    return QStringLiteral("rgba(%1, %2, %3, %4)")
        .arg(c.red())
        .arg(c.green())
        .arg(c.blue())
        .arg(alpha, 0, 'f', 3);
}

}  // namespace

QString stylesheet() {
    return QStringLiteral(R"(
QWidget {
    background: %BG%;
    color: %TEXT%;
    font-size: 13px;
}
QLabel, QCheckBox, QRadioButton { background: transparent; }

QFrame#Card, QWidget#Card {
    background: %SURFACE%;
    border: 1px solid %BORDER%;
    border-radius: 10px;
}
QFrame#Sidebar {
    background: %SURFACE%;
    border: none;
    border-right: 1px solid %BORDER%;
}

QLabel#Brand {
    color: %ACCENT%;
    font-size: 24px;
    font-weight: 700;
    letter-spacing: 5px;
}
QLabel#Subtle { color: %DIM%; }
QLabel#Hint   { color: %FAINT%; font-size: 12px; }
QLabel#PageTitle {
    font-size: 19px;
    font-weight: 700;
    letter-spacing: 1px;
    color: %TEXT%;
}
QLabel#Headline { font-size: 27px; font-weight: 700; }

QPushButton {
    background: %SURFACE2%;
    border: 1px solid %BORDER_BRIGHT%;
    border-radius: 6px;
    padding: 8px 16px;
    color: %TEXT%;
    font-weight: 500;
}
QPushButton:hover {
    background: #20202c;
    border-color: %ACCENT_DIM%;
    color: %ACCENT%;
}
QPushButton:pressed { background: #0f0f16; }
QPushButton:disabled { color: %FAINT%; background: #101018; border-color: %BORDER%; }

QPushButton#Primary {
    background: %ACCENT_FILL%;
    border: 1px solid %ACCENT%;
    color: %ACCENT%;
    font-weight: 700;
    letter-spacing: 1px;
}
QPushButton#Primary:hover    { background: %ACCENT_FILL_HOVER%; }
QPushButton#Primary:disabled {
    background: #101018; border-color: %BORDER%; color: %FAINT%;
}

QPushButton#Danger {
    background: %DANGER_FILL%;
    border: 1px solid %DANGER%;
    color: %DANGER%;
    font-weight: 700;
}
QPushButton#Danger:hover { background: %DANGER_FILL_HOVER%; }

QPushButton#NavItem {
    background: transparent;
    border: none;
    border-left: 2px solid transparent;
    border-radius: 0px;
    padding: 10px 14px;
    text-align: left;
    font-size: 13px;
    color: %DIM%;
    letter-spacing: 0.5px;
}
QPushButton#NavItem:hover   { color: %TEXT%; background: %SURFACE2%; }
QPushButton#NavItem:checked {
    color: %ACCENT%;
    background: %ACCENT_FILL%;
    border-left: 2px solid %ACCENT%;
    font-weight: 700;
}

QComboBox, QLineEdit, QSpinBox, QPlainTextEdit, QTextEdit {
    background: %BG%;
    border: 1px solid %BORDER_BRIGHT%;
    border-radius: 6px;
    padding: 6px 10px;
    color: %TEXT%;
    selection-background-color: %ACCENT_DIM%;
    selection-color: #000000;
}
QComboBox:focus, QLineEdit:focus, QSpinBox:focus { border-color: %ACCENT%; }
QComboBox::drop-down { border: none; width: 20px; }
QComboBox QAbstractItemView {
    background: %SURFACE%;
    border: 1px solid %BORDER_BRIGHT%;
    selection-background-color: %ACCENT_FILL%;
    selection-color: %ACCENT%;
    outline: none;
}

QTableWidget, QTableView, QTreeWidget {
    background: %SURFACE%;
    border: 1px solid %BORDER%;
    border-radius: 8px;
    gridline-color: %BORDER%;
    selection-background-color: %ACCENT_FILL%;
    selection-color: %ACCENT%;
    outline: none;
}
QHeaderView::section {
    background: %BG%;
    color: %DIM%;
    border: none;
    border-bottom: 1px solid %BORDER_BRIGHT%;
    padding: 8px;
    font-weight: 500;
    letter-spacing: 0.5px;
}
QTreeWidget::item:hover, QTableWidget::item:hover { background: %SURFACE2%; }

QScrollBar:vertical   { background: transparent; width: 7px; margin: 2px; }
QScrollBar::handle:vertical {
    background: %BORDER_BRIGHT%; border-radius: 3px; min-height: 30px;
}
QScrollBar::handle:vertical:hover { background: %ACCENT_DIM%; }
QScrollBar:horizontal { background: transparent; height: 7px; margin: 2px; }
QScrollBar::handle:horizontal {
    background: %BORDER_BRIGHT%; border-radius: 3px; min-width: 30px;
}
QScrollBar::add-line, QScrollBar::sub-line { height: 0; width: 0; }
QScrollBar::add-page, QScrollBar::sub-page { background: transparent; }

QProgressBar {
    background: %BG%;
    border: 1px solid %BORDER%;
    border-radius: 3px;
    height: 6px;
    text-align: center;
}
QProgressBar::chunk { background: %ACCENT%; border-radius: 3px; }

QCheckBox::indicator { width: 15px; height: 15px; }
QCheckBox::indicator:unchecked {
    border: 1px solid %BORDER_BRIGHT%; border-radius: 3px; background: %BG%;
}
QCheckBox::indicator:hover { border-color: %ACCENT_DIM%; }
QCheckBox::indicator:checked {
    border: 1px solid %ACCENT%; border-radius: 3px; background: %ACCENT%;
}

QToolTip {
    background: %SURFACE%; color: %ACCENT%;
    border: 1px solid %ACCENT_DIM%; padding: 6px;
}
QSplitter::handle { background: %BORDER%; }
QScrollArea { border: none; background: transparent; }
)")
        .replace("%BG%", color::background.name())
        .replace("%SURFACE2%", color::surfaceHigh.name())
        .replace("%SURFACE%", color::surface.name())
        .replace("%BORDER_BRIGHT%", color::borderBright.name())
        .replace("%BORDER%", color::border.name())
        .replace("%TEXT%", color::text.name())
        .replace("%DIM%", color::textMuted.name())
        .replace("%FAINT%", color::textFaint.name())
        .replace("%ACCENT_FILL_HOVER%", rgba(color::accent, 0.20))
        .replace("%ACCENT_FILL%", rgba(color::accent, 0.10))
        .replace("%ACCENT_HOVER%", color::accentHover.name())
        .replace("%ACCENT_DIM%", color::accentDim.name())
        .replace("%ACCENT%", color::accent.name())
        .replace("%DANGER_FILL_HOVER%", rgba(color::bad, 0.22))
        .replace("%DANGER_FILL%", rgba(color::bad, 0.12))
        .replace("%DANGER%", color::bad.name());
}

void apply(QApplication& app) {
    app.setStyle(QStringLiteral("Fusion"));

    QPalette palette;
    palette.setColor(QPalette::Window, color::background);
    palette.setColor(QPalette::WindowText, color::text);
    palette.setColor(QPalette::Base, color::background);
    palette.setColor(QPalette::AlternateBase, color::surfaceHigh);
    palette.setColor(QPalette::Text, color::text);
    palette.setColor(QPalette::Button, color::surfaceHigh);
    palette.setColor(QPalette::ButtonText, color::text);
    palette.setColor(QPalette::Highlight, color::accentDim);
    palette.setColor(QPalette::HighlightedText, Qt::black);
    palette.setColor(QPalette::ToolTipBase, color::surface);
    palette.setColor(QPalette::ToolTipText, color::accent);
    palette.setColor(QPalette::PlaceholderText, color::textFaint);
    app.setPalette(palette);

    QFont font(loadFonts());
    font.setPointSizeF(9.5);
    app.setFont(font);

    app.setStyleSheet(stylesheet());
}

void addGlow(QWidget* widget, const QColor& colour, double strength) {
    if (!widget) return;
    auto* glow = new QGraphicsDropShadowEffect(widget);
    glow->setOffset(0, 0);                       // without an offset it looks like a glow
    glow->setBlurRadius(24.0 * strength);
    QColor lit = colour;
    lit.setAlphaF(static_cast<float>(qBound(0.0, 0.55 * strength, 1.0)));
    glow->setColor(lit);
    widget->setGraphicsEffect(glow);
}

void removeGlow(QWidget* widget) {
    if (widget) widget->setGraphicsEffect(nullptr);
}

void pulseGlow(QWidget* widget, const QColor& colour) {
    if (!widget) return;
    auto* glow = new QGraphicsDropShadowEffect(widget);
    glow->setOffset(0, 0);
    QColor lit = colour;
    lit.setAlphaF(0.45f);
    glow->setColor(lit);
    glow->setBlurRadius(10.0);
    widget->setGraphicsEffect(glow);

    auto* animation = new QPropertyAnimation(glow, "blurRadius", widget);
    animation->setDuration(2000);
    animation->setStartValue(10.0);
    animation->setKeyValueAt(0.5, 30.0);
    animation->setEndValue(10.0);
    animation->setLoopCount(-1);
    animation->start(QAbstractAnimation::DeleteWhenStopped);
}

}  // namespace caruspex::theme
