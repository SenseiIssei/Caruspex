# Cable KKL (VAG 409.1)

Un cable KKL es un cable USB con un chip serie y un driver de K-Line, sin ningún ELM327 dentro. Caruspex maneja él mismo la K-Line, byte a byte. Eso es lo que necesitan los VW, Audi, Seat y Skoda antiguos: su protocolo KWP1281 confirma cada byte por separado, algo que ningún ELM327 puede hacer.

## Qué funciona

- Todas las unidades de control de VW, Audi, Seat y Skoda de 1990 a 2004 aproximadamente: motor, caja de cambios, ABS, airbag, cuadro de instrumentos, confort y más.
- KWP1281 y KWP2000 (ISO 14230), detectados automáticamente por los bytes clave tras el despertar.
- Memoria de averías por unidad de control, bloques de valores de medición y el ajuste básico real de la mariposa (bloque de valores 060 o 098).
- Los canales de adaptación son de solo lectura a propósito. La prueba de salidas activa piezas reales y solo empieza tras tu confirmación.

Estas páginas están en el **modo profesional**: **Unidades de control por KKL**, **Mariposa** y **Adaptación y pruebas de salidas**. Cada una abre el cable por sí misma: elige el puerto serie, la unidad de control y empieza.

## ¿Qué cable?

Los cables con un FTDI FT232RL original o un CP2102 son mucho más fiables en la K-Line que los clones más baratos. Para KWP1281 se recomienda un FTDI original. El cable tiene que dejar pasar la K-Line en bruto: las interfaces "HEX-USB" con firmware propio no funcionan.

Los coches de antes de 1996 aproximadamente a menudo no tienen toma OBD, sino un conector 2x2 en el vano motor o debajo del salpicadero. Un adaptador de 2x2 a la toma de 16 pines es barato.

## Comprobar el cable

La herramienta de línea de comandos prueba el cable sin coche:

```
caruspex_cli kklcheck --port COM3
```

## Primera prueba en el coche

La página del modo profesional **Primera prueba en el coche** te guía en la primera sesión con un cable KKL nuevo. Está pensada para el Audi A2 y funciona en cualquier VW, Audi, Seat o Skoda con K-Line. Solo lee: no borra nada, ni prueba de salidas ni ajuste básico. El puerto solo se abre cuando empieza una prueba.

1. **Comprueba el cable sin el coche.** Enchufa el cable solo al USB, elige su puerto y pulsa **Comprobar cable**. Caruspex lee el chip (FTDI, CH340, Prolific, CP210x), en un FTDI el temporizador de latencia, y mide los bits de una dirección a 5 baudios en la línea de break. Sin el coche no vuelve ningún eco, y así debe ser.
2. **Pon el temporizador de latencia en 1 ms** (cables FTDI en Windows): Administrador de dispositivos, Puertos (COM y LPT), doble clic en el USB Serial Port, Configuración de puerto, Opciones avanzadas, Latency Timer (msec) a 1, OK, y luego desenchufa y vuelve a enchufar el cable una vez. Con los 16 ms de fábrica, la respuesta a los bytes de clave puede llegar fuera de su margen de 50 ms. Caruspex lee este ajuste, pero nunca lo cambia.
3. **En el coche:** contacto puesto, motor apagado, el cable bien metido en la toma OBD y conectado directamente al ordenador (sin hub USB), ningún otro programa de diagnóstico abierto. Marca la lista y pulsa **Iniciar la primera prueba**.
4. La prueba despierta el motor (dirección 01) y mide cada bit de la dirección, el byte de sincronización 0x55, los bytes de clave y la respuesta a ellos. Después lee la identificación, la memoria de averías y los bloques de valores medidos 001 a 003, y a continuación el cuadro de instrumentos 17, ABS 03, airbag 15, confort 46 y electricidad central 09. Una unidad de control que no responde puede que simplemente no esté montada.
5. **Guardar informe** guarda un archivo de texto y un archivo JSON en la carpeta `test-reports` dentro de la carpeta de ajustes de Caruspex. **Abrir el informe de prueba en GitHub** rellena por ti el formulario del informe. El VIN y el número del inmovilizador se acortan a menos que marques la casilla.

Qué significan los resultados:

| Resultado | Qué hacer |
|---|---|
| Sin eco | El cable no se oye a sí mismo: el conector no está bien metido o al pin 16 le faltan los 12 V (revisa el fusible de la toma de diagnóstico). Sin el coche, esto es normal. |
| El eco está alterado | Otro dispositivo en la toma, el interruptor de un cable K+DCAN no está en K-Line, o un cable defectuoso. |
| La dirección de activación tiene una temporización demasiado irregular | Los bits se desvían más de 20 ms de 200 ms: sin hub USB, cierra otros programas, temporizador de latencia a 1 ms. Caruspex lo intenta otra vez con las llamadas de break adelantadas. |
| Cable correcto, pero sin respuesta del coche | ¿Está el contacto puesto? La dirección 01 es el motor. Los coches a partir de 2005 aproximadamente se diagnostican por CAN. |
| La respuesta a los bytes de clave llegó demasiado tarde | Pon el temporizador de latencia en 1 ms. |

## Problemas típicos

- **La unidad de control no responde**: ¿contacto puesto? El despertar a 5 baudios necesita una temporización exacta; los hubs USB y algunas máquinas virtuales la estropean, conecta el cable directamente.
- **Se corta al cabo de un rato**: los cables baratos pierden bytes. Prueba otro cable antes de sospechar del coche.
