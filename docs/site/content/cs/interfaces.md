# Adaptéry a rozhraní

Caruspex sám zjistí, co je zapojené a jakým protokolem auto mluví. Ať do auta vede jakákoli cesta, fungují na ní stejné funkce: OBD-II, každá řídicí jednotka přes UDS, odposlech vysílaných hodnot, nahrávání a přehrávání.

| Rozhraní | Příklady | Stránka |
|---|---|---|
| ELM327 přes USB | levné kabely ELM327, Vgate, Veepeak | [ELM327 přes USB](elm327-usb.md) |
| ELM327 přes Bluetooth | adaptéry s klasickým Bluetooth a Bluetooth LE, vLinker, Vgate iCar Pro | [ELM327 přes Bluetooth](elm327-bluetooth.md) |
| ELM327 přes Wi-Fi | adaptéry, které vytvoří vlastní Wi-Fi | [ELM327 přes Wi-Fi](elm327-wifi.md) |
| Čipy STN | OBDLink SX, EX, MX+, CX | [OBDLink a STN](obdlink-stn.md) |
| Kabel KKL (VAG 409.1) | kabely s FTDI nebo CP2102 | [Kabel KKL](kkl.md) |
| Kabel K+DCAN | kabely pro BMW s přepínačem | [Kabel K+DCAN](kdcan.md) |
| Kabel ENET | BMW a Mini od řady F | [BMW ENET](enet.md) |
| SAE J2534 PassThru | Tactrix OpenPort 2.0, Mongoose, VCX Nano, Scanmatik | [J2534](j2534.md) |
| SLCAN / Lawicel | CANable, USBtin, CANUSB | [SLCAN](slcan.md) |
| SocketCAN (Linux) | PEAK, Kvaser, CANable s candleLight, nástavce CAN pro Raspberry Pi | [SocketCAN](socketcan.md) |
| PEAK PCAN (Windows) | PCAN-USB, PCAN-USB FD | [PCAN](pcan.md) |
| Kvaser (Windows) | Kvaser Leaf, U100 | [Kvaser](kvaser.md) |
| Kabel DoIP | mnoho aut zhruba od roku 2016 | [DoIP](doip.md) |

## Co který umí?

Obyčejný ELM327 stačí na motor, převodovku a u většiny aut zhruba od roku 2008 i na všechny ostatní řídicí jednotky přes UDS. Neumí starý protokol KWP1281 koncernu Volkswagen, který potvrzuje každý jednotlivý bajt: pro VW, Audi, Seat a Škodu zhruba před rokem 2004 potřebujete kabel KKL.

Přímá rozhraní CAN (J2534, SLCAN, SocketCAN, PCAN, Kvaser) dávají Caruspexu holé rámce CAN. Transportní protokol si pak obstará sám, což umožňuje rychlé paralelní hledání řídicích jednotek a VW TP 2.0 pro auta koncernu Volkswagen zhruba z let 2004 až 2012.

![Test adaptéru](screenshot-adapter.png)

## Je můj adaptér pravý?

**Test adaptéru** se adaptéru zeptá na příkazy každé verze ELM327, změří doby odezvy a poctivě řekne, co z toho plyne: mnoho levných adaptérů hlásí verzi 1.5 nebo 2.1, ale zná jen příkazy verze 1.0. Caruspex do adaptéru nikdy nic nezapisuje.

> Zatím otestováno: OBDLink s čipem STN2232 prošel testem adaptéru, bez auta. Všechny ostatní adaptéry a rozhraní jsou postavené podle své dokumentace, ale na skutečném autě zatím otestované nejsou.
