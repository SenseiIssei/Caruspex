# KKL cable (VAG 409.1)

A KKL cable is a USB cable with a serial chip and a K-Line driver, without an ELM327 inside. Caruspex drives the K-Line itself, byte by byte. That is what older VW, Audi, Seat and Skoda need: their protocol KWP1281 acknowledges every single byte, which no ELM327 can do.

## What works

- Every control unit of VW, Audi, Seat and Skoda from about 1990 to 2004: engine, gearbox, ABS, airbag, instrument cluster, comfort and more.
- KWP1281 and KWP2000 (ISO 14230), detected automatically from the key bytes after the wake-up.
- Fault memory per control unit, measuring blocks, and the real basic setting of the throttle body (measuring block 060 or 098).
- Adaptation channels are read-only on purpose. The output test switches real parts and starts only after you confirm.

These pages are in the **pro mode**: **Control units via KKL**, **Throttle body** and **Adaptation and output tests**. Each opens the cable itself: pick the serial port, the control unit, start.

## Which cable?

Cables with a genuine FTDI FT232RL or a CP2102 are far more reliable on the K-Line than the cheapest clones. For KWP1281 a genuine FTDI is recommended. The cable must pass the K-Line through raw: "HEX-USB" interfaces with their own firmware do not work.

Cars before about 1996 often have no OBD socket but a 2x2 plug in the engine bay or under the dashboard. An adapter from 2x2 to the 16-pin socket is cheap.

## Check the cable

The command-line tool tests the cable without a car:

```
caruspex_cli kklcheck --port COM3
```

## First test on the car

The pro page **First test on the car** guides you through the first session with a new KKL cable. It is tailored to the Audi A2 and works on any VW, Audi, Seat or Skoda with a K-line. It only reads: nothing is cleared, no output test, no basic setting. The port is only opened when a test starts.

1. **Check the cable without the car.** Plug the cable into USB only, pick its port and press **Check the cable**. Caruspex reads the chip (FTDI, CH340, Prolific, CP210x), for FTDI the latency timer, and measures the bits of a 5-baud address on the break line. Without the car no echo comes back, which is correct.
2. **Set the latency timer to 1 ms** (FTDI cables on Windows): Device Manager, Ports (COM & LPT), double-click the USB Serial Port, Port Settings, Advanced, Latency Timer (msec) to 1, OK, then unplug the cable once. With the default of 16 ms the reply to the key bytes can miss its 50 ms window. Caruspex reads this setting but never changes it.
3. **On the car:** ignition on, engine off, the cable firmly in the OBD socket and directly on the computer (no USB hub), no other diagnostic program open. Tick the checklist and press **Start the first test**.
4. The test wakes the engine (address 01) and measures every bit of the address, the sync byte 0x55, the key bytes and the reply to them. Then it reads the identification, the fault memory and the measuring blocks 001 to 003, and after that the instrument cluster 17, ABS 03, airbag 15, comfort 46 and central electrics 09. A unit that does not answer may simply not be fitted.
5. **Save report** writes a text file and a JSON file into the folder `test-reports` in Caruspex's settings folder. **Open the test report on GitHub** fills in the report form for you. The VIN and the immobiliser number are shortened unless you tick the box.

What the results mean:

| Result | What to do |
|---|---|
| No echo | The cable does not hear itself: the plug is not seated, or pin 16 has no 12 V (check the fuse of the diagnostic socket). Without the car this is normal. |
| The echo is garbled | Another device on the socket, the switch of a K+DCAN cable not on K-line, or a faulty cable. |
| The wake-up address is clocked too unevenly | The bits are more than 20 ms off 200 ms: no USB hub, close other programs, latency timer 1 ms. Caruspex tries once more with the break calls started earlier. |
| Cable OK, but no answer from the car | Is the ignition on? Address 01 is the engine. Cars from about 2005 are diagnosed over CAN. |
| The reply to the key bytes came too late | Set the latency timer to 1 ms. |

## Typical problems

- **No answer from the control unit**: ignition on? The 5-baud wake-up needs exact timing; USB hubs and some virtual machines spoil it, connect the cable directly.
- **Breaks off after a while**: cheap cables lose bytes. Try another cable before you suspect the car.
