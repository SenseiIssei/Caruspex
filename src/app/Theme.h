// Look shared with Odysync: a calm dark canvas, raised surfaces with soft
// shadows, an indigo to sky gradient for the one thing that matters on a page.
// Text in Inter, headlines in Plus Jakarta Sans, codes and numbers in
// JetBrains Mono.
#pragma once

#include <QColor>
#include <QFont>
#include <QLinearGradient>
#include <QString>

class QApplication;
class QWidget;

namespace caruspex::theme {

namespace color {
// Base surfaces
inline const QColor background{"#0a0c11"};
inline const QColor surface{"#12151d"};
inline const QColor surfaceHigh{"#1a1e28"};
inline const QColor surfaceHover{"#20253a"};
inline const QColor border{"#232835"};
inline const QColor borderBright{"#323949"};
// Text
inline const QColor text{"#eceef3"};
inline const QColor textMuted{"#a4abb9"};
inline const QColor textFaint{"#727a8a"};
// Accents: indigo with a sky blue partner for gradients
inline const QColor accent{"#8f8cff"};
inline const QColor accentHover{"#a5a2ff"};
inline const QColor accentDim{"#5a54f0"};
inline const QColor sky{"#5ab8ff"};
inline const QColor purple{"#b18cff"};
inline const QColor purpleDim{"#7c5cf0"};
// States
inline const QColor good{"#4fd39a"};
inline const QColor warn{"#f2bc5c"};
inline const QColor bad{"#ff7a7a"};
inline const QColor info{"#6fb8ff"};
inline const QColor warm{"#ffc66d"};
}  // namespace color

// Font families, loaded from the resources at startup.
QString sansFamily();
QString displayFamily();
QString monoFamily();

// Ready-made fonts. Sizes are in pixels so they look the same on every
// platform, independent of the point to pixel ratio.
QFont sans(int px, int weight = 400);
QFont display(int px, int weight = 700);
QFont mono(int px, int weight = 400);

// The signature gradient, left to right across the given width.
QLinearGradient accentGradient(double x1, double y1, double x2, double y2);

// The colour for a 0..1 health value: red, amber, green.
QColor healthColor(double fraction);

// Mixes two colours, t = 0 gives a, t = 1 gives b.
QColor mix(const QColor& a, const QColor& b, double t);

// rgba() string for style sheets.
QString rgba(const QColor& c, double alpha);

void apply(QApplication& app);
QString stylesheet();

// Soft lift under a surface, like a card shadow on the web.
void addShadow(QWidget* widget, int blur = 32, int offsetY = 10, double alpha = 0.45);

// A gentle coloured halo. Kept for the places that want to draw attention,
// much quieter than the old neon glow.
void addGlow(QWidget* widget, const QColor& colour, double strength = 1.0);
void removeGlow(QWidget* widget);

// Makes the halo breathe, 2.4 seconds per cycle.
void pulseGlow(QWidget* widget, const QColor& colour);

// Re-applies the style sheet after an objectName change.
void repolish(QWidget* widget);

}  // namespace caruspex::theme
