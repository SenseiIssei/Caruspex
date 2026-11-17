# Kvaser (Windows)

Gli adattatori CAN di Kvaser (Leaf, U100, Memorator e altri) si possono usare direttamente su Windows tramite la libreria CANlib di Kvaser. Su Linux sono interfacce [SocketCAN](socketcan.md).

## Collegamento

1. Installa i driver Kvaser. Portano con sé `canlib32.dll`, che Caruspex carica.
2. Nella pagina **Collega** scegli **Interfaccia CAN**. Ogni canale Kvaser con un dispositivo collegato compare nell'elenco; i canali virtuali del driver vengono esclusi.
3. Scegli il bit rate (500 kbit/s per OBD su CAN) e collegati.

## Cosa funziona

OBD, tutte le centraline via UDS, la ricerca in parallelo delle centraline, VW TP 2.0 e l'ascolto dei valori trasmessi. CAN FD tramite CANlib non è ancora integrato.

## Riga di comando

```
caruspex_cli read --can kvaser:0@500000
```
