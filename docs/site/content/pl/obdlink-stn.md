# OBDLink i układy STN

Adaptery OBDLink (SX, EX, MX+, CX, LX) używają układów STN firmy OBD Solutions. Rozumieją wszystko, co ELM327, i więcej; Caruspex sam je rozpoznaje i korzysta z dodatków.

## Z czego korzysta Caruspex

- **Zapytania zbiorcze**: Caruspex pyta o maksymalnie 24 wartości na żywo naraz zamiast 6. Przez Bluetooth runda wartości jest dzięki temu prawie dwa razy szybsza.
- **Nasłuchiwanie**: rozgłaszane wartości (na przykład w Tesli) są nagrywane własnymi poleceniami STN, które same się zatrzymują po ustalonej liczbie komunikatów.
- **Ford MS-CAN i GM Single-Wire-CAN**: sterowniki nadwozia Forda i GM nie siedzą na zwykłej diagnostycznej magistrali CAN. Z adapterem, który ma dodatkowy transceiver, strona **Wszystkie sterowniki** oferuje "Ford: moduły nadwozia na MS-CAN" lub "GM: moduły nadwozia na Single-Wire-CAN".

| Układ | Adapter | MS-CAN | SW-CAN |
|---|---|---|---|
| STN2230 do STN2232 | OBDLink EX | tak | nie |
| STN1150 do 1152, 1170, 2120, 2255, 2256 | OBDLink MX+ i inne | tak | tak |
| inne | OBDLink SX, LX, CX | nie | nie |

## Łączenie

Adaptery OBDLink łączy się jak każdy ELM327: przez USB (EX i SX), Bluetooth (MX+, LX, CX) albo tak, jak opisano w [ELM327 przez USB](elm327-usb.md) i [ELM327 przez Bluetooth](elm327-bluetooth.md).

## Przetestowano

OBDLink z układem STN2232 przeszedł test adaptera: układ, firmware i polecenia zostały poprawnie rozpoznane. Nie był jeszcze używany w aucie, więc zapytania zbiorcze, MS-CAN i SW-CAN nie są jeszcze potwierdzone na prawdziwym sprzęcie.
