// Custom title bar for the frameless window: logo, the connection capsule in
// the middle, quick toggles and the window buttons.
#pragma once

#include <QWidget>

QT_BEGIN_NAMESPACE
class QLabel;
class QPushButton;
QT_END_NAMESPACE

namespace caruspex {

class Spinner;

class TitleBar : public QWidget {
    Q_OBJECT

public:
    enum class Link { Offline, Connecting, Online, Error };

    explicit TitleBar(QWidget* parent = nullptr);

    void setTitle(const QString& title);
    // Called when the window was maximised or restored, so the middle button
    // switches its symbol.
    void updateMaximiseState(bool maximised);
    // The capsule in the middle: state colour and a short text.
    void setConnection(Link state, const QString& text);
    void setLogOpen(bool open);
    // A newer version exists: a small button that opens its release page.
    void setUpdateAvailable(const QString& version, const QString& url);

signals:
    void minimiseRequested();
    void maximiseToggled();
    void closeRequested();
    void connectionClicked();
    void logToggled();
    void sidebarToggled();

protected:
    // Dragging and double-click are left to the window manager so snapping
    // and Aero Snap keep working.
    void mousePressEvent(QMouseEvent* event) override;
    void mouseDoubleClickEvent(QMouseEvent* event) override;
    void paintEvent(QPaintEvent* event) override;

private:
    QLabel* title_ = nullptr;
    QPushButton* capsule_ = nullptr;
    Spinner* capsuleSpinner_ = nullptr;
    QPushButton* log_ = nullptr;
    QPushButton* update_ = nullptr;
    QPushButton* sidebar_ = nullptr;
    QPushButton* minimise_ = nullptr;
    QPushButton* maximise_ = nullptr;
    QPushButton* close_ = nullptr;
};

}  // namespace caruspex
