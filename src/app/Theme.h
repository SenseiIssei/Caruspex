// Dunkles Erscheinungsbild: Farbpalette und Stylesheet.
#pragma once

#include <QColor>
#include <QString>

class QApplication;

namespace caruspex::theme {

namespace color {
inline const QColor background{"#11131a"};
inline const QColor surface{"#181b23"};
inline const QColor surfaceHigh{"#1f2430"};
inline const QColor border{"#2c3341"};
inline const QColor text{"#e2e8f0"};
inline const QColor textMuted{"#8a94a6"};
inline const QColor accent{"#4aa3ff"};
inline const QColor good{"#3ac47d"};
inline const QColor warn{"#f2b13c"};
inline const QColor bad{"#ef5350"};
}  // namespace color

void apply(QApplication& app);
QString stylesheet();

}  // namespace caruspex::theme
