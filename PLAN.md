# Caruspex: der große Ausbau

Begonnen am 09.11.2026 auf dem Zweig `grosser-ausbau`, Version 0.3.0.

Ziel: Man steckt einen Adapter ein, drückt auf einen Knopf, und Caruspex weiß, welches Auto da hängt, was es kann und was ihm fehlt. Für jedes Fahrzeug, vom Golf 2 mit KKL-Stecker bis zum E-Auto von heute. Die Oberfläche soll sich dabei anfühlen wie Odysync: ruhig, dunkel, mit Bewegung an den richtigen Stellen, und so erklärt, dass auch jemand ohne Werkstatterfahrung versteht, was los ist.

Ehrlich bleibt es trotzdem. Was ein Auto oder ein Adapter technisch nicht hergibt, sagt Caruspex klar, statt es vorzutäuschen.

## Phase 0: Grundlage

- [x] Arbeitszweig, Build und Tests laufen
- [x] Echte Umlaute in allen Texten statt ae/oe/ue, die englische Übersetzung wandert mit
- [x] Neue Dateien sauber in CMake

## Phase 1: Jedes Fahrzeug erkennen

- [x] VIN-Decoder: 365 Herstellerkennungen, 79 Länderbereiche, Modelljahr mit Zweideutigkeit, Prüfziffer, Modellcodes für VAG, Mercedes, Porsche, Tesla
- [x] Fahrzeugkatalog: 94 Marken, 2167 Generationen ab 1980 als CSV mit Generator
- [x] Je Generation Stecker, Lage der Buchse, Protokoll und was geht (OBD-II, teilweise, KKL, nur Hersteller, nichts)
- [x] Automatisch: nach dem Verbinden VIN lesen, Auto benennen, passendes Bild, Marke für Herstellercodes
- [x] Handauswahl durchsuchbar auf der Verbindungsseite, Fahrzeugseite mit Katalog und VIN-Prüfer

## Phase 2: Viel mehr OBD-II

- [x] Alle Modus-01-Messwerte bis PID C4: 145 PIDs, 317 Einzelwerte, nach Bereichen
- [x] Fehlercodes: 1381 Normcodes und 298 Herstellercodes, je Code Dringlichkeit, Weiterfahren, Ursachen, Selbsthilfe, Kosten
- [x] Alte Fehler im Fehlercode-Generator behoben (zählte hexadezimal statt dezimal)
- [x] Mehrere Steuergeräte über CAN, Absender je Fehler, Modus 06 und 09 je Steuergerät
- [x] Gesundheitscheck mit Punktzahl und Erklärung
- [x] AU-Bereitschaft mit Fahrzyklus-Hilfe
- [x] Batterie- und Ladeprüfung
- [x] Bordcomputer mit Verbrauch, Kosten, CO2 und Fahrstil
- [x] Leistungsmessung
- [x] Zündaussetzer je Zylinder, In-Use-Zähler
- [x] Garage mit Verlauf je VIN, Prüfbericht als PDF
- [x] Adaptertest: echter ELM327 oder Nachbau, STN-Erkennung, Antwortzeit
- [x] Eingebautes Demo-Auto über CAN mit Motor und Getriebe

## Phase 3: Neue Oberfläche

- [x] Farben, Schriften (Inter, Plus Jakarta Sans, JetBrains Mono), Abstände an einer Stelle
- [x] Eigene Linien-Symbole als SVG
- [x] Seitenleiste in Gruppen mit weicher Markierung, einklappbar
- [x] Verbindungskapsel in der Titelleiste, Protokoll als ausklappbare Leiste
- [x] Überblendung zwischen den Seiten
- [x] Startseite mit Fahrzeugbild, Gesundheitsring, Befunden und Kacheln
- [x] Fahrzeugbild neu: acht Karosserieformen aus echten Maßen, Lack, Felgen, Licht, Durchsicht auf die Bauteile
- [x] Rundanzeigen mit Verlauf und Skala
- [x] Verbinden Schritt für Schritt mit automatischer Anschlusssuche
- [x] Kurze Hinweise unten rechts, Ladeanimationen

## Phase 4: Abschluss

- [x] Tests: 148 Testfälle
- [x] Englische Übersetzung der neuen Texte
- [x] README deutsch und englisch
- [x] Neue Bildschirmfotos (tools/screenshots.sh)
- [ ] CI auf Linux und macOS: bisher nur mit MSVC gebaut, GCC und Clang prüft erst der nächste CI-Lauf

## Noch offen, auf einem echten Auto zu prüfen

- Bitbelegung einiger seltener PIDs (Gang in A4, Bereiche von 6B und 77, 8B, 9A, 9B)
- Versionsgrenzen der ELM327-Befehle im Adaptertest
- Lage der OBD-Buchse ist nur bei einigen Modellen hinterlegt, sonst die übliche Stelle

## Was bewusst nicht kommt

- Schreiben in Anpassungskanäle oder Codierung: ein falscher Wert legt das Auto lahm, das bleibt nur lesend
- Herstellerdiagnose über CAN (UDS, TP 2.0) für alle Marken: braucht je Hersteller eigene Datensätze, die frei nicht zu haben sind. Für VAG kann das später ein eigener Schritt werden
- Bluetooth Low Energy: Qt Bluetooth fehlt in der Qt-Installation. Klassisches Bluetooth läuft weiter über den COM-Anschluss, WLAN-Adapter über TCP
