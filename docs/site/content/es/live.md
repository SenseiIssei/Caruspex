# Valores en vivo

La página **Valores en vivo** (modo profesional) muestra lo que miden los sensores en este momento. En el modo principiante, el **Modo conducción** muestra los valores más importantes en letra grande.

![Valores en vivo](screenshot-live.png)

## Qué hay

- Todos los valores del modo 01 de SAE J1979 hasta el PID C4: 145 PID con más de 300 lecturas, agrupados y con búsqueda. Cuáles ofrece tu coche, te lo dice él mismo.
- Cuentakilómetros, marcha, temperatura del aceite, consumo de combustible, par, presión de turbo, temperaturas de escape, filtro de partículas, AdBlue, NOx y batería híbrida, en la medida en que el coche los indique.
- Una gráfica continua para varios valores a la vez. En ventanas largas (hasta una hora) la gráfica aclara los puntos antiguos, pero conserva cada pico y cada bajada.
- Grabación a CSV.

## Valores del fabricante y propios

- **Baterías de coches eléctricos**: estado de salud, carga, tensiones de celda con su dispersión y temperaturas para Hyundai Kona e Ioniq, Kia e-Niro, Renault Zoe, Nissan Leaf y Tesla Model 3 e Y (escuchando en el bus CAN), en la página **Batería del eléctrico**.
- **Valores de OBDb** para unos 550 vehículos más (presión de neumáticos, aceite, cuentakilómetros y más), que se ofrecen solos para el coche detectado.
- **Tus propios valores** en el formato CSV de Torque, muy extendido, ecuaciones incluidas.

![Batería del eléctrico](screenshot-battery.png)

## ¿Qué rapidez?

Cuántos valores por segundo llegan depende del coche y del adaptador. Los coches con CAN responden rápido; los coches con K-Line son lentos por diseño. Un OBDLink permite a Caruspex pedir hasta 24 valores de una vez (consulta [OBDLink y STN](obdlink-stn.md)); una interfaz CAN en bruto es el camino más rápido.

## Grabar y reproducir

Una sesión entera con el coche se puede grabar en un archivo y reproducir más tarde sin el coche, muy útil para buscar averías. Con **Compartir grabación** se puede preparar una grabación para el proyecto sin datos personales, consulta [privacidad](privacy.md).
