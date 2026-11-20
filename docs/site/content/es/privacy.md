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
| Informes de prueba de la primera prueba KKL | solo cuando guardas uno | la carpeta `test-reports` dentro de la carpeta de ajustes; el VIN y el número del inmovilizador se acortan salvo que marques la casilla |

Puedes borrar cualquiera de estos archivos cuando quieras.

## Qué se envía

En cada instalación, también tras actualizar desde una versión anterior, Caruspex pregunta una vez si debe buscar actualizaciones. Solo después de un sí pregunta a GitHub si hay una versión más nueva, al arrancar y cada seis horas mientras está en marcha. Con ello no se envía nada sobre ti ni sobre el coche. Sin un sí no se envía ninguna petición a GitHub. Puedes cambiarlo en cualquier momento en los ajustes, en **Ventana e idioma**. Aparte de eso, Caruspex no se conecta a internet. La comunicación con un adaptador Wi-Fi, un cable ENET o un coche DoIP se queda en tu red local.

Solo las versiones publicadas en GitHub buscan actualizaciones; una copia que compiles tú nunca lo hace. Cuando Caruspex busca o descarga una actualización, GitHub ve, como en cualquier descarga, la dirección IP del ordenador y la versión de Caruspex; Caruspex no envía ni guarda nada más, y nadie más, tampoco el proyecto, lo ve. Para ello rige la declaración de privacidad de GitHub. Las actualizaciones se descargan solo de las versiones del repositorio de Caruspex. Botones como **Informar en GitHub** solo abren tu navegador; lo que publiques allí lo decides tú.

## Android

Hasta Android 11 el sistema solo deja que una aplicación encuentre adaptadores Bluetooth LE con el permiso de ubicación. Caruspex lo pide solo por eso y nunca lee tu ubicación. Desde Android 12 pide en su lugar el permiso de Bluetooth, marcado como no usado nunca para la ubicación.

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
