# PEAK PCAN (Windows)

Gli adattatori CAN di PEAK-System (PCAN-USB, PCAN-USB FD, PCAN-USB Pro e altri) si possono usare direttamente su Windows, senza un driver J2534. Su Linux sono interfacce [SocketCAN](socketcan.md).

## Collegamento

1. Installa il driver di PEAK. Porta con sé la libreria `PCANBasic.dll`, che Caruspex carica.
2. Nella pagina **Collega** scegli **Interfaccia CAN**. Ogni canale PCAN con un dispositivo collegato compare da solo nell'elenco; i canali già usati da un altro programma vengono esclusi.
3. Scegli il bit rate (500 kbit/s per OBD su CAN) e collegati.

## Cosa funziona

OBD, tutte le centraline via UDS, la ricerca in parallelo delle centraline, VW TP 2.0 e l'ascolto dei valori trasmessi. CAN FD tramite PCAN-Basic non è ancora integrato.

## Riga di comando

```
caruspex_cli read --can pcan:USB1@500000
```
