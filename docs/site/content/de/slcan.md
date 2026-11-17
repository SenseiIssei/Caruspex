# SLCAN (Lawicel)

SLCAN ist ein einfaches Textprotokoll über einen seriellen Anschluss für CAN-Sticks: CANable mit der slcan-Firmware, USBtin, Lawicel CANUSB und viele selbst gebaute Geräte. Es geht unter Windows, Linux und macOS.

## Verbinden

1. Den Stick an den Rechner und mit einem Kabel an die OBD-Buchse anschließen: CAN High an Pin 6, CAN Low an Pin 14, Masse an Pin 4 und 5.
2. Auf der Seite **Verbinden** **CAN-Interface** wählen. SLCAN-Sticks findet die automatische Suche über die Anschlüsse, sie erscheinen in der Liste.
3. Die Bitrate wählen (500 kbit/s für OBD über CAN) und verbinden.

## Was geht

Wie bei jeder rohen CAN-Schnittstelle: OBD, alle Steuergeräte über UDS, die parallele Suche nach Steuergeräten, VW TP 2.0 und das Mithören von Rundfunkwerten. CAN FD geht über SLCAN nicht.

## Tipps

- Ein CANable mit der candleLight-Firmware ist kein SLCAN-Gerät: Unter Linux erscheint es als [SocketCAN](socketcan.md)-Schnittstelle.
- Der Stick braucht an der OBD-Buchse keinen Abschlusswiderstand, das Auto hat eigene.

## Kommandozeile

```
caruspex_cli read --can slcan:COM5@500000
caruspex_cli read --can slcan:/dev/ttyACM0@500000
```
