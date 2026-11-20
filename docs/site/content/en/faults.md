# Faults

The **Fault memory** page reads what the control units have stored and explains it in plain words.

![Fault memory](screenshot-faults.png)

## What you see

- **Stored, pending and permanent** codes. With several control units, every code shows who reported it (engine, gearbox).
- About 9,400 standard codes (P, C, B, U) and the manufacturer codes of VAG, Ford, GM, Toyota, Honda, Nissan, Hyundai and Kia, Subaru and more. A P1 code is read the way the detected brand means it.
- A **traffic light** per code and the question everyone asks first: may I keep driving?
- The usual causes, things you can check yourself and a rough cost class.
- Where the part sits: the car in see-through view, or the [X-ray view](xray.md).

## All control units

In the pro mode the page **All control units** asks every control unit, not only the engine: ABS, airbag, instrument cluster, climate and more. Each fault comes with its failure type (short circuit, open circuit, implausible signal), part number and software version. Older VW group cars are read over a [KKL cable](kkl.md).

## What should I do now?

The page **What to do?** sorts everything Caruspex found into three steps: stop now, to the workshop soon, can wait. Every point has one plain sentence why, a rough cost range and a note to show at the workshop. OBD cannot see whether the engine light flashes or a red warning light is on, so two boxes ask you; a flashing light with misfires moves the point to "stop now".

![What should I do now](screenshot-todo.png)

## Clearing codes

Clearing is possible per control unit, with a clear warning, and Caruspex reads the memory again right afterwards.

> Clearing is not a repair. It also resets the readiness monitors: until the car has done several full drive cycles, it fails an emissions test, and a code whose cause is still there comes back after a short drive.

## Export

The list can be saved as CSV, and the inspection report as a PDF.
