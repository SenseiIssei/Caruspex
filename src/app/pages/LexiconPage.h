#pragma once

#include <QWidget>

QT_BEGIN_NAMESPACE
class QLabel;
class QLineEdit;
class QTableWidget;
QT_END_NAMESPACE

namespace caruspex {

// Offline-Nachschlagewerk fuer Fehlercodes, funktioniert ohne Auto.
class LexiconPage : public QWidget {
    Q_OBJECT

public:
    explicit LexiconPage(QWidget* parent = nullptr);

private:
    void applyFilter(const QString& needle);

    QLineEdit* search_ = nullptr;
    QTableWidget* table_ = nullptr;
    QLabel* count_ = nullptr;
};

}  // namespace caruspex
