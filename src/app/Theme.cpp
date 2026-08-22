#include "app/Theme.h"

#include <QApplication>
#include <QFont>
#include <QPalette>

namespace caruspex::theme {

QString stylesheet() {
    return QString(R"(
QWidget {
    background: %1;
    color: %2;
    font-size: 14px;
}
QLabel, QCheckBox, QRadioButton {
    background: transparent;
}
QFrame#Card, QWidget#Card {
    background: %3;
    border: 1px solid %4;
    border-radius: 10px;
}
QFrame#Sidebar {
    background: %5;
    border-right: 1px solid %4;
}
QLabel#Brand {
    color: %6;
    font-size: 26px;
    font-weight: 700;
    letter-spacing: 3px;
}
QLabel#Subtle, QLabel#Hint {
    color: %7;
}
QLabel#PageTitle {
    font-size: 21px;
    font-weight: 600;
}
QLabel#Headline {
    font-size: 30px;
    font-weight: 700;
}
QPushButton {
    background: %8;
    border: 1px solid %4;
    border-radius: 8px;
    padding: 9px 16px;
    color: %2;
}
QPushButton:hover  { background: #2a3140; border-color: #3d4759; }
QPushButton:pressed{ background: #333c4d; }
QPushButton:disabled { color: #5b6474; background: #1a1e27; }
QPushButton#Primary {
    background: #1f6feb; border: none; font-weight: 600; color: #ffffff;
}
QPushButton#Primary:hover   { background: #2f80f5; }
QPushButton#Primary:disabled{ background: #24303f; color: #6b7484; }
QPushButton#Danger {
    background: #8a2c2c; border: none; color: #ffffff;
}
QPushButton#Danger:hover { background: #a63636; }
QPushButton#NavItem {
    background: transparent;
    border: none;
    border-radius: 8px;
    padding: 11px 14px;
    text-align: left;
    font-size: 15px;
    color: %7;
}
QPushButton#NavItem:hover   { background: %8; color: %2; }
QPushButton#NavItem:checked { background: #1d3557; color: %2; font-weight: 600; }
QComboBox, QLineEdit, QSpinBox, QPlainTextEdit, QTextEdit {
    background: %8;
    border: 1px solid %4;
    border-radius: 8px;
    padding: 7px 10px;
    selection-background-color: #1f6feb;
}
QComboBox::drop-down { border: none; width: 22px; }
QComboBox QAbstractItemView {
    background: %3; border: 1px solid %4; selection-background-color: #1f6feb;
}
QTableWidget, QTableView, QTreeWidget {
    background: %3;
    border: 1px solid %4;
    border-radius: 10px;
    gridline-color: %4;
    selection-background-color: #1d3557;
}
QHeaderView::section {
    background: %8; color: %7; border: none;
    border-bottom: 1px solid %4; padding: 8px;
}
QScrollBar:vertical   { background: transparent; width: 11px; margin: 2px; }
QScrollBar::handle:vertical { background: #39414f; border-radius: 5px; min-height: 30px; }
QScrollBar::handle:vertical:hover { background: #4a5568; }
QScrollBar::add-line, QScrollBar::sub-line { height: 0; width: 0; }
QScrollBar:horizontal { background: transparent; height: 11px; margin: 2px; }
QScrollBar::handle:horizontal { background: #39414f; border-radius: 5px; min-width: 30px; }
QProgressBar {
    background: %8; border: none; border-radius: 4px; height: 8px; text-align: center;
}
QProgressBar::chunk { background: %6; border-radius: 4px; }
QCheckBox::indicator, QRadioButton::indicator { width: 17px; height: 17px; }
QCheckBox::indicator:unchecked {
    border: 1px solid %4; border-radius: 4px; background: %8;
}
QCheckBox::indicator:checked {
    border: 1px solid %6; border-radius: 4px; background: %6;
}
QToolTip {
    background: %3; color: %2; border: 1px solid %4; padding: 6px;
}
QSplitter::handle { background: %4; }
)")
        .arg(color::background.name(),      // %1
             color::text.name(),            // %2
             color::surface.name(),         // %3
             color::border.name(),          // %4
             color::surfaceHigh.name(),     // %5
             color::accent.name(),          // %6
             color::textMuted.name(),       // %7
             color::surfaceHigh.name());    // %8
}

void apply(QApplication& app) {
    app.setStyle("Fusion");

    QPalette palette;
    palette.setColor(QPalette::Window, color::background);
    palette.setColor(QPalette::WindowText, color::text);
    palette.setColor(QPalette::Base, color::surface);
    palette.setColor(QPalette::AlternateBase, color::surfaceHigh);
    palette.setColor(QPalette::Text, color::text);
    palette.setColor(QPalette::Button, color::surfaceHigh);
    palette.setColor(QPalette::ButtonText, color::text);
    palette.setColor(QPalette::Highlight, color::accent);
    palette.setColor(QPalette::HighlightedText, Qt::white);
    palette.setColor(QPalette::ToolTipBase, color::surface);
    palette.setColor(QPalette::ToolTipText, color::text);
    palette.setColor(QPalette::PlaceholderText, color::textMuted);
    app.setPalette(palette);

    QFont font = app.font();
    font.setFamily("Segoe UI");
    font.setPointSizeF(10.0);
    app.setFont(font);

    app.setStyleSheet(stylesheet());
}

}  // namespace caruspex::theme
