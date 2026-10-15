#include "app/VehicleDiagram.h"

#include <QMouseEvent>
#include <QPainter>
#include <QPainterPath>
#include <QTimer>
#include <QtMath>

#include "app/Theme.h"

namespace caruspex {
namespace {

// The car is laid out in an imaginary field from 0 to 1 and stretched to the
// actual size when painting. That keeps the layout the same at every window
// size.
constexpr double kBodyTop = 0.36;
constexpr double kBodyBottom = 0.74;

}  // namespace

VehicleDiagram::VehicleDiagram(QWidget* parent) : QWidget(parent) {
    setMinimumHeight(230);
    animation_ = new QTimer(this);
    animation_->setInterval(33);
    connect(animation_, &QTimer::timeout, this, [this] {
        pulse_ += 0.055;
        if (pulse_ > 2.0 * M_PI) pulse_ -= 2.0 * M_PI;
        update();
    });
}

void VehicleDiagram::setPulse(double value) {
    pulse_ = value;
    update();
}

const std::vector<VehicleDiagram::Hotspot>& VehicleDiagram::hotspots() const {
    // Engine bay at the front left, cabin in the middle, tank and exhaust at
    // the back. The labels are translated when painting.
    static const std::vector<Hotspot> spots = {
        {CarZone::Intake,    {0.055, 0.30, 0.105, 0.115}, QT_TR_NOOP("Ansaug")},
        {CarZone::Throttle,  {0.170, 0.30, 0.105, 0.115}, QT_TR_NOOP("Drosselklappe")},
        {CarZone::Turbo,     {0.055, 0.575, 0.105, 0.11}, QT_TR_NOOP("Turbo")},
        {CarZone::Cylinders, {0.170, 0.435, 0.105, 0.115}, QT_TR_NOOP("Motor")},
        {CarZone::Ignition,  {0.170, 0.575, 0.105, 0.11}, QT_TR_NOOP("Zuendung")},
        {CarZone::Injection, {0.055, 0.435, 0.105, 0.115}, QT_TR_NOOP("Einspritzung")},
        {CarZone::Cooling,   {0.055, 0.70, 0.105, 0.10},  QT_TR_NOOP("Kuehlung")},
        {CarZone::Ecu,       {0.295, 0.30, 0.100, 0.115}, QT_TR_NOOP("Steuergeraet")},
        {CarZone::Electrics, {0.295, 0.435, 0.100, 0.115}, QT_TR_NOOP("Bordnetz")},
        {CarZone::Network,   {0.295, 0.575, 0.100, 0.11},  QT_TR_NOOP("Datenbus")},
        {CarZone::Cabin,     {0.420, 0.30, 0.150, 0.135},  QT_TR_NOOP("Innenraum")},
        {CarZone::Gearbox,   {0.420, 0.455, 0.150, 0.115}, QT_TR_NOOP("Getriebe")},
        {CarZone::FuelTank,  {0.595, 0.30, 0.130, 0.135},  QT_TR_NOOP("Tank")},
        {CarZone::Exhaust,   {0.595, 0.455, 0.130, 0.115}, QT_TR_NOOP("Abgas")},
        {CarZone::Wheels,    {0.755, 0.30, 0.130, 0.135},  QT_TR_NOOP("Raeder / ABS")},
    };
    return spots;
}

QRectF VehicleDiagram::scaled(const QRectF& unit) const {
    return QRectF(canvas_.left() + unit.left() * canvas_.width(),
                  canvas_.top() + unit.top() * canvas_.height(),
                  unit.width() * canvas_.width(), unit.height() * canvas_.height());
}

void VehicleDiagram::setAffected(const QSet<int>& zones) {
    affected_ = zones;
    if (!affected_.isEmpty()) {
        animation_->start();
    } else {
        animation_->stop();
    }
    update();
}

void VehicleDiagram::setFocus(CarZone zone) {
    focus_ = zone;
    update();
}

void VehicleDiagram::clear() {
    affected_.clear();
    focus_ = CarZone::Unknown;
    animation_->stop();
    update();
}

// Side view: body, windows, two wheels, exhaust.
void VehicleDiagram::drawCarBody(QPainter& painter) const {
    const double x = canvas_.left();
    const double y = canvas_.top();
    const double w = canvas_.width();
    const double h = canvas_.height();

    auto px = [&](double u) { return x + u * w; };
    auto py = [&](double v) { return y + v * h; };

    QPainterPath body;
    body.moveTo(px(0.03), py(kBodyBottom));
    body.lineTo(px(0.03), py(0.50));
    body.cubicTo(px(0.06), py(0.44), px(0.14), py(0.42), px(0.24), py(0.415));
    body.lineTo(px(0.34), py(kBodyTop - 0.10));
    body.cubicTo(px(0.40), py(0.17), px(0.56), py(0.16), px(0.64), py(0.20));
    body.lineTo(px(0.74), py(0.415));
    body.cubicTo(px(0.86), py(0.425), px(0.94), py(0.45), px(0.965), py(0.52));
    body.lineTo(px(0.965), py(kBodyBottom));
    body.closeSubpath();

    QPen outline(theme::color::border, 2.0);
    painter.setPen(outline);
    painter.setBrush(QColor(30, 35, 46));
    painter.drawPath(body);

    // Windows
    QPainterPath glass;
    glass.moveTo(px(0.355), py(0.315));
    glass.lineTo(px(0.415), py(0.215));
    glass.lineTo(px(0.60), py(0.215));
    glass.lineTo(px(0.655), py(0.315));
    glass.closeSubpath();
    painter.setBrush(QColor(38, 46, 60));
    painter.drawPath(glass);

    // Wheels
    painter.setBrush(QColor(22, 25, 32));
    const double wheelR = h * 0.115;
    painter.drawEllipse(QPointF(px(0.215), py(kBodyBottom)), wheelR, wheelR);
    painter.drawEllipse(QPointF(px(0.805), py(kBodyBottom)), wheelR, wheelR);
    painter.setBrush(QColor(48, 55, 68));
    painter.drawEllipse(QPointF(px(0.215), py(kBodyBottom)), wheelR * 0.45, wheelR * 0.45);
    painter.drawEllipse(QPointF(px(0.805), py(kBodyBottom)), wheelR * 0.45, wheelR * 0.45);

    // Exhaust at the back
    painter.setBrush(QColor(60, 68, 82));
    painter.setPen(Qt::NoPen);
    painter.drawRoundedRect(QRectF(px(0.955), py(kBodyBottom - 0.055), w * 0.04, h * 0.035),
                            3, 3);

    // Mark the front so it is clear which end is the front
    painter.setPen(QPen(theme::color::textMuted, 1.0, Qt::DashLine));
    painter.drawLine(QPointF(px(0.03), py(0.86)), QPointF(px(0.20), py(0.86)));
    painter.setPen(theme::color::textMuted);
    QFont small = painter.font();
    small.setPointSizeF(8.0);
    painter.setFont(small);
    painter.drawText(QRectF(px(0.03), py(0.87), w * 0.20, h * 0.10), Qt::AlignLeft,
                     tr("vorne"));
}

void VehicleDiagram::paintEvent(QPaintEvent*) {
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing);

