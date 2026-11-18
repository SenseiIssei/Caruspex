# Kvaser (Windows)

Die CAN-Adapter von Kvaser (Leaf, U100, Memorator und andere) lassen sich unter Windows direkt über die Bibliothek CANlib von Kvaser nutzen. Unter Linux sind sie [SocketCAN](socketcan.md)-Schnittstellen.

## Verbinden

1. Die Kvaser-Treiber installieren. Sie bringen `canlib32.dll` mit, die Caruspex lädt.
2. Auf der Seite **Verbinden** **CAN-Interface** wählen. Jeder Kvaser-Kanal mit einem Gerät dahinter erscheint in der Liste; die virtuellen Kanäle des Treibers fehlen.
3. Die Bitrate wählen (500 kbit/s für OBD über CAN) und verbinden.

## Was geht

OBD, alle Steuergeräte über UDS, die parallele Suche nach Steuergeräten, VW TP 2.0 und das Mithören von Rundfunkwerten. CAN FD über CANlib ist noch nicht eingebaut.

## Kommandozeile

```
caruspex_cli read --can kvaser:0@500000
```
