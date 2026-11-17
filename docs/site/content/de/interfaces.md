# Adapter und Schnittstellen

Caruspex findet selbst heraus, was eingesteckt ist und was das Auto spricht. Egal auf welchem Weg: Überall gehen dieselben Funktionen, also OBD-II, alle Steuergeräte über UDS, Mithören von Rundfunkwerten, Aufnehmen und Abspielen.

| Schnittstelle | Beispiele | Seite |
|---|---|---|
| ELM327 über USB | billige ELM327-Kabel, Vgate, Veepeak | [ELM327 über USB](elm327-usb.md) |
| ELM327 über Bluetooth | Stecker mit klassischem Bluetooth und Bluetooth LE, vLinker, Vgate iCar Pro | [ELM327 über Bluetooth](elm327-bluetooth.md) |
| ELM327 über WLAN | Stecker mit eigenem WLAN | [ELM327 über WLAN](elm327-wifi.md) |
| STN-Chips | OBDLink SX, EX, MX+, CX | [OBDLink und STN](obdlink-stn.md) |
| KKL-Kabel (VAG 409.1) | Kabel mit FTDI oder CP2102 | [KKL-Kabel](kkl.md) |
| K+DCAN-Kabel | BMW-Kabel mit Schalter | [K+DCAN-Kabel](kdcan.md) |
| ENET-Kabel | BMW und Mini ab der F-Baureihe | [BMW ENET](enet.md) |
| SAE J2534 PassThru | Tactrix OpenPort 2.0, Mongoose, VCX Nano, Scanmatik | [J2534](j2534.md) |
| SLCAN / Lawicel | CANable, USBtin, CANUSB | [SLCAN](slcan.md) |
| SocketCAN (Linux) | PEAK, Kvaser, CANable mit candleLight, CAN-Aufsätze für den Raspberry Pi | [SocketCAN](socketcan.md) |
| PEAK PCAN (Windows) | PCAN-USB, PCAN-USB FD | [PCAN](pcan.md) |
| Kvaser (Windows) | Kvaser Leaf, U100 | [Kvaser](kvaser.md) |
| DoIP-Kabel | viele Autos ab etwa 2016 | [DoIP](doip.md) |

## Was kann welche?

Ein einfacher ELM327 reicht für Motor, Getriebe und bei den meisten Autos ab etwa 2008 auch für alle anderen Steuergeräte über UDS. Das alte Protokoll KWP1281 des VW-Konzerns, das jedes einzelne Byte bestätigt, kann er nicht: Für VW, Audi, Seat und Skoda vor etwa 2004 braucht man ein KKL-Kabel.

Die rohen CAN-Schnittstellen (J2534, SLCAN, SocketCAN, PCAN, Kvaser) geben Caruspex die nackten CAN-Botschaften. Das Transportprotokoll macht Caruspex dann selbst, und damit gehen eine schnelle, parallele Suche nach Steuergeräten und VW TP 2.0 für Autos aus dem VW-Konzern von etwa 2004 bis 2012.

![Adaptertest](screenshot-adapter.png)

## Ist mein Adapter echt?

Der **Adaptertest** fragt den Adapter nach den Befehlen jeder ELM327-Version, misst die Antwortzeiten und sagt ehrlich, was daraus folgt: Viele billige Adapter melden Version 1.5 oder 2.1, kennen aber nur die Befehle von 1.0. Caruspex schreibt nie etwas in den Adapter.

> Bisher geprüft: Ein OBDLink mit dem Chip STN2232 hat den Adaptertest bestanden, ohne Auto. Alle anderen Adapter und Schnittstellen sind nach ihrer Dokumentation gebaut, aber noch nicht am echten Auto getestet.
