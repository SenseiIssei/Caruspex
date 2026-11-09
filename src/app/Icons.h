// Line icons drawn on a 24 by 24 grid, stroke based so they stay crisp at any
// size and take any colour. Rendered through QtSvg.
#pragma once

#include <QColor>
#include <QIcon>
#include <QPixmap>
#include <QString>

namespace caruspex::icons {

// A pixmap of the named icon. Unknown names give an empty pixmap.
QPixmap pixmap(const QString& name, int size, const QColor& colour, double strokeWidth = 1.8);

// An icon with a normal and a checked/active colour, for buttons.
QIcon icon(const QString& name, const QColor& normal, const QColor& active = QColor(),
           int size = 20);

// All names, for a quick look in tests or a debug page.
QStringList names();

}  // namespace caruspex::icons
