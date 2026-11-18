# PEAK PCAN (Windows)

The CAN adapters of PEAK-System (PCAN-USB, PCAN-USB FD, PCAN-USB Pro and others) can be used directly on Windows, without a J2534 driver. On Linux they are [SocketCAN](socketcan.md) interfaces.

## Connecting

1. Install PEAK's device driver. It brings the library `PCANBasic.dll`, which Caruspex loads.
2. On the **Connect** page choose **CAN interface**. Every PCAN channel with a device behind it appears in the list by itself; channels already used by another program are left out.
3. Pick the bit rate (500 kbit/s for OBD over CAN) and connect.

## What works

OBD, every control unit over UDS, the parallel control unit search, VW TP 2.0 and listening to broadcast values. CAN FD over PCAN-Basic is not built in yet.

## Command line

```
caruspex_cli read --can pcan:USB1@500000
```
