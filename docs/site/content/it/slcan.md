# SLCAN (Lawicel)

SLCAN è un semplice protocollo di testo su porta seriale per le chiavette CAN: CANable con il firmware slcan, USBtin, Lawicel CANUSB e molti dispositivi fatti in casa. Funziona su Windows, Linux e macOS.

## Collegamento

1. Collega la chiavetta al computer e, con un cavo, alla presa OBD: CAN high sul pin 6, CAN low sul pin 14, massa sui pin 4 e 5.
2. Nella pagina **Collega** scegli **Interfaccia CAN**. Le chiavette SLCAN vengono trovate dalla ricerca automatica delle porte e compaiono nell'elenco.
3. Scegli il bit rate (500 kbit/s per OBD su CAN) e collegati.

## Cosa funziona

Come ogni interfaccia CAN diretta: OBD, tutte le centraline via UDS, la ricerca in parallelo delle centraline, VW TP 2.0 e l'ascolto dei valori trasmessi. CAN FD non è possibile tramite SLCAN.

## Consigli

- Un CANable con il firmware candleLight non è un dispositivo SLCAN: su Linux compare come interfaccia [SocketCAN](socketcan.md).
- La chiavetta non ha bisogno di una resistenza di terminazione alla presa OBD, l'auto ha già la sua.

## Riga di comando

```
caruspex_cli read --can slcan:COM5@500000
caruspex_cli read --can slcan:/dev/ttyACM0@500000
```
