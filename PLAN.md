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
- [x] CI auf Windows, Linux (GCC) und macOS (Clang) grün

## Noch offen, auf einem echten Auto zu prüfen

- Bitbelegung einiger seltener PIDs (Gang in A4, Bereiche von 6B und 77, 8B, 9A, 9B)
- Versionsgrenzen der ELM327-Befehle im Adaptertest
- Lage der OBD-Buchse ist nur bei einigen Modellen hinterlegt, sonst die übliche Stelle

# Nächster Ausbau

Stand 09.11.2026, nach 0.3.0. Größen: klein heißt ein paar Stunden, mittel ein bis drei Tage, groß eine Woche oder mehr. Empfohlene Reihenfolge: 5, 6, 9, 8, 7, 10. Phase 11 läuft nebenher, sobald ein Auto in der Nähe ist.

## Phase 5: Schneller und runder (kleine Schritte, sofort spürbar)

- [x] Mehrere PIDs pro Anfrage auf CAN (bis zu sechs), damit die Live-Werte drei- bis fünfmal so oft kommen. Das Demo-Auto kann es schon. Mittel
- [x] Diagramm: jede Kurve auf ihren eigenen Messbereich, der echte Wert im Tooltip. Heute drückt die Drehzahl alles andere flach. Klein
- [x] Freeze Frame je Fehler (Frame-Nummer aus Modus 02), nicht nur der erste. Klein
- [x] Rundanzeigen auf der Startseite selbst wählen. Klein
- [x] Hellmodus wie in Odysync, umschaltbar oder nach System. Mittel
- [x] Kommandozeile: `health`, `vin`, `probe`, `freeze`, `--demo` und `--json` für Skripte. Mittel
- [ ] Bericht als PDF auch aus der Kommandozeile. Die kommt heute ohne Qt aus, für PDF bräuchte sie QtGui oder einen eigenen kleinen PDF-Schreiber. Klein

## Phase 6: Alle Steuergeräte über CAN mit dem ELM327 (größter Mehrwert)

Heute sieht Caruspex bei CAN-Autos nur Motor und Getriebe über OBD-II. Fast alle Autos ab etwa 2008 sprechen aber UDS (ISO 14229), und das geht lesend auch über einen ELM327.

- [x] UDS-Grundlage: Steuergerät per Kopfzeile ansprechen (ATSH, ATCRA), Sitzung, Antwort "busy" (7F 78) abwarten, ISO-TP in beide Richtungen. Mittel
- [x] Fehler je Steuergerät lesen (19 02), samt Fehlerart nach SAE J2012 (das dritte Byte: Kurzschluss, Unterbrechung, unplausibel ...). Mittel
- [x] Kennung je Steuergerät: Teilenummer, Hardware, Software, VIN (22 F187, F189, F191, F190). Klein
- [x] Alle Steuergeräte durchsuchen jetzt auch für CAN-Autos, mit demselben Prüfbericht wie über KKL. Mittel
- [x] Adresslisten je Marke als CSV wie der Katalog (VAG, BMW, Mercedes, Ford, GM, Toyota, Hyundai/Kia, Stellantis), dazu ein ehrlicher Suchlauf über den üblichen Bereich, wenn die Marke unbekannt ist. Mittel
- [x] Löschen je Steuergerät (14 FF FF FF) mit derselben Warnung wie heute. Klein
- [~] Airbag- und ABS-Codes (B und C) mit Klartext: die Normcodes und die Fehlerart sind da, Herstellerbedeutungen fehlen noch (nur mit Quelle). Mittel
- [x] Demo-Auto bekommt ABS, Airbag und Klima, Tests mit einem nachgebauten UDS-Steuergerät. Mittel
- Ehrlich bleibt: VAG mit TP 2.0 (etwa 2004 bis 2012) kann ein ELM327 kaum sauber. Dort bleibt es bei OBD-II und, wo vorhanden, KKL
- Stand: Adresslisten für VAG, Ford, Mazda, GM, Toyota, Nissan, Hyundai/Kia, Renault und PSA (77 Steuergeräte). BMW (Adressierung über 6F1) und Mercedes (eigene und 29-Bit-Schemata) fehlen, dort hilft der allgemeine Suchlauf nur teilweise. Ford-Karosseriegeräte auf MS-CAN erreicht ein normaler ELM327 nicht. Alle Adressen sind noch nicht am echten Auto geprüft

