# Live values

The **Live values** page (pro mode) shows what the sensors measure right now. In the beginner mode the **Driving mode** shows the most important values in large type.

![Live values](screenshot-live.png)

## What there is

- Every mode 01 value of SAE J1979 up to PID C4: 145 PIDs with more than 300 readings, grouped and searchable. Which ones your car offers, it says itself.
- Odometer, gear, oil temperature, fuel rate, torque, boost, exhaust temperatures, particulate filter, AdBlue, NOx and hybrid battery, as far as the car reports them.
- A rolling chart for several values at once. Over long windows (up to an hour) the chart thins out old points but keeps every peak and dip.
- Recording to CSV.

## Manufacturer and own values

- **Electric car batteries**: state of health, charge, cell voltages with spread and temperatures for Hyundai Kona and Ioniq, Kia e-Niro, Renault Zoe, Nissan Leaf and Tesla Model 3 and Y (by listening on the CAN bus), on the page **EV battery**.
- **OBDb values** for about 550 more vehicles (tyre pressure, oil, odometer and more), offered by themselves for the detected car.
- **Your own values** in the widely used Torque CSV format, equations included.

![EV battery](screenshot-battery.png)

## How fast?

How many values per second arrive depends on the car and the adapter. CAN cars answer fast; cars on the K-Line are slow by design. An OBDLink lets Caruspex ask up to 24 values in one go (see [OBDLink and STN](obdlink-stn.md)); a raw CAN interface is the fastest way.

## Recording and replay

A whole session with the car can be recorded into a file and played back later without the car, handy for troubleshooting. With **Share recording** a recording can be prepared for the project without personal data, see [privacy](privacy.md).
