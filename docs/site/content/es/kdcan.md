# Cable K+DCAN

El cable K+DCAN es el cable habitual para BMW de 1998 a 2017 aproximadamente con los programas de taller INPA e ISTA. Dentro lleva un chip serie FTDI y un pequeño convertidor para el D-CAN de BMW. Muchos de estos cables tienen un interruptor o un puente entre los pines 7 y 8 para coches más antiguos.

## Estado en Caruspex

> Estado real: Caruspex lee los BMW de la serie E por la K-Line de este cable: DS2 en los modelos antiguos, KWP2000 al estilo propio de BMW y BMW-FAST en la generación E60. Hasta ahora solo se ha probado con los coches simulados del programa, todavía no en un BMW real. Caruspex aún no habla el modo D-CAN del cable.

## En modo K-Line

- **Interruptor en K-Line** antes de meter el cable en la toma; en la mayoría de cables es la posición izquierda. Une los pines 7 y 8, para que también respondan las centralitas del pin 8 de los modelos antiguos.
- **Qué coches**: E36 (con el conector redondo de 20 pines mediante un adaptador), E38, E39, E46, E52, E53, E83, E85 y la generación E60 (E60, E61, E63, E64, E65, E87, E90) hasta 03/2007. Después, y desde la serie F, la toma habla D-CAN o Ethernet.
- **Cómo**: modo profesional activado, página *BMW por K-Line*, elegir el puerto, *Buscar*. Caruspex pregunta cada dirección conocida por DS2 (9600 baudios, paridad par), KWP2000 (10400 baudios con inicialización rápida) y BMW-FAST (115200 baudios) y muestra las centralitas encontradas con su identificación y memoria de averías. Tarda medio minuto aproximadamente.
- **Borrar** funciona por centralita o en todas, siempre tras preguntar. Después Caruspex vuelve a leer lo que queda.
- **Códigos de avería**: BMW no publica textos para sus códigos, por eso Caruspex los muestra como número hexadecimal.
- **Solo lectura**: aquí no hay codificación, adaptaciones ni pruebas de actuadores; eso es cosa del software de taller.
- **Un cable KKL** funciona igual, pero en los modelos antiguos solo llega a las centralitas del pin 7 si los pines 7 y 8 no están unidos.

Para probar sin coche, la lista de puertos de la página tiene un E46 (DS2) y un E60 (KWP2000) simulados.

## Qué más funciona

- **BMW y Mini a partir de la serie F (desde 2008 aproximadamente)**: un [cable ENET](enet.md) llega a todas las unidades de control por Ethernet, o un [ELM327](elm327-usb.md) sencillo con el direccionamiento propio de BMW por CAN.
- **OBD estándar en BMW con CAN (desde 2007 aproximadamente)**: cualquier ELM327 lee las averías del motor y los valores en vivo.

## Se busca ayuda

Si tienes un cable K+DCAN y un BMW antiguo, un [informe de prueba](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) con el registro ayuda a confirmar la parte de K-Line en un coche real.
