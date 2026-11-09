#include "app/VehicleDiagram.h"

#include <QLinearGradient>
#include <QMouseEvent>
#include <QPainter>
#include <QRadialGradient>
#include <QTimer>
#include <QtMath>

#include <algorithm>

#include "app/Theme.h"

namespace caruspex {

using namespace theme;

namespace {

constexpr double kFieldW = 1000.0;
constexpr double kFieldH = 400.0;

// Point on a straight line between a and b at height y.
double xAtY(QPointF a, QPointF b, double y) {
    if (qFuzzyCompare(a.y(), b.y())) return a.x();
    return a.x() + (b.x() - a.x()) * (y - a.y()) / (b.y() - a.y());
}

QPainterPath roundedRect(double x, double y, double w, double h, double r) {
    QPainterPath path;
    path.addRoundedRect(QRectF(x, y, w, h), r, r);
    return path;
}

}  // namespace

VehicleDiagram::VehicleDiagram(QWidget* parent) : QWidget(parent) {
    setMouseTracking(true);
    QSizePolicy policy(QSizePolicy::Expanding, QSizePolicy::Preferred);
    policy.setHeightForWidth(true);
    setSizePolicy(policy);
    animation_ = new QTimer(this);
    animation_->setInterval(33);
    connect(animation_, &QTimer::timeout, this, [this] {
        if (!isVisible()) return;
        pulse_ += 0.09;
        if (pulse_ > 2.0 * M_PI) pulse_ -= 2.0 * M_PI;
        sweep_ += scanning_ ? 0.022 : 0.006;
        if (sweep_ > 1.4) sweep_ = -0.4;
        update();
    });
    animation_->start();
    shape_ = shapeFor(style_);
    rebuild();
}

void VehicleDiagram::setBodyStyle(BodyStyle style) {
    if (style == style_) return;
    style_ = style;
    shape_ = shapeFor(style);
    rebuild();
    update();
}

void VehicleDiagram::setPaint(const QColor& colour) {
    paint_ = colour;
    update();
}

void VehicleDiagram::setAnatomyVisible(bool visible) {
    anatomy_ = visible;
    update();
}

void VehicleDiagram::setScanning(bool scanning) {
    scanning_ = scanning;
    if (scanning) sweep_ = -0.3;
    update();
}

void VehicleDiagram::setAffected(const QSet<int>& zones) {
    affected_ = zones;
    update();
}

void VehicleDiagram::setFocus(CarZone zone) {
    focus_ = zone;
    update();
}

void VehicleDiagram::clear() {
    affected_.clear();
    focus_ = CarZone::Unknown;
    update();
}

// --- Geometry -----------------------------------------------------------------

VehicleDiagram::Shape VehicleDiagram::shapeFor(BodyStyle style) {
    // Real proportions in metres of a typical car of each kind, measured from
    // the front bumper and from the road. Heights become y values below.
    struct Metres {
        double length, wheelbase, frontOverhang, wheelRadius;
        double sill, bumper, nose, hood, belt, roof;
        double aBase, roofFront, roofRear, cBase, deck, tail;
        bool notch;
        int pillars;
    };
    Metres m{};
    switch (style) {
        case BodyStyle::Hatchback:  // Golf class
            m = {4.26, 2.64, 0.86, 0.32, 0.22, 0.17, 0.74, 0.96, 0.99, 1.45,
                 1.24, 2.02, 3.42, 4.18, 1.02, 0.86, false, 1};
            break;
        case BodyStyle::Sedan:  // A4 class
            m = {4.70, 2.81, 0.93, 0.32, 0.22, 0.17, 0.72, 0.95, 0.99, 1.43,
                 1.38, 2.20, 3.50, 4.05, 1.02, 0.86, true, 1};
            break;
        case BodyStyle::Estate:
            m = {4.72, 2.81, 0.93, 0.32, 0.22, 0.17, 0.72, 0.95, 0.99, 1.45,
                 1.38, 2.20, 4.38, 4.62, 1.22, 1.00, false, 2};
            break;
        case BodyStyle::Suv:  // Tiguan class
            m = {4.50, 2.68, 0.92, 0.36, 0.38, 0.30, 0.92, 1.12, 1.16, 1.67,
                 1.40, 2.15, 4.08, 4.40, 1.16, 1.00, false, 2};
            break;
        case BodyStyle::Van:  // Transporter class
            m = {4.90, 3.00, 0.98, 0.34, 0.38, 0.30, 0.95, 1.10, 1.18, 1.97,
                 0.78, 1.30, 4.82, 4.87, 1.86, 1.76, false, 3};
            break;
        case BodyStyle::Coupe:  // TT class
            m = {4.18, 2.50, 0.85, 0.33, 0.20, 0.15, 0.70, 0.92, 0.97, 1.35,
                 1.50, 2.22, 2.80, 3.86, 0.98, 0.85, true, 1};
            break;
        case BodyStyle::Pickup:  // Hilux class
            m = {5.32, 3.09, 0.92, 0.38, 0.42, 0.36, 1.05, 1.25, 1.30, 1.80,
                 1.55, 2.20, 3.28, 3.38, 1.30, 1.22, true, 1};
            break;
        case BodyStyle::Microcar:  // fortwo class
            m = {2.70, 1.87, 0.45, 0.30, 0.24, 0.18, 0.72, 0.95, 1.00, 1.55,
                 0.75, 1.25, 2.38, 2.62, 1.10, 0.95, false, 0};
            break;
    }

    // As large as fits: at most 900 long and 330 high, never above 200 per metre.
    const double u = std::min({900.0 / m.length, 330.0 / m.roof, 200.0});
    const double ground = 380.0;
    const double front = (kFieldW - m.length * u) / 2.0;
    auto x = [&](double metres) { return front + metres * u; };
    auto y = [&](double metres) { return ground - metres * u; };

    Shape s{};
    s.front = front;
    s.rear = x(m.length);
    s.wheelF = x(m.frontOverhang);
    s.wheelR = x(m.frontOverhang + m.wheelbase);
    s.wheelRad = m.wheelRadius * u;
    s.ground = ground;
    s.sill = y(m.sill);
    s.bumper = y(m.bumper);
    s.nose = y(m.nose);
    s.hood = y(m.hood);
    s.belt = y(m.belt);
    s.roof = y(m.roof);
    s.aBase = x(m.aBase);
    s.roofFront = x(m.roofFront);
    s.roofRear = x(m.roofRear);
    s.cBase = x(m.cBase);
    s.deck = y(m.deck);
    s.tail = y(m.tail);
    s.notch = m.notch;
    s.pillars = m.pillars;
    return s;
}

void VehicleDiagram::rebuild() {
    const Shape& s = shape_;
    const double wheelY = s.ground - s.wheelRad;
    const double archR = s.wheelRad + 9.0;

    // --- Body outline, clockwise from the bottom of the front bumper ---------
    QPainterPath b;
    b.moveTo(s.front + 22, s.bumper);
    b.cubicTo(s.front + 4, s.bumper - 4, s.front, s.nose + 52, s.front, s.nose + 26);
    b.cubicTo(s.front, s.nose + 6, s.front + 12, s.nose - 6, s.front + 46, s.nose - 10);
    // Hood, slightly domed, up to the base of the windscreen.
    b.cubicTo(s.front + (s.aBase - s.front) * 0.45, s.hood - 6,
              s.aBase - 40, s.hood - 2, s.aBase, s.hood);
    // Windscreen
    b.cubicTo(s.aBase + (s.roofFront - s.aBase) * 0.55, s.hood - (s.hood - s.roof) * 0.62,
              s.roofFront - 30, s.roof + 2, s.roofFront, s.roof);
    // Roof
    b.cubicTo(s.roofFront + (s.roofRear - s.roofFront) * 0.35, s.roof - 7,
              s.roofRear - (s.roofRear - s.roofFront) * 0.25, s.roof - 4, s.roofRear, s.roof + 4);
    if (s.notch) {
        // Rear window down to the boot lid, then the boot.
        b.cubicTo(s.roofRear + (s.cBase - s.roofRear) * 0.45, s.roof + 18,
                  s.cBase - 22, s.deck - 8, s.cBase, s.deck);
        b.lineTo(s.rear - 28, s.deck + 3);
        b.cubicTo(s.rear - 8, s.deck + 5, s.rear, s.deck + 14, s.rear, s.tail);
    } else {
        // Hatch or estate: one sweep from the roof to the tail.
        const double run = s.cBase - s.roofRear;
        b.cubicTo(s.roofRear + run * 0.45, s.roof + 4 + (s.deck - s.roof) * 0.10,
                  s.cBase + 6 - run * 0.08, s.deck - (s.deck - s.roof) * 0.30, s.cBase + 6, s.deck);
        b.cubicTo(s.cBase + 8, s.deck + 6, s.rear, s.deck + 2, s.rear, s.tail);
    }
    b.lineTo(s.rear, s.bumper - 28);
    b.cubicTo(s.rear, s.bumper - 6, s.rear - 6, s.bumper, s.rear - 24, s.bumper);

    // Lower edge with the two wheel arches.
    auto arch = [&](double cx) {
        const double dy = qBound(-archR + 1, s.sill - wheelY, archR - 1);
        const double a = qRadiansToDegrees(std::asin(dy / archR));
        const double dx = std::sqrt(archR * archR - dy * dy);
        b.lineTo(cx + dx, s.sill);
        b.arcTo(QRectF(cx - archR, wheelY - archR, archR * 2, archR * 2), -a, 180 + 2 * a);
    };
    b.lineTo(s.rear - 40, s.sill);
    arch(s.wheelR);
    arch(s.wheelF);
    b.lineTo(s.front + 40, s.sill);
    b.cubicTo(s.front + 30, s.sill, s.front + 26, s.bumper, s.front + 22, s.bumper);
    b.closeSubpath();
    body_ = b;

    // --- Side windows -----------------------------------------------------------
    const QPointF wsBottom(s.aBase, s.hood);
    const QPointF wsTop(s.roofFront, s.roof);
    const double glassBottom = s.belt - 3;
    const double glassTop = s.roof + 13;
    const double xFrontBottom = xAtY(wsBottom, wsTop, glassBottom) + 16;
    const double xFrontTop = xAtY(wsBottom, wsTop, glassTop) + 14;
    QPointF rwTop(s.roofRear, s.roof + 4);
    QPointF rwBottom = s.notch ? QPointF(s.cBase, s.deck) : QPointF(s.cBase + 6, s.deck);
    double xRearBottom = xAtY(rwTop, rwBottom, glassBottom) - 18;
    double xRearTop = xAtY(rwTop, rwBottom, glassTop) - 12;
    if (s.pillars >= 2) {  // estate, SUV, van: the glass runs almost to the back
        xRearBottom = std::min(xRearBottom, s.cBase - 18);
    }
    QPainterPath g;
    g.moveTo(xFrontBottom, glassBottom);
    g.lineTo(xFrontTop, glassTop);
    g.cubicTo(xFrontTop + 40, glassTop - 4, xRearTop - 40, glassTop - 2, xRearTop, glassTop + 2);
    g.lineTo(xRearBottom, glassBottom);
    g.closeSubpath();

    // Pillars are cut out of the glass.
    QPainterPath cuts;
    const double span = xRearBottom - xFrontBottom;
    for (int i = 1; i <= s.pillars; ++i) {
        const double x = xFrontBottom + span * (double(i) / (s.pillars + 1)) * (s.pillars == 1 ? 0.95 : 1.0);
        cuts.addRect(QRectF(x - 7, glassTop - 10, 14, glassBottom - glassTop + 20));
    }
    glass_ = g.subtracted(cuts);

    // --- Door shut lines ----------------------------------------------------------
    doorLines_.clear();
    auto vertical = [&](double xTop, double xBottom) {
        QPainterPath line;
        line.moveTo(xTop, glassTop - 6);
        line.cubicTo(xTop, s.belt + 20, xBottom, s.belt + 40, xBottom, s.sill - 2);
        doorLines_.push_back(line);
    };
    const double frontDoor = s.wheelF + archR + 14;
    vertical(xFrontTop - 6, frontDoor);
    if (s.pillars >= 1) {
        const double xb = xFrontBottom + span * (1.0 / (s.pillars + 1)) * (s.pillars == 1 ? 0.95 : 1.0);
        vertical(xb, xb + 4);
        if (style_ != BodyStyle::Coupe && style_ != BodyStyle::Pickup) {
            const double rearDoor = std::min(xRearBottom - 4, s.wheelR - archR - 8);
            vertical(rearDoor + 6, rearDoor);
        }
    }

    // --- Parts behind the faults -----------------------------------------------------
    parts_.clear();
    const double bayLeft = s.front + 38;
    const double bayRight = s.aBase - 18;
    const double bayW = bayRight - bayLeft;
    const double bayTop = (s.nose + s.hood) / 2.0 + 10;
    const double floorY = s.sill + 2;

    const QRectF engine(bayLeft + bayW * 0.30, bayTop + 26, bayW * 0.46, (s.sill - 14) - (bayTop + 26));

    auto add = [&](CarZone zone, QPainterPath path, QPointF anchor, const char* label) {
        parts_.push_back({zone, std::move(path), anchor, label});
    };

    // Radiator right behind the grille.
    add(CarZone::Cooling, roundedRect(s.front + 16, s.nose + 6, 13, s.sill - s.nose - 22, 3),
        QPointF(s.front + 22, s.nose + 30), QT_TR_NOOP("Kühlung"));
    // Air box at the front top of the bay.
    add(CarZone::Intake, roundedRect(bayLeft + 4, bayTop, bayW * 0.30, 24, 7),
        QPointF(bayLeft + 4 + bayW * 0.15, bayTop + 12), QT_TR_NOOP("Ansaugung"));
    // Throttle body where the intake meets the engine.
    {
        QPainterPath t;
        t.addEllipse(QPointF(engine.left() - 4, bayTop + 34), 10, 10);
        add(CarZone::Throttle, t, QPointF(engine.left() - 4, bayTop + 34),
            QT_TR_NOOP("Drosselklappe"));
    }
    // Engine block with its head.
    {
        QPainterPath e = roundedRect(engine.x(), engine.y(), engine.width(), engine.height(), 8);
        add(CarZone::Cylinders, e, engine.center(), QT_TR_NOOP("Motor"));
    }
    // Fuel rail across the top of the head.
    add(CarZone::Injection,
        roundedRect(engine.left() + 6, engine.top() - 9, engine.width() - 12, 6, 3),
        QPointF(engine.center().x(), engine.top() - 6), QT_TR_NOOP("Einspritzung"));
    // Ignition coils sit on the head.
    {
        QPainterPath coils;
        for (int i = 0; i < 4; ++i) {
            const double x = engine.left() + 10 + i * (engine.width() - 20) / 3.0 - 4;
            coils.addRoundedRect(QRectF(x, engine.top() + 6, 8, 14), 2, 2);
        }
        add(CarZone::Ignition, coils, QPointF(engine.right() - 14, engine.top() + 12),
            QT_TR_NOOP("Zündung"));
    }
    // Turbo at the lower front of the engine.
    {
        QPainterPath t;
        t.addEllipse(QPointF(engine.left() - 14, engine.bottom() - 18), 13, 13);
        add(CarZone::Turbo, t, QPointF(engine.left() - 14, engine.bottom() - 18),
            QT_TR_NOOP("Turbo"));
    }
    // Gearbox behind the engine.
    {
        QPainterPath gb;
        gb.moveTo(engine.right(), engine.top() + 22);
        gb.lineTo(engine.right() + 58, engine.top() + 34);
        gb.lineTo(engine.right() + 58, engine.bottom() - 4);
        gb.lineTo(engine.right(), engine.bottom());
        gb.closeSubpath();
        add(CarZone::Gearbox, gb, QPointF(engine.right() + 30, engine.center().y() + 8),
            QT_TR_NOOP("Getriebe"));
    }
    // Battery at the back of the bay.
    add(CarZone::Electrics, roundedRect(bayRight - 52, bayTop - 2, 40, 26, 4),
        QPointF(bayRight - 32, bayTop + 11), QT_TR_NOOP("Bordnetz"));
    // Engine control unit behind the dashboard.
    add(CarZone::Ecu, roundedRect(s.aBase + 14, s.belt + 14, 34, 20, 4),
        QPointF(s.aBase + 31, s.belt + 24), QT_TR_NOOP("Steuergerät"));
    // Exhaust: from the engine down, under the floor, to the back.
    {
        QPainterPath ex;
        const double pipe = 5;
        const double catX = engine.right() + 78;
        const double mufflerX = s.wheelR + 30;
        ex.addRoundedRect(QRectF(engine.center().x() - 4, engine.bottom() - 2, 8, floorY - engine.bottom() + 4), 3, 3);
        ex.addRoundedRect(QRectF(engine.center().x(), floorY - pipe / 2, s.rear - 30 - engine.center().x(), pipe), 2, 2);
        ex.addRoundedRect(QRectF(catX, floorY - 10, 58, 20), 8, 8);
        ex.addRoundedRect(QRectF(mufflerX, floorY - 12, std::max(30.0, s.rear - 60 - mufflerX), 22), 9, 9);
        add(CarZone::Exhaust, ex, QPointF(catX + 29, floorY), QT_TR_NOOP("Abgas"));
    }
    // Fuel tank under the rear seat.
    add(CarZone::FuelTank,
        roundedRect(s.wheelR - archR - 150, s.sill - 34, 128, 30, 9),
        QPointF(s.wheelR - archR - 86, s.sill - 19), QT_TR_NOOP("Tank"));
    // Cabin: dashboard and the two seats.
    {
        QPainterPath cabin;
        const double seatY = s.belt + 18;
        auto seat = [&](double x) {
            QPainterPath back;
            back.moveTo(x, s.sill - 22);
            back.cubicTo(x - 6, seatY + 30, x - 12, seatY, x - 4, seatY - 22);
            back.lineTo(x + 10, seatY - 20);
            back.cubicTo(x + 6, seatY + 10, x + 12, seatY + 34, x + 16, s.sill - 40);
            back.lineTo(x + 70, s.sill - 40);
            back.lineTo(x + 70, s.sill - 22);
            back.closeSubpath();
            cabin.addPath(back);
        };
        const double frontSeat = s.roofFront + (s.roofRear - s.roofFront) * 0.30;
        seat(frontSeat);
        if (style_ != BodyStyle::Coupe && style_ != BodyStyle::Microcar && style_ != BodyStyle::Pickup) {
            seat(std::min(s.wheelR - archR - 70, frontSeat + 210));
        }
        QPainterPath wheel;
        wheel.addEllipse(QPointF(s.aBase + 70, s.belt + 2), 6, 16);
        cabin.addPath(wheel);
        add(CarZone::Cabin, cabin, QPointF(frontSeat + 20, s.belt + 30), QT_TR_NOOP("Innenraum"));
    }
    // Data bus: a line from the front to the back through the cabin.
    {
        QPainterPath bus;
        bus.moveTo(bayRight - 30, bayTop + 26);
        bus.cubicTo(s.aBase + 40, s.belt + 60, s.roofRear - 60, s.sill - 14, s.rear - 60, s.tail + 20);
        QPainterPathStroker stroker;
        stroker.setWidth(4);
        add(CarZone::Network, stroker.createStroke(bus),
            QPointF((s.aBase + s.roofRear) / 2.0, s.sill - 24), QT_TR_NOOP("Datenbus"));
    }
    // Wheels and brakes
    {
        QPainterPath w;
        w.addEllipse(QPointF(s.wheelF, wheelY), s.wheelRad * 0.45, s.wheelRad * 0.45);
        w.addEllipse(QPointF(s.wheelR, wheelY), s.wheelRad * 0.45, s.wheelRad * 0.45);
        add(CarZone::Wheels, w, QPointF(s.wheelF, wheelY), QT_TR_NOOP("Räder und ABS"));
    }
}

QTransform VehicleDiagram::fieldToWidget() const {
    const double scale = std::min(width() / kFieldW, height() / kFieldH);
    const double dx = (width() - kFieldW * scale) / 2.0;
    const double dy = (height() - kFieldH * scale) / 2.0;
    QTransform t;
    t.translate(dx, dy);
    t.scale(scale, scale);
    return t;
}

CarZone VehicleDiagram::zoneAt(const QPointF& widgetPos) const {
    if (!anatomy_) return CarZone::Unknown;
    bool invertible = false;
    const QPointF f = fieldToWidget().inverted(&invertible).map(widgetPos);
    if (!invertible) return CarZone::Unknown;
    // Small parts first so they win over the big ones behind them.
    for (int pass = 0; pass < 2; ++pass) {
        for (const Part& part : parts_) {
            const bool small = part.path.boundingRect().width() < 60;
            if ((pass == 0) != small) continue;
            QPainterPathStroker grow;
            grow.setWidth(10);
            if (part.path.contains(f) || grow.createStroke(part.path).contains(f)) return part.zone;
        }
    }
    return CarZone::Unknown;
}

// --- Painting -------------------------------------------------------------------

void VehicleDiagram::drawGround(QPainter& p) const {
    const Shape& s = shape_;
    const double cx = (s.front + s.rear) / 2.0;
    const double w = (s.rear - s.front) * 0.62;
    QRadialGradient shadow(QPointF(cx, s.ground), w);
    shadow.setColorAt(0.0, QColor(0, 0, 0, 170));
    shadow.setColorAt(0.6, QColor(0, 0, 0, 60));
    shadow.setColorAt(1.0, QColor(0, 0, 0, 0));
    p.save();
    p.translate(cx, s.ground);
    p.scale(1.0, 0.07);
    p.translate(-cx, -s.ground);
    p.setPen(Qt::NoPen);
    p.setBrush(shadow);
    p.drawEllipse(QPointF(cx, s.ground), w, w);
    p.restore();

    // A faint floor line that fades out to the sides.
    QLinearGradient floor(0, 0, kFieldW, 0);
    floor.setColorAt(0.0, QColor(255, 255, 255, 0));
    floor.setColorAt(0.5, QColor(255, 255, 255, 26));
    floor.setColorAt(1.0, QColor(255, 255, 255, 0));
    p.setPen(QPen(QBrush(floor), 1.2));
    p.drawLine(QPointF(10, s.ground + 1), QPointF(kFieldW - 10, s.ground + 1));
}

void VehicleDiagram::drawWheel(QPainter& p, double cx) const {
    const Shape& s = shape_;
    const double r = s.wheelRad;
    const QPointF c(cx, s.ground - r);

    // Dark arch behind the wheel
    p.setPen(Qt::NoPen);
    p.setBrush(QColor(6, 7, 10));
    p.drawEllipse(c, r + 6, r + 6);

    // Tyre
    QRadialGradient tyre(c, r);
    tyre.setColorAt(0.70, QColor(30, 33, 40));
    tyre.setColorAt(0.86, QColor(17, 19, 24));
    tyre.setColorAt(1.0, QColor(10, 11, 14));
    p.setBrush(tyre);
    p.drawEllipse(c, r, r);

    // Rim
    const double rim = r * 0.68;
    QRadialGradient metal(QPointF(c.x() - rim * 0.3, c.y() - rim * 0.4), rim * 1.4);
    metal.setColorAt(0.0, QColor(214, 220, 232));
    metal.setColorAt(0.5, QColor(132, 140, 158));
    metal.setColorAt(1.0, QColor(62, 68, 82));
    p.setBrush(metal);
    p.drawEllipse(c, rim, rim);

    // Brake disc and calliper visible between the spokes
    p.setBrush(QColor(40, 44, 54));
    p.drawEllipse(c, rim * 0.86, rim * 0.86);
    p.save();
    p.translate(c);
    QLinearGradient calliper(0, -rim * 0.8, 0, -rim * 0.3);
    calliper.setColorAt(0, color::accent.lighter(110));
    calliper.setColorAt(1, color::accentDim);
    p.setBrush(calliper);
    p.rotate(-35);
    p.drawRoundedRect(QRectF(-rim * 0.22, -rim * 0.82, rim * 0.44, rim * 0.38), 4, 4);
    p.restore();

    // Five spokes
    p.save();
    p.translate(c);
    for (int i = 0; i < 5; ++i) {
        p.save();
        p.rotate(i * 72.0 + 18.0);
        QPainterPath spoke;
        spoke.moveTo(-rim * 0.10, -rim * 0.18);
        spoke.lineTo(-rim * 0.20, -rim * 0.95);
        spoke.lineTo(rim * 0.20, -rim * 0.95);
        spoke.lineTo(rim * 0.10, -rim * 0.18);
        spoke.closeSubpath();
        QLinearGradient sg(-rim * 0.2, 0, rim * 0.2, 0);
        sg.setColorAt(0, QColor(170, 178, 194));
        sg.setColorAt(1, QColor(96, 104, 120));
        p.setBrush(sg);
        p.drawPath(spoke);
        p.restore();
    }
    p.restore();

    // Rim lip and hub
    p.setBrush(Qt::NoBrush);
    p.setPen(QPen(QColor(190, 198, 214, 160), 1.6));
    p.drawEllipse(c, rim, rim);
    p.setPen(Qt::NoPen);
    QRadialGradient hub(c, rim * 0.22);
    hub.setColorAt(0, QColor(220, 226, 236));
    hub.setColorAt(1, QColor(80, 88, 104));
    p.setBrush(hub);
    p.drawEllipse(c, rim * 0.2, rim * 0.2);
}

void VehicleDiagram::drawBody(QPainter& p) const {
    const Shape& s = shape_;
    const double alpha = anatomy_ ? 0.32 : 1.0;

    // Paint: light from above, a bright shoulder line, dark towards the sills.
    QLinearGradient lacquer(0, s.roof, 0, s.bumper);
    auto c = [&](const QColor& base, double a = 1.0) {
        QColor out = base;
        out.setAlphaF(static_cast<float>(alpha * a));
        return out;
    };
    lacquer.setColorAt(0.00, c(paint_.lighter(165)));
    lacquer.setColorAt(0.30, c(paint_.lighter(125)));
    lacquer.setColorAt(0.44, c(paint_.lighter(150)));
    lacquer.setColorAt(0.50, c(paint_));
    lacquer.setColorAt(0.80, c(paint_.darker(160)));
    lacquer.setColorAt(1.00, c(paint_.darker(260)));
    p.setPen(Qt::NoPen);
    p.setBrush(lacquer);
    p.drawPath(body_);

    p.save();
    p.setClipPath(body_);
    // Dark lower band (sills, bumpers)
    p.setBrush(c(QColor(14, 16, 22), 0.85));
    p.drawRect(QRectF(0, s.sill - 12, kFieldW, 80));
    // Shoulder highlight
    QLinearGradient shine(0, s.belt + 6, 0, s.belt + 22);
    shine.setColorAt(0, c(QColor(255, 255, 255), 0.0));
    shine.setColorAt(0.5, c(QColor(255, 255, 255), 0.16));
    shine.setColorAt(1, c(QColor(255, 255, 255), 0.0));
    p.setBrush(shine);
    p.drawRect(QRectF(0, s.belt + 6, kFieldW, 16));
    p.restore();

    // Windows
    QLinearGradient tint(0, s.roof, 0, s.belt);
    tint.setColorAt(0, c(QColor(54, 64, 88), anatomy_ ? 0.6 : 1.0));
    tint.setColorAt(1, c(QColor(14, 17, 26), anatomy_ ? 0.6 : 1.0));
    p.setBrush(tint);
    p.drawPath(glass_);
    if (!anatomy_) {
        // A diagonal reflection on the glass
        p.save();
        p.setClipPath(glass_);
        QLinearGradient refl(s.roofFront, s.roof, s.roofFront + 140, s.belt);
        refl.setColorAt(0.0, QColor(255, 255, 255, 0));
        refl.setColorAt(0.45, QColor(255, 255, 255, 34));
        refl.setColorAt(0.55, QColor(255, 255, 255, 0));
        p.setBrush(refl);
        p.drawRect(QRectF(0, 0, kFieldW, kFieldH));
        p.restore();
    }

    // Door lines and handles
    p.setBrush(Qt::NoBrush);
    p.setPen(QPen(c(QColor(0, 0, 0), 0.55), 1.4));
    for (const QPainterPath& line : doorLines_) p.drawPath(line);
    p.setPen(Qt::NoPen);
    p.setBrush(c(QColor(10, 12, 16), 0.7));
    for (int i = 0; i < doorLines_.size() - 1 && i < 2; ++i) {
        const double x = doorLines_[i + 1].pointAtPercent(0.4).x() - 34;
        p.drawRoundedRect(QRectF(x, s.belt + 18, 22, 5), 2.5, 2.5);
    }

    // Head light with a glow
    {
        QPainterPath lamp;
        lamp.moveTo(s.front + 10, s.nose + 6);
        lamp.cubicTo(s.front + 26, s.nose - 6, s.front + 60, s.nose - 8, s.front + 72, s.nose - 6);
        lamp.lineTo(s.front + 64, s.nose + 8);
        lamp.cubicTo(s.front + 44, s.nose + 12, s.front + 22, s.nose + 14, s.front + 10, s.nose + 6);
        QRadialGradient glow(QPointF(s.front + 20, s.nose + 2), 70);
        glow.setColorAt(0, QColor(150, 200, 255, int(90 * alpha)));
        glow.setColorAt(1, QColor(150, 200, 255, 0));
        p.setBrush(glow);
        p.drawEllipse(QPointF(s.front + 20, s.nose + 2), 70, 40);
        QLinearGradient lens(s.front + 10, 0, s.front + 72, 0);
        lens.setColorAt(0, QColor(235, 245, 255, int(255 * std::max(alpha, 0.6))));
        lens.setColorAt(1, QColor(120, 170, 255, int(200 * std::max(alpha, 0.6))));
        p.setBrush(lens);
        p.drawPath(lamp);
    }
    // Tail light
    {
        const double y = s.tail + 6;
        QRadialGradient glow(QPointF(s.rear - 6, y + 8), 50);
        glow.setColorAt(0, QColor(255, 70, 80, int(110 * alpha)));
        glow.setColorAt(1, QColor(255, 70, 80, 0));
        p.setBrush(glow);
        p.drawEllipse(QPointF(s.rear - 4, y + 8), 50, 26);
        QLinearGradient lens(0, y, 0, y + 16);
        lens.setColorAt(0, QColor(255, 120, 120, int(255 * std::max(alpha, 0.6))));
        lens.setColorAt(1, QColor(200, 30, 50, int(255 * std::max(alpha, 0.6))));
        p.setBrush(lens);
        p.drawRoundedRect(QRectF(s.rear - 34, y, 32, 15), 5, 5);
    }

    // Rim light along the top edge
    p.setBrush(Qt::NoBrush);
    QLinearGradient rimLight(s.front, 0, s.rear, 0);
    rimLight.setColorAt(0, QColor(255, 255, 255, 10));
    rimLight.setColorAt(0.45, QColor(255, 255, 255, anatomy_ ? 70 : 60));
    rimLight.setColorAt(1, QColor(255, 255, 255, 10));
    p.setPen(QPen(QBrush(rimLight), anatomy_ ? 1.6 : 1.2));
    p.drawPath(body_);
}

void VehicleDiagram::drawAnatomy(QPainter& p) const {
    const double wave = 0.5 + 0.5 * std::sin(pulse_);
    for (const Part& part : parts_) {
        const bool hit = affected_.contains(static_cast<int>(part.zone));
        const bool focused = part.zone == focus_;
        const bool hovered = part.zone == hover_;
        QColor line = color::accent;
        QColor fill = color::accent;
        if (hit) {
            line = color::bad;
            fill = color::bad;
            fill.setAlphaF(static_cast<float>(focused ? 0.30 + 0.35 * wave : 0.28));
            line.setAlphaF(1.0f);
        } else {
            fill.setAlphaF(hovered ? 0.22f : 0.08f);
            line.setAlphaF(hovered ? 0.95f : 0.45f);
        }
        if (hit && focused) {
            // A soft glow around the part in focus.
            QPainterPathStroker stroker;
            stroker.setWidth(10 + 8 * wave);
            stroker.setJoinStyle(Qt::RoundJoin);
            QColor halo = color::bad;
            halo.setAlphaF(static_cast<float>(0.12 + 0.12 * wave));
            p.setPen(Qt::NoPen);
            p.setBrush(halo);
            p.drawPath(stroker.createStroke(part.path));
        }
        p.setBrush(fill);
        p.setPen(QPen(line, hit ? 1.8 : 1.2));
        p.drawPath(part.path);
    }
}

void VehicleDiagram::drawLabels(QPainter& p, const QTransform& toWidget) const {
    // One chip per affected zone above the car, plus the hovered one, joined
    // to the part by a thin line. Chips are spread so they never overlap.
    struct Chip {
        const Part* part;
        double x;
        double w;
        bool hit;
    };
    QVector<Chip> chips;
    QFont f = sans(13, 600);
    const QFontMetricsF fm(f);
    for (const Part& part : parts_) {
        const bool hit = affected_.contains(static_cast<int>(part.zone));
        if (!hit && part.zone != hover_) continue;
        const QString text = tr(part.label);
        chips.push_back({&part, toWidget.map(part.anchor).x(), fm.horizontalAdvance(text) + 26, hit});
    }
    if (chips.isEmpty()) return;
    std::sort(chips.begin(), chips.end(), [](const Chip& a, const Chip& b) { return a.x < b.x; });

    // Spread left to right, then pull back inside the field on the right.
    double cursor = 8;
    for (Chip& c : chips) {
        c.x = std::max(c.x - c.w / 2.0, cursor);
        cursor = c.x + c.w + 8;
    }
    const double right = width() - 8;
    double overflow = cursor - 8 - right;
    for (int i = chips.size() - 1; i >= 0 && overflow > 0; --i) {
        const double room = i > 0 ? chips[i].x - (chips[i - 1].x + chips[i - 1].w + 8) : chips[i].x - 8;
        const double shift = std::min(overflow, std::max(0.0, room));
        for (int j = i; j < chips.size(); ++j) chips[j].x -= shift;
        overflow -= shift;
    }

    const double chipY = 6;
    const double chipH = 26;
    p.setFont(f);
    for (const Chip& c : chips) {
        const bool focused = c.part->zone == focus_;
        const QColor tone = c.hit ? color::bad : color::accent;
        const QPointF from(c.x + c.w / 2.0, chipY + chipH);
        const QPointF to = toWidget.map(c.part->anchor);
        QColor lineColour = tone;
        lineColour.setAlphaF(focused ? 0.9f : 0.55f);
        p.setPen(QPen(lineColour, 1.1, Qt::SolidLine));
        QPainterPath lead;
        lead.moveTo(from);
        lead.cubicTo(from.x(), from.y() + 40, to.x(), to.y() - 60, to.x(), to.y());
        p.setBrush(Qt::NoBrush);
        p.drawPath(lead);
        p.setPen(Qt::NoPen);
        p.setBrush(tone);
        p.drawEllipse(to, 3.2, 3.2);

        QRectF box(c.x, chipY, c.w, chipH);
        QColor bg = color::surface;
        bg.setAlpha(240);
        p.setBrush(bg);
        QColor border = tone;
        border.setAlphaF(focused ? 1.0f : 0.6f);
        p.setPen(QPen(border, focused ? 1.6 : 1.0));
        p.drawRoundedRect(box, 13, 13);
        p.setPen(Qt::NoPen);
        p.setBrush(tone);
        p.drawEllipse(QPointF(box.left() + 12, box.center().y()), 3.5, 3.5);
        p.setPen(color::text);
        p.drawText(box.adjusted(20, 0, -6, 0), Qt::AlignVCenter | Qt::AlignLeft, tr(c.part->label));
    }
}

void VehicleDiagram::drawSweep(QPainter& p) const {
    if (sweep_ < -0.15 || sweep_ > 1.15) return;
    const Shape& s = shape_;
    const double x = s.front + (s.rear - s.front) * sweep_;
    p.save();
    p.setClipPath(body_);
    QLinearGradient band(x - 60, 0, x + 60, 0);
    const int strength = scanning_ ? 70 : 26;
    band.setColorAt(0.0, QColor(255, 255, 255, 0));
    band.setColorAt(0.5, scanning_ ? QColor(143, 140, 255, strength) : QColor(255, 255, 255, strength));
    band.setColorAt(1.0, QColor(255, 255, 255, 0));
    p.setPen(Qt::NoPen);
    p.setBrush(band);
    p.drawRect(QRectF(x - 60, 0, 120, kFieldH));
    p.restore();
}

void VehicleDiagram::paintEvent(QPaintEvent*) {
    QPainter p(this);
    p.setRenderHint(QPainter::Antialiasing);
    p.setRenderHint(QPainter::SmoothPixmapTransform);
    p.setTransform(fieldToWidget());

    drawGround(p);
    drawWheel(p, shape_.wheelF);
    drawWheel(p, shape_.wheelR);
    drawBody(p);
    if (anatomy_) drawAnatomy(p);
    drawSweep(p);
    if (anatomy_) {
        const QTransform toWidget = p.transform();
        p.resetTransform();
        drawLabels(p, toWidget);
    }
}

void VehicleDiagram::mousePressEvent(QMouseEvent* event) {
    const CarZone zone = zoneAt(event->position());
    if (zone != CarZone::Unknown) emit zoneClicked(static_cast<int>(zone));
}

void VehicleDiagram::mouseMoveEvent(QMouseEvent* event) {
    const CarZone zone = zoneAt(event->position());
    if (zone != hover_) {
        hover_ = zone;
        setCursor(zone == CarZone::Unknown ? Qt::ArrowCursor : Qt::PointingHandCursor);
        update();
    }
}

void VehicleDiagram::leaveEvent(QEvent*) {
    if (hover_ != CarZone::Unknown) {
        hover_ = CarZone::Unknown;
        update();
    }
}

}  // namespace caruspex
