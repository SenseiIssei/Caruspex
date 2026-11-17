# SocketCAN (Linux)

SocketCAN es la capa CAN del kernel de Linux. Cualquier adaptador CAN con controlador en el kernel se puede usar a través de ella: PEAK PCAN-USB, Kvaser, CANable con el firmware candleLight, placas CAN para Raspberry Pi y muchos más.

## Preparar

Activa la interfaz con la velocidad del coche (500 kbit/s para OBD por CAN):

```
sudo ip link set can0 up type can bitrate 500000
```

Para CAN FD en los coches más nuevos, también con una velocidad de datos:

```
sudo ip link set can0 up type can bitrate 500000 dbitrate 2000000 fd on
```

## Conectar

En la página **Conectar** elige **Interfaz CAN**. Todas las interfaces CAN del ordenador están en la lista (`can0`, `can1`, etc.). Elígela, elige la velocidad y, para CAN FD, activa la casilla CAN FD.

## Qué funciona

OBD, todas las unidades de control por UDS, la búsqueda en paralelo de unidades de control, VW TP 2.0, escuchar valores difundidos en el bus y CAN FD.

## Línea de comandos

```
caruspex_cli interfaces
caruspex_cli read --can socketcan:can0@500000
caruspex_cli read --can socketcan:can0+fd@500000
```
