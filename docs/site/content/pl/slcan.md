# SLCAN (Lawicel)

SLCAN to prosty protokół tekstowy przez port szeregowy dla przystawek CAN: CANable z firmware slcan, USBtin, Lawicel CANUSB i wielu urządzeń zrobionych samodzielnie. Działa na Windows, Linux i macOS.

## Łączenie

1. Podłącz przystawkę do komputera, a kablem do gniazda OBD: CAN high na pin 6, CAN low na pin 14, masa na piny 4 i 5.
2. Na stronie **Połącz** wybierz **Interfejs CAN**. Przystawki SLCAN są znajdowane przez automatyczne wyszukiwanie portów i pojawiają się na liście.
3. Wybierz prędkość transmisji (500 kbit/s dla OBD przez CAN) i połącz.

## Co działa

Tak jak w każdym surowym interfejsie CAN: OBD, każdy sterownik przez UDS, równoległe wyszukiwanie sterowników, VW TP 2.0 i nasłuchiwanie rozgłaszanych wartości. CAN FD przez SLCAN nie jest możliwy.

## Wskazówki

- CANable z firmware candleLight nie jest urządzeniem SLCAN: w Linuksie pojawia się jako interfejs [SocketCAN](socketcan.md).
- Przystawka nie potrzebuje rezystora terminującego przy gnieździe OBD, auto ma własny.

## Wiersz poleceń

```
caruspex_cli read --can slcan:COM5@500000
caruspex_cli read --can slcan:/dev/ttyACM0@500000
```
