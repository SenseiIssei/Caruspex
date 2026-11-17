# Kvaser (Windows)

Adaptéry CAN od Kvaseru (Leaf, U100, Memorator a další) lze ve Windows používat přímo přes knihovnu CANlib od Kvaseru. V Linuxu jsou to rozhraní [SocketCAN](socketcan.md).

## Připojení

1. Nainstalujte ovladače Kvaser. Přinášejí `canlib32.dll`, kterou Caruspex načte.
2. Na stránce **Připojit** vyberte **Rozhraní CAN**. V seznamu se objeví každý kanál Kvaser, za kterým je zařízení; virtuální kanály ovladače jsou vynechány.
3. Vyberte přenosovou rychlost (500 kbit/s pro OBD přes CAN) a připojte se.

## Co funguje

OBD, každá řídicí jednotka přes UDS, paralelní hledání řídicích jednotek, VW TP 2.0 a odposlech vysílaných hodnot. CAN FD přes CANlib zatím zabudované není.

## Příkazový řádek

```
caruspex_cli read --can kvaser:0@500000
```
