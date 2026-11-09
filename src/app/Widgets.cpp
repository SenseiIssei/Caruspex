#include "app/Widgets.h"

#include <QConicalGradient>
#include <QEnterEvent>
#include <QGraphicsOpacityEffect>
#include <QHBoxLayout>
#include <QLabel>
#include <QList>
#include <QMouseEvent>
#include <QPainter>
#include <QPainterPath>
#include <QPropertyAnimation>
#include <QRadialGradient>
#include <QTimer>
#include <QVBoxLayout>
#include <QVariantAnimation>
#include <QtMath>

#include "app/Icons.h"
#include "app/Theme.h"

namespace caruspex {

using namespace theme;

// --- Layout helpers ---------------------------------------------------------

QFrame* makeCard(QVBoxLayout** layout, int padding) {
    auto* card = new QFrame;
    card->setObjectName("Card");
    auto* inner = new QVBoxLayout(card);
    inner->setContentsMargins(padding, padding - 2, padding, padding - 2);
    inner->setSpacing(12);
    if (layout) *layout = inner;
    return card;
}

QLabel* makeLabel(const QString& text, const char* objectName, bool wrap) {
    auto* label = new QLabel(text);
    if (objectName) label->setObjectName(QString::fromLatin1(objectName));
    label->setWordWrap(wrap);
    return label;
}

QWidget* makePageHeader(const QString& eyebrow, const QString& title, const QString& subtitle) {
    auto* box = new QWidget;
    auto* layout = new QVBoxLayout(box);
    layout->setContentsMargins(0, 0, 0, 4);
    layout->setSpacing(4);
    if (!eyebrow.isEmpty()) layout->addWidget(makeLabel(eyebrow.toUpper(), "Eyebrow"));
    layout->addWidget(makeLabel(title, "PageTitle"));
    if (!subtitle.isEmpty()) {
        QLabel* sub = makeLabel(subtitle, "Subtle", true);
        layout->addWidget(sub);
    }
    return box;
}

namespace {

class Dot : public QWidget {
public:
    explicit Dot(const QColor& colour, QWidget* parent = nullptr) : QWidget(parent), c_(colour) {
        setFixedSize(8, 8);
    }
    void setColour(const QColor& colour) {
        c_ = colour;
        update();
    }

protected:
    void paintEvent(QPaintEvent*) override {
        if (!c_.isValid()) return;
        QPainter p(this);
        p.setRenderHint(QPainter::Antialiasing);
        p.setPen(Qt::NoPen);
        p.setBrush(c_);
        p.drawEllipse(rect().adjusted(0, 0, -1, -1));
    }

private:
    QColor c_;
};

}  // namespace

QWidget* makeStatusLine(const QString& text, const QColor& dot) {
    auto* box = new QWidget;
    auto* layout = new QHBoxLayout(box);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(8);
    layout->addWidget(new Dot(dot), 0, Qt::AlignVCenter);
    auto* label = new QLabel(text);
    label->setStyleSheet(QStringLiteral("font-weight: 600;"));
    layout->addWidget(label);
    return box;
}

// --- AnimatedStack ------------------------------------------------------------

namespace {

// Paints the old page fading out and the new one rising in. Lives only for
// the length of the transition, the real pages stay untouched.
class TransitionOverlay : public QWidget {
public:
    TransitionOverlay(QWidget* parent, QPixmap from, QPixmap to)
        : QWidget(parent), from_(std::move(from)), to_(std::move(to)) {
        setAttribute(Qt::WA_TransparentForMouseEvents);
        auto* animation = new QVariantAnimation(this);
        animation->setDuration(280);
        animation->setStartValue(0.0);
        animation->setEndValue(1.0);
        animation->setEasingCurve(QEasingCurve::OutCubic);
        connect(animation, &QVariantAnimation::valueChanged, this, [this](const QVariant& v) {
            t_ = v.toDouble();
            update();
        });
        connect(animation, &QVariantAnimation::finished, this, &QObject::deleteLater);
        animation->start(QAbstractAnimation::DeleteWhenStopped);
    }

protected:
    void paintEvent(QPaintEvent*) override {
        QPainter p(this);
        p.fillRect(rect(), color::background);
        const double out = qBound(0.0, 1.0 - t_ * 1.6, 1.0);
        if (!from_.isNull() && out > 0.0) {
            p.setOpacity(out);
            p.drawPixmap(QPointF(0, -8.0 * t_), from_);
        }
        p.setOpacity(t_);
        p.drawPixmap(QPointF(0, 18.0 * (1.0 - t_)), to_);
    }

private:
    QPixmap from_;
    QPixmap to_;
    double t_ = 0.0;
};

}  // namespace

AnimatedStack::AnimatedStack(QWidget* parent) : QStackedWidget(parent) {}

void AnimatedStack::slideTo(int index) {
    if (index < 0 || index >= count()) return;
    if (index == currentIndex() || !isVisible()) {
        setCurrentIndex(index);
        return;
    }
    if (overlay_) overlay_->deleteLater();
    QWidget* from = currentWidget();
    const QPixmap before = from ? from->grab() : QPixmap();
    setCurrentIndex(index);
    QWidget* to = currentWidget();
    const QPixmap after = to->grab();
    auto* overlay = new TransitionOverlay(this, before, after);
    overlay->setGeometry(rect());
    overlay->show();
    overlay->raise();
    overlay_ = overlay;
}

// --- NavButton ------------------------------------------------------------------

NavButton::NavButton(const QString& iconName, const QString& text, QWidget* parent)
    : QAbstractButton(parent), iconName_(iconName) {
    setText(text);
    setCheckable(true);
    setCursor(Qt::PointingHandCursor);
    setFocusPolicy(Qt::NoFocus);
    setToolTip(QString());

    auto make = [this](double* field) {
        auto* a = new QVariantAnimation(this);
        a->setDuration(180);
        a->setEasingCurve(QEasingCurve::OutCubic);
        connect(a, &QVariantAnimation::valueChanged, this, [this, field](const QVariant& v) {
            *field = v.toDouble();
            update();
        });
        return a;
    };
    hoverAnimation_ = make(&hover_);
    checkAnimation_ = make(&checked_);
    connect(this, &QAbstractButton::toggled, this,
            [this](bool on) { animateTo(checkAnimation_, on ? 1.0 : 0.0); });
}

void NavButton::animateTo(QVariantAnimation* animation, double target) {
    const double current = animation == hoverAnimation_ ? hover_ : checked_;
    animation->stop();
    animation->setStartValue(current);
    animation->setEndValue(target);
    animation->start();
}

void NavButton::setBadge(const QString& text, const QColor& colour) {
    badge_ = text;
    badgeColour_ = colour;
    update();
}

void NavButton::setCompact(bool compact) {
    compact_ = compact;
    setToolTip(compact ? text() : QString());
    updateGeometry();
    update();
}

QSize NavButton::sizeHint() const { return compact_ ? QSize(44, 40) : QSize(220, 38); }

void NavButton::enterEvent(QEnterEvent* event) {
    animateTo(hoverAnimation_, 1.0);
    QAbstractButton::enterEvent(event);
}

void NavButton::leaveEvent(QEvent* event) {
    animateTo(hoverAnimation_, 0.0);
    QAbstractButton::leaveEvent(event);
}

void NavButton::paintEvent(QPaintEvent*) {
    QPainter p(this);
    p.setRenderHint(QPainter::Antialiasing);
    const QRectF box = QRectF(rect()).adjusted(1, 1, -1, -1);

    // Hover and selection blend into each other.
    QColor fill = color::surfaceHigh;
    fill.setAlphaF(static_cast<float>(0.9 * hover_ * (1.0 - checked_)));
    QColor selected = color::accent;
    selected.setAlphaF(static_cast<float>(0.13 * checked_));
    p.setPen(Qt::NoPen);
    p.setBrush(fill);
    p.drawRoundedRect(box, 10, 10);
    if (checked_ > 0.0) {
        p.setBrush(selected);
        QColor line = color::accent;
        line.setAlphaF(static_cast<float>(0.35 * checked_));
        p.setPen(QPen(line, 1.0));
        p.drawRoundedRect(box, 10, 10);
        // Small indicator on the left edge.
        p.setPen(Qt::NoPen);
        QLinearGradient bar(0, box.top() + 9, 0, box.bottom() - 9);
        bar.setColorAt(0, color::accent);
        bar.setColorAt(1, color::sky);
        p.setOpacity(checked_);
        p.setBrush(bar);
        p.drawRoundedRect(QRectF(box.left() + 1, box.top() + 9, 3, box.height() - 18), 1.5, 1.5);
        p.setOpacity(1.0);
    }

    const QColor iconColour = mix(color::textMuted, color::accentHover, checked_);
    const QColor textColour =
        mix(color::textMuted, color::text, qMax(hover_ * 0.8, checked_));
    const int iconSize = 18;
    const double iconX = compact_ ? (width() - iconSize) / 2.0 : 13.0;
    p.drawPixmap(QPointF(iconX, (height() - iconSize) / 2.0),
                 icons::pixmap(iconName_, iconSize, isEnabled() ? iconColour : color::textFaint));
    if (compact_) {
        if (!badge_.isEmpty()) {
            p.setPen(Qt::NoPen);
            p.setBrush(badgeColour_.isValid() ? badgeColour_ : color::accent);
            p.drawEllipse(QPointF(width() - 10, 9), 4, 4);
        }
        return;
    }

    QFont f = sans(14, checked_ > 0.5 ? 600 : 500);
    p.setFont(f);
    p.setPen(isEnabled() ? textColour : color::textFaint);
    QRectF textBox(iconX + iconSize + 12, 0, width() - iconX - iconSize - 20, height());

    if (!badge_.isEmpty()) {
        QFont bf = sans(11, 700);
        const QFontMetrics fm(bf);
        const double bw = qMax(20, fm.horizontalAdvance(badge_) + 12);
        const QRectF pill(width() - bw - 10, (height() - 20) / 2.0, bw, 20);
        QColor base = badgeColour_.isValid() ? badgeColour_ : color::accent;
        QColor pillFill = base;
        pillFill.setAlphaF(0.18f);
        p.setPen(Qt::NoPen);
        p.setBrush(pillFill);
        p.drawRoundedRect(pill, 10, 10);
        p.setFont(bf);
        p.setPen(base.lighter(115));
        p.drawText(pill, Qt::AlignCenter, badge_);
        textBox.setRight(pill.left() - 6);
        p.setFont(f);
        p.setPen(isEnabled() ? textColour : color::textFaint);
    }
    p.drawText(textBox, Qt::AlignVCenter | Qt::AlignLeft,
               QFontMetrics(f).elidedText(text(), Qt::ElideRight, int(textBox.width())));
}

// --- ScoreRing --------------------------------------------------------------------

ScoreRing::ScoreRing(QWidget* parent) : QWidget(parent) {
    countUp_ = new QVariantAnimation(this);
    countUp_->setDuration(1400);
    countUp_->setEasingCurve(QEasingCurve::OutCubic);
    connect(countUp_, &QVariantAnimation::valueChanged, this, [this](const QVariant& v) {
        shown_ = v.toDouble();
        update();
    });
    ticker_ = new QTimer(this);
    ticker_->setInterval(33);
    connect(ticker_, &QTimer::timeout, this, [this] {
        if (!isVisible()) return;
        spin_ += busy_ ? 7.0 : 0.25;
        if (spin_ >= 360.0) spin_ -= 360.0;
        breath_ += 0.045;
        if (breath_ > 2.0 * M_PI) breath_ -= 2.0 * M_PI;
        update();
    });
    ticker_->start();
}

void ScoreRing::setScore(int value, const QString& caption, const QString& subcaption) {
    caption_ = caption;
    subcaption_ = subcaption;
    const bool wasKnown = target_ >= 0;
    target_ = value;
    if (value < 0) {
        shown_ = 0.0;
        update();
        return;
    }
    countUp_->stop();
    countUp_->setStartValue(wasKnown ? shown_ : 0.0);
    countUp_->setEndValue(static_cast<double>(value));
    countUp_->start();
}

void ScoreRing::setBusy(bool busy) {
    busy_ = busy;
    update();
}

void ScoreRing::paintEvent(QPaintEvent*) {
    QPainter p(this);
    p.setRenderHint(QPainter::Antialiasing);
    const double side = qMin(width(), height());
    const QPointF c(width() / 2.0, height() / 2.0);
    const double outer = side / 2.0 - 4.0;
    const double radius = outer - 22.0;
    const double thickness = qMax(10.0, side * 0.055);
    const double fraction = target_ >= 0 ? qBound(0.0, shown_ / 100.0, 1.0) : 0.0;
    const QColor tone = target_ >= 0 ? healthColor(fraction) : color::accent;

    // Soft halo that breathes.
    QRadialGradient halo(c, outer);
    QColor h1 = tone;
    h1.setAlphaF(static_cast<float>(0.10 + 0.05 * std::sin(breath_)));
    QColor h0 = h1;
    h0.setAlphaF(0.0f);
    halo.setColorAt(0.55, h1);
    halo.setColorAt(1.0, h0);
    p.setPen(Qt::NoPen);
    p.setBrush(halo);
    p.drawEllipse(c, outer, outer);

    // Inner disc
    QRadialGradient disc(c, radius);
    disc.setColorAt(0.0, QColor("#1a1e2a"));
    disc.setColorAt(1.0, QColor("#11141c"));
    p.setBrush(disc);
    p.drawEllipse(c, radius - thickness / 2.0, radius - thickness / 2.0);

    // Dotted halo, turning slowly.
    p.save();
    p.translate(c);
    p.rotate(spin_);
    QColor dotColour = color::borderBright;
    for (int i = 0; i < 72; ++i) {
        const double a = qDegreesToRadians(i * 5.0);
        dotColour.setAlphaF(i % 6 == 0 ? 0.9f : 0.45f);
        p.setBrush(dotColour);
        p.drawEllipse(QPointF(std::cos(a) * outer, std::sin(a) * outer), 1.1, 1.1);
    }
    p.restore();

    // Track
    const QRectF ring(c.x() - radius, c.y() - radius, radius * 2, radius * 2);
    p.setBrush(Qt::NoBrush);
    p.setPen(QPen(QColor("#1d2230"), thickness, Qt::SolidLine, Qt::RoundCap));
    p.drawEllipse(ring);

    if (busy_ || target_ < 0) {
        if (busy_) {
            QConicalGradient g(c, -spin_);
            g.setColorAt(0.0, color::sky);
            g.setColorAt(0.25, color::accent);
            g.setColorAt(0.26, Qt::transparent);
            g.setColorAt(1.0, Qt::transparent);
            p.setPen(QPen(QBrush(g), thickness, Qt::SolidLine, Qt::RoundCap));
            p.drawArc(ring, static_cast<int>(-spin_ * 16), 90 * 16);
        }
    } else if (fraction > 0.0) {
        // Start at the top, run clockwise.
        QConicalGradient g(c, 90.0);
        g.setColorAt(0.0, tone.lighter(125));
        g.setColorAt(1.0 - fraction * 0.999, tone);
        g.setColorAt(1.0, tone.lighter(125));
        p.setPen(QPen(QBrush(g), thickness, Qt::SolidLine, Qt::RoundCap));
        p.drawArc(ring, 90 * 16, static_cast<int>(-360.0 * fraction * 16));

        // End cap with a little light.
        const double a = qDegreesToRadians(90.0 - 360.0 * fraction);
        const QPointF tip(c.x() + std::cos(a) * radius, c.y() - std::sin(a) * radius);
        QColor glow = tone;
        glow.setAlphaF(0.35f);
        p.setPen(Qt::NoPen);
        p.setBrush(glow);
        p.drawEllipse(tip, thickness * 0.9, thickness * 0.9);
        p.setBrush(QColor(255, 255, 255, 230));
        p.drawEllipse(tip, thickness * 0.22, thickness * 0.22);
    }

    // Number
    const QString number = target_ >= 0 ? QString::number(qRound(shown_)) : QStringLiteral("--");
    QFont big = display(qMax(30, int(side * 0.22)), 800);
    p.setFont(big);
    const QFontMetricsF fm(big);
    const QRectF numberBox(c.x() - radius, c.y() - fm.height() * 0.78, radius * 2, fm.height());
    QLinearGradient textGradient(numberBox.topLeft(), numberBox.bottomLeft());
    textGradient.setColorAt(0, QColor("#ffffff"));
    textGradient.setColorAt(1, target_ >= 0 ? tone.lighter(110) : color::textMuted);
    p.setPen(QPen(QBrush(textGradient), 1));
    p.drawText(numberBox, Qt::AlignCenter, number);

    p.setFont(sans(qMax(13, int(side * 0.058)), 600));
    p.setPen(color::text);
    const QRectF captionBox(c.x() - radius, numberBox.bottom() - 2, radius * 2, side * 0.09);
    p.drawText(captionBox, Qt::AlignHCenter | Qt::AlignTop, caption_);
    p.setFont(sans(qMax(11, int(side * 0.045)), 400));
    p.setPen(color::textFaint);
    p.drawText(QRectF(c.x() - radius, captionBox.bottom(), radius * 2, side * 0.08),
               Qt::AlignHCenter | Qt::AlignTop, subcaption_);
}

// --- Spinner ------------------------------------------------------------------------

Spinner::Spinner(int size, QWidget* parent) : QWidget(parent), size_(size) {
    setFixedSize(size, size);
    timer_ = new QTimer(this);
    timer_->setInterval(16);
    connect(timer_, &QTimer::timeout, this, [this] {
        angle_ += 9.0;
        if (angle_ >= 360.0) angle_ -= 360.0;
        update();
    });
    setVisible(false);
}

void Spinner::setRunning(bool running) {
    setVisible(running);
    if (running) {
        timer_->start();
    } else {
        timer_->stop();
    }
}

void Spinner::paintEvent(QPaintEvent*) {
    QPainter p(this);
    p.setRenderHint(QPainter::Antialiasing);
    const double w = qMax(2.0, size_ * 0.12);
    const QRectF box = QRectF(rect()).adjusted(w, w, -w, -w);
    p.setPen(QPen(color::border, w));
    p.drawEllipse(box);
    QConicalGradient g(box.center(), -angle_);
    g.setColorAt(0.0, color::sky);
    g.setColorAt(0.3, color::accent);
    g.setColorAt(0.31, Qt::transparent);
    g.setColorAt(1.0, Qt::transparent);
    p.setPen(QPen(QBrush(g), w, Qt::SolidLine, Qt::RoundCap));
    p.drawArc(box, static_cast<int>(-angle_ * 16), 110 * 16);
}

// --- StatTile ----------------------------------------------------------------------

StatTile::StatTile(const QString& iconName, const QString& caption, QWidget* parent)
    : QFrame(parent) {
    setObjectName("CardFlat");
    setMinimumWidth(150);
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(16, 14, 16, 14);
    layout->setSpacing(6);

    auto* top = new QHBoxLayout;
    top->setSpacing(8);
    icon_ = new QLabel;
    icon_->setPixmap(icons::pixmap(iconName, 16, color::textMuted));
    auto* label = new QLabel(caption);
    label->setStyleSheet(QStringLiteral("color: %1; font-size: 12px; font-weight: 600;")
                             .arg(color::textMuted.name()));
    top->addWidget(icon_);
    top->addWidget(label, 1);
    auto* dot = new Dot(QColor());
    dot_ = dot;
    top->addWidget(dot_);
    layout->addLayout(top);

    auto* valueRow = new QHBoxLayout;
    valueRow->setSpacing(6);
    value_ = new QLabel(QStringLiteral("--"));
    value_->setFont(display(24, 700));
    unit_ = new QLabel;
    unit_->setStyleSheet(QStringLiteral("color: %1; font-size: 13px;").arg(color::textFaint.name()));
    valueRow->addWidget(value_, 0, Qt::AlignBottom);
    valueRow->addWidget(unit_, 0, Qt::AlignBottom);
    valueRow->addStretch();
    layout->addLayout(valueRow);

    note_ = new QLabel;
    note_->setObjectName("Hint");
    note_->setWordWrap(true);
    note_->setVisible(false);
    layout->addWidget(note_);
}

void StatTile::setValue(const QString& value, const QString& unit, const QColor& state) {
    value_->setText(value);
    unit_->setText(unit);
    static_cast<Dot*>(dot_)->setColour(state);
}

void StatTile::setNote(const QString& note) {
    note_->setText(note);
    note_->setVisible(!note.isEmpty());
}

void StatTile::clear() {
    setValue(QStringLiteral("--"));
    setNote(QString());
}

// --- FindingRow ----------------------------------------------------------------------

FindingRow::FindingRow(const QString& iconName, const QColor& tint, const QString& title,
                       const QString& text, const QString& meta, QWidget* parent)
    : QWidget(parent) {
    setAttribute(Qt::WA_Hover);
    auto* layout = new QHBoxLayout(this);
    layout->setContentsMargins(12, 10, 12, 10);
    layout->setSpacing(14);

    auto* tile = new QLabel;
    tile->setFixedSize(38, 38);
    tile->setAlignment(Qt::AlignCenter);
    tile->setPixmap(icons::pixmap(iconName, 18, tint));
    tile->setStyleSheet(QStringLiteral("background: %1; border-radius: 10px;")
                            .arg(rgba(tint, 0.13)));
    layout->addWidget(tile, 0, Qt::AlignTop);

    auto* texts = new QVBoxLayout;
    texts->setSpacing(2);
    auto* head = new QLabel(title);
    head->setWordWrap(true);
    head->setStyleSheet(QStringLiteral("font-weight: 600; font-size: 14px;"));
    texts->addWidget(head);
    if (!text.isEmpty()) {
        auto* body = new QLabel(text);
        body->setWordWrap(true);
        body->setStyleSheet(
            QStringLiteral("color: %1; font-size: 13px;").arg(color::textMuted.name()));
        texts->addWidget(body);
    }
    layout->addLayout(texts, 1);

    if (!meta.isEmpty()) {
        auto* right = new QLabel(meta);
        right->setStyleSheet(
            QStringLiteral("color: %1; font-size: 12px;").arg(color::textFaint.name()));
        layout->addWidget(right, 0, Qt::AlignVCenter);
    }
}

void FindingRow::setClickable(bool clickable) {
    clickable_ = clickable;
    setCursor(clickable ? Qt::PointingHandCursor : Qt::ArrowCursor);
}

void FindingRow::paintEvent(QPaintEvent*) {
    QPainter p(this);
    p.setRenderHint(QPainter::Antialiasing);
    if (hover_) {
        p.setPen(Qt::NoPen);
        p.setBrush(QColor(255, 255, 255, 8));
        p.drawRoundedRect(QRectF(rect()).adjusted(0, 1, 0, -1), 10, 10);
    }
    // Hairline between rows
    p.setPen(QPen(QColor("#1d2230"), 1));
    p.drawLine(QPointF(64, height() - 0.5), QPointF(width() - 12, height() - 0.5));
}

void FindingRow::mouseReleaseEvent(QMouseEvent* event) {
    if (clickable_ && event->button() == Qt::LeftButton && rect().contains(event->pos())) {
        emit clicked();
    }
    QWidget::mouseReleaseEvent(event);
}

void FindingRow::enterEvent(QEnterEvent* event) {
    hover_ = true;
    update();
    QWidget::enterEvent(event);
}

void FindingRow::leaveEvent(QEvent* event) {
    hover_ = false;
    update();
    QWidget::leaveEvent(event);
}

// --- Toast -------------------------------------------------------------------------

namespace toast {
namespace {

class ToastWidget : public QFrame {
public:
    ToastWidget(QWidget* host, const QString& text, Kind kind) : QFrame(host) {
        QColor tone = color::info;
        QString iconName = QStringLiteral("info");
        switch (kind) {
            case Kind::Success: tone = color::good; iconName = QStringLiteral("check-circle"); break;
            case Kind::Warning: tone = color::warn; iconName = QStringLiteral("alert"); break;
            case Kind::Error: tone = color::bad; iconName = QStringLiteral("x-circle"); break;
            case Kind::Info: break;
        }
        setStyleSheet(QStringLiteral("QFrame { background: #1b2030; border: 1px solid %1; "
                                     "border-radius: 12px; } QLabel { border: none; }")
                          .arg(rgba(tone, 0.45)));
        auto* layout = new QHBoxLayout(this);
        layout->setContentsMargins(14, 12, 16, 12);
        layout->setSpacing(10);
        auto* icon = new QLabel;
        icon->setPixmap(icons::pixmap(iconName, 18, tone));
        layout->addWidget(icon, 0, Qt::AlignTop);
        auto* label = new QLabel(text);
        label->setWordWrap(true);
        label->setMaximumWidth(340);
        label->setStyleSheet(QStringLiteral("font-size: 13px;"));
        layout->addWidget(label, 1);
        adjustSize();
        addShadow(this, 30, 8, 0.5);
    }
};

QList<QPointer<ToastWidget>>& active() {
    static QList<QPointer<ToastWidget>> list;
    return list;
}

void relayout(QWidget* host) {
    int bottom = host->height() - 20;
    QList<QPointer<ToastWidget>>& list = active();
    for (int i = list.size() - 1; i >= 0; --i) {
        ToastWidget* t = list[i];
        if (!t) {
            list.removeAt(i);
            continue;
        }
        if (t->parentWidget() != host) continue;
        bottom -= t->height();
        auto* move = new QPropertyAnimation(t, "pos", t);
        move->setDuration(220);
        move->setEasingCurve(QEasingCurve::OutCubic);
        move->setEndValue(QPoint(host->width() - t->width() - 20, bottom));
        move->start(QAbstractAnimation::DeleteWhenStopped);
        bottom -= 10;
    }
}

}  // namespace

void show(QWidget* anyWidgetInWindow, const QString& text, Kind kind, int milliseconds) {
    if (!anyWidgetInWindow) return;
    QWidget* host = anyWidgetInWindow->window();
    if (!host) return;
    auto* t = new ToastWidget(host, text, kind);
    t->move(host->width() - t->width() - 20, host->height());
    t->show();
    t->raise();
    active().append(t);
    relayout(host);

    QPointer<ToastWidget> guard(t);
    QPointer<QWidget> hostGuard(host);
    QTimer::singleShot(milliseconds, t, [guard, hostGuard] {
        if (!guard) return;
        auto* effect = new QGraphicsOpacityEffect(guard);
        guard->setGraphicsEffect(effect);
        auto* fade = new QPropertyAnimation(effect, "opacity", guard);
        fade->setDuration(260);
        fade->setStartValue(1.0);
        fade->setEndValue(0.0);
        QObject::connect(fade, &QPropertyAnimation::finished, guard, [guard, hostGuard] {
            if (guard) guard->deleteLater();
            active().removeAll(guard);
            if (hostGuard) QTimer::singleShot(0, hostGuard, [hostGuard] { relayout(hostGuard); });
        });
        fade->start(QAbstractAnimation::DeleteWhenStopped);
    });
}

}  // namespace toast

// --- FadeIn ---------------------------------------------------------------------------

void fadeIn(QWidget* widget, int delayMs, int durationMs) {
    if (!widget || widget->graphicsEffect()) return;
    auto* effect = new QGraphicsOpacityEffect(widget);
    effect->setOpacity(0.0);
    widget->setGraphicsEffect(effect);
    auto* fade = new QPropertyAnimation(effect, "opacity", widget);
    fade->setDuration(durationMs);
    fade->setStartValue(0.0);
    fade->setEndValue(1.0);
    fade->setEasingCurve(QEasingCurve::OutCubic);
    QPointer<QWidget> guard(widget);
    QObject::connect(fade, &QPropertyAnimation::finished, widget, [guard] {
        if (guard) guard->setGraphicsEffect(nullptr);
    });
    QTimer::singleShot(delayMs, fade, [fade] { fade->start(QAbstractAnimation::DeleteWhenStopped); });
}

}  // namespace caruspex
