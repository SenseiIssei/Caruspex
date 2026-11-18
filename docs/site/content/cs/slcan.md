# SLCAN (Lawicel)

SLCAN je jednoduchý textový protokol přes sériový port pro převodníky CAN: CANable s firmwarem slcan, USBtin, Lawicel CANUSB a mnoho podomácku vyrobených zařízení. Funguje ve Windows, Linuxu i macOS.

## Připojení

1. Připojte převodník k počítači a kabelem k zásuvce OBD: CAN high na pin 6, CAN low na pin 14, kostra na piny 4 a 5.
2. Na stránce **Připojit** vyberte **Rozhraní CAN**. Převodníky SLCAN najde automatické hledání portů a objeví se v seznamu.
3. Vyberte přenosovou rychlost (500 kbit/s pro OBD přes CAN) a připojte se.

## Co funguje

Jako u každého přímého rozhraní CAN: OBD, každá řídicí jednotka přes UDS, paralelní hledání řídicích jednotek, VW TP 2.0 a odposlech vysílaných hodnot. CAN FD přes SLCAN možné není.

## Tipy

- CANable s firmwarem candleLight není zařízení SLCAN: v Linuxu se objeví jako rozhraní [SocketCAN](socketcan.md).
- Převodník u zásuvky OBD nepotřebuje zakončovací odpor, auto má vlastní.

## Příkazový řádek

```
caruspex_cli read --can slcan:COM5@500000
caruspex_cli read --can slcan:/dev/ttyACM0@500000
```
