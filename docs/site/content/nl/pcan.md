# PEAK PCAN (Windows)

De CAN-adapters van PEAK-System (PCAN-USB, PCAN-USB FD, PCAN-USB Pro en andere) kun je op Windows direct gebruiken, zonder J2534-driver. Op Linux zijn het [SocketCAN](socketcan.md)-interfaces.

## Verbinden

1. Installeer de apparaatdriver van PEAK. Die brengt de bibliotheek `PCANBasic.dll` mee, die Caruspex laadt.
2. Kies op de pagina **Verbinden** voor **CAN-interface**. Elk PCAN-kanaal met een apparaat erachter verschijnt vanzelf in de lijst; kanalen die al door een ander programma gebruikt worden, worden weggelaten.
3. Kies de bitrate (500 kbit/s voor OBD via CAN) en verbind.

## Wat werkt

OBD, elke regeleenheid via UDS, de parallelle zoektocht naar regeleenheden, VW TP 2.0 en meeluisteren met uitgezonden waarden. CAN FD via PCAN-Basic is nog niet ingebouwd.

## Opdrachtregel

```
caruspex_cli read --can pcan:USB1@500000
```
