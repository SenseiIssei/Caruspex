# Tuning simulator

The **Tuning simulator** page (pro mode, under Measure) is a virtual BMW N55: the 3.0 litre straight six with one twin-scroll turbo of the F30 335i and the F32/F36 435i, 306 PS and 400 Nm. Change boost, ignition and lambda per rpm band and see what the model says, long before anything touches a car.

> **Simulation only.** Caruspex never writes engine maps and never flashes. It knows no flash file format, no checksum and no map address, and it copies no commercial tune. Real tuning belongs with established tools and a workshop, and the change needs approval (in Germany by the TÜV). Without it the car loses its type approval.

![Tuning simulator](screenshot-tune.png)

## What you can change

- Offsets in five rpm bands (1000 to 2000, 2000 to 3500, 3500 to 5000, 5000 to 6000, 6000 to 7000): boost in bar, ignition in degrees, lambda.
- Fuel (RON 95, 98, 100 or 102), outside temperature, air pressure and the gearbox (8HP45 automatic or six-speed manual).
- **Stock** sets the offsets back to zero, **Reset** the conditions as well.

The page shows torque and power stock against changed, every limit the change touches with its reason, and the inner values: boost, charge air, ignition, knock reserve, exhaust temperature, turbo, high-pressure pump, mean effective pressure.

## How the model works

1. **Air.** The boost target is capped by what the turbo can build at that rpm, by its pressure ratio and by its flow. The compressor heats the air, the intercooler takes part of the heat out again. Pressure, temperature and volumetric efficiency give the air mass.
2. **Fuel and ignition.** The fuel follows from air and lambda. The ignition is the stock timing plus your offset, but never earlier than the knock limit allows: the engine's knock control would pull it back. The knock limit moves with octane, charge air temperature, boost and lambda.
3. **Output.** Fuel energy times an efficiency that drops with the distance from the best timing, minus friction, gives torque and power.
4. **Limits.** Turbo, exhaust temperature, high-pressure pump, injectors, low-speed pre-ignition, gearbox, charge air and mixture each give a warning in plain words.

A whole curve (25 points from 1000 to 7000 rpm) takes about 0.015 ms in the release build.

## Published figures

| Value | Figure | Source |
|---|---|---|
| Displacement, bore, stroke, compression | 2979 cm³, 84.0 mm, 89.6 mm, 10.2:1 | Wikipedia "BMW N55" (CC BY-SA 4.0) |
| Torque | 400 Nm from 1200 to 5000 rpm | Wikipedia "BMW N55", BMW data of the 435i |
| Power | 225 kW (306 PS) at 5700 to 6000 rpm | Wikipedia (5700 to 5800), BMW (5800 to 6000) |
| Redline | 7000 rpm | Wikipedia "BMW N55" |
| Rated input torque of the 8HP45 | 450 Nm | Wikipedia "ZF 8HP transmission" (CC BY-SA 4.0) |

## Assumptions

Nothing freely licensed publishes the inner maps of the N55. These values are my assumptions; every one is explained in [spec.rs](https://github.com/SenseiIssei/Caruspex/blob/main/crates/caruspex-tune/src/spec.rs).

- **Stock boost** is back-calculated with the model from the published curve (about 0.7 to 0.8 bar up to 5000 rpm, less above). That the stock curve matches therefore shows consistency, not truth.
- Volumetric efficiency 0.86 to 0.97, indicated efficiency 0.39, friction as a quadratic in rpm.
- Knock-limited ignition 3 to 15 degrees before top dead centre with 1.5 degrees reserve; it moves 1 degree per octane number, 0.1 degree per kelvin of charge air, 6 degrees per bar and 2.5 degrees per 0.1 lambda.
- Turbo: pressure ratio up to 2.55, corrected flow up to 0.32 kg/s, efficiency 0.72. Intercooler effectiveness 0.85, less at high flow.
- Exhaust temperature limit 980 °C, high-pressure pump 0.027 kg/s, injection window 40 % of the cycle, pre-ignition risk above 18 bar mean effective pressure below 2500 rpm, six-speed manual about 450 Nm.
- Fuel E5: air to fuel ratio 14.4, 42.4 MJ/kg.

## How close is it?

- Stock against the published curve: 400 Nm within 1 % from 1250 to 5000 rpm, 306 PS at 6000 rpm and 304 PS at 5750 rpm. The tests allow 3 %.
- The model has not been checked against a real N55 yet. Every number is an estimate: about ±8 % for torque and power, ±0.1 bar boost, ±10 K charge air, ±3 degrees knock reserve, ±60 K exhaust temperature.

## Comparing with your car

1. Connect, open **Live values** with engine speed, load, manifold pressure, intake air temperature, ignition timing, lambda and air mass, and start a recording.
2. Accelerate once at full throttle through the rev range, only where that is legal and safe.
3. Open the recording on the simulator page. Caruspex keeps the full-load samples, groups them in bands of 500 rpm and shows measured against simulated: manifold pressure and charge air with your maps, air mass and ignition with the measured pressure and temperature.

OBD reports the manifold pressure only up to 255 kPa, and which sensor the engine control uses for the intake temperature is not documented. **Show example** fills the table with invented values, so you can see how it reads.

## Good to know

- More torque than stock at low rpm raises the risk of pre-ignition, which can break pistons in one go.
- Ignition earlier than the knock limit gains nothing: the knock control pulls it back.
- Better fuel alone adds reserve, not power: the stock ignition map is the ceiling.
