# Asistente de conexión y toma OBD

## ¿Dónde está la toma?

Todos los coches vendidos en Europa desde 2001 (gasolina) o 2004 (diésel), y en EE. UU. desde 1996, tienen una toma OBD de 16 pines al alcance del conductor. Sitios habituales:

- debajo del salpicadero, a la izquierda o a la derecha de la columna de dirección,
- encima de los pedales,
- en el guarnecido lateral del hueco de los pies,
- detrás de la tapa de la caja de fusibles,
- en la consola central, a veces detrás de una pequeña tapa.

En más de 200 generaciones de coches y 35 motos, Caruspex conoce el sitio exacto. El asistente de conexión lo marca en un plano del habitáculo y en un primer plano del lado del conductor; en las motos muestra una vista lateral con el conector de diagnosis y el cable adaptador que hace falta.

> La lista de ubicaciones de la toma es una recopilación nuestra y no se ha comprobado en todos los coches. Si en el tuyo el sitio no es correcto, avísanos con un [informe de prueba](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml).

## El asistente de conexión

El asistente se abre solo en el primer arranque. Pregunta cuatro cosas, y cada paso se puede saltar:

1. **¿Qué adaptador?** ELM327 por USB, Bluetooth o Wi-Fi, un cable ENET o un cable KKL. Las imágenes ayudan a distinguirlos.
2. **¿Qué coche?** Elígelo en el catálogo y el asistente te muestra dónde está la toma.
3. **Contacto puesto.** La mayoría de las unidades de control solo responden con el contacto puesto.
4. **Conectar.** Caruspex busca en los puertos y prueba los protocolos.

Más adelante encontrarás el asistente en los ajustes y en la página Conectar.

## Buscar y recordar

Con la búsqueda automática, Caruspex recorre todos los puertos serie, pregunta a cada uno si hay un ELM327 y luego prueba uno a uno los protocolos del coche. Cuando un coche se ha conectado, Caruspex recuerda el camino (adaptador, puerto, protocolo, velocidad) para su VIN y lo prueba primero la próxima vez, así la conexión es más rápida.

## Cuando no conecta

Si no responde ningún adaptador o el coche no dice nada, Caruspex muestra una lista de comprobación corta para ese tipo de conexión, junto con lo que ya ha averiguado: controlador instalado, Bluetooth emparejado, contacto puesto, velocidad correcta, un adaptador clónico que solo aparenta ser una versión más nueva. Las páginas de [adaptadores e interfaces](interfaces.md) explican cada tipo en detalle.
