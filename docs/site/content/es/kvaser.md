# Kvaser (Windows)

Los adaptadores CAN de Kvaser (Leaf, U100, Memorator y otros) se pueden usar directamente en Windows a través de la biblioteca CANlib de Kvaser. En Linux son interfaces [SocketCAN](socketcan.md).

## Conectar

1. Instala los controladores de Kvaser. Traen `canlib32.dll`, que Caruspex carga.
2. En la página **Conectar** elige **Interfaz CAN**. Cada canal Kvaser con un dispositivo detrás aparece en la lista; los canales virtuales del controlador se dejan fuera.
3. Elige la velocidad (500 kbit/s para OBD por CAN) y conecta.

## Qué funciona

OBD, todas las unidades de control por UDS, la búsqueda en paralelo de unidades de control, VW TP 2.0 y escuchar valores difundidos en el bus. CAN FD por CANlib aún no está integrado.

## Línea de comandos

```
caruspex_cli read --can kvaser:0@500000
```
