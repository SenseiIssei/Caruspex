# Connect assistant and OBD socket

## Where is the socket?

Every car sold in Europe since 2001 (petrol) or 2004 (diesel), and in the US since 1996, has a 16-pin OBD socket within reach of the driver. Usual places:

- below the dashboard, left or right of the steering column,
- above the pedals,
- in the side trim of the footwell,
- behind the cover of the fuse box,
- in the centre console, sometimes behind a small flap.

For more than 200 car generations and 35 motorcycles Caruspex knows the exact spot. The connect assistant marks it in a floor plan and in a close-up of the driver's side; for motorcycles it shows a side view with the diagnostic plug and the adapter cable that is needed.

> The list of socket locations is our own collection and has not been checked on every car. If the spot is wrong for yours, please tell us with a [test report](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml).

## The connect assistant

The assistant opens by itself on the first start. It asks four things, and every step can be skipped:

1. **Which adapter?** ELM327 with USB, Bluetooth or Wi-Fi, an ENET cable or a KKL cable. Pictures help to tell them apart.
2. **Which car?** Pick it from the catalog, then the assistant shows where the socket is.
3. **Ignition on.** Most control units only answer with the ignition on.
4. **Connect.** Caruspex searches the ports and tries the protocols.

Later you find the assistant in the settings and on the Connect page.

## Searching and remembering

With the automatic search Caruspex goes through all serial ports, asks each one for an ELM327 and then tries the protocols of the car one by one. Once a car has connected, Caruspex remembers the way (adapter, port, protocol, bit rate) for its VIN and tries it first next time, which makes connecting faster.

## When it does not connect

If no adapter answers or the car stays silent, Caruspex shows a short checklist for that kind of connection, together with what it has already found out: driver installed, Bluetooth paired, ignition on, right bit rate, a clone adapter that only pretends to be a newer version. The pages under [adapters and interfaces](interfaces.md) explain every kind in detail.
