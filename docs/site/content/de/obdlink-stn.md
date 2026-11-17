# OBDLink und STN-Chips

Die OBDLink-Adapter (SX, EX, MX+, CX, LX) nutzen STN-Chips von OBD Solutions. Sie verstehen alles, was ein ELM327 kann, und mehr; Caruspex erkennt sie selbst und nutzt die Extras.

## Was Caruspex nutzt

- **Sammelabfragen**: Caruspex fragt bis zu 24 Live-Werte auf einmal statt 6. Über Bluetooth wird eine Runde Werte damit fast doppelt so schnell.
- **Mithören**: Rundfunkwerte (zum Beispiel bei einem Tesla) werden mit den eigenen Befehlen des STN mitgeschnitten, die nach einer festen Zahl Botschaften von selbst aufhören.
- **Ford MS-CAN und GM Single-Wire-CAN**: Die Karosseriesteuergeräte von Ford und GM sitzen nicht am normalen Diagnose-CAN. Mit einem Adapter, der den zusätzlichen Transceiver hat, bietet die Seite **Alle Steuergeräte** "Ford: Karosseriegeräte am MS-CAN" oder "GM: Karosseriegeräte am Single-Wire-CAN" an.

| Chip | Adapter | MS-CAN | SW-CAN |
|---|---|---|---|
| STN2230 bis STN2232 | OBDLink EX | ja | nein |
| STN1150 bis 1152, 1170, 2120, 2255, 2256 | OBDLink MX+ und andere | ja | ja |
| andere | OBDLink SX, LX, CX | nein | nein |

## Verbinden

OBDLink-Adapter werden wie jeder ELM327 verbunden: über USB (EX und SX), Bluetooth (MX+, LX, CX) oder wie unter [ELM327 über USB](elm327-usb.md) und [ELM327 über Bluetooth](elm327-bluetooth.md) beschrieben.

## Geprüft

Ein OBDLink mit dem Chip STN2232 hat den Adaptertest bestanden: Chip, Firmware und Befehle wurden richtig erkannt. An einem Auto war er noch nicht, deshalb sind Sammelabfragen, MS-CAN und SW-CAN an echter Hardware noch nicht bestätigt.
