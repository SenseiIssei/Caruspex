// EOBD-Dienste: Fehler lesen/loeschen, Live-Werte, Freeze Frame, Readiness, FIN.
#pragma once

#include <set>
#include <string>
#include <vector>

#include "core/Elm327.h"

namespace caruspex {

struct Fault {
    std::string code;
    std::string kind;   // "gespeichert", "sporadisch", "dauerhaft"
    std::string text;
    bool throttleRelated = false;
};

struct ReadinessItem {
    std::string name;
    bool ready = false;
};

struct VehicleStatus {
    bool milOn = false;
    int faultCount = 0;
    std::vector<ReadinessItem> readiness;
    bool valid = false;
};

// Ein Ergebnis aus der laufenden Bauteilueberwachung (Modus 06).
//
// Das ist der ausfuehrlichste Teil von EOBD: das Steuergeraet nennt zu jedem
// internen Test nicht nur bestanden oder nicht, sondern den Messwert samt
// Unter- und Obergrenze. Man sieht also, wie knapp ein Bauteil noch im Rahmen
// liegt, lange bevor die Fehlerlampe angeht.
struct MonitorResult {
    int testId = 0;
    int componentId = 0;
    std::string name;        // Klartext zum Test, falls hinterlegt
    std::string component;   // Klartext zur Baugruppe
    double value = 0.0;
    double minimum = 0.0;
    double maximum = 0.0;
    std::string unit;
    bool hasMinimum = false;
    bool hasMaximum = false;
    bool passed = true;
};

// Ergebnis einer Lambdasonden-Pruefung (Modus 05). Gibt es nur bei Fahrzeugen
// ohne CAN, also genau bei den aelteren wie dem Audi A2.
struct OxygenSensorTest {
    int testId = 0;
    int sensor = 0;
    std::string name;
    double value = 0.0;
    std::string unit;
};

// Kennungen aus Modus 09.
struct VehicleIdentity {
    std::string vin;
    std::string calibrationId;
    std::string calibrationVerification;
    std::string ecuName;
};

struct LiveValue {
    int pid = 0;
    std::string name;
    double value = 0.0;
    std::string text;   // gesetzt, wenn es ein Statuswert ist
    std::string unit;
    bool warn = false;
};

class Obd2 {
public:
    explicit Obd2(Elm327& elm) : elm_(elm) {}

    std::vector<Fault> readFaults(std::string& error);
    bool clearFaults(std::string& error);
    VehicleStatus readStatus();
    std::set<int> supportedPids();
    bool readLive(int pid, LiveValue& out);
    std::vector<LiveValue> readLive(const std::vector<int>& pids);
    std::string readVin();
    std::vector<LiveValue> readFreezeFrame(std::string& triggerCode);

    // Modus 09: Fahrgestellnummer, Kalibrierkennung, Pruefsumme, Name des
    // Steuergeraets. Was fehlt, bleibt leer.
    VehicleIdentity readIdentity();

    // Modus 06: die laufende Bauteilueberwachung mit Messwert und Grenzen.
    // Auf der K-Line werden die Tests einzeln abgefragt, auf CAN alle auf
    // einmal. Beides wird hier abgedeckt.
    std::vector<MonitorResult> readMonitorResults(std::string& error);

    // Modus 05: Lambdasonden-Pruefergebnisse. Gibt es nur ohne CAN.
    std::vector<OxygenSensorTest> readOxygenSensorTests(std::string& error);

private:
    std::vector<Fault> readCodes(const std::string& service, int sid, const char* kind);

    Elm327& elm_;
};

}  // namespace caruspex
