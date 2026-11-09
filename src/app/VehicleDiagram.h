// Side view of a car, drawn in code. Two uses:
//
//  - On the start page as a clean picture of the connected car, in the body
//    style the catalog knows for it.
//  - On the fault page in "see-through" mode: the body turns into glass and
//    the parts behind a fault (engine, intake, exhaust, tank, gearbox, ...)
//    light up where they sit in a real car.
//
// The car is laid out in a fixed 1000 by 400 field and scaled to the widget
// without distortion, so it looks right at every window size.
#pragma once

#include <QHash>
#include <QPainterPath>
#include <QSet>
#include <QWidget>

#include "data/DtcLocation.h"

class QTimer;

namespace caruspex {

enum class BodyStyle { Hatchback, Sedan, Estate, Suv, Van, Coupe, Pickup, Microcar };

class VehicleDiagram : public QWidget {
    Q_OBJECT

public:
    explicit VehicleDiagram(QWidget* parent = nullptr);

    void setBodyStyle(BodyStyle style);
    BodyStyle bodyStyle() const { return style_; }
    void setPaint(const QColor& colour);
    // Glass body with the parts inside. Off for the plain picture.
    void setAnatomyVisible(bool visible);
    // A light sweep across the car while something is being read.
    void setScanning(bool scanning);

    // All zones in which something was found.
    void setAffected(const QSet<int>& zones);
    // The currently selected zone is highlighted and pulses.
    void setFocus(CarZone zone);
    void clear();

    QSize minimumSizeHint() const override { return {360, 170}; }
    QSize sizeHint() const override { return {720, 300}; }
    bool hasHeightForWidth() const override { return true; }
    int heightForWidth(int width) const override { return int(width * 0.42); }

signals:
    void zoneClicked(int zone);

protected:
    void paintEvent(QPaintEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;
    void mouseMoveEvent(QMouseEvent* event) override;
    void leaveEvent(QEvent* event) override;

private:
    struct Shape {
        double front, rear;            // x extents of the body
        double wheelF, wheelR, wheelRad;
        double ground;                 // y of the road
        double sill;                   // lower edge of the doors
        double bumper;                 // lower edge of the bumpers
        double nose;                   // top of the front face
        double hood;                   // hood at the windscreen
        double belt;                   // lower edge of the side windows
        double roof;
        double aBase, roofFront, roofRear, cBase;
        double deck;                   // boot lid or load bed height
        double tail;                   // where the rear face starts
        bool notch;                    // separate boot (saloon, pick-up)
        int pillars;                   // side window splits
    };

    struct Part {
        CarZone zone;
        QPainterPath path;      // in field coordinates
        QPointF anchor;         // where the label line ends
        const char* label;      // untranslated, see QT_TR_NOOP
    };

    static Shape shapeFor(BodyStyle style);
    void rebuild();
    QTransform fieldToWidget() const;
    CarZone zoneAt(const QPointF& widgetPos) const;

    void drawGround(QPainter& p) const;
    void drawBody(QPainter& p) const;
    void drawWheel(QPainter& p, double cx) const;
    void drawAnatomy(QPainter& p) const;
    void drawLabels(QPainter& p, const QTransform& toWidget) const;
    void drawSweep(QPainter& p) const;

    BodyStyle style_ = BodyStyle::Hatchback;
    Shape shape_{};
    QColor paint_{"#4b5878"};
    bool anatomy_ = false;
    bool scanning_ = false;

    QPainterPath body_;
    QPainterPath glass_;
    QVector<QPainterPath> doorLines_;
    QVector<Part> parts_;

    QSet<int> affected_;
    CarZone focus_ = CarZone::Unknown;
    CarZone hover_ = CarZone::Unknown;
    double pulse_ = 0.0;
    double sweep_ = -0.2;
    QTimer* animation_ = nullptr;
};

}  // namespace caruspex
