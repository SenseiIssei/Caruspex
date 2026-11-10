#include "app/Theme.h"

#include "core/Lang.h"

#include <QApplication>

#include <algorithm>
#include <QFont>
#include <QFontDatabase>
#include <QGraphicsDropShadowEffect>
#include <QPalette>
#include <QPainter>
#include <QPainterPath>
#include <QPropertyAnimation>
#include <QRegularExpression>
#include <QProxyStyle>
#include <QStyleOption>
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

    // The bundled fonts only cover Latin, Greek and Cyrillic. For other
    // scripts Qt would pick a fallback font per character, which mixes
    // styles within one word (Chinese most of all). One fixed font per
    // script keeps the text even: the system fonts of Windows, macOS and
    // Linux, the first one present wins.
    const Language active = language();
    QStringList script;
    if (active == Language::Chinese)
        script = {"Microsoft YaHei UI", "Microsoft YaHei", "PingFang SC", "Noto Sans CJK SC", "Noto Sans SC"};
    else if (active == Language::Japanese)
        script = {"Yu Gothic UI", "Meiryo UI", "Hiragino Sans", "Noto Sans CJK JP", "Noto Sans JP"};
    else if (active == Language::Korean)
        script = {"Malgun Gothic", "Apple SD Gothic Neo", "Noto Sans CJK KR", "Noto Sans KR"};
    else if (active == Language::Hindi)
        script = {"Nirmala UI", "Kohinoor Devanagari", "Noto Sans Devanagari"};
    else if (active == Language::Arabic)
        script = {"Segoe UI", "Geeza Pro", "Noto Sans Arabic", "Noto Naskh Arabic"};
    if (!script.isEmpty()) {
        for (const QString& family : {f.sans, f.display, f.mono}) QFont::insertSubstitutions(family, script);
    }
}

QFont make(const QString& family, int px, int weight) {
    QFont font(family);
    font.setPixelSize(px);
    font.setWeight(static_cast<QFont::Weight>(weight));
    font.setHintingPreference(QFont::PreferNoHinting);
    font.setStyleStrategy(QFont::PreferAntialias);
    return font;
}

// Draws the few primitives the style sheet cannot: check boxes with the
// gradient and a white tick, and the small arrow in combo boxes. Everything
// else is Fusion.
class CaruspexStyle : public QProxyStyle {
public:
    CaruspexStyle() : QProxyStyle(QStringLiteral("Fusion")) {}

    int pixelMetric(PixelMetric metric, const QStyleOption* option,
                    const QWidget* widget) const override {
        if (metric == PM_IndicatorWidth || metric == PM_IndicatorHeight) return 18;
        return QProxyStyle::pixelMetric(metric, option, widget);
    }

    void drawPrimitive(PrimitiveElement element, const QStyleOption* option, QPainter* p,
                       const QWidget* widget) const override {
        if (element == PE_IndicatorCheckBox || element == PE_IndicatorItemViewItemCheck) {
            p->save();
            p->setRenderHint(QPainter::Antialiasing);
            const QRectF box = QRectF(option->rect).adjusted(1, 1, -1, -1);
            const double side = std::min(box.width(), box.height());
            const QRectF square(box.center().x() - side / 2, box.center().y() - side / 2, side, side);
            const bool on = option->state & State_On;
            const bool hover = option->state & State_MouseOver;
            const bool enabled = option->state & State_Enabled;
            QPainterPath path;
            path.addRoundedRect(square, 5, 5);
            if (on) {
                p->fillPath(path, accentGradient(square.left(), square.top(), square.right(),
                                                 square.bottom()));
                QPen tick(Qt::white, 2.0, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin);
                p->setPen(tick);
                QPainterPath mark;
                mark.moveTo(square.left() + side * 0.27, square.top() + side * 0.52);
                mark.lineTo(square.left() + side * 0.43, square.top() + side * 0.68);
                mark.lineTo(square.left() + side * 0.74, square.top() + side * 0.34);
                p->drawPath(mark);
            } else {
                p->fillPath(path, color::input);
                p->setPen(QPen(hover ? color::accent : color::hoverLine, 1.2));
                p->drawPath(path);
            }
            if (!enabled) p->fillPath(path, QColor(10, 12, 17, 150));
            p->restore();
            return;
        }
        if (element == PE_IndicatorArrowDown) {
            p->save();
            p->setRenderHint(QPainter::Antialiasing);
            const QPointF c = QRectF(option->rect).center();
            QPen pen(color::textMuted, 1.8, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin);
            p->setPen(pen);
            QPainterPath v;
            v.moveTo(c.x() - 4.5, c.y() - 2);
            v.lineTo(c.x(), c.y() + 2.5);
            v.lineTo(c.x() + 4.5, c.y() - 2);
            p->drawPath(v);
            p->restore();
            return;
        }
        QProxyStyle::drawPrimitive(element, option, p, widget);
    }
};

}  // namespace

namespace {
bool g_light = false;
}  // namespace

