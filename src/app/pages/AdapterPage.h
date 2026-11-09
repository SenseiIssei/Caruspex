// What the adapter really is: version, chip, speed, the commands it truly
// understands, and an honest guess whether it is a clone.
#pragma once

#include <QWidget>

#include "app/Session.h"

class QLabel;
class QPushButton;
class QTableWidget;
class QVBoxLayout;

namespace caruspex {

class Spinner;
class StatTile;

class AdapterPage : public QWidget {
    Q_OBJECT

public:
    explicit AdapterPage(QWidget* parent = nullptr);

public slots:
    void setConnected(bool connected);
    void onReport(const caruspex::AdapterRow& row);

signals:
    void probeRequested();

private:
    QPushButton* start_ = nullptr;
    Spinner* spinner_ = nullptr;
    QLabel* verdict_ = nullptr;
    StatTile* version_ = nullptr;
    StatTile* chip_ = nullptr;
    StatTile* speed_ = nullptr;
    StatTile* voltage_ = nullptr;
    QVBoxLayout* facts_ = nullptr;
    QTableWidget* checks_ = nullptr;
};

}  // namespace caruspex
