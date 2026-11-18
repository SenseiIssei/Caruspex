# OBDLink and STN chips

The OBDLink adapters (SX, EX, MX+, CX, LX) use STN chips from OBD Solutions. They understand everything an ELM327 does and more; Caruspex recognises them by itself and uses the extras.

## What Caruspex uses

- **Batched requests**: Caruspex asks up to 24 live values in one go instead of 6. Over Bluetooth this makes a round of values almost twice as fast.
- **Listening**: broadcast values (for example on a Tesla) are recorded with the STN's own commands, which stop by themselves after a set number of messages.
- **Ford MS-CAN and GM Single-Wire-CAN**: body control units of Ford and GM do not sit on the normal diagnostic CAN. With an adapter that has the extra transceiver, the **All control units** page offers "Ford: body units on MS-CAN" or "GM: body units on Single-Wire-CAN".

| Chip | Adapter | MS-CAN | SW-CAN |
|---|---|---|---|
| STN2230 to STN2232 | OBDLink EX | yes | no |
| STN1150 to 1152, 1170, 2120, 2255, 2256 | OBDLink MX+ and others | yes | yes |
| others | OBDLink SX, LX, CX | no | no |

## Connecting

OBDLink adapters are connected like any ELM327: over USB (the EX and SX), Bluetooth (MX+, LX, CX) or as described under [ELM327 over USB](elm327-usb.md) and [ELM327 over Bluetooth](elm327-bluetooth.md).

## Tested

An OBDLink with the STN2232 chip passed the adapter test: chip, firmware and commands were recognised correctly. It has not been used on a car yet, so batched requests, MS-CAN and SW-CAN are still unconfirmed on real hardware.