    const double margin = 12.0;
    canvas_ = QRectF(margin, margin, width() - 2 * margin, height() - 2 * margin);

    drawCarBody(painter);

    QFont label = font();
    label.setPointSizeF(8.5);
    painter.setFont(label);

    for (const Hotspot& spot : hotspots()) {
        const QRectF box = scaled(spot.box);
        const bool hit = affected_.contains(static_cast<int>(spot.zone));
        const bool focused = focus_ == spot.zone;

        QColor fill = QColor(26, 31, 40, 220);
        QColor border = theme::color::border;
        QColor text = theme::color::textMuted;

        if (hit) {
            const double wave = focused ? (0.5 + 0.5 * std::sin(pulse_)) : 0.35;
            fill = QColor(theme::color::bad.red(), theme::color::bad.green(),
                          theme::color::bad.blue(), static_cast<int>(50 + 110 * wave));
            border = theme::color::bad;
            text = theme::color::text;
        }
        if (focused) {
            border = theme::color::warn;
        }

        painter.setBrush(fill);
        painter.setPen(QPen(border, focused ? 2.4 : 1.2));
        painter.drawRoundedRect(box, 7, 7);

        painter.setPen(text);
        painter.drawText(box, Qt::AlignCenter | Qt::TextWordWrap, tr(spot.label));
    }
}

void VehicleDiagram::mousePressEvent(QMouseEvent* event) {
    for (const Hotspot& spot : hotspots()) {
        if (scaled(spot.box).contains(event->position())) {
            emit zoneClicked(static_cast<int>(spot.zone));
            return;
        }
    }
}

}  // namespace caruspex
