# Kvaser (Windows)

Adapterów CAN firmy Kvaser (Leaf, U100, Memorator i innych) można używać w Windows bezpośrednio, przez bibliotekę Kvasera CANlib. W Linuksie są to interfejsy [SocketCAN](socketcan.md).

## Łączenie

1. Zainstaluj sterowniki Kvasera. Dostarczają one `canlib32.dll`, którą ładuje Caruspex.
2. Na stronie **Połącz** wybierz **Interfejs CAN**. Każdy kanał Kvasera z podłączonym urządzeniem pojawia się na liście; wirtualne kanały sterownika są pomijane.
3. Wybierz prędkość transmisji (500 kbit/s dla OBD przez CAN) i połącz.

## Co działa

OBD, każdy sterownik przez UDS, równoległe wyszukiwanie sterowników, VW TP 2.0 i nasłuchiwanie rozgłaszanych wartości. CAN FD przez CANlib nie jest jeszcze wbudowany.

## Wiersz poleceń

```
caruspex_cli read --can kvaser:0@500000
```
