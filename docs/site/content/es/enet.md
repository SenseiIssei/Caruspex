# Cable ENET de BMW

Los BMW y Mini a partir de la serie F (desde 2008 aproximadamente) tienen Ethernet en la toma OBD. Un cable ENET va de la toma OBD al puerto de red del ordenador, sin ningún chip adaptador en medio. Caruspex habla HSFZ con el gateway del coche (puertos 6811 y 6801) y llega a todas las unidades de control.

## Conectar

1. Enchufa el cable en el coche y en el puerto de red del ordenador (o en un adaptador de red USB). Pon el contacto.
2. Espera hasta un minuto: sin servidor DHCP, Windows, macOS y Linux dan por sí solos una dirección automática al ordenador (169.254.x.x).
3. En la página **Conectar** elige **Cable de red** y deja el campo vacío: Caruspex busca el coche en la red. Si la búsqueda no encuentra nada, escribe la dirección del coche.

## Qué funciona

- Todas las unidades de control por UDS con el direccionamiento de BMW, códigos de avería en el formato de BMW tal como los muestra ISTA (por ejemplo 480A10).
- Memoria de averías, identificación y borrado por unidad de control, valores en vivo por OBD.
- Aquí Caruspex también solo lee. La codificación y la programación se quedan para los programas de taller.

## ¿Qué cable?

Un cable ENET sencillo lleva una resistencia entre los pines 8 y 16 que activa el Ethernet del coche. Los cables sin ella se quedan a oscuras. Los puertos de red de 1 Gbit/s funcionan, el coche habla a 100 Mbit/s.

## Línea de comandos

```
caruspex_cli ecus --enet auto
```
