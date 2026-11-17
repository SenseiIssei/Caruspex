# ELM327 over USB

A plug for the OBD socket with a cable to the USB port. It is the most reliable kind of ELM327: no pairing, no radio, no flat batteries.

## Driver

Inside the plug a USB chip turns the cable into a serial port (COM port on Windows). Which driver is needed depends on that chip:

| Chip | Driver |
|---|---|
| FTDI FT232 | usually installed by Windows itself, built into Linux and macOS |
| CH340 / CH341 | usually installed by Windows itself, built into Linux and macOS |
| Silicon Labs CP210x | from the maker's website if Windows does not find it |
| Prolific PL2303 | from the maker's website; old clones often only work with an old driver |

After plugging in, the Device Manager of Windows shows the adapter under "Ports (COM and LPT)".

## Connecting

1. Plug the adapter into the car, switch the ignition on, plug the cable into the computer.
2. On the **Connect** page choose **USB or COM port**.
3. Leave the port on **Search automatically** or pick it from the list, then connect.

The automatic search tries every serial port at the usual speeds. If you know the speed of your adapter, you can set it in the settings under **Adapter baud rate**.

## Linux and macOS

On Linux the adapter appears as `/dev/ttyUSB0` or `/dev/ttyACM0`. Your user needs to be in the `dialout` group (see [Install](install.md)). On macOS it appears as `/dev/cu.usbserial-...` or `/dev/cu.wchusbserial...`.

## Typical problems

- **No port appears**: the driver is missing or the cable is a charging-only cable.
- **Adapter answers, car does not**: ignition off, or the adapter is a clone that does not handle the car's protocol. The [adapter test](interfaces.md) tells you more.
- **Very slow live values on older cars**: the K-Line itself is slow, that is not the fault of the adapter.
