# Installieren

Jede Version liegt auf der [Release-Seite auf GitHub](https://github.com/SenseiIssei/Caruspex/releases/latest). Die Pakete sind noch nicht signiert, deshalb fragen Windows und macOS vor dem ersten Start einmal nach.

## Windows 10 und 11

| Datei | Was es ist |
|---|---|
| `Caruspex-<version>-windows-x64-setup.exe` | Das Installationsprogramm. Es braucht keine Administratorrechte, kann das Kommandozeilenwerkzeug `caruspex_cli` in den Suchpfad legen und richtet die WebView2-Laufzeit nur ein, wenn sie fehlt. |
| `Caruspex-<version>-windows-x64.zip` | Zum Mitnehmen: entpacken und `caruspex.exe` starten. |

Beim ersten Start meldet Windows SmartScreen vielleicht, dass die App unbekannt ist: auf "Weitere Informationen" und dann "Trotzdem ausführen" klicken.

USB-Adapter brauchen einen Treiber für ihren USB-Chip. Windows 10 und 11 finden ihn meist selbst; wenn nicht, siehe [ELM327 über USB](elm327-usb.md).

## macOS 13 und neuer (Apple Silicon)

`Caruspex-<version>-macos.dmg` herunterladen, öffnen und Caruspex in den Ordner Programme ziehen. Weil die App noch nicht notarisiert ist, beim ersten Mal mit Rechtsklick und "Öffnen" starten.

## Linux (x86_64)

| Datei | Was es ist |
|---|---|
| `Caruspex-<version>-linux-x86_64.AppImage` | Läuft auf fast jeder Distribution: mit `chmod +x` ausführbar machen und starten. Es aktualisiert sich auch selbst. |
| `Caruspex-<version>-linux-amd64.deb` | Für Debian und Ubuntu: `sudo apt install ./Caruspex-<version>-linux-amd64.deb` |

Damit serielle Adapter ohne `sudo` gehen, sich einmal zur Gruppe `dialout` hinzufügen, dann ab- und wieder anmelden:

```
sudo usermod -aG dialout $USER
```

## Nur das Kommandozeilenwerkzeug

Für Skripte und Server gibt es `caruspex_cli` ohne Oberfläche, für Windows, Linux und macOS. `caruspex_cli help` listet die Befehle.

`caruspex_cli report` schreibt den Prüfbericht als PDF-Datei, ohne die App und über jede Verbindung, die die anderen Befehle kennen (`--demo`, `--port`, `--tcp`, `--enet`, `--can`, `--replay`):

```
caruspex_cli report --demo report.pdf
caruspex_cli report --port COM5 --lang de report.pdf
```

Den Bericht gibt es auf Deutsch, Englisch, Spanisch, Französisch, Portugiesisch, Italienisch, Niederländisch und Indonesisch (`--lang`). In den anderen Sprachen kommt er auf Englisch, weil den Schriften, die jeder PDF-Betrachter eingebaut hat, deren Buchstaben fehlen.

## Updates

Caruspex fragt bei jeder Installation einmal, ob es nach Updates suchen soll. Erst nach einem Ja fragt es bei GitHub nach, ob es eine neuere Version gibt, und sonst nichts. Eine neue Version wird nur installiert, wenn ihre Signatur zum eingebauten Schlüssel passt. Die Suche und die automatische Installation lassen sich jederzeit in den Einstellungen ändern.

## Paketverwaltungen

Pakete für winget, Homebrew, Flathub und Snap werden vorbereitet. Bis sie veröffentlicht sind, installiert man Caruspex mit den Dateien oben.
