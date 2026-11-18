# ELM327 over Wi-Fi

Some adapters open their own Wi-Fi network. The computer joins it and talks to the adapter over TCP.

## Connecting

1. Plug the adapter in and switch the ignition on. After a few seconds a Wi-Fi network appears, often named `WiFi_OBDII`, `V-LINK` or after the maker.
2. Join that network with your computer. It has no internet, so your system may warn that the connection is limited: that is fine.
3. In Caruspex choose **Wi-Fi** on the **Connect** page. Most adapters use the address `192.168.0.10` and port `35000`; this is preset. If yours uses another one (for example `192.168.4.1`), type it in as `address:port`.

## Tips

- While the computer is in the adapter's network, it has no internet. That does not matter to Caruspex.
- Wi-Fi adapters work on every system without drivers.
- If the connection breaks off, move the computer closer to the car: the radio of these plugs is weak.
