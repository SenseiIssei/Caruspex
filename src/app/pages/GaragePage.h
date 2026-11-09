// The garage: all cars seen so far, each with its history of checks.
#pragma once

#include <QWidget>

class QLabel;
class QPlainTextEdit;
class QPushButton;
class QVBoxLayout;

namespace caruspex {

class Garage;
class VehicleDiagram;

class GaragePage : public QWidget {
    Q_OBJECT

public:
    GaragePage(Garage* garage, QWidget* parent = nullptr);

    // Highlights the car that is connected right now.
    void setCurrentVin(const QString& vin);

signals:
    void reportRequested();

private:
    void rebuildList();
    void showCar(const QString& vin);

    Garage* garage_ = nullptr;
    QString selected_;
    QString current_;
    QVBoxLayout* list_ = nullptr;
    QWidget* detail_ = nullptr;
    QWidget* empty_ = nullptr;
    VehicleDiagram* picture_ = nullptr;
    QLabel* name_ = nullptr;
    QLabel* meta_ = nullptr;
    QVBoxLayout* history_ = nullptr;
    QPlainTextEdit* note_ = nullptr;
    QPushButton* remove_ = nullptr;
    QPushButton* report_ = nullptr;
};

}  // namespace caruspex
