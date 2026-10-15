// Round gauge with a smoothly trailing needle.
#pragma once

#include <QColor>
#include <QString>
#include <QWidget>

class QPropertyAnimation;

namespace caruspex {

class GaugeWidget : public QWidget {
    Q_OBJECT
    Q_PROPERTY(double displayValue READ displayValue WRITE setDisplayValue)

public:
    explicit GaugeWidget(QWidget* parent = nullptr);

    void configure(const QString& title, const QString& unit, double minimum, double maximum,
                   int decimals = 0);
    void setWarnRange(double from, double to);   // outside of it the arc changes colour
    void setValue(double value);                 // animated
    void setActive(bool active);                 // grey while nothing is being measured

    double displayValue() const { return display_; }
    void setDisplayValue(double value);

    QSize sizeHint() const override { return {180, 180}; }
    QSize minimumSizeHint() const override { return {140, 140}; }

protected:
    void paintEvent(QPaintEvent* event) override;

private:
    QColor arcColor() const;

    QString title_;
    QString unit_;
    double minimum_ = 0.0;
    double maximum_ = 100.0;
    double target_ = 0.0;
    double display_ = 0.0;
    double warnFrom_ = 0.0;
    double warnTo_ = 0.0;
    bool hasWarn_ = false;
    bool active_ = false;
    int decimals_ = 0;
    QPropertyAnimation* animation_ = nullptr;
};

}  // namespace caruspex
