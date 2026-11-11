# Caruspex

<!-- languages -->
[English](README.md) · [Deutsch](README.de.md) · **Español** · [Français](README.fr.md) · [Português](README.pt.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Čeština](README.cs.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [Türkçe](README.tr.md) · [العربية](README.ar.md) · [中文](README.zh.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [हिन्दी](README.hi.md) · [Bahasa Indonesia](README.id.md)
<!-- /languages -->

[![Build](https://github.com/SenseiIssei/Caruspex/actions/workflows/build.yml/badge.svg)](https://github.com/SenseiIssei/Caruspex/actions/workflows/build.yml)
![Rust](https://img.shields.io/badge/Rust-2024-orange)
![Tauri 2](https://img.shields.io/badge/Tauri-2-24c8db)
![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)

Diagnóstico de vehículos de código abierto para Windows, Linux y macOS, escrito en Rust con una interfaz Tauri 2.
Un cable o adaptador, una aplicación: leer y borrar códigos de avería, ver datos de sensores en vivo,
comprobar la disponibilidad para las emisiones y hablar con las unidades de control de VW/Audi a nivel de byte por un cable K-Line.

El nombre viene del *haruspex*, el adivino romano que leía los presagios en las entrañas. Caruspex lee las del coche.

La aplicación habla 18 idiomas, tanto la interfaz como cada texto de los códigos de avería: inglés, alemán, español,
francés, portugués, italiano, neerlandés, polaco, checo, ruso, ucraniano, turco, árabe (de derecha a izquierda), chino,
japonés, coreano, hindi e indonesio. Sigue el idioma del sistema, se puede cambiar en los ajustes o
forzar en cada ejecución con `--language fr` y similares. Cómo se hacen y se revisan las traducciones está en
[translations/TRANSLATING.md](translations/TRANSLATING.md).

![Resumen con indicadores en vivo](docs/screenshot-overview.png)

## Lo más destacado

- **Dos pilas de diagnóstico completas**: OBD-II/EOBD estándar con cualquier adaptador ELM327, y diagnóstico
  del fabricante de VW/Audi (KWP1281 y KWP2000) con un cable KKL K-Line directo.
- **Protocolos implementados desde cero**: KWP1281 confirma cada byte con su complemento a uno,
  algo que el firmware del ELM327 no puede hacer. Caruspex maneja la K-Line por sí mismo, incluida la dirección de
  activación a 5 baudios, que se genera bit a bit conmutando la señal de break del puerto serie con tiempos basados en plazos, de modo que
  la irregularidad de la planificación del sistema operativo no se acumula a lo largo de los diez bits.
- **Detección automática del protocolo**: tras la activación, los dos bytes de clave de la unidad de control indican si habla
  KWP1281 o KWP2000 (ISO 14230); Caruspex cambia en consecuencia.
- **Espacio de trabajo Rust modular**: un crate por área (enlaces con el adaptador, protocolos, red y CAN, vehículos, datos,
  grabaciones, análisis, coche de demostración, herramienta de línea de comandos) y encima la aplicación Tauri. El núcleo no sabe nada
  de la interfaz, así que la aplicación y la herramienta de línea de comandos ejecutan exactamente el mismo código.
- **Probado sin coche**: unidades de control simuladas (`crates/caruspex-protocols/tests/common/`) reproducen el
  intercambio byte a byte, pasarelas falsas hacen de ENET y DoIP, un controlador J2534 falso cubre la ruta PassThru
  y un simulador ELM327 en Python permite ejecuciones completas de la aplicación.
- **Un único conjunto de traducciones**: el núcleo y la interfaz comparten los mismos archivos de traducción en
  `translations/<code>/`, en 18 idiomas. El núcleo busca sus textos por el texto en inglés, la interfaz construye
  sus catálogos a partir de los mismos archivos.
- **Reconoce cualquier coche**: un decodificador de VIN con 365 códigos de fabricante y un catálogo de más de 2100
  generaciones de modelos desde 1980 indican qué coche está conectado y qué funciona con él.
- **Coche de demostración integrado**: un ELM327 en el propio proceso con un motor y una caja de cambios en CAN, para probarlo todo
  sin hardware.
- **Datos generados, no escritos a mano**: unos 1700 códigos de avería, muchos de ellos calculados a partir de familias de códigos regulares
  (por cilindro, por bancada, por sonda de oxígeno) con `tools/gen_dtc.py`.

## Funciones

### Conoce el coche
- Tras conectar, Caruspex lee el VIN y te dice qué coche es: marca, modelo, generación, año del modelo.
- Un catálogo de más de 2100 generaciones de modelos de 94 marcas, de 1980 a hoy, incluidas marcas que ya no existen.
- Para cada generación: qué conector de diagnóstico, dónde está, qué protocolo y si funcionará OBD-II, un cable KKL o solo las herramientas del propio fabricante.
- Un verificador de VIN para cualquier VIN: fabricante, país, año del modelo, dígito de control, código del modelo.
- Un garaje que reconoce cada coche por su VIN y guarda el historial de sus revisiones y lecturas del cuentakilómetros.

![Vehículos](docs/screenshot-vehicles.png)

### Chequeo de salud con un botón
- Lee la memoria de averías, los monitores de disponibilidad y las pruebas de componentes, y luego vigila la mezcla, las temperaturas, las sondas de oxígeno y la tensión de carga durante medio minuto.
- Una puntuación de 0 a 100 en la que cada punto perdido se explica con palabras sencillas, con el siguiente paso.
- ¿Listo para la prueba de emisiones? Qué monitor sigue abierto y cómo conducir para completarlo.
- Un informe de inspección en PDF, para imprimir o para el taller.

![Chequeo de salud](docs/screenshot-health.png)

### Memoria de averías
- Códigos almacenados, pendientes y permanentes; con varias unidades de control, cada código muestra quién lo notificó (motor, caja de cambios).
- Unos 1400 códigos estándar (P, C, B, U) más códigos de fabricante para VAG, Ford, GM, Toyota, Honda, Nissan, Hyundai/Kia, Subaru y más. Un código P1 se lee como lo entiende la marca detectada.
- Un semáforo por código y la pregunta que todo el mundo hace primero: ¿puedo seguir conduciendo?
- Causas habituales, cosas que puedes comprobar tú mismo y una clase de coste aproximada.
- El coche en vista transparente muestra dónde está la pieza afectada.
- Borrado con una advertencia clara y relectura automática; exportación a CSV.

![Memoria de averías](docs/screenshot-faults.png)

### Valores en vivo
- Todos los valores del modo 01 de SAE J1979 hasta el PID C4: 145 PID con más de 300 lecturas, agrupadas y con búsqueda.
- Cuentakilómetros, marcha, temperatura del aceite, consumo de combustible, par, presión del turbo, temperaturas de escape, filtro de partículas, AdBlue, NOx y batería híbrida, en la medida en que el coche los comunique.
- Gráfico continuo y grabación en CSV.

![Valores en vivo](docs/screenshot-live.png)

### Medir
- Ordenador de a bordo: consumo en vivo y medio, coste, CO2, distancia y una puntuación de estilo de conducción suave, incluso para coches sin ordenador de a bordo propio.
- Rendimiento: de 0 a 100 km/h, de 80 a 120, cuarto de milla y distancia de frenado a partir de la señal de velocidad (solo en vías cerradas).
- Batería y alternador: tensión en reposo, caída en el arranque, tensión de carga con y sin carga, guiado en cuatro pasos.

### Inspección
- Monitores de disponibilidad en palabras sencillas, datos de congelación, VIN.
- Supervisión de componentes del modo 06 con valor y límites, recuento de fallos de encendido por cilindro.
- Nombres de unidades de control, ID de calibración y sumas de verificación del modo 09, por unidad de control.
- Terminal ELM327 en bruto.

### Comprobación del adaptador
- ¿ELM327 original o clon? Pide los comandos de cada versión, mide el tiempo de respuesta y dice con honestidad qué se deduce de ello.
- Reconoce OBDLink y otros adaptadores STN.
- Nunca escribe nada en el adaptador.

![Comprobación del adaptador](docs/screenshot-adapter.png)

### Todos los cables, todas las vías de entrada al coche
Caruspex averigua por sí mismo qué está conectado y qué habla el coche.

| Interfaz | Ejemplos | Cómo |
|---|---|---|
| ELM327 por USB, Bluetooth, Bluetooth LE, Wi-Fi | ELM327 barato, Vgate, vLinker, Veepeak | puerto serie, emparejado o BLE directo, TCP |
| Chips STN | OBDLink SX, EX, MX+, CX | como ELM327, con tiempos más rápidos |
| SAE J2534 PassThru | Tactrix OpenPort 2.0, Mongoose, VCX Nano, Scanmatik, Godiag, VAS 5054A en modo J2534 | el controlador de 64 bits del fabricante, CAN con ISO-TP a cargo de Caruspex |
| SLCAN / Lawicel | CANable, USBtin, CANUSB | puerto serie, encontrado por la búsqueda automática de puertos |
| SocketCAN (Linux) | PEAK PCAN-USB, Kvaser, CANable con candleLight, placas CAN para Raspberry Pi | can0 y compañía |
| Cable KKL (VAG 409.1) | cables FTDI o CP2102 | K-Line, KWP1281 y KWP2000 |
| Cable ENET | BMW y Mini desde la serie F | HSFZ sobre Ethernet |
| Cable DoIP | muchos coches desde 2016 aproximadamente | ISO 13400 sobre Ethernet |

En todos ellos funcionan las mismas funciones: OBD-II y OBD sobre UDS, todas las unidades de control sobre UDS, direccionamiento
BMW, escucha de valores de difusión (Tesla), grabación y reproducción.

![Conectar](docs/screenshot-connect.png)

### Todas las unidades de control por CAN (UDS)
- Con un ELM327 normal, Caruspex pregunta a cada unidad de control una por una: motor, caja de cambios, ABS, airbag, cuadro de instrumentos, climatización y más, en la mayoría de los coches desde 2008 aproximadamente.
- Memoria de averías por unidad con el tipo de fallo (cortocircuito, circuito abierto, no plausible ...), número de pieza, versión de software y de hardware.
- Listas de direcciones para VAG, BMW y Mini, Ford, Mazda, GM, Toyota, Nissan, Hyundai/Kia, Renault y PSA; para todo lo demás, un barrido por las direcciones habituales.
- BMW y Mini desde la serie F con el direccionamiento extendido propio de BMW sobre 6F1, además de un barrido por las 128 direcciones de diagnóstico; códigos de avería en el formato de BMW tal como los muestra ISTA (p. ej. 480A10).
- Borrado por unidad, con una advertencia.

![Todas las unidades de control](docs/screenshot-ecus.png)

### VW y Audi con un cable KKL (KWP1281 y KWP2000)
- Adaptación de la mariposa: el ajuste básico real en el bloque de medición 060 o 098.
- Escaneo de todas las unidades de control, incluidos ABS, airbag, cuadro de instrumentos y confort, con un informe imprimible.
- Canales de adaptación y pruebas de salidas, de solo lectura a propósito.

### Valores del fabricante y personalizados
- Baterías de coches eléctricos: estado de salud, carga, tensiones de celdas con dispersión, temperaturas, para Hyundai Kona e Ioniq, Kia e-Niro, Renault Zoe, Nissan Leaf y Tesla Model 3 e Y (escuchando el bus CAN); cada valor con una fuente documentada.
- Carga tus propios valores en el formato CSV de Torque, muy extendido, con un evaluador completo de las ecuaciones de Torque.

![Batería del eléctrico](docs/screenshot-battery.png)

### Grabar y reproducir
- Graba una sesión con el coche en un archivo y reprodúcela más tarde sin el coche, útil para localizar averías.

### Sin coche ni adaptador
- Un coche de demostración integrado: circula, se calienta, tiene algunas averías almacenadas y habla como un ELM327 real por CAN.
- Un léxico sin conexión de todos los códigos, que se puede filtrar por marca.

### Aspecto y manejo
- Una interfaz nueva al estilo de Odysync: tranquila, oscura, con transiciones suaves.
- Conexión paso a paso con una búsqueda automática por todos los puertos.
- 18 idiomas; los ajustes se aplican al instante.
- Te avisa cuando hay una versión nueva: al arrancar pregunta a GitHub por la última versión y nada más (ningún dato
  sobre ti ni sobre el coche); se puede desactivar en los ajustes.

## Vehículos compatibles

Cualquier vehículo con OBD-II o EOBD: en Europa, coches de gasolina desde 2001 y diésel desde 2004; en EE. UU., desde 1996. Los VW, Audi, Seat y Skoda más antiguos, desde 1990 aproximadamente, funcionan con el cable KKL. La página Vehículos te dice qué vía funciona para un coche concreto.

| Época | Bus | Ejemplos |
|---|---|---|
| hasta 1995 aproximadamente | específico del fabricante | VAG con KKL por los conectores 2x2, en los demás solo herramientas de taller |
| de 1996 a 2003 aproximadamente | K-Line (ISO 9141-2, KWP2000) | Audi A2, VW Golf IV, Toyota, Renault |
| desde 2004 aproximadamente | CAN (ISO 15765-4) | Audi A4 B7, VW Golf V, coches japoneses modernos |
| EE. UU. hasta 2005 aproximadamente | SAE J1850 PWM y VPW | Ford, Chevrolet |
| los coches más nuevos | OBD sobre UDS (SAE J1979-2) | obligatorio en EE. UU. desde el año modelo 2027, ya presente en algunos coches recientes; detectado y traducido automáticamente |
| BMW y Mini desde la serie F | ENET (HSFZ sobre Ethernet) o D-CAN con direccionamiento 6F1 | todas las unidades de control |
| muchos coches desde 2016 aproximadamente | DoIP (ISO 13400) con un cable DoIP | OBD sobre Ethernet, encontrado automáticamente |
| Tesla Model 3 e Y | sin OBD: tramas CAN de difusión | batería, celdas, temperaturas, velocidad, cuentakilómetros por escucha (cable adaptador al conector de diagnóstico) |

Los coches totalmente eléctricos no tienen por qué ofrecer datos de emisiones OBD-II. Algunos responden en parte de todos modos; para los coches que no responden pero tienen tráfico en el bus, Caruspex se conecta para escuchar y lee lo que saben los paquetes integrados (por ahora Tesla Model 3 e Y).

## Adaptadores compatibles

- Cualquier adaptador compatible con **ELM327**: USB, Bluetooth clásico (directamente por RFCOMM en Windows y Linux una vez emparejado, o por su puerto COM), **Bluetooth LE**
  (Vgate iCar Pro BLE, vLinker, OBDLink CX, Veepeak BLE y otros adaptadores con los servicios GATT habituales; se encuentran con un
  escaneo, sin emparejar) o Wi-Fi por TCP
- Un **cable ENET** (OBD a RJ45) para BMW y Mini desde la serie F: Caruspex encuentra el coche en la red
  (HSFZ, puertos 6811 y 6801) y lee todas las unidades de control por Ethernet, sin chip adaptador de por medio. El ordenador
  necesita una dirección automática (169.254.x.x), que Windows, macOS y Linux asignan por sí mismos cuando ningún DHCP responde.
  Caruspex aquí también solo lee; la codificación y el flasheo quedan para E-Sys, ISTA o las herramientas de tuning.
- Cualquier **cable KKL (VAG 409.1)**. Los cables basados en FTDI o CP2102 son mucho más fiables en la K-Line que los
  clones más baratos; para KWP1281 se recomienda un FTDI FT232RL original.

Las interfaces propietarias como Delphi DS150E, Autocom o VAS 5054A no son compatibles, no hablan ELM327.

| Función | ELM327 | KKL 409.1 |
|---|---|---|
| Leer y borrar averías del motor | sí | sí |
| Valores en vivo, datos de congelación, disponibilidad | sí | sí |
| ABS, airbag, cuadro de instrumentos, confort | **no** | sí |
| Bloques de medición del fabricante | **no** | sí |
| **Adaptación de la mariposa** | **no** | sí |

Es un límite del hardware, no del software: KWP1281 es un protocolo de bloques en el que cada byte es
confirmado por la otra parte. El firmware del ELM327 solo gestiona pares completos de petición y respuesta.
Un cable KKL transmite la K-Line en bruto, así que Caruspex puede hablar el protocolo por sí mismo.

### Dos protocolos VAG, detectados automáticamente

| Protocolo | Típico de | Ajustes básicos |
|---|---|---|
| KWP1281 (protocolo de bloques) | motores 1.4 MPI | comando propio (bloque 0x28) |
| KWP2000 (ISO 14230) | 1.6 FSI con Bosch MED | sesión de diagnóstico propia y después lectura de bloque |

Los canales de adaptación y las pruebas de salidas están implementados por ahora en la ruta KWP1281; en KWP2000 la
aplicación lo indica en lugar de fallar en silencio.

Todavía no implementado: diagnóstico del fabricante por CAN con VW TP 2.0 (vehículos desde 2004 aproximadamente).

## Tecnología

| Área | Tecnología |
|---|---|
| Lenguaje | Rust (edición 2024) |
| Interfaz | Tauri 2 con React 19 y TypeScript, compilado con Vite |
| Bibliotecas | `serialport` (puertos serie), `btleplug` (Bluetooth LE), `libloading` (controladores J2534) |
| Localización | Un único conjunto de archivos JSON en `translations/` para núcleo e interfaz, 18 idiomas |
| Compilación | Workspace de Cargo, npm para la interfaz |
| Pruebas | `cargo test` contra unidades de control simuladas, pasarelas falsas y un controlador J2534 falso |
| Herramientas | Python 3 (generadores de código y tablas, revisión de traducciones, simulador ELM327) |
| Plataformas | Windows 10 y 11, Linux, macOS |

## Descarga

Los paquetes listos están en la [página de versiones](https://github.com/SenseiIssei/Caruspex/releases):

| Sistema | Archivo | Notas |
|---|---|---|
| Windows 10 y 11, 64 bits | `Caruspex-<version>-windows-x64-setup.exe` | Se instala sin derechos de administrador y puede añadir `caruspex_cli` a la ruta de búsqueda. Actualiza una 0.8 instalada en el sitio e instala el entorno WebView2 solo si falta |
| Windows 10 y 11, 64 bits | `Caruspex-<version>-windows-x64.zip` | Portátil, descomprime y arranca. Necesita el entorno WebView2, que Windows 10 y 11 ya incluyen |
| macOS 13 y posterior, Apple Silicon | `Caruspex-<version>-macos.dmg` | Arrastra la aplicación a Aplicaciones |
| Linux, x86_64 | `Caruspex-<version>-linux-x86_64.AppImage` | `chmod +x` y arranca |
| Debian y Ubuntu, 64 bits | `Caruspex-<version>-linux-amd64.deb` | Instala con `sudo apt install ./Caruspex-<version>-linux-amd64.deb` |
| Solo la herramienta de línea de comandos | `caruspex_cli-<version>-windows-x64.zip`, `caruspex_cli-<version>-linux-x86_64.tar.gz`, `caruspex_cli-<version>-macos.tar.gz` | Para scripts y servidores, sin interfaz |

Los paquetes todavía no están firmados. Windows SmartScreen pregunta en el primer arranque ("Más información", "Ejecutar de todos modos"); en macOS
abre la aplicación la primera vez con clic derecho y "Abrir".

## Compilación

Requisitos: la cadena de herramientas estable de Rust (edición 2024, Rust 1.85 o posterior), Node 24 con npm y Python 3 para
los generadores y las revisiones de traducciones. En Linux, los paquetes del sistema que necesitan la interfaz y los enlaces con el adaptador
(nombres de Debian y Ubuntu):

```bash
sudo apt-get install libwebkit2gtk-4.1-dev librsvg2-dev libdbus-1-dev libudev-dev pkg-config
```

Windows necesita el entorno WebView2 (parte de Windows 10 y 11); macOS no necesita nada más.

Instala una vez las dependencias de la interfaz y luego compila la herramienta de línea de comandos y la aplicación:

```bash
npm --prefix app ci
```

```bash
cargo build --release -p caruspex-cli
```

```bash
npm --prefix app run tauri -- build
```

`npm --prefix app run tauri -- dev` arranca la aplicación con recarga en vivo mientras trabajas en ella. La
interfaz sola, sin la ventana de Tauri, funciona en un navegador normal con el coche de demostración integrado:

```bash
npm --prefix app run dev
```

Abre `http://localhost:5287/?demo` para conectar al coche de demostración de inmediato. `?lang=de` elige un idioma y
`?theme=light` el tema claro, por ejemplo `?demo&lang=de&theme=light`. Sin la carcasa de Tauri no hay
acceso al adaptador, así que la versión de navegador solo conoce el coche de demostración.

### Permisos del puerto serie en Linux

Para acceder al adaptador sin `sudo`:

```bash
sudo usermod -aG dialout $USER
```

Después cierra la sesión y vuelve a iniciarla.

## Resultados de la compilación

| Resultado | Descripción |
|---|---|
| `target/release/caruspex` / `caruspex.exe` | La aplicación con la interfaz Tauri (`tauri build` también escribe los instaladores y paquetes en `target/release/bundle/`) |
| `target/release/caruspex_cli` / `caruspex_cli.exe` | Herramienta de línea de comandos con scripts sobre el mismo núcleo, véase [Herramienta de línea de comandos](#herramienta-de-línea-de-comandos) |

### Paquetes y versiones

`bash tools/package.sh` crea en `dist/` los paquetes del sistema actual: el instalador y dos zips en
Windows (el instalador necesita [Inno Setup 6](https://jrsoftware.org/isinfo.php)), el dmg y el archivo de la línea de comandos
en macOS, el AppImage, el `.deb` y el archivo de la línea de comandos en Linux. Después `bash tools/smoke_test.sh` los instala o
abre como lo haría un usuario, arranca ambos programas y los vuelve a quitar. La CI hace ambas cosas en cada ejecución.

Una versión sale de una etiqueta. `bash tools/release.sh <version>` la prepara en su propia rama sin subir nada:
fija la versión en `Cargo.toml` y `app/package.json`, crea `changelog/<version>.md` (alemán e inglés),
ejecuta las comprobaciones, las pruebas, los paquetes y la prueba de humo, y actualiza las capturas de pantalla del README (`tools/screenshots.sh de` y
`tools/screenshots.sh en`). Cuando se fusiona la pull request, etiqueta `v<version>` y sube la etiqueta. La CI compila los
tres sistemas y publica la versión con los paquetes y la lista de cambios.

La CI (`.github/workflows/build.yml`) ejecuta primero el formato, la comprobación de las tablas generadas, la comprobación de traducciones, la
compilación de la interfaz y clippy en Linux, y después prueba, empaqueta y hace la prueba de humo en Windows, Linux y macOS.
Un segundo flujo de trabajo (`.github/workflows/data-check.yml`) busca una vez al mes nuevos códigos VIN, vehículos y archivos DBC
en el origen y abre una incidencia con las novedades.

## Pruebas

```bash
cargo test --workspace
```

Unas 370 pruebas se ejecutan en pocos segundos, todas sin coche. Cubren el analizador de respuestas ELM327, la decodificación
de códigos de avería, la conversión de PID, los perfiles de vehículo y el decodificador de VIN, los servicios OBD-II y UDS contra el coche de demostración,
sesiones completas de KWP1281 y KWP2000 contra unidades de control simuladas, ENET (HSFZ) y DoIP contra pasarelas falsas,
SLCAN y la ruta J2534 contra un controlador falso, grabaciones y reproducción, y el análisis (chequeo de salud, prueba de batería,
ordenador de a bordo, cronómetro de rendimiento). Las pruebas de la línea de comandos ejecutan el `caruspex_cli` compilado y comparan su salida con
los archivos de referencia de `crates/caruspex-cli/tests/golden/`, que contienen lo que imprimía la versión anterior en C++ con los mismos
comandos, de modo que cualquier diferencia con la herramienta antigua queda a la vista.

Las comprobaciones que ejecuta la CI además de las pruebas:

```bash
cargo fmt --all --check
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
python tools/i18n_rust.py --strict
```

```bash
npm --prefix app run build
```

Las dos primeras son el formato y los avisos del linter. `tools/i18n_rust.py --strict` falla cuando falta un texto del núcleo en
los archivos de traducción o falta una traducción en uno de los 18 idiomas; la compilación de la interfaz comprueba sus propios
textos del mismo modo cuando se define `I18N_STRICT=1`. Ejecuta `python tools/gen_dtc.py`, `gen_vehicles.py`, `gen_ecus.py` y
`gen_pids.py` para actualizar las tablas generadas; la CI falla si difieren de lo que está registrado.

## Pruébalo sin coche

La forma más rápida es el coche de demostración integrado: elige "Coche de demostración" en la página Conectar, o arranca con

```bash
target/release/caruspex --demo --language en
```

`caruspex --help` lista todas las opciones de la aplicación: `--tcp`, `--port`, `--bluetooth`, `--enet`, `--can`,
`--replay`, `--report`, `--language`, `--theme`, `--page`, `--shot` y `--size`.

El simulador de Python hace de motor del Audi A2 en la K-Line o, con `--can`, de un coche CAN con motor
y caja de cambios (cabeceras, ISO-TP, modos 06 y 09 por unidad de control):

```bash
python tools/mock_elm327.py --can
```

Después conecta la aplicación (en Windows el binario es `target/release/caruspex.exe`):

```bash
target/release/caruspex --tcp 127.0.0.1:35000 --language en
```

La herramienta de línea de comandos también funciona contra el simulador:

```bash
target/release/caruspex_cli read --tcp 127.0.0.1:35000
```

### Herramienta de línea de comandos

`caruspex_cli <command>` con `--demo`, `--tcp HOST[:PORT]`, `--port NAME [--baud N]`, `--enet HOST|auto`,
`--can SPEC` o `--replay FILE`:

| Comando | Qué hace |
|---|---|
| `read` | Testigo de avería, disponibilidad y códigos de avería, con el significado para la marca cuando el VIN la delata |
| `vin` | VIN, lo que dice (fabricante, país, año del modelo, dígito de control) y el coche correspondiente del catálogo |
| `freeze` | Todos los datos de congelación, uno por avería almacenada |
| `health` | Chequeo de salud: averías, disponibilidad, modo 06 y una serie en vivo (`--seconds N`, 15 por defecto) |
| `probe` | Prueba del adaptador: qué es realmente el ELM327, qué comandos conoce, tiempos de ida y vuelta |
| `uds` | UDS por CAN: identificación y memoria de averías de todas las unidades de control (`--ids A,B,...` para tus propias direcciones) |
| `custom` | Valores del fabricante y personalizados de un paquete integrado (`--pack NAME`) o de un CSV de Torque (`--csv FILE`), una vez |
| `live`, `info`, `clear`, `ecus`, `ports`, `kklcheck` | Valores en vivo, VIN y primeros datos de congelación, borrado, todo por unidad de control, puertos serie, autoprueba del cable KKL |
| `enet`, `interfaces` | Busca coches BMW y Mini y coches DoIP en la red; lista las interfaces CAN de este ordenador (controladores J2534, SocketCAN) |
| `version`, `help` | Versión del programa, la lista completa de comandos y opciones |

`--json` imprime `read`, `vin`, `health`, `probe`, `freeze`, `uds` y `custom` como JSON para scripts, `--record FILE`
guarda toda la sesión con el adaptador para `--replay`, `--fast-timing` pone el adaptador en un
modo de tiempos adaptativos agresivos, `-v` registra cada comando del adaptador (en stderr junto con `--json`). `caruspex_cli help` lista todo.

```bash
target/release/caruspex_cli health --demo --json > health.json
```

Las capturas de pantalla de esta página proceden del coche de demostración, tomadas por la propia aplicación:
`--demo --page faults --shot shot.png --size 1440x920` abre una página, guarda una imagen de la ventana tras unos
segundos y sale (`tools/screenshots.sh` toma el conjunto completo).

## Estructura del proyecto

```
crates/caruspex-i18n/       Idiomas: alemán e inglés en el código, los otros 16 desde translations/
crates/caruspex-vehicles/   Decodificador de VIN y catálogo de vehículos (marcas, modelos, generaciones, conectores, buses)
crates/caruspex-data/       Códigos de avería, PID, direcciones de unidades de control, asignación de código a componente
crates/caruspex-link/       Vías hacia el adaptador (serie, TCP, Bluetooth LE), controlador ELM327, comprobación del adaptador
crates/caruspex-protocols/  OBD-II, UDS, KWP2000 y KWP1281 (K-Line, byte a byte)
crates/caruspex-net/        BMW ENET (HSFZ), DoIP, SLCAN, SocketCAN, J2534 PassThru
crates/caruspex-record/     Grabaciones y reproducción, valores personalizados (CSV de Torque, paquetes de PID), escritor de JSON
crates/caruspex-analysis/   Chequeo de salud, guía del ciclo de conducción, ordenador de a bordo, cronómetro de rendimiento, prueba de batería
crates/caruspex-demo/       El coche de demostración integrado: un ELM327 con unidades de control simuladas
crates/caruspex-cli/        La herramienta de línea de comandos, con sus pruebas de archivos de referencia
app/src/                    La interfaz: React 19 y TypeScript (páginas, estructura, estado, informe, i18n)
app/src-tauri/              La aplicación Tauri: ventana, comandos, comprobación de actualizaciones, opciones de línea de comandos
translations/               Archivos de traducción compartidos para núcleo e interfaz, 18 idiomas
data/                       El catálogo de vehículos como CSV mantenido a mano, fuentes de unidades de control y PID
tools/                      Generadores, revisiones de traducciones, empaquetado, scripts de versión y capturas, simulador ELM327
docs/                       Capturas de pantalla del README
packaging/                  Script del instalador de Windows, archivo desktop de Linux, Info.plist de macOS
changelog/                  Notas de versión por versión, alemán e inglés
```

La base de datos de códigos de avería se genera: `tools/gen_dtc.py` expande las familias regulares de códigos EOBD y las
listas seleccionadas de `tools/dtc/` en `crates/caruspex-data/src/generated/*.rs`. Vuelve a ejecutarlo tras cambiar los datos:

```bash
python tools/gen_dtc.py
```

El catálogo de vehículos funciona igual: edita los archivos CSV de `data/vehicles/` y después ejecuta

```bash
python tools/gen_vehicles.py
```

Las direcciones de unidades de control y los paquetes de PID tienen `tools/gen_ecus.py` y `tools/gen_pids.py`. Las traducciones se describen
en [translations/TRANSLATING.md](translations/TRANSLATING.md); tras añadir un texto en el código,
`python tools/i18n_rust.py` lista lo que aún falta por traducir.

## Una nota sobre el borrado de códigos

El modo 04 de OBD borra la memoria de averías, los datos de congelación y los monitores de disponibilidad. Justo después el
coche no pasará una prueba de emisiones hasta que haya completado varios ciclos de conducción completos, y un código cuya causa
siga presente volverá tras un trayecto corto. Borrar no es reparar.

## Aviso legal

Caruspex es un proyecto de afición y se ofrece sin garantía. Úsalo bajo tu propia responsabilidad, solo en vehículos
en los que tengas permiso para trabajar y nunca mientras conduces. Todos los nombres de productos y marcas son marcas comerciales de sus
respectivos propietarios; Caruspex no está afiliado a ningún fabricante de vehículos ni cuenta con su respaldo.

## Licencia

MIT, véase [LICENSE](LICENSE). Las fuentes incluidas tienen licencia SIL Open Font License 1.1:
JetBrains Mono ([OFL.txt](resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](resources/fonts/OFL-Inter.txt)) y
Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](resources/fonts/OFL-PlusJakartaSans.txt)).
