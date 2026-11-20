# Seguridad y actualizaciones

Esta página reúne la información de seguridad de Caruspex: para qué sirve el programa, qué riesgos conviene
conocer, cómo funcionan las actualizaciones y cómo desinstalarlo. Sigue la información para usuarios que pide el
Reglamento europeo de ciberresiliencia (Cyber Resilience Act, anexo II), en lo que se aplica a un programa libre y
no comercial.

## El programa

- **Nombre**: Caruspex, la aplicación `caruspex` y la herramienta de línea de comandos `caruspex_cli`, para Windows,
  Linux y macOS. La versión aparece en el nombre de cada descarga y en la lista de cambios de cada versión.
- **Publicado por**: el proyecto Caruspex en [GitHub](https://github.com/SenseiIssei/Caruspex), como software libre
  con licencia MIT. Las preguntas y los errores van a los [issues](https://github.com/SenseiIssei/Caruspex/issues).
- **Finalidad**: leer y explicar los datos de diagnóstico de coches, motos y camiones con su propio adaptador, en su
  propio ordenador. Sin cuenta, sin nube y sin rastreo ([Privacidad](privacy.md)).
- **Propiedades de seguridad**: leer nunca cambia nada en el coche, y todo lo que hace actuar al coche pregunta
  antes. Una actualización solo se instala si su firma coincide con la clave integrada en la aplicación. Caruspex
  no abre ningún puerto de red ni ejecuta código de archivos o de internet.

## Informar de una vulnerabilidad {#report}

Informe de los problemas de seguridad en privado con el
[formulario de nuevo aviso de seguridad](https://github.com/SenseiIssei/Caruspex/security/advisories/new), no en un
issue público. La [política de seguridad](https://github.com/SenseiIssei/Caruspex/blob/main/SECURITY.md) explica
qué pasa después. Las vulnerabilidades corregidas se publican como
[avisos de seguridad](https://github.com/SenseiIssei/Caruspex/security/advisories).

## Riesgos que conviene conocer {#risks}

- Descargue Caruspex solo desde la [página de versiones](https://github.com/SenseiIssei/Caruspex/releases). Una copia
  de otro sitio puede estar modificada.
- Abra grabaciones, paquetes de PID y modelos 3D solo si vienen de personas de confianza. No pueden ejecutar código,
  pero un archivo dañado puede ralentizar o cerrar Caruspex.
- Borre códigos de avería, inicie pruebas de actuadores y ajustes básicos solo con el coche parado y en un lugar seguro.
- Muchos adaptadores Bluetooth y Wi-Fi no tienen PIN o tienen una red abierta: cualquiera cerca puede conectarse.
  Desenchufe el adaptador al terminar.
- Los cables ENET, DoIP y los adaptadores Wi-Fi no cifran nada: úselos directamente o solo en su propia red.
- Los controladores J2534 son programas de sus fabricantes y se ejecutan con sus permisos. Instálelos solo desde el
  fabricante.

## Actualizaciones y soporte {#support}

- En cada instalación, también tras actualizar desde una versión anterior, Caruspex pregunta una vez "¿Buscar actualizaciones?". Solo después de un sí pregunta a GitHub si hay una versión más nueva, al arrancar y cada seis horas mientras está en marcha. Una versión nueva solo enciende un botón en la barra de título y nunca interrumpe. Instalarla es un clic, y la aplicación solo instala un paquete cuya firma coincide. La AppImage y la aplicación de macOS se actualizan solas; el paquete .deb y la aplicación de Android reciben la página de descarga.
- La búsqueda y la instalación automática son interruptores de los ajustes, en **Ventana e idioma**, y se pueden cambiar en cualquier momento. Sin un sí no se envía ninguna petición a GitHub; entonces mire de vez en cuando la página de versiones.
- Las correcciones de seguridad solo van en la versión más reciente. Las versiones antiguas no reciben correcciones
  aparte, así que mantenga Caruspex al día.
- Caruspex lo mantiene una sola persona en su tiempo libre, en la medida en que el tiempo lo permite y sin ninguna promesa: ni periodo de soporte ni plazos de respuesta. Si el mantenimiento termina, el repositorio se archivará y lo indicará.

## Comprobar una descarga {#verify}

Cada versión trae un archivo `SHA256SUMS` con la suma de comprobación de cada archivo y el registro firmado de GitHub
de cómo se construyeron. Con los archivos en una misma carpeta:

```
sha256sum --ignore-missing -c SHA256SUMS
gh attestation verify Caruspex-<version>-windows-x64-setup.exe --repo SenseiIssei/Caruspex
```

Sin estas herramientas, `Get-FileHash <archivo>` en PowerShell (Windows) o `shasum -a 256 <archivo>` (macOS) muestra
la suma para compararla con la línea de `SHA256SUMS`.

## Quitar Caruspex y sus datos {#remove}

1. Desinstale el programa: en Windows en Configuración, Aplicaciones; en macOS arrastre Caruspex fuera de
   Aplicaciones; en Linux borre la AppImage o use `sudo apt remove caruspex`.
2. Borre sus datos: el botón **Abrir la ubicación de almacenamiento** de los ajustes muestra las carpetas con los
   ajustes, el garaje y los informes de fallos. Bórrelas. Las grabaciones e informes que guardó usted están donde
   los guardó.

## Para desarrolladores y empresas {#integrators}

- Cada versión trae, por plataforma, una lista de materiales de software `Caruspex-<version>-sbom-<platform>.cdx.json`
  (CycloneDX) con cada biblioteca y su versión.
- La evaluación de riesgos, la lista de requisitos y el ámbito del Cyber Resilience Act para Caruspex están en
  [docs/cra](https://github.com/SenseiIssei/Caruspex/tree/main/docs/cra).
- Caruspex no se introduce en el mercado como producto en el sentido del Cyber Resilience Act, por eso no hay
  declaración UE de conformidad ni marcado CE.
