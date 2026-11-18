# Adaptadores e interfaces

Caruspex averigua por sí solo qué hay enchufado y qué habla el coche. Sea cual sea el camino hasta el coche, funcionan las mismas funciones: OBD-II, todas las unidades de control por UDS, escuchar valores difundidos en el bus, grabar y reproducir.

| Interfaz | Ejemplos | Página |
|---|---|---|
| ELM327 por USB | cables ELM327 baratos, Vgate, Veepeak | [ELM327 por USB](elm327-usb.md) |
| ELM327 por Bluetooth | conectores Bluetooth clásico y Bluetooth LE, vLinker, Vgate iCar Pro | [ELM327 por Bluetooth](elm327-bluetooth.md) |
| ELM327 por Wi-Fi | conectores que abren su propia Wi-Fi | [ELM327 por Wi-Fi](elm327-wifi.md) |
| Chips STN | OBDLink SX, EX, MX+, CX | [OBDLink y STN](obdlink-stn.md) |
| Cable KKL (VAG 409.1) | cables FTDI o CP2102 | [Cable KKL](kkl.md) |
| Cable K+DCAN | cables BMW con interruptor | [Cable K+DCAN](kdcan.md) |
| Cable ENET | BMW y Mini a partir de la serie F | [BMW ENET](enet.md) |
| SAE J2534 PassThru | Tactrix OpenPort 2.0, Mongoose, VCX Nano, Scanmatik | [J2534](j2534.md) |
| SLCAN / Lawicel | CANable, USBtin, CANUSB | [SLCAN](slcan.md) |
| SocketCAN (Linux) | PEAK, Kvaser, CANable con candleLight, placas CAN para Raspberry Pi | [SocketCAN](socketcan.md) |
| PEAK PCAN (Windows) | PCAN-USB, PCAN-USB FD | [PCAN](pcan.md) |
| Kvaser (Windows) | Kvaser Leaf, U100 | [Kvaser](kvaser.md) |
| Cable DoIP | muchos coches a partir de 2016 aproximadamente | [DoIP](doip.md) |

## ¿Qué puede hacer cada uno?

Un ELM327 sencillo basta para el motor, la caja de cambios y, en la mayoría de los coches desde 2008 aproximadamente, todas las demás unidades de control por UDS. No puede con el antiguo protocolo del grupo Volkswagen, KWP1281, que confirma cada byte por separado: para VW, Audi, Seat y Skoda de antes de 2004 aproximadamente necesitas un cable KKL.

Las interfaces CAN en bruto (J2534, SLCAN, SocketCAN, PCAN, Kvaser) le dan a Caruspex las tramas CAN tal cual. Entonces él mismo se encarga del protocolo de transporte, lo que permite una búsqueda rápida y en paralelo de unidades de control y VW TP 2.0 para coches del grupo Volkswagen de 2004 a 2012 aproximadamente.

![Prueba del adaptador](screenshot-adapter.png)

## ¿Mi adaptador es original?

La **Prueba del adaptador** pide al adaptador los comandos de cada versión de ELM327, mide los tiempos de respuesta y dice con sinceridad lo que se deduce de ello: muchos adaptadores baratos dicen ser la versión 1.5 o 2.1 pero solo conocen los comandos de la 1.0. Caruspex nunca escribe nada en el adaptador.

> Probado hasta ahora: un OBDLink con el chip STN2232 pasó la prueba del adaptador, sin coche. Todos los demás adaptadores e interfaces están hechos según su documentación, pero aún no se han probado en un coche real.
