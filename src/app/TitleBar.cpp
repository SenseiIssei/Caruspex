#include "app/TitleBar.h"

#include <QApplication>
#include <QHBoxLayout>
#include <QLabel>
#include <QMouseEvent>
#include <QPainter>
#include <QPainterPath>
#include <QPushButton>
#include <QWindow>

#include "app/Icons.h"
#include "app/Theme.h"
#include "app/Widgets.h"

namespace caruspex {

using namespace theme;

namespace {

constexpr int kBarHeight = 46;

QPushButton* makeWindowButton(const QString& iconName, const QColor& hoverColour,
                              const QString& tip) {
    auto* button = new QPushButton;
    button->setIcon(icons::icon(iconName, color::textMuted, color::text, 16));
    button->setIconSize(QSize(16, 16));
    button->setFixedSize(46, kBarHeight - 1);
    button->setToolTip(tip);
    button->setCursor(Qt::ArrowCursor);
    button->setFocusPolicy(Qt::NoFocus);
    button->setStyleSheet(QStringLiteral("QPushButton { background: transparent; border: none; "
                                         "border-radius: 0px; padding: 0; }"
                                         "QPushButton:hover { background: %1; }")
                              .arg(rgba(hoverColour, 0.18)));
    return button;
}

QPushButton* makeToolButton(const QString& iconName, const QString& tip) {
    auto* button = new QPushButton;
    button->setIcon(icons::icon(iconName, color::textMuted, color::accentHover, 18));
    button->setIconSize(QSize(18, 18));
    button->setFixedSize(34, 32);
    button->setToolTip(tip);
    button->setCheckable(false);
    button->setFocusPolicy(Qt::NoFocus);
    button->setCursor(Qt::PointingHandCursor);
    button->setStyleSheet(QStringLiteral("QPushButton { background: transparent; border: none; "
                                         "border-radius: 8px; padding: 0; }"
                                         "QPushButton:hover { background: %1; }"
                                         "QPushButton:checked { background: %2; }")
                              .arg(color::surfaceHigh.name(), rgba(color::accent, 0.14)));
    return button;
}

// Rounded square with the gradient and a small stylised car, the app mark.
class LogoMark : public QWidget {
public:
    LogoMark() { setFixedSize(26, 26); }

protected:
    void paintEvent(QPaintEvent*) override {
        QPainter p(this);
        p.setRenderHint(QPainter::Antialiasing);
        QPainterPath box;
        box.addRoundedRect(QRectF(rect()).adjusted(0.5, 0.5, -0.5, -0.5), 7, 7);
        p.fillPath(box, accentGradient(0, 0, width(), height()));
        p.drawPixmap(4, 4, icons::pixmap(QStringLiteral("car"), 18, Qt::white, 2.0));
    }
};

}  // namespace

TitleBar::TitleBar(QWidget* parent) : QWidget(parent) {
    setFixedHeight(kBarHeight);
    setAttribute(Qt::WA_StyledBackground, false);

    auto* layout = new QHBoxLayout(this);
    layout->setContentsMargins(10, 0, 0, 0);
    layout->setSpacing(6);

    sidebar_ = makeToolButton(QStringLiteral("sidebar"), tr("Seitenleiste ein- oder ausklappen"));
    layout->addWidget(sidebar_);
    layout->addSpacing(2);
    layout->addWidget(new LogoMark);
    layout->addSpacing(4);

    title_ = new QLabel(QStringLiteral("Caruspex"));
    title_->setFont(display(15, 800));
    auto* version = new QLabel(QStringLiteral("v") + QApplication::applicationVersion());
    version->setStyleSheet(QStringLiteral("color: %1; font-size: 12px;").arg(color::textFaint.name()));
    layout->addWidget(title_);
    layout->addWidget(version);
    layout->addStretch(1);

    // Connection capsule
    capsule_ = new QPushButton;
    capsule_->setCursor(Qt::PointingHandCursor);
    capsule_->setFocusPolicy(Qt::NoFocus);
    capsule_->setMinimumHeight(30);
    auto* capsuleLayout = new QHBoxLayout(capsule_);
    capsuleLayout->setContentsMargins(10, 0, 12, 0);
    capsuleSpinner_ = new Spinner(14);
    capsuleLayout->addWidget(capsuleSpinner_);
    capsuleLayout->addStretch();
    layout->addWidget(capsule_);
    layout->addStretch(1);

    log_ = makeToolButton(QStringLiteral("terminal"), tr("Protokoll ein- oder ausblenden"));
    log_->setCheckable(true);
    layout->addWidget(log_);
    layout->addSpacing(6);

    minimise_ = makeWindowButton(QStringLiteral("minimise"), color::text, tr("Minimieren"));
    maximise_ = makeWindowButton(QStringLiteral("maximise"), color::text, tr("Maximieren"));
    close_ = makeWindowButton(QStringLiteral("x"), color::bad, tr("Schließen"));
    layout->addWidget(minimise_);
    layout->addWidget(maximise_);
    layout->addWidget(close_);

    connect(minimise_, &QPushButton::clicked, this, &TitleBar::minimiseRequested);
    connect(maximise_, &QPushButton::clicked, this, &TitleBar::maximiseToggled);
    connect(close_, &QPushButton::clicked, this, &TitleBar::closeRequested);
    connect(capsule_, &QPushButton::clicked, this, &TitleBar::connectionClicked);
    connect(log_, &QPushButton::clicked, this, &TitleBar::logToggled);
    connect(sidebar_, &QPushButton::clicked, this, &TitleBar::sidebarToggled);

    setConnection(Link::Offline, tr("Nicht verbunden"));
}

void TitleBar::setTitle(const QString& title) { title_->setText(title); }

void TitleBar::setConnection(Link state, const QString& text) {
    QColor tone = color::textFaint;
    QString iconName = QStringLiteral("unplug");
    switch (state) {
        case Link::Offline: break;
        case Link::Connecting: tone = color::warn; iconName.clear(); break;
        case Link::Online: tone = color::good; iconName = QStringLiteral("plug"); break;
        case Link::Error: tone = color::bad; break;
    }
    capsuleSpinner_->setRunning(state == Link::Connecting);
    capsule_->setText(QStringLiteral("    ") + text);
    capsule_->setIcon(iconName.isEmpty() ? QIcon() : icons::icon(iconName, tone, tone, 14));
    capsule_->setIconSize(QSize(14, 14));
    capsule_->setToolTip(tr("Verbindung öffnen"));
    capsule_->setStyleSheet(QStringLiteral("QPushButton { background: %1; border: 1px solid %2; "
                                           "border-radius: 15px; padding: 4px 14px 4px 10px; "
                                           "font-size: 13px; font-weight: 600; color: %3; }"
                                           "QPushButton:hover { border-color: %4; }")
                                .arg(rgba(tone, 0.10), rgba(tone, 0.35), color::text.name(),
                                     rgba(tone, 0.75)));
}

void TitleBar::setLogOpen(bool open) { log_->setChecked(open); }

void TitleBar::updateMaximiseState(bool maximised) {
    maximise_->setIcon(icons::icon(maximised ? QStringLiteral("restore") : QStringLiteral("maximise"),
                                   color::textMuted, color::text, 16));
    maximise_->setToolTip(maximised ? tr("Wiederherstellen") : tr("Maximieren"));
}

void TitleBar::mousePressEvent(QMouseEvent* event) {
    if (event->button() != Qt::LeftButton) return;
    // The window manager does the moving. Only that way do Aero Snap and
    // snapping to the screen edge keep working.
    if (QWindow* handle = window()->windowHandle()) handle->startSystemMove();
}

void TitleBar::mouseDoubleClickEvent(QMouseEvent* event) {
    if (event->button() == Qt::LeftButton) emit maximiseToggled();
}

void TitleBar::paintEvent(QPaintEvent*) {
    QPainter p(this);
    p.setRenderHint(QPainter::Antialiasing);
    // Top corners follow the rounded window frame.
    const bool square = window()->isMaximized() || window()->isFullScreen();
    const double r = square ? 0.0 : 11.0;
    QPainterPath shape;
    const QRectF box(rect());
    shape.moveTo(box.left(), box.bottom());
    shape.lineTo(box.left(), box.top() + r);
    shape.quadTo(box.left(), box.top(), box.left() + r, box.top());
    shape.lineTo(box.right() - r, box.top());
    shape.quadTo(box.right(), box.top(), box.right(), box.top() + r);
    shape.lineTo(box.right(), box.bottom());
    shape.closeSubpath();
    p.fillPath(shape, color::sidebar);
    p.setPen(color::border);
    p.drawLine(0, height() - 1, width(), height() - 1);
}

}  // namespace caruspex
