# X-ray view

The **X-ray view** shows the connected car as a glass model you can turn. Inside you see engine or electric motors, gearbox, tank or high-voltage battery, exhaust, radiator, the 12 V battery and the OBD socket, laid out after the body type and drivetrain of the detected car.

![X-ray view](screenshot-xray.png)

## What lights up

- **Faults** colour their area, and the responsible control unit turns red.
- **Live values** sit as labels on the part they belong to: coolant at the radiator, engine speed at the engine, voltage at the battery.
- **Control units** found by the search sit where they usually are, joined by their data lines (powertrain CAN, comfort CAN, diagnostic CAN, K-Line). While Caruspex reads, small packets travel to the unit being asked.
- **Check car** flies the camera to every finding, one after the other.

Turn with the mouse or a finger, zoom with the wheel or two fingers; the keyboard works too. Layers (body, drivetrain, cooling, fuel or high voltage, electrics, network) can be switched off one by one.

## How exact is the model?

The shape comes from the real dimensions of the generation where they are known (from Wikidata or the maker's technical data) and from typical values for the body type and era otherwise. It is not a replica of the exact model: the makers' design data and wiring diagrams are not freely available. The page says where the shape comes from.

If you have a 3D model of your car as glTF (`.glb` or `.gltf`), you can load it with **Load your own 3D model**; the inside stays from the layout.

## Without 3D

On computers without WebGL the page falls back to a flat side view with the same information.
