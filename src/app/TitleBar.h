// Eigene Titelleiste fuer das rahmenlose Fenster.
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
    // Wird gerufen, wenn das Fenster maximiert oder wiederhergestellt wurde,
    // damit der mittlere Knopf sein Zeichen wechselt.
    void updateMaximiseState(bool maximised);

signals:
    void minimiseRequested();
    void maximiseToggled();
    void closeRequested();

protected:
    // Ziehen und Doppelklick uebernimmt der Fenstermanager, damit Andocken
    // und Aero Snap weiter funktionieren.
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
