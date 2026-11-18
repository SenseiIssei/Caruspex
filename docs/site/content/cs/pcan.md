# PEAK PCAN (Windows)

Adaptéry CAN od PEAK-System (PCAN-USB, PCAN-USB FD, PCAN-USB Pro a další) lze ve Windows používat přímo, bez ovladače J2534. V Linuxu jsou to rozhraní [SocketCAN](socketcan.md).

## Připojení

1. Nainstalujte ovladač zařízení od PEAK. Přináší knihovnu `PCANBasic.dll`, kterou Caruspex načte.
2. Na stránce **Připojit** vyberte **Rozhraní CAN**. Každý kanál PCAN, za kterým je zařízení, se v seznamu objeví sám; kanály, které už používá jiný program, jsou vynechány.
3. Vyberte přenosovou rychlost (500 kbit/s pro OBD přes CAN) a připojte se.

## Co funguje

OBD, každá řídicí jednotka přes UDS, paralelní hledání řídicích jednotek, VW TP 2.0 a odposlech vysílaných hodnot. CAN FD přes PCAN-Basic zatím zabudované není.

## Příkazový řádek

```
caruspex_cli read --can pcan:USB1@500000
```
