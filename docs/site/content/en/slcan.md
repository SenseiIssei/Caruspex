# SLCAN (Lawicel)

SLCAN is a simple text protocol over a serial port for CAN sticks: CANable with the slcan firmware, USBtin, Lawicel CANUSB and many home-made devices. It works on Windows, Linux and macOS.

## Connecting

1. Connect the stick to the computer and with a cable to the OBD socket: CAN high on pin 6, CAN low on pin 14, ground on pins 4 and 5.
2. On the **Connect** page choose **CAN interface**. SLCAN sticks are found by the automatic port search and appear in the list.
3. Pick the bit rate (500 kbit/s for OBD over CAN) and connect.

## What works

Like every raw CAN interface: OBD, every control unit over UDS, the parallel control unit search, VW TP 2.0 and listening to broadcast values. CAN FD is not possible over SLCAN.

## Tips

- A CANable with the candleLight firmware is not an SLCAN device: on Linux it appears as a [SocketCAN](socketcan.md) interface.
- The stick needs no termination resistor at the OBD socket, the car has its own.

## Command line

```
caruspex_cli read --can slcan:COM5@500000
caruspex_cli read --can slcan:/dev/ttyACM0@500000
```
