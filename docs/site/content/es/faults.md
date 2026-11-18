# Averías

La página **Memoria de averías** lee lo que han guardado las unidades de control y lo explica con palabras sencillas.

![Memoria de averías](screenshot-faults.png)

## Qué ves

- Códigos **guardados, pendientes y permanentes**. Con varias unidades de control, cada código muestra quién lo ha registrado (motor, caja de cambios).
- Unos 1.400 códigos estándar (P, C, B, U) y los códigos de fabricante de VAG, Ford, GM, Toyota, Honda, Nissan, Hyundai y Kia, Subaru y más. Un código P1 se interpreta como lo entiende la marca detectada.
- Un **semáforo** por código y la pregunta que todos se hacen primero: ¿puedo seguir conduciendo?
- Las causas habituales, cosas que puedes comprobar tú mismo y una clase de coste aproximada.
- Dónde está la pieza: el coche en vista transparente, o la [vista de rayos X](xray.md).

## Todas las unidades de control

En el modo profesional, la página **Todas las unidades de control** pregunta a cada unidad de control, no solo al motor: ABS, airbag, cuadro de instrumentos, climatización y más. Cada avería viene con su tipo de fallo (cortocircuito, circuito abierto, señal no plausible), referencia de la pieza y versión de software. Los coches más antiguos del grupo VW se leen con un [cable KKL](kkl.md).

## ¿Qué hago ahora?

La página **¿Qué hacer?** ordena todo lo que ha encontrado Caruspex en tres pasos: parar ya, al taller pronto, puede esperar. Cada punto tiene una frase sencilla con el porqué, un rango de coste aproximado y una nota para enseñar en el taller. El OBD no puede ver si el testigo del motor parpadea o si hay un testigo rojo encendido, así que dos casillas te lo preguntan; un testigo parpadeando con fallos de encendido pasa el punto a "parar ya".

![Qué hago ahora](screenshot-todo.png)

## Borrar códigos

Se puede borrar por unidad de control, con un aviso claro, y Caruspex vuelve a leer la memoria justo después.

> Borrar no es reparar. También reinicia los monitores de disponibilidad: hasta que el coche haya hecho varios ciclos de conducción completos, no pasa una prueba de emisiones, y un código cuya causa sigue ahí vuelve tras un trayecto corto.

## Exportar

La lista se puede guardar como CSV, y el informe de revisión como PDF.
