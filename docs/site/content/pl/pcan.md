# PEAK PCAN (Windows)

Adapterów CAN firmy PEAK-System (PCAN-USB, PCAN-USB FD, PCAN-USB Pro i innych) można używać w Windows bezpośrednio, bez sterownika J2534. W Linuksie są to interfejsy [SocketCAN](socketcan.md).

## Łączenie

1. Zainstaluj sterownik urządzenia od PEAK. Dostarcza on bibliotekę `PCANBasic.dll`, którą ładuje Caruspex.
2. Na stronie **Połącz** wybierz **Interfejs CAN**. Każdy kanał PCAN z podłączonym urządzeniem sam pojawia się na liście; kanały zajęte już przez inny program są pomijane.
3. Wybierz prędkość transmisji (500 kbit/s dla OBD przez CAN) i połącz.

## Co działa

OBD, każdy sterownik przez UDS, równoległe wyszukiwanie sterowników, VW TP 2.0 i nasłuchiwanie rozgłaszanych wartości. CAN FD przez PCAN-Basic nie jest jeszcze wbudowany.

## Wiersz poleceń

```
caruspex_cli read --can pcan:USB1@500000
```
