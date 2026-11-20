# Instalar

Todas las versiones están en la [página de versiones de GitHub](https://github.com/SenseiIssei/Caruspex/releases/latest). Los paquetes aún no están firmados, así que Windows y macOS preguntan una vez antes del primer arranque.

## Windows 10 y 11

| Archivo | Qué es |
|---|---|
| `Caruspex-<version>-windows-x64-setup.exe` | El instalador. No necesita permisos de administrador, puede añadir la herramienta de línea de comandos `caruspex_cli` a la ruta de búsqueda e instala el runtime de WebView2 solo si falta. |
| `Caruspex-<version>-windows-x64.zip` | Portable: descomprímelo y arranca `caruspex.exe`. |

En el primer arranque, Windows SmartScreen puede decir que la aplicación es desconocida: pulsa "Más información" y luego "Ejecutar de todas formas".

Los adaptadores USB necesitan un controlador para su chip USB. Windows 10 y 11 suelen encontrarlo solos; si no, consulta [ELM327 por USB](elm327-usb.md).

## macOS 13 y posteriores (Apple Silicon)

Descarga `Caruspex-<version>-macos.dmg`, ábrelo y arrastra Caruspex a Aplicaciones. Como la aplicación aún no está notarizada, ábrela la primera vez con clic derecho y "Abrir".

## Linux (x86_64)

| Archivo | Qué es |
|---|---|
| `Caruspex-<version>-linux-x86_64.AppImage` | Funciona en casi todas las distribuciones: hazlo ejecutable con `chmod +x` y arráncalo. También se actualiza solo. |
| `Caruspex-<version>-linux-amd64.deb` | Para Debian y Ubuntu: `sudo apt install ./Caruspex-<version>-linux-amd64.deb` |

Para usar adaptadores serie sin `sudo`, añádete una vez al grupo `dialout` y luego cierra sesión y vuelve a entrar:

```
sudo usermod -aG dialout $USER
```

## Solo la herramienta de línea de comandos

Para scripts y servidores existe `caruspex_cli` sin interfaz gráfica, para Windows, Linux y macOS. `caruspex_cli help` muestra los comandos.

`caruspex_cli report` escribe el informe de diagnóstico como archivo PDF, sin la aplicación y con cualquier conexión que conocen los demás comandos (`--demo`, `--port`, `--tcp`, `--enet`, `--can`, `--replay`):

```
caruspex_cli report --demo report.pdf
caruspex_cli report --port COM5 --lang de report.pdf
```

El informe puede salir en alemán, inglés, español, francés, portugués, italiano, neerlandés e indonesio (`--lang`). En los demás idiomas sale en inglés, porque a las fuentes que trae todo visor de PDF les faltan sus letras.

## Actualizaciones

En cada instalación Caruspex pregunta una vez si debe buscar actualizaciones. Solo después de un sí pregunta a GitHub si hay una versión más nueva, y nada más. Una versión nueva solo se instala si su firma coincide con la clave integrada en la aplicación. La comprobación y la instalación automática se pueden cambiar en los ajustes en cualquier momento.

## Gestores de paquetes

Se están preparando paquetes para winget, Homebrew, Flathub y Snap. Hasta que se publiquen, los archivos de arriba son la forma de instalar Caruspex.
