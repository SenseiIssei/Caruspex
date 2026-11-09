#include "app/AppIcon.h"

#include <QLinearGradient>
#include <QPainter>
#include <QPainterPath>
#include <QRadialGradient>

#include "app/Icons.h"

namespace caruspex {

QImage renderAppIcon(int size) {
    QImage image(size, size, QImage::Format_ARGB32_Premultiplied);
    image.fill(Qt::transparent);

    QPainter p(&image);
    p.setRenderHint(QPainter::Antialiasing);
    p.setRenderHint(QPainter::SmoothPixmapTransform);

    // Small icons fill the square so they stay readable in a task bar; large
    // ones keep a margin like the system icons around them.
    const double margin = size >= 64 ? size * 0.07 : 0.0;
    const QRectF square(margin, margin, size - 2 * margin, size - 2 * margin);
    const double radius = square.width() * 0.23;
    QPainterPath box;
    box.addRoundedRect(square, radius, radius);

    // Fixed colours, the same in the light and the dark theme: the indigo to
    // sky gradient of the dark palette, running diagonally.
    QLinearGradient fill(square.topLeft(), square.bottomRight());
    fill.setColorAt(0.0, QColor("#6d68ff"));
    fill.setColorAt(0.55, QColor("#7f8dff"));
    fill.setColorAt(1.0, QColor("#4fb2ff"));
    p.fillPath(box, fill);

    // Soft light from the top left gives the square some depth.
    if (size >= 32) {
        QRadialGradient glow(square.left() + square.width() * 0.3,
                             square.top() + square.height() * 0.2, square.width() * 0.8);
        glow.setColorAt(0.0, QColor(255, 255, 255, 70));
        glow.setColorAt(1.0, QColor(255, 255, 255, 0));
        p.fillPath(box, glow);
        p.setPen(QPen(QColor(255, 255, 255, 40), qMax(1.0, size / 256.0)));
        p.drawPath(box);
    }

    // The car, thicker lines at small sizes so it does not blur away.
    const int carSize = qRound(square.width() * 0.72);
    const double stroke = size <= 24 ? 2.6 : (size <= 48 ? 2.2 : 1.9);
    const QPixmap car = icons::pixmap(QStringLiteral("car"), carSize, Qt::white, stroke);
    const QPointF at(square.center().x() - carSize / 2.0,
                     square.center().y() - carSize / 2.0 + square.height() * 0.02);
    p.drawPixmap(at, car);
    p.end();
    return image;
}

QIcon appIcon() {
    QIcon icon;
    for (int size : {16, 24, 32, 48, 64, 128, 256}) {
        icon.addPixmap(QPixmap::fromImage(renderAppIcon(size)));
    }
    return icon;
}

}  // namespace caruspex
