# Cable K+DCAN

El cable K+DCAN es el cable habitual para BMW de 1998 a 2017 aproximadamente con los programas de taller INPA e ISTA. Dentro lleva un chip serie FTDI y un pequeño convertidor para el D-CAN de BMW. Muchos de estos cables tienen un interruptor o un puente entre los pines 7 y 8 para coches más antiguos.

## Estado en Caruspex

> Estado real: Caruspex aún no habla el modo D-CAN de este cable. Los antiguos protocolos de BMW (DS2 y KWP2000 al estilo propio de BMW) por K-Line están previstos, pero antes necesitan pruebas en un coche real.

Lo que puedes usar hoy en su lugar:

- **BMW y Mini a partir de la serie F (desde 2008 aproximadamente)**: un [cable ENET](enet.md) llega a todas las unidades de control por Ethernet, o un [ELM327](elm327-usb.md) sencillo con el direccionamiento propio de BMW por CAN.
- **OBD estándar en BMW con CAN (desde 2007 aproximadamente)**: cualquier ELM327 lee las averías del motor y los valores en vivo.
- **En modo K-Line** el cable deja pasar la K-Line como un [cable KKL](kkl.md). Eso sirve para coches del grupo VW; las unidades de control de BMW en la K-Line necesitan los protocolos que aún faltan.

## Se busca ayuda

Si tienes un cable K+DCAN y un BMW antiguo, un [informe de prueba](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) con una grabación ayuda a construir esta parte.
