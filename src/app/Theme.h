// Look modelled on Odysync: dark cyber theme with neon cyan and violet, a glow
// around the important surfaces, JetBrains Mono.
#pragma once

#include <QColor>
#include <QString>

class QApplication;
class QWidget;

namespace caruspex::theme {

namespace color {
// Base surfaces
inline const QColor background{"#0a0a0f"};
inline const QColor surface{"#13131a"};
inline const QColor surfaceHigh{"#1a1a24"};
inline const QColor border{"#1e1e2a"};
inline const QColor borderBright{"#2a2a3e"};
// Text
inline const QColor text{"#e0e0e8"};
inline const QColor textMuted{"#8888a0"};
inline const QColor textFaint{"#555568"};
// Accents
inline const QColor accent{"#00f0ff"};
inline const QColor accentHover{"#00d4e6"};
inline const QColor accentDim{"#00a8b5"};
inline const QColor purple{"#a855f7"};
inline const QColor purpleDim{"#7c3aed"};
// States
inline const QColor good{"#22c55e"};
inline const QColor warn{"#f59e0b"};
inline const QColor bad{"#ef4444"};
}  // namespace color

void apply(QApplication& app);
QString stylesheet();

// Puts a coloured glow around a control. Without an offset the drop shadow
// looks like a glow, which creates the neon effect.
void addGlow(QWidget* widget, const QColor& colour, double strength = 1.0);
void removeGlow(QWidget* widget);

// Makes the glow breathe, two seconds per cycle as in Odysync.
void pulseGlow(QWidget* widget, const QColor& colour);

}  // namespace caruspex::theme
