# ELM327 por USB

Un conector para la toma OBD con un cable al puerto USB. Es el tipo de ELM327 más fiable: sin emparejar, sin radio, sin pilas gastadas.

## Controlador

Dentro del conector, un chip USB convierte el cable en un puerto serie (puerto COM en Windows). El controlador que hace falta depende de ese chip:

| Chip | Controlador |
|---|---|
| FTDI FT232 | normalmente lo instala el propio Windows, viene integrado en Linux y macOS |
| CH340 / CH341 | normalmente lo instala el propio Windows, viene integrado en Linux y macOS |
| Silicon Labs CP210x | de la web del fabricante si Windows no lo encuentra |
| Prolific PL2303 | de la web del fabricante; los clones antiguos a menudo solo funcionan con un controlador antiguo |

Después de enchufarlo, el Administrador de dispositivos de Windows muestra el adaptador en "Puertos (COM y LPT)".

## Conectar

1. Enchufa el adaptador en el coche, pon el contacto y conecta el cable al ordenador.
2. En la página **Conectar** elige **Puerto USB o COM**.
3. Deja el puerto en **Buscar automáticamente** o elígelo de la lista, y conecta.

La búsqueda automática prueba todos los puertos serie a las velocidades habituales. Si conoces la velocidad de tu adaptador, puedes ajustarla en los ajustes, en **Velocidad en baudios del adaptador**.

## Linux y macOS

En Linux el adaptador aparece como `/dev/ttyUSB0` o `/dev/ttyACM0`. Tu usuario tiene que estar en el grupo `dialout` (consulta [Instalar](install.md)). En macOS aparece como `/dev/cu.usbserial-...` o `/dev/cu.wchusbserial...`.

## Problemas típicos

- **No aparece ningún puerto**: falta el controlador o el cable es solo de carga.
- **El adaptador responde, el coche no**: contacto quitado, o el adaptador es un clon que no maneja el protocolo del coche. La [prueba del adaptador](interfaces.md) te dice más.
- **Valores en vivo muy lentos en coches antiguos**: la propia K-Line es lenta, no es culpa del adaptador.
