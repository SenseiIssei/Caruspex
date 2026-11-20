# ELM327 over Bluetooth

Small plugs without a cable. There are two kinds of Bluetooth, and Caruspex speaks both.

## Bluetooth LE

Newer adapters such as the Vgate iCar Pro BLE, vLinker, OBDLink CX and Veepeak BLE use Bluetooth Low Energy. They need **no pairing**: on the **Connect** page choose **Bluetooth**, click **Search for Bluetooth devices** and pick the adapter from the list. This works on Windows, Linux, macOS and Android.

## Classic Bluetooth

Older and many cheap adapters use classic Bluetooth with a serial profile.

1. Pair the adapter once in the Bluetooth settings of your system. The PIN is usually `1234` or `0000`.
2. In Caruspex choose **Bluetooth** and pick the paired adapter; the list says whether a device is LE, classic or both.

On Windows and Linux Caruspex connects to paired classic adapters directly. Alternatively Windows creates a COM port for the adapter, which works like [ELM327 over USB](elm327-usb.md). On macOS classic Bluetooth adapters only work if the system offers a serial port for them; adapters with Bluetooth LE are the better choice there.

## On Android

- Bluetooth LE adapters need no pairing here either: choose **Bluetooth** and search; the list shows the signal strength of each device.
- Classic adapters are paired once in the Android settings, then they are in the list.
- Android 12 and newer ask once for access to nearby devices. Up to Android 11 the search only finds Bluetooth LE adapters when Caruspex may use the location and the location is switched on; Caruspex does not read it. Paired classic adapters work without it.

## Tips

- Unplug the adapter when you do not use it: many cheap ones keep the car battery busy.
- If the adapter does not show up, switch Bluetooth on the computer off and on again and make sure no phone app is connected to it at the same time.
- Bluetooth is slower than a cable. With an OBDLink, Caruspex asks several values at once, which makes up for most of it (see [OBDLink and STN](obdlink-stn.md)).
