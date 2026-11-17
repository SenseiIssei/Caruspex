# Kvaser (Windows)

De CAN-adapters van Kvaser (Leaf, U100, Memorator en andere) kun je op Windows direct gebruiken via Kvasers bibliotheek CANlib. Op Linux zijn het [SocketCAN](socketcan.md)-interfaces.

## Verbinden

1. Installeer de Kvaser-drivers. Die brengen `canlib32.dll` mee, die Caruspex laadt.
2. Kies op de pagina **Verbinden** voor **CAN-interface**. Elk Kvaser-kanaal met een apparaat erachter verschijnt in de lijst; de virtuele kanalen van de driver worden weggelaten.
3. Kies de bitrate (500 kbit/s voor OBD via CAN) en verbind.

## Wat werkt

OBD, elke regeleenheid via UDS, de parallelle zoektocht naar regeleenheden, VW TP 2.0 en meeluisteren met uitgezonden waarden. CAN FD via CANlib is nog niet ingebouwd.

## Opdrachtregel

```
caruspex_cli read --can kvaser:0@500000
```
