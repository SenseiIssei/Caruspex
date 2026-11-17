# SLCAN (Lawicel)

SLCAN is een eenvoudig tekstprotocol via een seriële poort voor CAN-sticks: CANable met de slcan-firmware, USBtin, Lawicel CANUSB en veel zelfgebouwde apparaten. Het werkt op Windows, Linux en macOS.

## Verbinden

1. Sluit de stick aan op de computer en met een kabel op de OBD-aansluiting: CAN high op pin 6, CAN low op pin 14, massa op pin 4 en 5.
2. Kies op de pagina **Verbinden** voor **CAN-interface**. SLCAN-sticks worden gevonden door het automatisch zoeken naar poorten en verschijnen in de lijst.
3. Kies de bitrate (500 kbit/s voor OBD via CAN) en verbind.

## Wat werkt

Zoals bij elke ruwe CAN-interface: OBD, elke regeleenheid via UDS, de parallelle zoektocht naar regeleenheden, VW TP 2.0 en meeluisteren met uitgezonden waarden. CAN FD is via SLCAN niet mogelijk.

## Tips

- Een CANable met de candleLight-firmware is geen SLCAN-apparaat: op Linux verschijnt hij als [SocketCAN](socketcan.md)-interface.
- De stick heeft geen afsluitweerstand nodig op de OBD-aansluiting, de auto heeft er zelf een.

## Opdrachtregel

```
caruspex_cli read --can slcan:COM5@500000
caruspex_cli read --can slcan:/dev/ttyACM0@500000
```
