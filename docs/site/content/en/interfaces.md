# Adapters and interfaces

Caruspex finds out by itself what is plugged in and what the car speaks. Whatever the way into the car, the same functions work on it: OBD-II, every control unit over UDS, listening to broadcast values, recording and replay.

| Interface | Examples | Page |
|---|---|---|
| ELM327 over USB | cheap ELM327 cables, Vgate, Veepeak | [ELM327 over USB](elm327-usb.md) |
| ELM327 over Bluetooth | classic Bluetooth and Bluetooth LE plugs, vLinker, Vgate iCar Pro | [ELM327 over Bluetooth](elm327-bluetooth.md) |
| ELM327 over Wi-Fi | plugs that open their own Wi-Fi | [ELM327 over Wi-Fi](elm327-wifi.md) |
| STN chips | OBDLink SX, EX, MX+, CX | [OBDLink and STN](obdlink-stn.md) |
| KKL cable (VAG 409.1) | FTDI or CP2102 cables | [KKL cable](kkl.md) |
| K+DCAN cable | BMW cables with a switch | [K+DCAN cable](kdcan.md) |
| ENET cable | BMW and Mini from the F series | [BMW ENET](enet.md) |
| SAE J2534 PassThru | Tactrix OpenPort 2.0, Mongoose, VCX Nano, Scanmatik | [J2534](j2534.md) |
| SLCAN / Lawicel | CANable, USBtin, CANUSB | [SLCAN](slcan.md) |
| SocketCAN (Linux) | PEAK, Kvaser, CANable with candleLight, Raspberry Pi CAN hats | [SocketCAN](socketcan.md) |
| PEAK PCAN (Windows) | PCAN-USB, PCAN-USB FD | [PCAN](pcan.md) |
| Kvaser (Windows) | Kvaser Leaf, U100 | [Kvaser](kvaser.md) |
| DoIP cable | many cars from about 2016 | [DoIP](doip.md) |

## What can each one do?

A plain ELM327 is enough for the engine, the gearbox and, on most cars from about 2008, all other control units over UDS. It cannot do the old Volkswagen group protocol KWP1281, which acknowledges every single byte: for VW, Audi, Seat and Skoda before about 2004 you need a KKL cable.

The raw CAN interfaces (J2534, SLCAN, SocketCAN, PCAN, Kvaser) give Caruspex the bare CAN frames. It then does the transport protocol itself, which allows a fast parallel search for control units and VW TP 2.0 for Volkswagen group cars from about 2004 to 2012.

![Adapter test](screenshot-adapter.png)

## Is my adapter genuine?

The **Adapter test** asks the adapter for the commands of every ELM327 version, measures the response times and says honestly what follows from that: many cheap adapters report version 1.5 or 2.1 but only know the commands of 1.0. Caruspex never writes anything to the adapter.

> Tested so far: an OBDLink with the STN2232 chip passed the adapter test, without a car. All other adapters and interfaces are built after their documentation but not yet tested on a real car.
