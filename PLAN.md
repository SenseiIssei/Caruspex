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
- Stand: Adresslisten für VAG, BMW, Ford, Mazda, GM, Toyota, Nissan, Hyundai/Kia, Renault und PSA (105 Steuergeräte). BMW ab F-Baureihe mit erweiterter Adressierung über 6F1 im Rohmodus (ISO-TP in Caruspex), Suchlauf über alle 128 Adressen, BMW-Fehlercodes sechsstellig; am echten BMW noch ungeprüft. Mercedes (eigene und 29-Bit-Schemata) fehlt weiter: Stand 10.11. gibt es keine belastbare öffentliche Adresstabelle, nur widersprüchliche Forenhinweise (Mercedes nutzt nicht 7DF bis 7E7). Kommt erst mit einem Mitschnitt von einem echten Auto, bis dahin hilft der allgemeine Suchlauf nur teilweise. Ford-Karosseriegeräte auf MS-CAN erreicht ein normaler ELM327 nicht. Alle Adressen sind noch nicht am echten Auto geprüft

## Phase 7: Herstellerwerte (Modus 22) und eigene PIDs

- [x] Eigene Messwerte importieren im verbreiteten Torque-CSV-Format, damit jeder seinem Auto Werte hinzufügen kann. Mittel
- [x] Mitgelieferte Pakete nur mit belegter Quelle, zuerst E-Autos: Akkugesundheit (SOH), Zellspannungen und Temperaturen bei Hyundai/Kia, Renault Zoe, Nissan Leaf. Mittel je Paket
- [ ] Danach VAG-Diesel: Rußbeladung und Regeneration des Partikelfilters, Ölstand. Offen, weil sich keine verlässliche Quelle fand und die Kennungen je Motor verschieden sind. Mittel
- [x] Seite für E-Autos und Hybride: Akkugesundheit, Zellspreizung, Laden. Mittel
- Stand: Pakete für Hyundai Kona und Kia e-Niro (33 Werte), Hyundai Ioniq 28 kWh (29), Renault Zoe Phase 1 (24) und Nissan Leaf ZE0/AZE0 (7), jeder Wert mit Quelle (OVMS, CanZE, OBDb, JejuSoul, EVNotify), geprüft gegen aufgezeichnete Antworten echter Autos

## Phase 8: Adapter und Anschluss

- [x] Rohe CAN-Schnittstellen mit eigenem ISO-TP und ELM327-Vorbau (`src/core/CanBus`): SAE J2534 PassThru über den Herstellertreiber (nur 64-Bit-Treiber ladbar, 32-Bit-Treiber werden angezeigt), SLCAN/Lawicel (die automatische Suche findet die Sticks), SocketCAN unter Linux. Geprüft mit einem nachgebauten Steuergerät, einem SLCAN-Stick im Test und einer J2534-Test-DLL; am echten Gerät noch ungeprüft. Offen: K-Line über J2534, eine 32-Bit-Brücke für alte Treiber
- [x] BMW ENET (HSFZ über Ethernet, Ports 6811/6801): Fahrzeugsuche per Broadcast, Sitzung mit Bestätigung, Lebenszeichen und 7F xx 78, darüber ein ELM327-Vorbau, damit OBD, Gesundheitscheck und der BMW-Suchlauf unverändert laufen. Geprüft mit einem nachgebauten Gateway über echtes TCP; am echten BMW noch ungeprüft
- [x] Bluetooth Low Energy (vLinker, OBDLink CX, Veepeak BLE) über Qt Bluetooth: Suche, automatische Wahl des Adapters, Kanalwahl nach bekannten GATT-Profilen (FFF0, 18F0, FFE0, E781, Nordic UART) mit Rückfall auf jeden Herstellerdienst. Ohne das Modul qtconnectivity wird wie bisher gebaut. Am echten OBD-Adapter noch ungeprüft, Suche und Verbindungsaufbau unter Windows an einem anderen BLE-Gerät geprüft
- [x] Adaptive Taktung ATAT2 mit Rückfall, wahlweise nur auf STN-Adaptern; STN wird erkannt. STPX und STBR bewusst weggelassen, weil nicht nachprüfbar oder dauerhaft speichernd. Klein
- [x] Sitzung aufzeichnen und wieder abspielen, als Transport wie das Demo-Auto. Hilft bei Fehlersuche und wenn jemand eine Aufnahme schickt. Mittel

## Phase 9: Die Garage wird zur Fahrzeugakte

