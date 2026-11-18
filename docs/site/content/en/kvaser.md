# Kvaser (Windows)

Kvaser's CAN adapters (Leaf, U100, Memorator and others) can be used directly on Windows through Kvaser's library CANlib. On Linux they are [SocketCAN](socketcan.md) interfaces.

## Connecting

1. Install the Kvaser drivers. They bring `canlib32.dll`, which Caruspex loads.
2. On the **Connect** page choose **CAN interface**. Every Kvaser channel with a device behind it appears in the list; the virtual channels of the driver are left out.
3. Pick the bit rate (500 kbit/s for OBD over CAN) and connect.

## What works

OBD, every control unit over UDS, the parallel control unit search, VW TP 2.0 and listening to broadcast values. CAN FD over CANlib is not built in yet.

## Command line

```
caruspex_cli read --can kvaser:0@500000
```
