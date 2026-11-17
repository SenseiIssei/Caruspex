# ELM327 über Bluetooth

Kleine Stecker ohne Kabel. Es gibt zwei Arten von Bluetooth, und Caruspex spricht beide.

## Bluetooth LE

Neuere Adapter wie Vgate iCar Pro BLE, vLinker, OBDLink CX und Veepeak BLE nutzen Bluetooth Low Energy. Sie müssen **nicht gekoppelt** werden: Auf der Seite **Verbinden** **Bluetooth** wählen, auf **Nach Bluetooth-Geräten suchen** klicken und den Adapter aus der Liste nehmen. Das geht unter Windows, Linux und macOS.

## Klassisches Bluetooth

Ältere und viele billige Adapter nutzen klassisches Bluetooth mit einem seriellen Profil.

1. Den Adapter einmal in den Bluetooth-Einstellungen des Systems koppeln. Die PIN ist meist `1234` oder `0000`.
2. In Caruspex **Bluetooth** wählen und den gekoppelten Adapter nehmen; die Liste sagt, ob ein Gerät LE, klassisch oder beides kann.

Unter Windows und Linux verbindet sich Caruspex direkt mit gekoppelten klassischen Adaptern. Alternativ legt Windows einen COM-Port für den Adapter an, der wie [ELM327 über USB](elm327-usb.md) funktioniert. Unter macOS gehen klassische Bluetooth-Adapter nur, wenn das System einen seriellen Anschluss für sie anbietet; Adapter mit Bluetooth LE sind dort die bessere Wahl.

## Tipps

- Den Adapter abziehen, wenn er nicht gebraucht wird: Viele billige halten die Autobatterie wach.
- Taucht der Adapter nicht auf, Bluetooth am Rechner aus- und wieder einschalten und darauf achten, dass nicht gleichzeitig eine Handy-App mit ihm verbunden ist.
- Bluetooth ist langsamer als ein Kabel. Mit einem OBDLink fragt Caruspex mehrere Werte auf einmal, das gleicht das meiste aus (siehe [OBDLink und STN](obdlink-stn.md)).
