# Caruspex

Caruspex is a free diagnostics program for cars and motorcycles. It runs on Windows, Linux and macOS, speaks 18 languages and works with the adapters people really own: the cheap ELM327 plug, an OBDLink, a KKL cable for older Volkswagen group cars, an ENET cable for BMW, and professional CAN interfaces.

![The overview with live gauges](screenshot-overview.png)

## What it does

- Reads and clears fault codes, explains them in plain words and says whether you may keep driving.
- Shows live values: temperatures, speed, mixture, boost, battery voltage and many more, as numbers and as charts.
- Asks every control unit of the car, not only the engine, as far as the car and the adapter allow it.
- Checks a used car before you buy it: does the VIN match, do the mileages agree, were faults cleared shortly before?
- Shows the car as a glass model in which faults and values light up where they belong.
- Tells you what to do next: stop now, go to the workshop soon, or it can wait.

## Where to begin

1. [Install Caruspex](install.md) on your computer.
2. Follow the [first steps](first-steps.md): try the demo car, then connect your own.
3. Look up your adapter under [adapters and interfaces](interfaces.md).
4. Read what Caruspex stores and sends under [privacy](privacy.md). In short: nothing leaves your computer unless you share it yourself.

> Caruspex only reads. It never codes control units, never changes the mileage and never touches the immobiliser. Clearing fault codes is the only thing it writes to the car.

## Honest state

Caruspex is built after the specifications and tested against simulated control units, fake gateways and fake drivers. On real hardware only an OBDLink with the STN2232 chip has been checked so far, without a car. If you try Caruspex on your car, please send a [test report](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml): every report makes the list of tested cars longer.
