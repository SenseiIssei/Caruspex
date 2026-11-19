# K+DCAN cable

The K+DCAN cable is the usual cable for BMW from about 1998 to 2017 with the workshop programs INPA and ISTA. Inside is an FTDI serial chip and a small converter for the D-CAN of BMW. Many of these cables have a switch or a bridge between pins 7 and 8 for older cars.

## State in Caruspex

> Honest state: Caruspex reads BMW E series cars over the K-Line of this cable: DS2 on the older models, KWP2000 in BMW's own style and BMW-FAST on the E60 generation. So far this is only tested against the simulated cars in the program, not yet on a real BMW. Caruspex does not speak the D-CAN mode of the cable yet.

## In K-Line mode

- **Switch to K-Line** before the cable goes into the socket; on most cables that is the left position. It joins pins 7 and 8, so the control units on pin 8 of the older models answer too.
- **Which cars**: E36 (with the 20-pin round plug through an adapter), E38, E39, E46, E52, E53, E83, E85 and the E60 generation (E60, E61, E63, E64, E65, E87, E90) up to 03/2007. After that, and from the F series on, the socket speaks D-CAN or Ethernet.
- **How**: Pro mode on, page *BMW via K-line*, choose the port, *Scan*. Caruspex asks every known address over DS2 (9600 baud, even parity), KWP2000 (10400 baud with fast init) and BMW-FAST (115200 baud) and shows the control units it finds with their identification and fault memory. It takes about half a minute.
- **Clearing** works per control unit or everywhere, both only after asking. Afterwards Caruspex reads back what is left.
- **Fault codes**: BMW publishes no texts for its codes, so Caruspex shows them as hex numbers.
- **Read only**: no coding, adaptation or output tests here; those belong in the workshop software.
- **A KKL cable** works the same way, but on the older models it only reaches the control units on pin 7 unless pins 7 and 8 are joined.

To try it without a car, the port list of the page has a simulated E46 (DS2) and E60 (KWP2000).

## What else works

- **BMW and Mini from the F series (about 2008 onwards)**: an [ENET cable](enet.md) reaches every control unit over Ethernet, or a plain [ELM327](elm327-usb.md) with BMW's own addressing over CAN.
- **Standard OBD on BMW with CAN (about 2007 onwards)**: any ELM327 reads engine faults and live values.

## Help wanted

If you own a K+DCAN cable and an older BMW, a [test report](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) with the log helps to confirm the K-Line part on a real car.