void setLight(bool light) {
    g_light = light;
    if (!light) return;
    // The light palette of Odysync: paper white surfaces, ink text, the
    // accents a little deeper so they hold their contrast on white.
    color::background = QColor("#f4f5fa");
    color::surface = QColor("#ffffff");
    color::surfaceHigh = QColor("#eef0f6");
    color::surfaceHover = QColor("#e4e7f0");
    color::border = QColor("#e2e5ee");
    color::borderBright = QColor("#cdd2de");
    color::sidebar = QColor("#fbfbfe");
    color::input = QColor("#ffffff");
    color::cardTop = QColor("#ffffff");
    color::track = QColor("#e6e8f0");
    color::rowLine = QColor("#eceef4");
    color::alternate = QColor("#f8f9fc");
    color::hoverLine = QColor("#b8bfcf");
    color::scroll = QColor("#cfd4df");
    color::scrollHover = QColor("#b3bacb");
    color::pressed = QColor("#e4e7f0");
    color::disabled = QColor("#f0f1f5");
    color::text = QColor("#141821");
    color::textMuted = QColor("#515b6b");
    color::textFaint = QColor("#788091");
    color::accent = QColor("#5a54f0");
    color::accentHover = QColor("#4842d8");
    color::accentDim = QColor("#4842d8");
    color::sky = QColor("#1b8fe0");
    color::purple = QColor("#7c5cf0");
    color::purpleDim = QColor("#6a48e0");
    color::good = QColor("#12875a");
    color::warn = QColor("#b97a06");
    color::bad = QColor("#c93636");
    color::info = QColor("#1b6fc0");
    color::warm = QColor("#b97a06");
}

bool isLight() { return g_light; }

bool spacedLetters() {
    const Language l = language();
    return l != Language::Hindi && l != Language::Arabic;
}

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
    background: qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 %CARDTOP%, stop:1 %SURFACE%);
    border: 1px solid %BORDER%;
    border-radius: 16px;
}
QFrame#CardFlat {
    background: %SURFACE%;
    border: 1px solid %BORDER%;
    border-radius: 12px;
}
QFrame#Sidebar {
    background: %SIDEBAR%;
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
    border-color: %HOVERLINE%;
}
QPushButton:pressed { background: %PRESSED%; }
QPushButton:disabled { color: %FAINT%; background: %DISABLED%; border-color: %BORDER%; }

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
    background: %SURFACE2%; color: %FAINT%; border: 1px solid %BORDER%;
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
    background: %INPUT%;
    border: 1px solid %BORDER_BRIGHT%;
    border-radius: 10px;
    padding: 8px 12px;
    color: %TEXT%;
    selection-background-color: %ACCENT_DIM%;
    selection-color: #ffffff;
}
QComboBox:hover, QLineEdit:hover, QSpinBox:hover, QDoubleSpinBox:hover { border-color: %HOVERLINE%; }
QComboBox:focus, QLineEdit:focus, QSpinBox:focus, QDoubleSpinBox:focus,
QPlainTextEdit:focus, QTextEdit:focus { border-color: %ACCENT%; }
QComboBox { padding-right: 30px; }
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
    alternate-background-color: %ALTERNATE%;
    border: 1px solid %BORDER%;
    border-radius: 12px;
    gridline-color: transparent;
    selection-background-color: %ACCENT_FILL%;
    selection-color: %TEXT%;
    outline: none;
}
QTableWidget::item, QTreeWidget::item, QListWidget::item {
    padding: 6px 8px;
    border-bottom: 1px solid %ROWLINE%;
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
    background: %SCROLL%; border-radius: 4px; min-height: 36px;
}
QScrollBar::handle:vertical:hover { background: %SCROLLHOVER%; }
QScrollBar::handle:horizontal:hover { background: %SCROLLHOVER%; }
QScrollBar:horizontal { background: transparent; height: 10px; margin: 2px; }
QScrollBar::handle:horizontal {
    background: %SCROLL%; border-radius: 4px; min-width: 36px;
}
QScrollBar::add-line, QScrollBar::sub-line { height: 0; width: 0; }
QScrollBar::add-page, QScrollBar::sub-page { background: transparent; }

QProgressBar {
    background: %TRACK%;
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
        .replace("%CARDTOP%", color::cardTop.name())
        .replace("%SIDEBAR%", color::sidebar.name())
        .replace("%HOVERLINE%", color::hoverLine.name())
        .replace("%PRESSED%", color::pressed.name())
        .replace("%DISABLED%", color::disabled.name())
        .replace("%INPUT%", color::input.name())
        .replace("%ALTERNATE%", color::alternate.name())
        .replace("%ROWLINE%", color::rowLine.name())
        .replace("%SCROLLHOVER%", color::scrollHover.name())
        .replace("%SCROLL%", color::scroll.name())
        .replace("%TRACK%", color::track.name())
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
    app.setStyle(new CaruspexStyle);
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
    QString sheet = stylesheet();
    // Spacing between letters tears joined scripts apart (Devanagari, Arabic).
    if (!spacedLetters()) sheet.replace(QRegularExpression(QStringLiteral("letter-spacing: *[0-9.]+px")),
                                        QStringLiteral("letter-spacing: 0px"));
    app.setStyleSheet(sheet);
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

void styleFont(QWidget* widget, Family family, int px, int weight) {
    if (!widget) return;
    const QString name = family == Family::Display ? families().display
                         : family == Family::Mono  ? families().mono
                                                   : families().sans;
    widget->setStyleSheet(widget->styleSheet() +
                          QStringLiteral(" font-family: \"%1\"; font-size: %2px; font-weight: %3;")
                              .arg(name)
                              .arg(px)
                              .arg(weight));
}

void repolish(QWidget* widget) {
    if (!widget) return;
    widget->style()->unpolish(widget);
    widget->style()->polish(widget);
    widget->update();
}

}  // namespace caruspex::theme
