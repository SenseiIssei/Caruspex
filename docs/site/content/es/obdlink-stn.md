# OBDLink y chips STN

Los adaptadores OBDLink (SX, EX, MX+, CX, LX) usan chips STN de OBD Solutions. Entienden todo lo que entiende un ELM327 y más; Caruspex los reconoce por sí solo y aprovecha los extras.

## Qué usa Caruspex

- **Peticiones agrupadas**: Caruspex pide hasta 24 valores en vivo de una vez en lugar de 6. Por Bluetooth, esto hace que una ronda de valores sea casi el doble de rápida.
- **Escucha**: los valores difundidos en el bus (por ejemplo en un Tesla) se graban con los comandos propios del STN, que se detienen solos tras un número fijado de mensajes.
- **Ford MS-CAN y GM Single-Wire-CAN**: las unidades de control de carrocería de Ford y GM no están en el CAN de diagnosis normal. Con un adaptador que tenga el transceptor adicional, la página **Todas las unidades de control** ofrece "Ford: módulos de carrocería en el MS-CAN" o "GM: módulos de carrocería en el CAN de un solo hilo".

| Chip | Adaptador | MS-CAN | SW-CAN |
|---|---|---|---|
| STN2230 a STN2232 | OBDLink EX | sí | no |
| STN1150 a 1152, 1170, 2120, 2255, 2256 | OBDLink MX+ y otros | sí | sí |
| otros | OBDLink SX, LX, CX | no | no |

## Conectar

Los adaptadores OBDLink se conectan como cualquier ELM327: por USB (el EX y el SX), por Bluetooth (MX+, LX, CX) o como se describe en [ELM327 por USB](elm327-usb.md) y [ELM327 por Bluetooth](elm327-bluetooth.md).

## Probado

Un OBDLink con el chip STN2232 pasó la prueba del adaptador: el chip, el firmware y los comandos se reconocieron correctamente. Aún no se ha usado en un coche, así que las peticiones agrupadas, el MS-CAN y el SW-CAN siguen sin confirmar en hardware real.
