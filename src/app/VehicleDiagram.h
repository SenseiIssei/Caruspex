// Schematic side view of a car that shows where a fault comes from.
#pragma once

#include <QHash>
#include <QSet>
#include <QWidget>

#include "data/DtcLocation.h"

class QTimer;

namespace caruspex {

class VehicleDiagram : public QWidget {
    Q_OBJECT
    Q_PROPERTY(double pulse READ pulse WRITE setPulse)

public:
    explicit VehicleDiagram(QWidget* parent = nullptr);

    // All zones in which something was found.
    void setAffected(const QSet<int>& zones);
    // The currently selected zone is highlighted and pulses.
    void setFocus(CarZone zone);
    void clear();

    double pulse() const { return pulse_; }
    void setPulse(double value);

    QSize minimumSizeHint() const override { return {420, 210}; }
    QSize sizeHint() const override { return {640, 300}; }

signals:
    void zoneClicked(int zone);

protected:
    void paintEvent(QPaintEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;

private:
    struct Hotspot {
        CarZone zone;
        QRectF box;      // in units from 0 to 1, scaled to the widget area
        const char* label;   // untranslated source text, see QT_TR_NOOP
    };

    const std::vector<Hotspot>& hotspots() const;
    QRectF scaled(const QRectF& unit) const;
    void drawCarBody(QPainter& painter) const;

    QSet<int> affected_;
    CarZone focus_ = CarZone::Unknown;
    double pulse_ = 0.0;
    QTimer* animation_ = nullptr;
    QRectF canvas_;
};

}  // namespace caruspex
