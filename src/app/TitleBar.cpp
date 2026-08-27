#include "app/TitleBar.h"

#include <QHBoxLayout>
#include <QLabel>
#include <QMouseEvent>
#include <QPainter>
#include <QPushButton>
#include <QWindow>

#include "app/Theme.h"

namespace caruspex {
namespace {

constexpr int kBarHeight = 38;

QPushButton* makeButton(const QString& glyph, const QColor& hoverColour,
                        const QString& tip) {
    auto* button = new QPushButton(glyph);
    button->setFixedSize(44, kBarHeight - 2);
    button->setToolTip(tip);
    button->setCursor(Qt::ArrowCursor);
    button->setFocusPolicy(Qt::NoFocus);
    button->setStyleSheet(
        QStringLiteral("QPushButton { background: transparent; border: none; "
                       "border-radius: 0px; color: %1; font-size: 14px; }"
                       "QPushButton:hover { background: %2; color: %3; }")
            .arg(theme::color::textMuted.name(),
                 QStringLiteral("rgba(%1, %2, %3, 0.18)")
                     .arg(hoverColour.red())
                     .arg(hoverColour.green())
                     .arg(hoverColour.blue()),
                 hoverColour.name()));
    return button;
}

}  // namespace

TitleBar::TitleBar(QWidget* parent) : QWidget(parent) {
    setFixedHeight(kBarHeight);
    setAttribute(Qt::WA_StyledBackground, false);

    auto* layout = new QHBoxLayout(this);
    layout->setContentsMargins(14, 0, 0, 0);
    layout->setSpacing(0);

    title_ = new QLabel(QStringLiteral("CARUSPEX"));
    title_->setStyleSheet(QStringLiteral("color: %1; font-weight: 700; "
                                         "letter-spacing: 2px; font-size: 12px;")
                              .arg(theme::color::accent.name()));
    auto* subtitle = new QLabel(tr("  Fahrzeugdiagnose"));
    subtitle->setStyleSheet(
        QStringLiteral("color: %1; font-size: 12px;").arg(theme::color::textFaint.name()));

    layout->addWidget(title_);
    layout->addWidget(subtitle);
    layout->addStretch();

    minimise_ = makeButton(QStringLiteral("–"), theme::color::accent, tr("Minimieren"));
    maximise_ = makeButton(QStringLiteral("□"), theme::color::accent, tr("Maximieren"));
    close_ = makeButton(QStringLiteral("✕"), theme::color::bad, tr("Schliessen"));
    layout->addWidget(minimise_);
    layout->addWidget(maximise_);
    layout->addWidget(close_);

    connect(minimise_, &QPushButton::clicked, this, &TitleBar::minimiseRequested);
    connect(maximise_, &QPushButton::clicked, this, &TitleBar::maximiseToggled);
    connect(close_, &QPushButton::clicked, this, &TitleBar::closeRequested);
}

void TitleBar::setTitle(const QString& title) { title_->setText(title); }

void TitleBar::updateMaximiseState(bool maximised) {
    maximise_->setText(maximised ? QStringLiteral("❐") : QStringLiteral("□"));
    maximise_->setToolTip(maximised ? tr("Wiederherstellen") : tr("Maximieren"));
}

void TitleBar::mousePressEvent(QMouseEvent* event) {
    if (event->button() != Qt::LeftButton) return;
    // Das Verschieben uebernimmt der Fenstermanager. Nur so bleiben Aero Snap
    // und das Andocken an den Bildschirmrand erhalten.
    if (QWindow* handle = window()->windowHandle()) handle->startSystemMove();
}

void TitleBar::mouseDoubleClickEvent(QMouseEvent* event) {
    if (event->button() == Qt::LeftButton) emit maximiseToggled();
}

void TitleBar::paintEvent(QPaintEvent*) {
    QPainter p(this);
    p.setRenderHint(QPainter::Antialiasing);
    p.fillRect(rect(), theme::color::surface);
    // Feine Trennlinie nach unten, wie in Odysync.
    p.setPen(theme::color::border);
    p.drawLine(0, height() - 1, width(), height() - 1);
}

}  // namespace caruspex
