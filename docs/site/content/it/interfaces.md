# Adattatori e interfacce

Caruspex scopre da solo cosa è collegato e cosa parla l'auto. Qualunque sia la strada per entrare nell'auto, le stesse funzioni ci lavorano sopra: OBD-II, tutte le centraline via UDS, l'ascolto dei valori trasmessi, la registrazione e la riproduzione.

| Interfaccia | Esempi | Pagina |
|---|---|---|
| ELM327 via USB | cavi ELM327 economici, Vgate, Veepeak | [ELM327 via USB](elm327-usb.md) |
| ELM327 via Bluetooth | spine Bluetooth classiche e Bluetooth LE, vLinker, Vgate iCar Pro | [ELM327 via Bluetooth](elm327-bluetooth.md) |
| ELM327 via Wi-Fi | spine che aprono una propria rete Wi-Fi | [ELM327 via Wi-Fi](elm327-wifi.md) |
| Chip STN | OBDLink SX, EX, MX+, CX | [OBDLink e STN](obdlink-stn.md) |
| Cavo KKL (VAG 409.1) | cavi FTDI o CP2102 | [Cavo KKL](kkl.md) |
| Cavo K+DCAN | cavi BMW con un interruttore | [Cavo K+DCAN](kdcan.md) |
| Cavo ENET | BMW e Mini dalla serie F | [BMW ENET](enet.md) |
| SAE J2534 PassThru | Tactrix OpenPort 2.0, Mongoose, VCX Nano, Scanmatik | [J2534](j2534.md) |
| SLCAN / Lawicel | CANable, USBtin, CANUSB | [SLCAN](slcan.md) |
| SocketCAN (Linux) | PEAK, Kvaser, CANable con candleLight, schede CAN per Raspberry Pi | [SocketCAN](socketcan.md) |
| PEAK PCAN (Windows) | PCAN-USB, PCAN-USB FD | [PCAN](pcan.md) |
| Kvaser (Windows) | Kvaser Leaf, U100 | [Kvaser](kvaser.md) |
| Cavo DoIP | molte auto dal 2016 circa | [DoIP](doip.md) |

## Cosa sa fare ciascuno?

Un semplice ELM327 basta per il motore, il cambio e, sulla maggior parte delle auto dal 2008 circa, tutte le altre centraline via UDS. Non sa gestire il vecchio protocollo del gruppo Volkswagen KWP1281, che conferma ogni singolo byte: per VW, Audi, Seat e Skoda precedenti al 2004 circa serve un cavo KKL.

Le interfacce CAN dirette (J2534, SLCAN, SocketCAN, PCAN, Kvaser) danno a Caruspex i frame CAN nudi. Il protocollo di trasporto lo fa poi Caruspex stesso, il che permette una ricerca veloce in parallelo delle centraline e VW TP 2.0 per le auto del gruppo Volkswagen dal 2004 al 2012 circa.

![Test dell'adattatore](screenshot-adapter.png)

## Il mio adattatore è originale?

Il **Test dell'adattatore** chiede all'adattatore i comandi di ogni versione di ELM327, misura i tempi di risposta e dice sinceramente cosa ne risulta: molti adattatori economici dichiarano la versione 1.5 o 2.1 ma conoscono solo i comandi della 1.0. Caruspex non scrive mai niente sull'adattatore.

> Provato finora: un OBDLink con il chip STN2232 ha superato il test dell'adattatore, senza auto. Tutti gli altri adattatori e interfacce sono costruiti secondo la loro documentazione ma non ancora provati su un'auto vera.
