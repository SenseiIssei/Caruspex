# BMW ENET cable

BMW and Mini from the F series on (about 2008 onwards) have Ethernet at the OBD socket. An ENET cable goes from the OBD socket to the network port of the computer, with no adapter chip in between. Caruspex talks HSFZ to the car's gateway (ports 6811 and 6801) and reaches every control unit.

## Connecting

1. Plug the cable into the car and into the computer's network port (or a USB network adapter). Switch the ignition on.
2. Wait up to a minute: with no DHCP server, Windows, macOS and Linux give the computer an automatic address (169.254.x.x) by themselves.
3. On the **Connect** page choose **Network cable** and leave the field empty: Caruspex searches the network for the car. If the search finds nothing, type in the car's address.

## What works

- Every control unit over UDS with BMW's addressing, fault codes in BMW's format as ISTA shows them (for example 480A10).
- Fault memory, identification and clearing per control unit, live values over OBD.
- Caruspex only reads here as well. Coding and flashing stay with the workshop programs.

## Which cable?

A plain ENET cable has a resistor between pins 8 and 16 that switches the Ethernet of the car on. Cables without it stay dark. Network ports with 1 Gbit/s work, the car speaks 100 Mbit/s.

## Command line

```
caruspex_cli ecus --enet auto
```
