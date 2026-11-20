# Sicherheit und Updates

Diese Seite ist die Sicherheitsinformation zu Caruspex: wofür das Programm gedacht ist, welche Risiken man kennen
sollte, wie Updates funktionieren und wie man es wieder entfernt. Sie folgt den Informationen für Nutzer, die der
EU Cyber Resilience Act verlangt (Anhang II), soweit sie für ein freies, nicht kommerzielles Programm gelten.

## Das Programm

- **Name**: Caruspex, die App `caruspex` und das Kommandozeilenwerkzeug `caruspex_cli`, für Windows, Linux und
  macOS. Die Version steht im Namen jedes Downloads und in der Änderungsliste jedes Release.
- **Herausgegeben von**: dem Caruspex-Projekt auf [GitHub](https://github.com/SenseiIssei/Caruspex), als freie
  Software unter der MIT-Lizenz. Fragen und Fehler gehören in die [Issues](https://github.com/SenseiIssei/Caruspex/issues).
- **Zweck**: Diagnosedaten von Autos, Motorrädern und Lkw über einen eigenen Adapter am eigenen Rechner lesen und
  erklären. Es gibt kein Konto, keine Cloud und kein Tracking ([Datenschutz](privacy.md)).
- **Sicherheitseigenschaften**: Lesen ändert nie etwas am Auto, und alles, was das Auto etwas tun lässt, fragt
  vorher. Updates werden nur installiert, wenn ihre Signatur zum eingebauten Schlüssel passt. Caruspex öffnet keinen
  Netzwerkport und führt keinen Code aus Dateien oder aus dem Internet aus.

## Eine Sicherheitslücke melden {#report}

Sicherheitsprobleme bitte privat über das
[Formular für eine neue Sicherheitsmeldung](https://github.com/SenseiIssei/Caruspex/security/advisories/new) melden,
nicht in einem öffentlichen Issue. Die
[Sicherheitsrichtlinie](https://github.com/SenseiIssei/Caruspex/blob/main/SECURITY.md) beschreibt, wie es weitergeht.
Behobene Lücken werden als [Sicherheitshinweise](https://github.com/SenseiIssei/Caruspex/security/advisories)
veröffentlicht.

## Risiken, die man kennen sollte {#risks}

- Caruspex nur von der [Release-Seite](https://github.com/SenseiIssei/Caruspex/releases) laden. Kopien von woanders
  können verändert sein.
- Aufnahmen, PID-Pakete und 3D-Modelle nur von Leuten öffnen, denen man vertraut. Sie können keinen Code ausführen,
  aber eine kaputte Datei kann Caruspex verlangsamen oder schließen.
- Fehlercodes löschen, Stellglieder testen und Grundeinstellungen nur bei stehendem Auto an einem sicheren Ort.
- Viele Bluetooth- und WLAN-Adapter haben keine PIN oder ein offenes Netz: Jeder in der Nähe kann sich verbinden.
  Den Adapter nach der Arbeit abziehen.
- ENET-Kabel, DoIP und WLAN-Adapter verschlüsseln nichts: nur direkt oder im eigenen Netz benutzen.
- J2534-Treiber sind Programme ihrer Hersteller und laufen mit den eigenen Rechten. Nur vom Hersteller installieren.

## Updates und Unterstützung {#support}

- Beim Start fragt Caruspex bei GitHub nach einer neueren Version und zeigt sie an. Installieren ist ein Klick, und
  die App installiert nur ein Paket mit passender Signatur. AppImage und macOS-App aktualisieren sich selbst; das
  .deb-Paket und die Android-App bekommen die Download-Seite.
- Die Suche und die automatische Installation sind Schalter in den Einstellungen unter **Fenster und Sprache**.
  Ohne die Suche ab und zu auf der Release-Seite nachsehen.
- Sicherheitskorrekturen gibt es nur im neuesten Release. Ältere Versionen bekommen keine eigenen Korrekturen,
  also Caruspex aktuell halten.
- Caruspex wird in der Freizeit gepflegt und hat kein festes Enddatum. Endet die Pflege, sagen das Repository und
  ein letztes Release es vorher an.

## Einen Download prüfen {#verify}

Jedes Release hat eine Datei `SHA256SUMS` mit der Prüfsumme jeder Datei und GitHubs signierten Nachweis, wie die
Dateien gebaut wurden. Mit den Dateien in einem Ordner:

```
sha256sum --ignore-missing -c SHA256SUMS
gh attestation verify Caruspex-<version>-windows-x64-setup.exe --repo SenseiIssei/Caruspex
```

Ohne diese Werkzeuge zeigt `Get-FileHash <datei>` in der PowerShell (Windows) oder `shasum -a 256 <datei>` (macOS)
die Prüfsumme zum Vergleich mit der Zeile in `SHA256SUMS`.

## Caruspex und die eigenen Daten entfernen {#remove}

1. Das Programm deinstallieren: unter Windows in den Einstellungen unter Apps; unter macOS Caruspex aus den
   Programmen ziehen; unter Linux das AppImage löschen oder `sudo apt remove caruspex`.
2. Die Daten löschen: Der Knopf **Ablageort öffnen** in den Einstellungen zeigt die Ordner mit den Einstellungen,
   der Garage und den Absturzberichten. Diese löschen. Selbst gespeicherte Aufnahmen und Berichte liegen dort, wo
   man sie gespeichert hat.

## Für Entwickler und Firmen {#integrators}

- Jedes Release hat je Plattform eine Software-Stückliste, `Caruspex-<version>-sbom-<platform>.cdx.json`
  (CycloneDX), mit jeder Bibliothek und ihrer Version.
- Risikobewertung, Anforderungsliste und Anwendungsbereich des Cyber Resilience Act für Caruspex stehen in
  [docs/cra](https://github.com/SenseiIssei/Caruspex/tree/main/docs/cra).
- Caruspex wird nicht als Produkt im Sinne des Cyber Resilience Act in Verkehr gebracht, deshalb gibt es keine
  EU-Konformitätserklärung und keine CE-Kennzeichnung.
