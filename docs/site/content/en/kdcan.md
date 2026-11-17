# K+DCAN cable

The K+DCAN cable is the usual cable for BMW from about 1998 to 2017 with the workshop programs INPA and ISTA. Inside is an FTDI serial chip and a small converter for the D-CAN of BMW. Many of these cables have a switch or a bridge between pins 7 and 8 for older cars.

## State in Caruspex

> Honest state: Caruspex does not speak the D-CAN mode of this cable yet. The old BMW protocols (DS2 and KWP2000 in BMW's own style) over the K-Line are planned but need tests on a real car first.

What you can use instead today:

- **BMW and Mini from the F series (about 2008 onwards)**: an [ENET cable](enet.md) reaches every control unit over Ethernet, or a plain [ELM327](elm327-usb.md) with BMW's own addressing over CAN.
- **Standard OBD on BMW with CAN (about 2007 onwards)**: any ELM327 reads engine faults and live values.
- **In K-Line mode** the cable passes the K-Line through like a [KKL cable](kkl.md). That is useful for VW group cars; BMW control units on the K-Line need the protocols that are still missing.

## Help wanted

If you own a K+DCAN cable and an older BMW, a [test report](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) with a recording helps to build this part.
