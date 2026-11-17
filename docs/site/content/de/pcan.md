# PEAK PCAN (Windows)

Die CAN-Adapter von PEAK-System (PCAN-USB, PCAN-USB FD, PCAN-USB Pro und andere) lassen sich unter Windows direkt nutzen, ohne J2534-Treiber. Unter Linux sind sie [SocketCAN](socketcan.md)-Schnittstellen.

## Verbinden

1. Den Gerätetreiber von PEAK installieren. Er bringt die Bibliothek `PCANBasic.dll` mit, die Caruspex lädt.
2. Auf der Seite **Verbinden** **CAN-Interface** wählen. Jeder PCAN-Kanal mit einem Gerät dahinter erscheint von selbst in der Liste; Kanäle, die ein anderes Programm schon belegt, fehlen.
3. Die Bitrate wählen (500 kbit/s für OBD über CAN) und verbinden.

## Was geht

OBD, alle Steuergeräte über UDS, die parallele Suche nach Steuergeräten, VW TP 2.0 und das Mithören von Rundfunkwerten. CAN FD über PCAN-Basic ist noch nicht eingebaut.

## Kommandozeile

```
caruspex_cli read --can pcan:USB1@500000
```
