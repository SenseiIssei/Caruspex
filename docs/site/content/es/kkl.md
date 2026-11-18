# Cable KKL (VAG 409.1)

Un cable KKL es un cable USB con un chip serie y un driver de K-Line, sin ningún ELM327 dentro. Caruspex maneja él mismo la K-Line, byte a byte. Eso es lo que necesitan los VW, Audi, Seat y Skoda antiguos: su protocolo KWP1281 confirma cada byte por separado, algo que ningún ELM327 puede hacer.

## Qué funciona

- Todas las unidades de control de VW, Audi, Seat y Skoda de 1990 a 2004 aproximadamente: motor, caja de cambios, ABS, airbag, cuadro de instrumentos, confort y más.
- KWP1281 y KWP2000 (ISO 14230), detectados automáticamente por los bytes clave tras el despertar.
- Memoria de averías por unidad de control, bloques de valores de medición y el ajuste básico real de la mariposa (bloque de valores 060 o 098).
- Los canales de adaptación y las pruebas de salidas son de solo lectura a propósito.

Estas páginas están en el **modo profesional**: **Unidades de control por KKL**, **Mariposa** y **Adaptación y pruebas de salidas**. Cada una abre el cable por sí misma: elige el puerto serie, la unidad de control y empieza.

## ¿Qué cable?

Los cables con un FTDI FT232RL original o un CP2102 son mucho más fiables en la K-Line que los clones más baratos. Para KWP1281 se recomienda un FTDI original. El cable tiene que dejar pasar la K-Line en bruto: las interfaces "HEX-USB" con firmware propio no funcionan.

Los coches de antes de 1996 aproximadamente a menudo no tienen toma OBD, sino un conector 2x2 en el vano motor o debajo del salpicadero. Un adaptador de 2x2 a la toma de 16 pines es barato.

## Comprobar el cable

La herramienta de línea de comandos prueba el cable sin coche:

```
caruspex_cli kklcheck --port COM3
```

## Problemas típicos

- **La unidad de control no responde**: ¿contacto puesto? El despertar a 5 baudios necesita una temporización exacta; los hubs USB y algunas máquinas virtuales la estropean, conecta el cable directamente.
- **Se corta al cabo de un rato**: los cables baratos pierden bytes. Prueba otro cable antes de sospechar del coche.
