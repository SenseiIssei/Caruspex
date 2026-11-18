# Adapters en interfaces

Caruspex zoekt zelf uit wat er is aangesloten en wat de auto spreekt. Welke weg je ook gebruikt om in de auto te komen, dezelfde functies werken: OBD-II, elke regeleenheid via UDS, meeluisteren met uitgezonden waarden, opnemen en afspelen.

| Interface | Voorbeelden | Pagina |
|---|---|---|
| ELM327 via USB | goedkope ELM327-kabels, Vgate, Veepeak | [ELM327 via USB](elm327-usb.md) |
| ELM327 via Bluetooth | stekkers met klassiek Bluetooth en Bluetooth LE, vLinker, Vgate iCar Pro | [ELM327 via Bluetooth](elm327-bluetooth.md) |
| ELM327 via wifi | stekkers die hun eigen wifi-netwerk openen | [ELM327 via wifi](elm327-wifi.md) |
| STN-chips | OBDLink SX, EX, MX+, CX | [OBDLink en STN](obdlink-stn.md) |
| KKL-kabel (VAG 409.1) | kabels met FTDI of CP2102 | [KKL-kabel](kkl.md) |
| K+DCAN-kabel | BMW-kabels met een schakelaar | [K+DCAN-kabel](kdcan.md) |
| ENET-kabel | BMW en Mini vanaf de F-serie | [BMW ENET](enet.md) |
| SAE J2534 PassThru | Tactrix OpenPort 2.0, Mongoose, VCX Nano, Scanmatik | [J2534](j2534.md) |
| SLCAN / Lawicel | CANable, USBtin, CANUSB | [SLCAN](slcan.md) |
| SocketCAN (Linux) | PEAK, Kvaser, CANable met candleLight, CAN-hats voor de Raspberry Pi | [SocketCAN](socketcan.md) |
| PEAK PCAN (Windows) | PCAN-USB, PCAN-USB FD | [PCAN](pcan.md) |
| Kvaser (Windows) | Kvaser Leaf, U100 | [Kvaser](kvaser.md) |
| DoIP-kabel | veel auto's vanaf ongeveer 2016 | [DoIP](doip.md) |

## Wat kan elk ervan?

Een gewone ELM327 is genoeg voor de motor, de versnellingsbak en, bij de meeste auto's vanaf ongeveer 2008, alle andere regeleenheden via UDS. Hij kan niet overweg met het oude protocol KWP1281 van de Volkswagen-groep, dat elke afzonderlijke byte bevestigt: voor VW, Audi, Seat en Skoda van vóór ongeveer 2004 heb je een KKL-kabel nodig.

De ruwe CAN-interfaces (J2534, SLCAN, SocketCAN, PCAN, Kvaser) geven Caruspex de kale CAN-frames. Caruspex doet dan zelf het transportprotocol, wat een snelle parallelle zoektocht naar regeleenheden mogelijk maakt en VW TP 2.0 voor auto's van de Volkswagen-groep van ongeveer 2004 tot 2012.

![Adaptertest](screenshot-adapter.png)

## Is mijn adapter echt?

De **Adaptertest** vraagt de adapter naar de commando's van elke ELM327-versie, meet de reactietijden en zegt eerlijk wat daaruit volgt: veel goedkope adapters melden versie 1.5 of 2.1, maar kennen alleen de commando's van 1.0. Caruspex schrijft nooit iets naar de adapter.

> Tot nu toe getest: een OBDLink met de STN2232-chip is door de adaptertest gekomen, zonder auto. Alle andere adapters en interfaces zijn gebouwd volgens hun documentatie, maar nog niet getest op een echte auto.
