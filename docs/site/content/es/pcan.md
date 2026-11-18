# PEAK PCAN (Windows)

Los adaptadores CAN de PEAK-System (PCAN-USB, PCAN-USB FD, PCAN-USB Pro y otros) se pueden usar directamente en Windows, sin controlador J2534. En Linux son interfaces [SocketCAN](socketcan.md).

## Conectar

1. Instala el controlador de dispositivo de PEAK. Trae la biblioteca `PCANBasic.dll`, que Caruspex carga.
2. En la página **Conectar** elige **Interfaz CAN**. Cada canal PCAN con un dispositivo detrás aparece solo en la lista; los canales que ya usa otro programa se dejan fuera.
3. Elige la velocidad (500 kbit/s para OBD por CAN) y conecta.

## Qué funciona

OBD, todas las unidades de control por UDS, la búsqueda en paralelo de unidades de control, VW TP 2.0 y escuchar valores difundidos en el bus. CAN FD por PCAN-Basic aún no está integrado.

## Línea de comandos

```
caruspex_cli read --can pcan:USB1@500000
```
