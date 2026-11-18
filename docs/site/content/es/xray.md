# Vista de rayos X

La **Vista de rayos X** muestra el coche conectado como un modelo de cristal que puedes girar. Dentro ves el motor o los motores eléctricos, la caja de cambios, el depósito o la batería de alta tensión, el escape, el radiador, la batería de 12 V y la toma OBD, colocados según el tipo de carrocería y la tracción del coche detectado.

![Vista de rayos X](screenshot-xray.png)

## Qué se ilumina

- Las **averías** colorean su zona, y la unidad de control responsable se pone en rojo.
- Los **valores en vivo** aparecen como etiquetas sobre la pieza a la que pertenecen: el refrigerante en el radiador, las revoluciones en el motor, la tensión en la batería.
- Las **unidades de control** encontradas en la búsqueda están donde suelen estar, unidas por sus líneas de datos (CAN de tracción, CAN de confort, CAN de diagnosis, K-Line). Mientras Caruspex lee, pequeños paquetes viajan hasta la unidad a la que se pregunta.
- **Revisar coche** lleva la cámara a cada hallazgo, uno tras otro.

Gira con el ratón o con un dedo, haz zoom con la rueda o con dos dedos; el teclado también funciona. Las capas (carrocería, tracción, refrigeración, combustible o alta tensión, electricidad, red) se pueden desactivar una a una.

## ¿Qué tan exacto es el modelo?

La forma sale de las medidas reales de la generación cuando se conocen (de Wikidata o de los datos técnicos del fabricante) y, si no, de valores típicos para el tipo de carrocería y la época. No es una réplica del modelo exacto: los datos de diseño y los esquemas eléctricos de los fabricantes no están disponibles libremente. La página indica de dónde sale la forma.

Si tienes un modelo 3D de tu coche en glTF (`.glb` o `.gltf`), puedes cargarlo con **Cargar tu propio modelo 3D**; el interior sigue saliendo de la disposición calculada.

## Sin 3D

En ordenadores sin WebGL, la página pasa a una vista lateral plana con la misma información.
