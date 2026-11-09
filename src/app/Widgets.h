// Small building blocks for the interface: animated page stack, navigation
// buttons, the health ring, toasts, tiles and finding rows.
#pragma once

#include <QAbstractButton>
#include <QColor>
#include <QFrame>
#include <QPixmap>
#include <QPointer>
#include <QStackedWidget>
#include <QString>
#include <QWidget>

class QLabel;
class QVariantAnimation;
class QVBoxLayout;
class QHBoxLayout;
class QTimer;

namespace caruspex {

// --- Layout helpers ---------------------------------------------------------

// A raised card with the standard look. padding applies to a QVBoxLayout that
// is created right away and returned through layout (optional).
QFrame* makeCard(QVBoxLayout** layout = nullptr, int padding = 22);

// Eyebrow, title and an optional subtitle stacked the way every page starts.
QWidget* makePageHeader(const QString& eyebrow, const QString& title,
                        const QString& subtitle = QString());

// A label with objectName set, for the style sheet.
QLabel* makeLabel(const QString& text, const char* objectName = nullptr, bool wrap = false);

// Severity or state colours as a small dot with text, e.g. "5 bereit".
QWidget* makeStatusLine(const QString& text, const QColor& dot);

// --- AnimatedStack ------------------------------------------------------------

// QStackedWidget that cross-fades and slides when the page changes.
class AnimatedStack : public QStackedWidget {
    Q_OBJECT

public:
    explicit AnimatedStack(QWidget* parent = nullptr);
    void slideTo(int index);
    void slideTo(QWidget* page) { slideTo(indexOf(page)); }

private:
    QPointer<QWidget> overlay_;
};

// --- NavButton ------------------------------------------------------------------

// Sidebar entry: icon, text, optional badge. Hover and selection fade in.
class NavButton : public QAbstractButton {
    Q_OBJECT

public:
    NavButton(const QString& iconName, const QString& text, QWidget* parent = nullptr);
    void setBadge(const QString& text, const QColor& colour = QColor());
    void setCompact(bool compact);
    QSize sizeHint() const override;

protected:
    void paintEvent(QPaintEvent* event) override;
    void enterEvent(QEnterEvent* event) override;
    void leaveEvent(QEvent* event) override;

private:
    void animateTo(QVariantAnimation* animation, double target);

    QString iconName_;
    QString badge_;
    QColor badgeColour_;
    bool compact_ = false;
    double hover_ = 0.0;
    double checked_ = 0.0;
    QVariantAnimation* hoverAnimation_ = nullptr;
    QVariantAnimation* checkAnimation_ = nullptr;
};

// --- ScoreRing --------------------------------------------------------------------

// The big health ring: a gradient arc that fills up to the score, the number
// counting up in the middle, a slowly turning dotted halo around it.
class ScoreRing : public QWidget {
    Q_OBJECT

public:
    explicit ScoreRing(QWidget* parent = nullptr);

    // value 0..100, or a negative number for "not known yet".
    void setScore(int value, const QString& caption, const QString& subcaption = QString());
    // Turns the arc into a spinner while something is being read.
    void setBusy(bool busy);

    QSize sizeHint() const override { return {260, 260}; }
    QSize minimumSizeHint() const override { return {180, 180}; }

protected:
    void paintEvent(QPaintEvent* event) override;

private:
    int target_ = -1;
    double shown_ = 0.0;
    double spin_ = 0.0;
    double breath_ = 0.0;
    bool busy_ = false;
    QString caption_;
    QString subcaption_;
    QVariantAnimation* countUp_ = nullptr;
    QTimer* ticker_ = nullptr;
};

// --- Spinner ------------------------------------------------------------------------

class Spinner : public QWidget {
    Q_OBJECT

public:
    explicit Spinner(int size = 18, QWidget* parent = nullptr);
    void setRunning(bool running);
    QSize sizeHint() const override { return {size_, size_}; }

protected:
    void paintEvent(QPaintEvent* event) override;

private:
    int size_;
    double angle_ = 0.0;
    QTimer* timer_ = nullptr;
};

// --- StatTile ----------------------------------------------------------------------

// Icon, caption, a big value with unit and a coloured state dot.
class StatTile : public QFrame {
    Q_OBJECT

public:
    StatTile(const QString& iconName, const QString& caption, QWidget* parent = nullptr);
    void setValue(const QString& value, const QString& unit = QString(),
                  const QColor& state = QColor());
    void setNote(const QString& note);
    void clear();

private:
    QLabel* icon_ = nullptr;
    QLabel* value_ = nullptr;
    QLabel* unit_ = nullptr;
    QLabel* note_ = nullptr;
    QWidget* dot_ = nullptr;
};

// --- FindingRow ----------------------------------------------------------------------

// One line in a "what was found" list: tinted icon tile, title, explanation,
// something small on the right.
class FindingRow : public QWidget {
    Q_OBJECT

public:
    FindingRow(const QString& iconName, const QColor& tint, const QString& title,
               const QString& text, const QString& meta = QString(), QWidget* parent = nullptr);
    void setClickable(bool clickable);

signals:
    void clicked();

protected:
    void paintEvent(QPaintEvent* event) override;
    void mouseReleaseEvent(QMouseEvent* event) override;
    void enterEvent(QEnterEvent* event) override;
    void leaveEvent(QEvent* event) override;

private:
    bool clickable_ = false;
    bool hover_ = false;
};

// --- Toast -------------------------------------------------------------------------

// Short notice in the lower right corner of the window, fades out on its own.
namespace toast {
enum class Kind { Info, Success, Warning, Error };
void show(QWidget* anyWidgetInWindow, const QString& text, Kind kind = Kind::Info,
          int milliseconds = 4200);
}  // namespace toast

// --- FadeIn ---------------------------------------------------------------------------

// Fades a widget in from slightly below. Used when content appears.
void fadeIn(QWidget* widget, int delayMs = 0, int durationMs = 360);

}  // namespace caruspex
