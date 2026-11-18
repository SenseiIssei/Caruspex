# DoIP (Diagnostics over IP)

Many cars from about 2016 can be diagnosed over Ethernet after ISO 13400, called DoIP. A DoIP cable connects the OBD socket with the network port of the computer.

## Connecting

1. Plug in the cable, switch the ignition on and wait until the computer has its automatic address (169.254.x.x), which can take up to a minute.
2. On the **Connect** page choose **Network cable** and leave the field empty. Caruspex looks for DoIP vehicles on the network and recognises by itself whether the car speaks DoIP or BMW's HSFZ.

## What works

OBD over DoIP, and the control units over UDS whose addresses Caruspex knows for the brand. The address lists for DoIP cars are still short; recordings of real cars fill them (see [privacy](privacy.md) for what a shared recording contains).

## Which cable?

DoIP needs the Ethernet lines at pins 3, 11, 12 and 13 of the OBD socket and an activation line on pin 8. Cables sold for a particular brand usually wire exactly that; the BMW ENET cable is one of them.

## Command line

```
caruspex_cli enet
```

lists the BMW and DoIP vehicles found on the network.
