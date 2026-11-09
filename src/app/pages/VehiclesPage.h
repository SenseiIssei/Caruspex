// Every car Caruspex knows, from 1980 to today: search by name, check a VIN,
// and see for each generation which socket it has, where it sits and what
// works with it.
#pragma once

#include <QWidget>

#include "data/VehicleCatalog.h"

class QLabel;
class QLineEdit;
class QListWidget;
class QPushButton;
class QSpinBox;
class QVBoxLayout;

namespace caruspex {

class VehicleDiagram;

class VehiclesPage : public QWidget {
    Q_OBJECT

public:
    explicit VehiclesPage(QWidget* parent = nullptr);

    // Shows a VIN, e.g. the one of the connected car.
    void showVin(const QString& vin);

signals:
    // The user wants to connect to this car; the connect page takes it over.
    void useForConnection(const caruspex::VehicleGeneration* generation, int year);

private:
    void runSearch(const QString& text);
    void showGeneration(const VehicleGeneration* generation);
    void checkVin();

    QLineEdit* search_ = nullptr;
    QListWidget* results_ = nullptr;
    QLabel* count_ = nullptr;

    QLineEdit* vinEdit_ = nullptr;
    QVBoxLayout* vinResult_ = nullptr;

    QWidget* detail_ = nullptr;
    VehicleDiagram* picture_ = nullptr;
    QLabel* name_ = nullptr;
    QLabel* years_ = nullptr;
    QSpinBox* year_ = nullptr;
    QLabel* support_ = nullptr;
    QLabel* diagnosis_ = nullptr;
    QLabel* connector_ = nullptr;
    QLabel* location_ = nullptr;
    QLabel* note_ = nullptr;
    QPushButton* use_ = nullptr;
    const VehicleGeneration* current_ = nullptr;
};

}  // namespace caruspex

Q_DECLARE_METATYPE(const caruspex::VehicleGeneration*)
