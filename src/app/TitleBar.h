// Custom title bar for the frameless window.
#pragma once

#include <QWidget>

QT_BEGIN_NAMESPACE
class QLabel;
class QPushButton;
QT_END_NAMESPACE

namespace caruspex {

class TitleBar : public QWidget {
    Q_OBJECT

public:
    explicit TitleBar(QWidget* parent = nullptr);

    void setTitle(const QString& title);
    // Called when the window was maximised or restored, so the middle button
    // switches its symbol.
    void updateMaximiseState(bool maximised);

signals:
    void minimiseRequested();
    void maximiseToggled();
    void closeRequested();

protected:
    // Dragging and double-click are left to the window manager so snapping
    // and Aero Snap keep working.
    void mousePressEvent(QMouseEvent* event) override;
    void mouseDoubleClickEvent(QMouseEvent* event) override;
    void paintEvent(QPaintEvent* event) override;

private:
    QLabel* title_ = nullptr;
    QPushButton* minimise_ = nullptr;
    QPushButton* maximise_ = nullptr;
    QPushButton* close_ = nullptr;
};

}  // namespace caruspex
