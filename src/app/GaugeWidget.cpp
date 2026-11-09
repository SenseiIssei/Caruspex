#include "app/GaugeWidget.h"

#include <QConicalGradient>
#include <QEasingCurve>
#include <QFont>
#include <QPainter>
#include <QPainterPath>
#include <QPropertyAnimation>
#include <QtMath>

#include "app/Theme.h"

namespace caruspex {
namespace {

// Scale from bottom left to bottom right, i.e. a 240 degree opening.
constexpr double kStartAngle = 210.0;
constexpr double kSweep = -240.0;

}  // namespace

GaugeWidget::GaugeWidget(QWidget* parent) : QWidget(parent) {
    animation_ = new QPropertyAnimation(this, "displayValue", this);
    animation_->setDuration(450);
    animation_->setEasingCurve(QEasingCurve::OutCubic);
}

void GaugeWidget::configure(const QString& title, const QString& unit, double minimum,
                            double maximum, int decimals) {
    title_ = title;
    unit_ = unit;
    minimum_ = minimum;
    maximum_ = maximum > minimum ? maximum : minimum + 1.0;
    decimals_ = decimals;
    update();
}

void GaugeWidget::setWarnRange(double from, double to) {
    warnFrom_ = from;
    warnTo_ = to;
    hasWarn_ = true;
}

void GaugeWidget::setValue(double value) {
    target_ = qBound(minimum_, value, maximum_);
    active_ = true;
    animation_->stop();
    animation_->setStartValue(display_);
    animation_->setEndValue(target_);
    animation_->start();
}

void GaugeWidget::setDisplayValue(double value) {
    display_ = value;
    update();
}

void GaugeWidget::setActive(bool active) {
    active_ = active;
    update();
}

QColor GaugeWidget::arcColor() const {
    if (!active_) return theme::color::textMuted;
    if (hasWarn_ && (target_ < warnFrom_ || target_ > warnTo_)) return theme::color::warn;
    return theme::color::accent;
}

void GaugeWidget::paintEvent(QPaintEvent*) {
    using namespace theme;
    QPainter p(this);
    p.setRenderHint(QPainter::Antialiasing);

    const double titleSpace = 26.0;
    const double side = qMin<double>(width(), height() - titleSpace);
    const double thickness = qMax(7.0, side * 0.07);
    const QRectF box((width() - side) / 2.0 + thickness, (height() - titleSpace - side) / 2.0 + thickness,
                     side - 2 * thickness, side - 2 * thickness);
    const QPointF centre = box.center();
    const double radius = box.width() / 2.0;

    // Ticks just inside the arc, every tenth a little longer.
    for (int i = 0; i <= 40; ++i) {
        const double a = qDegreesToRadians(kStartAngle + kSweep * i / 40.0);
        const bool major = i % 10 == 0;
        const double r1 = radius - thickness * (major ? 1.55 : 1.25);
        const double r2 = radius - thickness * 0.95;
        QColor tick = color::borderBright;
        tick.setAlphaF(major ? 0.9f : 0.5f);
        p.setPen(QPen(tick, major ? 1.4 : 1.0));
        p.drawLine(QPointF(centre.x() + std::cos(a) * r1, centre.y() - std::sin(a) * r1),
                   QPointF(centre.x() + std::cos(a) * r2, centre.y() - std::sin(a) * r2));
    }

    // Track
    p.setPen(QPen(QColor("#1d2230"), thickness, Qt::SolidLine, Qt::RoundCap));
    p.drawArc(box, static_cast<int>(kStartAngle * 16), static_cast<int>(kSweep * 16));

    // Value arc. It stays hidden while nothing is measured: on scales that
    // start below zero, the start value would otherwise already look like a
    // reading.
    const double span = maximum_ - minimum_;
    const double fraction = span > 0 ? qBound(0.0, (display_ - minimum_) / span, 1.0) : 0.0;
    const bool warning = active_ && hasWarn_ && (target_ < warnFrom_ || target_ > warnTo_);
    if (active_ && fraction > 0.001) {
        QConicalGradient g(centre, kStartAngle);
        if (warning) {
            g.setColorAt(0.0, color::warn.lighter(115));
            g.setColorAt(1.0, color::bad);
        } else {
            // Conical gradients run counter-clockwise, the arc clockwise:
            // so the start colour sits at 1.0.
            g.setColorAt(1.0, color::accent);
            g.setColorAt(1.0 - (-kSweep / 360.0), color::sky);
            g.setColorAt(0.0, color::sky);
        }
        p.setPen(QPen(QBrush(g), thickness, Qt::SolidLine, Qt::RoundCap));
        p.drawArc(box, static_cast<int>(kStartAngle * 16), static_cast<int>(kSweep * fraction * 16));

        const double angle = qDegreesToRadians(kStartAngle + kSweep * fraction);
        const QPointF tip(centre.x() + std::cos(angle) * radius, centre.y() - std::sin(angle) * radius);
        QColor halo = warning ? color::warn : color::sky;
        halo.setAlphaF(0.30f);
        p.setPen(Qt::NoPen);
        p.setBrush(halo);
        p.drawEllipse(tip, thickness * 1.05, thickness * 1.05);
        p.setBrush(QColor(255, 255, 255, 235));
        p.drawEllipse(tip, thickness * 0.30, thickness * 0.30);
    }

    // Number centred, unit directly below.
    const double valueHeight = side * 0.24;
    const QRectF valueBox(box.left(), centre.y() - valueHeight * 0.72, box.width(), valueHeight);
    const QRectF unitBox(box.left(), valueBox.bottom(), box.width(), side * 0.12);
    p.setFont(display(qMax(16, int(side * 0.17)), 700));
    p.setPen(active_ ? color::text : color::textFaint);
    p.drawText(valueBox, Qt::AlignCenter,
               active_ ? QString::number(display_, 'f', decimals_) : QStringLiteral("--"));
    p.setFont(sans(qMax(11, int(side * 0.07)), 500));
    p.setPen(color::textFaint);
    p.drawText(unitBox, Qt::AlignHCenter | Qt::AlignTop, unit_);

    // Title
    p.setFont(sans(13, 600));
    p.setPen(color::textMuted);
    p.drawText(QRectF(0, height() - titleSpace, width(), titleSpace), Qt::AlignCenter, title_);
}

}  // namespace caruspex
