# KKL cable (VAG 409.1)

A KKL cable is a USB cable with a serial chip and a K-Line driver, without an ELM327 inside. Caruspex drives the K-Line itself, byte by byte. That is what older VW, Audi, Seat and Skoda need: their protocol KWP1281 acknowledges every single byte, which no ELM327 can do.

## What works

- Every control unit of VW, Audi, Seat and Skoda from about 1990 to 2004: engine, gearbox, ABS, airbag, instrument cluster, comfort and more.
- KWP1281 and KWP2000 (ISO 14230), detected automatically from the key bytes after the wake-up.
- Fault memory per control unit, measuring blocks, and the real basic setting of the throttle body (measuring block 060 or 098).
- Adaptation channels and output tests are read-only on purpose.

These pages are in the **pro mode**: **Control units via KKL**, **Throttle body** and **Adaptation and output tests**. Each opens the cable itself: pick the serial port, the control unit, start.

## Which cable?

Cables with a genuine FTDI FT232RL or a CP2102 are far more reliable on the K-Line than the cheapest clones. For KWP1281 a genuine FTDI is recommended. The cable must pass the K-Line through raw: "HEX-USB" interfaces with their own firmware do not work.

Cars before about 1996 often have no OBD socket but a 2x2 plug in the engine bay or under the dashboard. An adapter from 2x2 to the 16-pin socket is cheap.

## Check the cable

The command-line tool tests the cable without a car:

```
caruspex_cli kklcheck --port COM3
```

## Typical problems

- **No answer from the control unit**: ignition on? The 5-baud wake-up needs exact timing; USB hubs and some virtual machines spoil it, connect the cable directly.
- **Breaks off after a while**: cheap cables lose bytes. Try another cable before you suspect the car.
