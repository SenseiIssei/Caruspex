# SocketCAN (Linux)

SocketCAN ist die CAN-Schicht des Linux-Kernels. Jeder CAN-Adapter mit Kerneltreiber lässt sich darüber nutzen: PEAK PCAN-USB, Kvaser, CANable mit der candleLight-Firmware, CAN-Aufsätze für den Raspberry Pi und viele mehr.

## Einrichten

Die Schnittstelle mit der Bitrate des Autos starten (500 kbit/s für OBD über CAN):

```
sudo ip link set can0 up type can bitrate 500000
```

Für CAN FD bei den neuesten Autos, zusätzlich mit einer Datenbitrate:

```
sudo ip link set can0 up type can bitrate 500000 dbitrate 2000000 fd on
```

## Verbinden

Auf der Seite **Verbinden** **CAN-Interface** wählen. Jede CAN-Schnittstelle des Rechners steht in der Liste (`can0`, `can1` und so weiter). Sie wählen, die Bitrate einstellen und für CAN FD den Schalter CAN FD einschalten.

## Was geht

OBD, alle Steuergeräte über UDS, die parallele Suche nach Steuergeräten, VW TP 2.0, das Mithören von Rundfunkwerten und CAN FD.

## Kommandozeile

```
caruspex_cli interfaces
caruspex_cli read --can socketcan:can0@500000
caruspex_cli read --can socketcan:can0+fd@500000
```
