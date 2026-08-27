#include "app/GaugeWidget.h"

#include <QEasingCurve>
#include <QFont>
#include <QPainter>
#include <QPainterPath>
#include <QPropertyAnimation>
#include <QtMath>

#include "app/Theme.h"

namespace caruspex {
namespace {

// Skala von unten links nach unten rechts, also 240 Grad Oeffnung.
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
    QPainter p(this);
    p.setRenderHint(QPainter::Antialiasing);

    const double side = qMin(width(), height());
    const QRectF box((width() - side) / 2.0 + 12, (height() - side) / 2.0 + 12, side - 24,
                     side - 24);
    const double thickness = qMax(7.0, side * 0.075);

    // Hintergrundbogen
    QPen backPen(theme::color::border, thickness, Qt::SolidLine, Qt::RoundCap);
    p.setPen(backPen);
    p.drawArc(box, static_cast<int>(kStartAngle * 16), static_cast<int>(kSweep * 16));

    // Wertbogen. Solange nichts gemessen wird, bleibt er weg: bei Skalen, die
    // im Minus anfangen, saehe sonst schon der Startwert nach einem Messwert aus.
    const double span = maximum_ - minimum_;
    const double fraction = span > 0 ? qBound(0.0, (display_ - minimum_) / span, 1.0) : 0.0;
    if (active_) {
        QPen valuePen(arcColor(), thickness, Qt::SolidLine, Qt::RoundCap);
        p.setPen(valuePen);
        p.drawArc(box, static_cast<int>(kStartAngle * 16),
                  static_cast<int>(kSweep * fraction * 16));
    }

    // Zeigerpunkt am Bogenende
    if (active_) {
        const double angle = qDegreesToRadians(kStartAngle + kSweep * fraction);
        const QPointF centre = box.center();
        const double radius = box.width() / 2.0;
        const QPointF tip(centre.x() + std::cos(angle) * radius,
                          centre.y() - std::sin(angle) * radius);
        // Leuchtpunkt am Bogenende: aussen ein weicher Hof, innen hell.
        const QColor lit = arcColor();
        QColor halo = lit;
        halo.setAlpha(70);
        p.setPen(Qt::NoPen);
        p.setBrush(halo);
        p.drawEllipse(tip, thickness * 0.95, thickness * 0.95);
        p.setBrush(lit);
        p.drawEllipse(tip, thickness * 0.42, thickness * 0.42);
    }

    // Zahl mittig, Einheit direkt darunter. Getrennte Rechtecke, damit sich
    // beides nicht ueberlagert.
    const QPointF centre = box.center();
    const double valueHeight = side * 0.26;
    const QRectF valueBox(box.left(), centre.y() - valueHeight * 0.78, box.width(),
                          valueHeight);
    const QRectF unitBox(box.left(), valueBox.bottom() + 1, box.width(), side * 0.13);

    QFont valueFont = font();
    valueFont.setPointSizeF(qMax(13.0, side * 0.150));
    valueFont.setBold(true);
    p.setFont(valueFont);
    p.setPen(active_ ? theme::color::text : theme::color::textMuted);
    p.drawText(valueBox, Qt::AlignCenter,
               active_ ? QString::number(display_, 'f', decimals_) : QStringLiteral("--"));

    QFont unitFont = font();
    unitFont.setPointSizeF(qMax(8.0, side * 0.066));
    p.setFont(unitFont);
    p.setPen(theme::color::textMuted);
    p.drawText(unitBox, Qt::AlignHCenter | Qt::AlignTop, unit_);

    // Titel
    QFont titleFont = font();
    titleFont.setPointSizeF(qMax(8.5, side * 0.072));
    p.setFont(titleFont);
    p.setPen(theme::color::textMuted);
    p.drawText(QRectF(0, height() - 22, width(), 20), Qt::AlignCenter, title_);
}

}  // namespace caruspex
