# SocketCAN (Linux)

SocketCAN è lo strato CAN del kernel Linux. Ogni adattatore CAN con un driver del kernel si può usare tramite lui: PEAK PCAN-USB, Kvaser, CANable con il firmware candleLight, schede CAN per Raspberry Pi e molti altri.

## Configurazione

Attiva l'interfaccia con il bit rate dell'auto (500 kbit/s per OBD su CAN):

```
sudo ip link set can0 up type can bitrate 500000
```

Per CAN FD sulle auto più recenti, anche con un bit rate dei dati:

```
sudo ip link set can0 up type can bitrate 500000 dbitrate 2000000 fd on
```

## Collegamento

Nella pagina **Collega** scegli **Interfaccia CAN**. Ogni interfaccia CAN del computer è nell'elenco (`can0`, `can1` e così via). Selezionala, scegli il bit rate e, per CAN FD, attiva la casella CAN FD.

## Cosa funziona

OBD, tutte le centraline via UDS, la ricerca in parallelo delle centraline, VW TP 2.0, l'ascolto dei valori trasmessi e CAN FD.

## Riga di comando

```
caruspex_cli interfaces
caruspex_cli read --can socketcan:can0@500000
caruspex_cli read --can socketcan:can0+fd@500000
```
