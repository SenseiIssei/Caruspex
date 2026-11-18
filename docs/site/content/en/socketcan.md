# SocketCAN (Linux)

SocketCAN is the CAN layer of the Linux kernel. Every CAN adapter with a kernel driver can be used through it: PEAK PCAN-USB, Kvaser, CANable with the candleLight firmware, Raspberry Pi CAN hats and many more.

## Setting up

Bring the interface up with the bit rate of the car (500 kbit/s for OBD over CAN):

```
sudo ip link set can0 up type can bitrate 500000
```

For CAN FD on the newest cars, with a data bit rate as well:

```
sudo ip link set can0 up type can bitrate 500000 dbitrate 2000000 fd on
```

## Connecting

On the **Connect** page choose **CAN interface**. Every CAN interface of the computer is in the list (`can0`, `can1` and so on). Pick it, choose the bit rate and, for CAN FD, switch on the CAN FD box.

## What works

OBD, every control unit over UDS, the parallel control unit search, VW TP 2.0, listening to broadcast values and CAN FD.

## Command line

```
caruspex_cli interfaces
caruspex_cli read --can socketcan:can0@500000
caruspex_cli read --can socketcan:can0+fd@500000
```
