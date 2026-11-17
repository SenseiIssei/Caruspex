# Adaptery i interfejsy

Caruspex sam sprawdza, co jest podłączone i jakim protokołem mówi auto. Niezależnie od drogi do auta działają te same funkcje: OBD-II, każdy sterownik przez UDS, nasłuchiwanie rozgłaszanych wartości, nagrywanie i odtwarzanie.

| Interfejs | Przykłady | Strona |
|---|---|---|
| ELM327 przez USB | tanie kable ELM327, Vgate, Veepeak | [ELM327 przez USB](elm327-usb.md) |
| ELM327 przez Bluetooth | wtyczki z klasycznym Bluetooth i Bluetooth LE, vLinker, Vgate iCar Pro | [ELM327 przez Bluetooth](elm327-bluetooth.md) |
| ELM327 przez Wi-Fi | wtyczki, które tworzą własną sieć Wi-Fi | [ELM327 przez Wi-Fi](elm327-wifi.md) |
| Układy STN | OBDLink SX, EX, MX+, CX | [OBDLink i STN](obdlink-stn.md) |
| Kabel KKL (VAG 409.1) | kable z FTDI lub CP2102 | [Kabel KKL](kkl.md) |
| Kabel K+DCAN | kable do BMW z przełącznikiem | [Kabel K+DCAN](kdcan.md) |
| Kabel ENET | BMW i Mini od serii F | [BMW ENET](enet.md) |
| SAE J2534 PassThru | Tactrix OpenPort 2.0, Mongoose, VCX Nano, Scanmatik | [J2534](j2534.md) |
| SLCAN / Lawicel | CANable, USBtin, CANUSB | [SLCAN](slcan.md) |
| SocketCAN (Linux) | PEAK, Kvaser, CANable z candleLight, nakładki CAN do Raspberry Pi | [SocketCAN](socketcan.md) |
| PEAK PCAN (Windows) | PCAN-USB, PCAN-USB FD | [PCAN](pcan.md) |
| Kvaser (Windows) | Kvaser Leaf, U100 | [Kvaser](kvaser.md) |
| Kabel DoIP | wiele aut od około 2016 roku | [DoIP](doip.md) |

## Co potrafi każdy z nich?

Zwykły ELM327 wystarczy do silnika, skrzyni biegów, a w większości aut od około 2008 roku do wszystkich innych sterowników przez UDS. Nie obsługuje starego protokołu KWP1281 koncernu Volkswagena, który potwierdza każdy pojedynczy bajt: do VW, Audi, Seata i Skody sprzed około 2004 roku potrzebny jest kabel KKL.

Surowe interfejsy CAN (J2534, SLCAN, SocketCAN, PCAN, Kvaser) dają Caruspex gołe ramki CAN. Program sam obsługuje wtedy protokół transportowy, co pozwala na szybkie równoległe wyszukiwanie sterowników i na VW TP 2.0 w autach koncernu Volkswagena z lat około 2004 do 2012.

![Test adaptera](screenshot-adapter.png)

## Czy mój adapter jest oryginalny?

**Test adaptera** pyta adapter o polecenia każdej wersji ELM327, mierzy czasy odpowiedzi i uczciwie mówi, co z tego wynika: wiele tanich adapterów podaje wersję 1.5 lub 2.1, a zna tylko polecenia wersji 1.0. Caruspex nigdy nic nie zapisuje w adapterze.

> Przetestowano do tej pory: OBDLink z układem STN2232 przeszedł test adaptera, bez samochodu. Wszystkie pozostałe adaptery i interfejsy są zbudowane według ich dokumentacji, ale nie zostały jeszcze przetestowane na prawdziwym aucie.
