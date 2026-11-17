# SocketCAN (Linux)

SocketCAN je vrstva CAN v jádře Linuxu. Přes ni lze použít každý adaptér CAN s ovladačem v jádře: PEAK PCAN-USB, Kvaser, CANable s firmwarem candleLight, nástavce CAN pro Raspberry Pi a mnoho dalších.

## Nastavení

Zapněte rozhraní s přenosovou rychlostí auta (500 kbit/s pro OBD přes CAN):

```
sudo ip link set can0 up type can bitrate 500000
```

Pro CAN FD u nejnovějších aut, navíc s datovou přenosovou rychlostí:

```
sudo ip link set can0 up type can bitrate 500000 dbitrate 2000000 fd on
```

## Připojení

Na stránce **Připojit** vyberte **Rozhraní CAN**. V seznamu je každé rozhraní CAN počítače (`can0`, `can1` a tak dále). Vyberte ho, zvolte přenosovou rychlost a pro CAN FD zaškrtněte políčko CAN FD.

## Co funguje

OBD, každá řídicí jednotka přes UDS, paralelní hledání řídicích jednotek, VW TP 2.0, odposlech vysílaných hodnot a CAN FD.

## Příkazový řádek

```
caruspex_cli interfaces
caruspex_cli read --can socketcan:can0@500000
caruspex_cli read --can socketcan:can0+fd@500000
```
