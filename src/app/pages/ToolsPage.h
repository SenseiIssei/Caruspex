#pragma once

#include <QWidget>

#include "app/Session.h"

QT_BEGIN_NAMESPACE
class QLabel;
class QLineEdit;
class QPlainTextEdit;
class QPushButton;
class QTableWidget;
QT_END_NAMESPACE

namespace caruspex {

// Bereitschaftstests, Umgebungsdaten, Fahrzeugkennung und Rohbefehle.
class ToolsPage : public QWidget {
    Q_OBJECT

public:
    explicit ToolsPage(QWidget* parent = nullptr);

public slots:
    void onStatus(const caruspex::StatusRow& row);
    void onVehicleInfo(const QString& vin, const QString& freezeCode,
                       const QVector<caruspex::LiveRow>& freezeRows);
    void onRawReply(const QString& reply);
    void setConnected(bool connected);

signals:
    void detailsRequested();
    void rawRequested(const QString& command);

private:
    void sendRaw();

    QLabel* readinessSummary_ = nullptr;
    QTableWidget* readiness_ = nullptr;
    QLabel* vin_ = nullptr;
    QLabel* freezeTitle_ = nullptr;
    QTableWidget* freeze_ = nullptr;
    QPushButton* loadButton_ = nullptr;
    QLineEdit* command_ = nullptr;
    QPushButton* sendButton_ = nullptr;
    QPlainTextEdit* replies_ = nullptr;
};

}  // namespace caruspex
