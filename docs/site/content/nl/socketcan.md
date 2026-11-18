# SocketCAN (Linux)

SocketCAN is de CAN-laag van de Linux-kernel. Elke CAN-adapter met een kerneldriver kan daarmee gebruikt worden: PEAK PCAN-USB, Kvaser, CANable met de candleLight-firmware, CAN-hats voor de Raspberry Pi en nog veel meer.

## Instellen

Breng de interface omhoog met de bitrate van de auto (500 kbit/s voor OBD via CAN):

```
sudo ip link set can0 up type can bitrate 500000
```

Voor CAN FD bij de nieuwste auto's, ook met een databitrate:

```
sudo ip link set can0 up type can bitrate 500000 dbitrate 2000000 fd on
```

## Verbinden

Kies op de pagina **Verbinden** voor **CAN-interface**. Elke CAN-interface van de computer staat in de lijst (`can0`, `can1` enzovoort). Kies hem, kies de bitrate en zet voor CAN FD het vakje CAN FD aan.

## Wat werkt

OBD, elke regeleenheid via UDS, de parallelle zoektocht naar regeleenheden, VW TP 2.0, meeluisteren met uitgezonden waarden en CAN FD.

## Opdrachtregel

```
caruspex_cli interfaces
caruspex_cli read --can socketcan:can0@500000
caruspex_cli read --can socketcan:can0+fd@500000
```
