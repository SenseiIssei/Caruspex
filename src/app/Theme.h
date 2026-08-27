// Erscheinungsbild nach dem Vorbild von Odysync: dunkles Cyber-Thema mit
// Neon-Cyan und Violett, Glut um die wichtigen Flaechen, JetBrains Mono.
#pragma once

#include <QColor>
#include <QString>

class QApplication;
class QWidget;

namespace caruspex::theme {

namespace color {
// Grundflaechen
inline const QColor background{"#0a0a0f"};
inline const QColor surface{"#13131a"};
inline const QColor surfaceHigh{"#1a1a24"};
inline const QColor border{"#1e1e2a"};
inline const QColor borderBright{"#2a2a3e"};
// Schrift
inline const QColor text{"#e0e0e8"};
inline const QColor textMuted{"#8888a0"};
inline const QColor textFaint{"#555568"};
// Akzente
inline const QColor accent{"#00f0ff"};
inline const QColor accentHover{"#00d4e6"};
inline const QColor accentDim{"#00a8b5"};
inline const QColor purple{"#a855f7"};
inline const QColor purpleDim{"#7c3aed"};
// Zustaende
inline const QColor good{"#22c55e"};
inline const QColor warn{"#f59e0b"};
inline const QColor bad{"#ef4444"};
}  // namespace color

void apply(QApplication& app);
QString stylesheet();

// Legt eine farbige Glut um ein Bedienelement. Ohne Versatz wirkt der
// Schlagschatten wie ein Leuchten, so entsteht der Neon-Eindruck.
void addGlow(QWidget* widget, const QColor& colour, double strength = 1.0);
void removeGlow(QWidget* widget);

// Laesst die Glut atmen, zwei Sekunden je Zyklus wie in Odysync.
void pulseGlow(QWidget* widget, const QColor& colour);

}  // namespace caruspex::theme
