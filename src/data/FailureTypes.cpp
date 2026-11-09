// The failure type byte of a UDS fault code (SAE J2012-DA, also listed in
// ISO 15031-6 and ISO 14229-1 annex D).
//
// A UDS DTC has three bytes: the first two are the familiar code ("C0035"),
// the third says what kind of fault it is. The high nibble is the category
// (electrical, signal, internal, mechanical ...), the low nibble the exact
// type within it. Only the subtypes with a well known meaning are listed;
// for the others the category is still worth showing, so the text falls
// back to it. 0xF0 to 0xFF belong to the manufacturer.

#include <cstdio>
#include <string>

#include "core/Lang.h"
#include "data/EcuAddresses.h"

namespace caruspex {
namespace {

struct FailureType {
    int value;
    Text text;
};

const FailureType kFailureTypes[] = {
    // 0x0_: general failure information, the category without detail.
    {0x01, {"allgemeiner elektrischer Fehler", "general electrical failure"}},
    {0x02, {"allgemeiner Signalfehler", "general signal failure"}},
    {0x03, {"Fehler im Frequenz- oder PWM-Signal", "FM (frequency modulated) or PWM failure"}},
    {0x04, {"interner Fehler des Systems", "system internal failure"}},
    {0x05, {"Programmierfehler des Systems", "system programming failure"}},
    {0x06, {"Fehler aus einer Berechnung", "algorithm based failure"}},
    {0x07, {"mechanischer Fehler", "mechanical failure"}},
    {0x08, {"Fehler im Bussignal oder in der Botschaft", "bus signal or message failure"}},
    {0x09, {"Bauteilfehler", "component failure"}},

    // 0x1_: electrical.
    {0x11, {"Kurzschluss nach Masse", "circuit short to ground"}},
    {0x12, {"Kurzschluss nach Plus", "circuit short to battery"}},
    {0x13, {"Unterbrechung", "circuit open"}},
    {0x14, {"Kurzschluss nach Masse oder Unterbrechung", "circuit short to ground or open"}},
    {0x15, {"Kurzschluss nach Plus oder Unterbrechung", "circuit short to battery or open"}},
    {0x16, {"Spannung unter der Schwelle", "circuit voltage below threshold"}},
    {0x17, {"Spannung über der Schwelle", "circuit voltage above threshold"}},
    {0x18, {"Strom unter der Schwelle", "circuit current below threshold"}},
    {0x19, {"Strom über der Schwelle", "circuit current above threshold"}},
    {0x1A, {"Widerstand unter der Schwelle", "circuit resistance below threshold"}},
    {0x1B, {"Widerstand über der Schwelle", "circuit resistance above threshold"}},
    {0x1C, {"Spannung außerhalb des Bereichs", "circuit voltage out of range"}},
    {0x1D, {"Strom außerhalb des Bereichs", "circuit current out of range"}},
    {0x1E, {"Widerstand außerhalb des Bereichs", "circuit resistance out of range"}},
    {0x1F, {"sporadischer Fehler im Stromkreis", "circuit intermittent"}},

    // 0x2_: signal.
    {0x21, {"Signalamplitude zu klein", "signal amplitude below minimum"}},
    {0x22, {"Signalamplitude zu groß", "signal amplitude above maximum"}},
    {0x23, {"Signal bleibt auf niedrig stehen", "signal stuck low"}},
    {0x24, {"Signal bleibt auf hoch stehen", "signal stuck high"}},
    {0x25, {"Signalform falsch", "signal shape or waveform failure"}},
    {0x26, {"Signal ändert sich zu langsam", "signal rate of change below threshold"}},
    {0x27, {"Signal ändert sich zu schnell", "signal rate of change above threshold"}},
    {0x28, {"Nullpunkt oder Grundwert des Signals außerhalb des Bereichs",
            "signal bias level out of range or zero adjustment failure"}},
    {0x29, {"Signal ungültig", "signal invalid"}},
    {0x2A, {"Signal bleibt innerhalb des Bereichs stehen", "signal stuck in range"}},
    {0x2B, {"Signal mit einem anderen verkoppelt", "signal cross coupled"}},
    {0x2F, {"Signal unruhig", "signal erratic"}},

    // 0x3_: frequency and pulse width modulated signals.
    {0x31, {"kein Signal", "no signal"}},
    {0x32, {"Low-Zeit des Signals zu kurz", "signal low time below minimum"}},
    {0x33, {"Low-Zeit des Signals zu lang", "signal low time above maximum"}},
    {0x34, {"High-Zeit des Signals zu kurz", "signal high time below minimum"}},
    {0x35, {"High-Zeit des Signals zu lang", "signal high time above maximum"}},
    {0x36, {"Signalfrequenz zu niedrig", "signal frequency too low"}},
    {0x37, {"Signalfrequenz zu hoch", "signal frequency too high"}},
    {0x38, {"Signalfrequenz falsch", "signal frequency incorrect"}},
    {0x39, {"zu wenige Impulse", "signal has too few pulses"}},
    {0x3A, {"zu viele Impulse", "signal has too many pulses"}},

    // 0x4_: inside the control unit.
    {0x41, {"Prüfsummenfehler", "general checksum failure"}},
    {0x42, {"Speicherfehler", "general memory failure"}},
    {0x43, {"Fehler im Sonderspeicher", "special memory failure"}},
    {0x44, {"Fehler im Datenspeicher (RAM)", "data memory failure"}},
    {0x45, {"Fehler im Programmspeicher", "program memory failure"}},
    {0x46, {"Fehler im Kalibrier- oder Parameterspeicher", "calibration or parameter memory failure"}},
    {0x47, {"Fehler der Überwachung (Watchdog)", "watchdog or safety processor failure"}},
    {0x48, {"Fehler der Überwachungssoftware", "supervision software failure"}},
    {0x49, {"interner elektronischer Fehler", "internal electronic failure"}},
    {0x4A, {"falsches Bauteil eingebaut", "incorrect component installed"}},
    {0x4B, {"Übertemperatur", "over temperature"}},

    // 0x5_: programming and configuration.
    {0x51, {"nicht programmiert", "not programmed"}},
    {0x52, {"nicht aktiviert", "not activated"}},
    {0x53, {"deaktiviert", "deactivated"}},
    {0x54, {"Kalibrierung fehlt", "missing calibration"}},
    {0x55, {"nicht konfiguriert", "not configured"}},
    {0x56, {"ungültige oder unpassende Konfiguration", "invalid or incompatible configuration"}},
    {0x57, {"ungültige oder unpassende Software", "invalid or incompatible software component"}},

    // 0x6_: found by a calculation or comparison.
    {0x61, {"Fehler bei der Signalberechnung", "signal calculation failure"}},
    {0x62, {"Signalvergleich fehlgeschlagen", "signal compare failure"}},
    {0x63, {"Schutzabschaltung nach Zeitüberschreitung", "circuit or component protection time-out"}},
    {0x64, {"Signal unplausibel", "signal plausibility failure"}},
    {0x65, {"zu wenige Signalwechsel", "signal has too few transitions or events"}},
    {0x66, {"zu viele Signalwechsel", "signal has too many transitions or events"}},
    {0x67, {"Signal nach dem Ereignis falsch", "signal incorrect after event"}},
    {0x68, {"Ereignisinformation", "event information"}},

    // 0x7_: mechanical.
    {0x71, {"Stellglied klemmt", "actuator stuck"}},
    {0x72, {"Stellglied klemmt offen", "actuator stuck open"}},
    {0x73, {"Stellglied klemmt geschlossen", "actuator stuck closed"}},
    {0x74, {"Stellglied rutscht durch", "actuator slipping"}},
    {0x75, {"Notlaufstellung nicht erreichbar", "emergency position not reachable"}},
    {0x76, {"falsche Einbaulage", "wrong mounting position"}},
    {0x77, {"Sollstellung nicht erreichbar", "commanded position not reachable"}},
    {0x78, {"Einstellung oder Ausrichtung falsch", "alignment or adjustment incorrect"}},
    {0x79, {"mechanische Verbindung defekt", "mechanical linkage failure"}},
    {0x7A, {"Undichtigkeit", "fluid leak or seal failure"}},
    {0x7B, {"Flüssigkeitsstand zu niedrig", "low fluid level"}},

    // 0x8_: data bus.
    {0x81, {"ungültige Daten empfangen", "invalid serial data received"}},
    {0x82, {"Botschaftszähler falsch oder nicht aktualisiert",
            "alive or sequence counter incorrect or not updated"}},
    {0x83, {"Prüfwert der Botschaft falsch", "value of signal protection calculation incorrect"}},
    {0x84, {"Signal unter dem zulässigen Bereich", "signal below allowable range"}},
    {0x85, {"Signal über dem zulässigen Bereich", "signal above allowable range"}},
    {0x86, {"Signal ungültig", "invalid signal"}},
    {0x87, {"Botschaft fehlt", "missing message"}},
    {0x88, {"Bus abgeschaltet (Bus off)", "bus off"}},

    // 0x9_: the component as a whole.
    {0x91, {"Kennwerte außerhalb", "parametric"}},
    {0x92, {"Leistung zu gering oder falsche Funktion", "performance or incorrect operation"}},
    {0x93, {"keine Funktion", "no operation"}},
    {0x94, {"unerwartete Funktion", "unexpected operation"}},
    {0x95, {"falsch montiert", "incorrect assembly"}},
    {0x96, {"interner Bauteilfehler", "component internal failure"}},
    {0x97, {"Funktion behindert oder blockiert", "component or system operation obstructed or blocked"}},
    {0x98, {"Übertemperatur des Bauteils", "component or system over temperature"}},
};

// Name of the category in the high nibble.
const char* categoryName(int category) {
    switch (category) {
        case 0x0: return L("allgemeiner Fehler", "general failure");
        case 0x1: return L("elektrischer Fehler", "electrical failure");
        case 0x2: return L("Signalfehler", "signal failure");
        case 0x3: return L("Fehler im Frequenz- oder PWM-Signal", "FM or PWM failure");
        case 0x4: return L("interner Fehler des Steuergeräts", "system internal failure");
        case 0x5: return L("Programmier- oder Konfigurationsfehler", "programming failure");
        case 0x6: return L("Fehler aus einer Berechnung", "algorithm based failure");
        case 0x7: return L("mechanischer Fehler", "mechanical failure");
        case 0x8: return L("Fehler im Bussignal", "bus signal or message failure");
        case 0x9: return L("Bauteilfehler", "component failure");
        default: return nullptr;
    }
}

}  // namespace

std::string failureTypeText(int failureType) {
    failureType &= 0xFF;
    if (failureType == 0x00) return {};
    for (const FailureType& f : kFailureTypes) {
        if (f.value == failureType) return f.text.get();
    }
    if (failureType >= 0xF0) return L("herstellerspezifische Fehlerart", "manufacturer specific failure type");
    if (const char* category = categoryName(failureType >> 4)) return category;
    // 0xA0 to 0xEF are reserved by the standard; show the raw value.
    char buffer[48];
    std::snprintf(buffer, sizeof(buffer), "%s 0x%02X", L("Fehlerart", "failure type"), failureType);
    return buffer;
}

}  // namespace caruspex
