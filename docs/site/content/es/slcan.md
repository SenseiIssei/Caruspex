# SLCAN (Lawicel)

SLCAN es un protocolo de texto sencillo por puerto serie para memorias CAN: CANable con el firmware slcan, USBtin, Lawicel CANUSB y muchos dispositivos caseros. Funciona en Windows, Linux y macOS.

## Conectar

1. Conecta la memoria al ordenador y, con un cable, a la toma OBD: CAN high en el pin 6, CAN low en el pin 14, masa en los pines 4 y 5.
2. En la página **Conectar** elige **Interfaz CAN**. Las memorias SLCAN se encuentran con la búsqueda automática de puertos y aparecen en la lista.
3. Elige la velocidad (500 kbit/s para OBD por CAN) y conecta.

## Qué funciona

Como con cualquier interfaz CAN en bruto: OBD, todas las unidades de control por UDS, la búsqueda en paralelo de unidades de control, VW TP 2.0 y escuchar valores difundidos en el bus. CAN FD no es posible por SLCAN.

## Consejos

- Un CANable con el firmware candleLight no es un dispositivo SLCAN: en Linux aparece como interfaz [SocketCAN](socketcan.md).
- La memoria no necesita resistencia de terminación en la toma OBD, el coche ya tiene la suya.

## Línea de comandos

```
caruspex_cli read --can slcan:COM5@500000
caruspex_cli read --can slcan:/dev/ttyACM0@500000
```
