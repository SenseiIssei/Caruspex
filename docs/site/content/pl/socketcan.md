# SocketCAN (Linux)

SocketCAN to warstwa CAN jądra Linuksa. Można przez nią używać każdego adaptera CAN ze sterownikiem w jądrze: PEAK PCAN-USB, Kvaser, CANable z firmware candleLight, nakładki CAN do Raspberry Pi i wiele innych.

## Konfiguracja

Włącz interfejs z prędkością transmisji auta (500 kbit/s dla OBD przez CAN):

```
sudo ip link set can0 up type can bitrate 500000
```

Dla CAN FD w najnowszych autach, dodatkowo z prędkością fazy danych:

```
sudo ip link set can0 up type can bitrate 500000 dbitrate 2000000 fd on
```

## Łączenie

Na stronie **Połącz** wybierz **Interfejs CAN**. Każdy interfejs CAN komputera jest na liście (`can0`, `can1` i tak dalej). Wybierz go, ustaw prędkość transmisji, a dla CAN FD zaznacz pole CAN FD.

## Co działa

OBD, każdy sterownik przez UDS, równoległe wyszukiwanie sterowników, VW TP 2.0, nasłuchiwanie rozgłaszanych wartości i CAN FD.

## Wiersz poleceń

```
caruspex_cli interfaces
caruspex_cli read --can socketcan:can0@500000
caruspex_cli read --can socketcan:can0+fd@500000
```