## Phase 7: Herstellerwerte (Modus 22) und eigene PIDs

- [ ] Eigene Messwerte importieren im verbreiteten Torque-CSV-Format, damit jeder seinem Auto Werte hinzufügen kann. Mittel
- [ ] Mitgelieferte Pakete nur mit belegter Quelle, zuerst E-Autos: Akkugesundheit (SOH), Zellspannungen und Temperaturen bei Hyundai/Kia, Renault Zoe, Nissan Leaf. Mittel je Paket
- [ ] Danach VAG-Diesel: Rußbeladung und Regeneration des Partikelfilters, Ölstand. Mittel
- [ ] Seite für E-Autos und Hybride: Akkugesundheit, Zellspreizung, Laden. Mittel

## Phase 8: Adapter und Anschluss

- [ ] Bluetooth Low Energy (vLinker, OBDLink CX, Veepeak BLE) über Qt Bluetooth. Braucht das Modul qtconnectivity, auch in der CI. Groß
- [ ] STN-Adapter wie OBDLink schneller fahren (größere Puffer, STPX), adaptive Taktung (ATAT2) für alle. Klein
- [ ] Sitzung aufzeichnen und wieder abspielen, als Transport wie das Demo-Auto. Hilft bei Fehlersuche und wenn jemand eine Aufnahme schickt. Mittel

## Phase 9: Die Garage wird zur Fahrzeugakte

- [x] Fahrten aus dem Bordcomputer speichern, Verbrauch über Wochen und Monate, Tankbuch. Mittel
- [x] Wartung: Ölwechsel, HU und AU, Zahnriemen nach Kilometer oder Datum, Erinnerung beim Verbinden. Mittel
- [x] Kilometerstand je Prüfung vergleichen und einen Rücksprung melden, als Hinweis beim Gebrauchtwagenkauf. Klein
- [x] Export und Import der Garage. Klein
- [ ] Bericht über alle Prüfungen eines Autos (Verlauf als PDF). Klein

## Phase 10: Verteilen

- [ ] Installer aus der CI: Windows (Inno Setup oder WiX), macOS dmg, Linux AppImage. Mittel
- [ ] Releases per Tag mit Änderungsliste auf Deutsch und Englisch. Getaggt und gepusht wird erst nach Jakobs Ja. Klein
- [ ] Windows-Programm signieren. Mittel, hängt an einem Zertifikat
- [ ] Mehr Sprachen. Der Kern kennt heute nur Deutsch und Englisch (Text{de, en}), das bräuchte einen Umbau auf Schlüssel und Kataloge. Groß

## Phase 11: Am echten Auto prüfen

- [ ] Testrunde mit dem A2 (K-Line und KKL), einem CAN-VAG, einem Japaner und einem E-Auto
- [ ] Die offenen Punkte von oben: seltene PIDs, Versionsgrenzen im Adaptertest
- [ ] Herstellercodes für BMW, Opel, Renault, PSA, Fiat und Volvo ergänzen, nur mit Quelle
- [ ] Genaue Lage der OBD-Buchse für die häufigsten 200 Modelle

## Was bewusst nicht kommt

- Schreiben in Anpassungskanäle oder Codierung: ein falscher Wert legt das Auto lahm, das bleibt nur lesend. Löschen von Fehlern ist die einzige schreibende Aktion
- Wegfahrsperre, Schlüssel anlernen, Kilometerstand verändern: rechtlich heikel und nicht der Zweck dieses Werkzeugs
- VAG TP 2.0 über ELM327 (siehe Phase 6), dafür bleibt KKL