- [x] Fahrten aus dem Bordcomputer speichern, Verbrauch über Wochen und Monate, Tankbuch. Mittel
- [x] Wartung: Ölwechsel, HU und AU, Zahnriemen nach Kilometer oder Datum, Erinnerung beim Verbinden. Mittel
- [x] Kilometerstand je Prüfung vergleichen und einen Rücksprung melden, als Hinweis beim Gebrauchtwagenkauf. Klein
- [x] Export und Import der Garage. Klein
- [x] Fahrzeugakte als PDF: Prüfungen, Wartung, Fahrten, Tankbuch, auch ohne verbundenes Auto. Klein

## Phase 10: Verteilen

- [x] Installer aus der CI: Windows (Inno Setup, dazu ein Zip), macOS dmg, Linux AppImage. `tools/package.sh` baut sie, `tools/smoke_test.sh` installiert, startet und entfernt sie in jedem CI-Lauf. Eigenes Programmsymbol (`src/app/AppIcon.cpp`, `tools/gen_icons.py`), Versionsangaben in der exe
- [x] Releases per Tag mit Änderungsliste auf Deutsch und Englisch (`changelog/<version>.md`, Job `release` in der CI). Getaggt und gepusht wird erst nach Jakobs Ja
- [ ] Windows-Programm signieren. Mittel, hängt an einem Zertifikat
- [x] Mehr Sprachen: 18 Sprachen für Oberfläche und Kern. Deutsch und Englisch bleiben im Code, alle anderen kommen aus Katalogen (`translations/<code>/`, Schlüssel ist der englische Text, Rückfall Englisch). `tools/i18n_extract.py` sammelt, `tools/gen_i18n.py` prüft Platzhalter, Zeilenumbrüche und Gedankenstriche und baut Katalog und Qt-Dateien. Arabisch von rechts nach links, feste Schrift je Schriftsystem. Neue Texte: extrahieren, nachübersetzen, einbauen (`translations/TRANSLATING.md`)

## Phase 12: Alle Marken, alle Wege ins Auto

- [x] Katalog um Great Wall (Haval, WEY, Tank), Chery, Jaecoo, Hongqi, Maxus, Xiaomi, Li Auto, Aito, Voyah, Avatr, Wuling, Mahindra, Tata, Proton, Holden, Alpina und INEOS ergänzt, neue Modelle bei Xpeng, Leapmotor und KGM; VIN-Kennungen nach der WMI-Liste auf Wikibooks (111 Marken, 2242 Generationen)
- [x] OBD über UDS (SAE J1979-2): wird beim Verbinden erkannt, wenn das Auto die klassischen Dienste ablehnt; 01, 03, 04, 07, 09 und 0A werden übersetzt. Freeze Frame und Modus 06 gibt es in dieser Form nicht
- [x] DoIP (ISO 13400-2): Fahrzeugsuche, Freigabe, Diagnose mit Bestätigung und Lebenszeichen; OBD über die funktionale Adresse 0xE400. Der Netzwerkanschluss probiert ENET und DoIP selbst
- [x] Mithören am CAN-Bus mit DBC-Signalen (auch Multiplexer); Autos ohne OBD-Antwort, aber mit Verkehr, werden zum Mithören verbunden. Tesla Model 3 und Y aus dem MIT-lizenzierten DBC von joshwardell, VIN aus 0x405
- [ ] Steuergerätelisten für DoIP-Autos (VW MEB, Volvo, Mercedes ab MBUX): keine belastbaren öffentlichen Adresslisten, kommt erst mit Mitschnitten
- [ ] Tesla Model S und X, BYD, Xpeng, NIO: eigene Signale nur mit belastbarer, frei nutzbarer Quelle
- [ ] Mercedes-Adressliste (siehe Phase 6)
- [ ] Ältere BMW (E-Reihe vor 2007) über K-Line mit DS2/KWP2000 braucht ein K+DCAN-Kabel im K-Line-Modus und Tests am Auto

## Phase 11: Am echten Auto prüfen

- [ ] Testrunde mit dem A2 (K-Line und KKL), einem CAN-VAG, einem Japaner und einem E-Auto
- [ ] Die offenen Punkte von oben: seltene PIDs, Versionsgrenzen im Adaptertest
- [ ] Herstellercodes für BMW, Opel, Renault, PSA, Fiat und Volvo ergänzen, nur mit Quelle
- [ ] Genaue Lage der OBD-Buchse für die häufigsten 200 Modelle

## Was bewusst nicht kommt

- Schreiben in Anpassungskanäle oder Codierung: ein falscher Wert legt das Auto lahm, das bleibt nur lesend. Löschen von Fehlern ist die einzige schreibende Aktion
- Wegfahrsperre, Schlüssel anlernen, Kilometerstand verändern: rechtlich heikel und nicht der Zweck dieses Werkzeugs
- VAG TP 2.0 über ELM327 (siehe Phase 6), dafür bleibt KKL
