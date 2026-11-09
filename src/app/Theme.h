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
inline QColor background{"#0a0c11"};
inline QColor surface{"#12151d"};
inline QColor surfaceHigh{"#1a1e28"};
inline QColor surfaceHover{"#20253a"};
inline QColor border{"#232835"};
// Surfaces that only appear in a few places
inline QColor sidebar{"#0d1016"};
inline QColor input{"#0e1118"};
inline QColor cardTop{"#151924"};
inline QColor track{"#1d2230"};
inline QColor rowLine{"#1a1e29"};
inline QColor alternate{"#141823"};
inline QColor hoverLine{"#454e66"};
inline QColor scroll{"#2a3040"};
inline QColor scrollHover{"#3a4256"};
inline QColor pressed{"#151924"};
inline QColor disabled{"#12151c"};
inline QColor borderBright{"#323949"};
// Text
inline QColor text{"#eceef3"};
inline QColor textMuted{"#a4abb9"};
inline QColor textFaint{"#727a8a"};
// Accents: indigo with a sky blue partner for gradients
inline QColor accent{"#8f8cff"};
inline QColor accentHover{"#a5a2ff"};
inline QColor accentDim{"#5a54f0"};
inline QColor sky{"#5ab8ff"};
inline QColor purple{"#b18cff"};
inline QColor purpleDim{"#7c5cf0"};
// States
inline QColor good{"#4fd39a"};
inline QColor warn{"#f2bc5c"};
inline QColor bad{"#ff7a7a"};
inline QColor info{"#6fb8ff"};
inline QColor warm{"#ffc66d"};
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

// Light or dark, decided once at startup before any widget is built.
// The colours above are filled from the chosen palette.
void setLight(bool light);
bool isLight();

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
