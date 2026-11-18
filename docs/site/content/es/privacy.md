# Privacidad

Caruspex funciona en tu ordenador. No tiene cuenta, ni nube, ni seguimiento.

## Qué se guarda

Todo se queda en tu ordenador:

| Qué | Cuándo | Dónde |
|---|---|---|
| Ajustes | siempre | la carpeta de ajustes de tu sistema (el botón **Abrir la ubicación de almacenamiento** de los ajustes la muestra) |
| Garaje: VIN, coche, revisiones, lecturas del cuentakilómetros, la última conexión que funcionó | tras conectar un coche | junto a los ajustes, en la carpeta de datos de Caruspex |
| Grabaciones y archivos CSV | solo cuando tú las inicias | donde las guardes |
| Archivo de registro | solo si lo activas en los ajustes | la ruta que elijas |
| Informes de fallos | solo si Caruspex se cuelga | la carpeta `crash-reports` junto al garaje |

Puedes borrar cualquiera de estos archivos cuando quieras.

## Qué se envía

Al arrancar, Caruspex pregunta a GitHub si hay una versión más nueva. Con ello no se envía nada sobre ti ni sobre el coche. La comprobación se puede desactivar en los ajustes. Aparte de eso, Caruspex no se conecta a internet. La comunicación con un adaptador Wi-Fi, un cable ENET o un coche DoIP se queda en tu red local.

## Informes de fallos solo si tú quieres

Si Caruspex se cuelga, escribe un informe breve en tu ordenador: versión, sistema, el punto del código y la cadena de llamadas. Los VIN, los bytes del tráfico con el coche y tu carpeta personal se recortan. No se envía nada. Caruspex te muestra el informe y te ofrece dos botones: **Informar en GitHub** abre en tu navegador una incidencia ya rellenada que lees y cambias antes de publicarla, **No informar** la descarta.

## Compartir grabaciones sin datos personales

Las grabaciones de coches reales ayudan a que Caruspex funcione mejor en más coches. **Compartir grabación** prepara una copia y te muestra exactamente qué se queda y qué se va:

- **Se quita**: los últimos dígitos del VIN (en todo el archivo), el nombre del puerto y la dirección Bluetooth o de red del adaptador, la hora del día.
- **Nunca en una grabación**: tu ubicación y tus datos de usuario.
- **Se queda**: las peticiones y respuestas, el tipo de adaptador, el protocolo y la fecha.

El archivo se queda en tu ordenador hasta que lo guardes y lo adjuntes tú mismo a una incidencia de GitHub. No se sube nada automáticamente.

## El programa de pruebas

El [formulario del informe de prueba](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) pide el coche, el año, el motor y el adaptador. Solo pide un VIN abreviado; nunca publiques el VIN completo, tu matrícula ni tu nombre.
