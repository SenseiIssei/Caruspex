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

# Neubau in Rust und Tauri (0.9.0)

Erledigt im November 2026: die ganze Anwendung neu, mit gleichem Verhalten.

- [x] Cargo-Workspace mit einem Crate je Bereich (Sprachen, Fahrzeuge, Daten, Adapter, Protokolle, Netzwerk und CAN, Mitschnitte, Auswertungen, Demo-Auto, Kommandozeile), Aufbau in `crates/README.md`
- [x] Oberfläche mit Tauri 2, React und TypeScript: alle Seiten der Qt-Fassung, Demo-Modus im Browser, Arabisch von rechts nach links
- [x] Übersetzungen weiterverwendet: 18 Sprachen ohne Neuübersetzung, dazu die Katalog-Hinweise, die in der Qt-Fassung fehlten; `tools/i18n_rust.py` und `I18N_STRICT` in der CI
- [x] Kommandozeile mit gleicher Ausgabe und gleichem JSON, geprüft gegen Vergleichsdateien der 0.8
- [x] Garage im alten Format, Einstellungen aus der Registry übernommen, Windows-Installer mit gleicher AppId aktualisiert eine 0.8 an Ort und Stelle
- [x] Klassisches Bluetooth direkt über RFCOMM (Windows, Linux), Bluetooth LE über btleplug
- [x] Neue CI: Prüfungen, Tests und Pakete für Windows, Linux (AppImage und .deb) und macOS
- [x] PDF-Bericht ohne Druckdialog auch auf macOS und Linux: Windows über WebView2 (PrintToPdf), macOS über WKWebView (createPDF), Linux über einen WebKitGTK-Druckauftrag an GTKs Dateidrucker; der Druckdialog bleibt nur als Ausweg, wenn das scheitert
- [x] `--shot` für Bilder auch auf macOS und Linux (WKWebView takeSnapshot, WebKitGTK get_snapshot); der Rauchtest öffnet dort das Demo-Auto und schreibt Bild und PDF-Bericht, unter Linux mit xvfb
- [x] Übernahme der macOS-Einstellungen der Qt-Fassung aus `~/Library/Preferences/com.caruspex.Caruspex.plist`

# Großer Ausbauplan ab 0.8.0

Stand 10.11.2026, nach Version 0.8.0. Aufwand: Klein (Tage), Mittel (eine bis zwei Wochen), Groß (mehrere Wochen). Nutzen: wie viele Leute es merken. Jede Phase bekommt ihren eigenen Zweig, Push und PR nur nach Jakobs Ja.

Datenquellen, die wir nutzen dürfen:
- opendbc (comma.ai, MIT): DBC-Dateien für viele Marken, Rundfunkwerte zum Mithören
- OBDb (898 Fahrzeuge, CC BY-SA 4.0): Signale über Modus 22 und 01 je Modell; nutzbar als eigenes Datenpaket mit Namensnennung und gleicher Lizenz für die Daten
- model3dbc (MIT): schon im Einsatz für Tesla
- WMI-Liste auf Wikibooks: schon im Einsatz für die Erkennung
- Nur als Protokollwissen, ohne Code oder Daten zu übernehmen: EdiabasLib (GPL), CanZE und OVMS (Lizenz unklar)

## Phase 13: Immer aktuell bleiben

Damit nichts veraltet, ohne dass jemand daran denken muss.

- [x] Übersetzungen in der CI prüfen: der Linux-Lauf zieht die Texte neu aus dem Code (`update_translations`, `tools/i18n_extract.py`), bricht ab, wenn die Quelldateien nicht eingecheckt sind, und `tools/gen_i18n.py --check --strict` schlägt fehl, sobald in einer Sprache ein Text fehlt
- [x] Marken-Prüfung als Test: jede Katalogmarke muss aus der VIN erkannt werden, jede Generation hat gültige Jahre (`tests/test_vehicles.cpp`)
- [x] Monatlicher Datencheck (`.github/workflows/data-check.yml`, `tools/check_sources.py`): neue VIN-Kennungen auf Wikibooks, Zahl der OBDb-Fahrzeuge, neue Stände von opendbc und model3dbc. Gibt es etwas Neues, legt der Lauf ein Issue mit der Liste an. Der Ausgangsstand liegt in `data/sources.json` und `data/wmi_seen.txt`
- [x] Dependabot für die GitHub Actions (monatlich). Conan kennt Dependabot nicht; Catch2 und Qt werden weiter gezielt angehoben
- [x] Update-Hinweis in der App: beim Start (in den Einstellungen abschaltbar) fragt die App nur GitHub nach der neuesten Version, bei einer neueren erscheint in der Titelleiste "Version X ist da" und öffnet die Seite mit Download und Änderungsliste
- [x] Ein-Klick-Update: die App liest `latest.json` des neuesten Release, zeigt die Änderungsliste in der eigenen Sprache, lädt das Paket mit Fortschritt, prüft die minisign-Signatur gegen den eingebauten Schlüssel und installiert: Windows über den Inno-Setup-Installer im stillen Modus, Linux ersetzt die AppImage, macOS das .app-Bündel (sonst öffnet sich die dmg). Die CI signiert mit dem Geheimnis `CARUSPEX_UPDATER_KEY` und legt die Pakete zusätzlich unter Namen ohne Version ab (`tools/release_assets.py`). Einstellung "Updates automatisch installieren", aus als Vorgabe
- [x] Release-Skript `tools/release.sh <version>`: Zweig, Version, Gerüst für die Änderungsliste in Deutsch und Englisch (prüft Platzhalter und lange Gedankenstriche), Übersetzungs-Check, Build, Tests, Pakete und Rauchtest, Commit. Push, Merge und Tag bleiben Handarbeit nach einem Ja
- [x] README-Bilder pro Release: das Release-Skript nimmt die Bilder in beiden Sprachen neu auf, damit sie zur Version passen. Bewusst auf Windows mit den echten Schriften statt in der CI

## Phase 14: Leichter zu verstehen und zu bedienen

Das größte Hindernis ist nicht die Technik, sondern der erste Schritt.

- [x] Einrichtungs-Assistent beim ersten Start: Welcher Adapter (mit Bildern der üblichen Bauformen), wo ist die Buchse bei diesem Auto (Lage aus dem Katalog, mit Skizze), Zündung an, fertig. Groß, Nutzen sehr hoch. Umgesetzt mit SVG-Zeichnungen für USB, Bluetooth, WLAN, ENET und KKL, Skizze von oben je SocketLocation (`catalog::location_for`), überspringbar, wieder aufrufbar unter Einstellungen und Verbinden; erscheint nur bei neuen Installationen (`setupDone`)
- [x] Seite "Was soll ich jetzt tun?": aus Fehlern, Bereitschaft und Werten eine Liste in drei Stufen (sofort anhalten, bald in die Werkstatt, kann warten) mit grober Kostenspanne und einem Satz Begründung. Mittel
- [x] Gebrauchtwagen-Check als eigener Ablauf: VIN gegen Katalog, Kilometerstand aus mehreren Steuergeräten vergleichen (wo die Kennungen belegt sind), Zeit und Strecke seit dem letzten Löschen (PID 31 und 4E), Bereitschaft, Akkuzustand bei E-Autos; Ergebnis als PDF für den Kauf. Mittel, Nutzen sehr hoch. Umgesetzt als Seite Gebrauchtwagen unter Diagnose (Bewertung in caruspex-analysis used_car); belegte Kilometerstand-Kennungen bisher nur Tesla und das Demo-Auto, weitere kommen mit OBDb aus Phase 15
- [x] Verbindungs-Assistent bei Fehlschlägen: statt einer Meldung eine kurze Prüfliste je Schnittstelle (Treiber, Kopplung, Zündung, Bitrate, Nachbau-Adapter) mit dem, was Caruspex schon herausgefunden hat. Mittel
- [x] Einsteiger- und Profimodus: Einsteiger sehen Start, Fehler, Gesundheit, Akku, Garage; KKL, Anpassung, Rohbefehle und Adressen erst im Profimodus. Klein
- [x] Erklärungen direkt am Begriff: kurzer Hinweis beim Darüberfahren für Fachwörter (Bereitschaft, Freeze Frame, Langzeitkorrektur), ein Glossar in allen Sprachen. Mittel. 18 Begriffe (auch PID, Steuergerät, UDS, KWP, CAN, die drei Fehlerarten), Glossar unter Wissen auch für Einsteiger
- [x] Fahrmodus mit großer Schrift und wenigen Werten, für den Halter am Armaturenbrett; Warnung, nicht während der Fahrt zu bedienen. Klein. Tempo, Drehzahl, Kühlmittel, Bordspannung und Warnungen, liest nur, solange die Seite offen ist
- [x] Barrierefreiheit: Namen für Bildschirmleser, Tastaturbedienung überall, hoher Kontrast, Schriftgröße einstellbar. Mittel. Fokusfalle in Dialogen, Pfeiltasten in der Seitenleiste, Schriftgröße als Zoom des Fensters (80 bis 160 %)
- [x] Bericht teilen: QR-Code mit der Zusammenfassung, damit der Bericht aufs Handy oder zur Werkstatt kommt, ohne Konto und ohne Server. Klein. Eigener QR-Kodierer (`app/src/lib/qr.ts`, Byte-Modus, Stufe M), auf der Startseite und in der Berichtsvorschau
- [ ] Übersetzungen von Muttersprachlern gegenlesen lassen; Knopf "Übersetzung verbessern", der zur Anleitung führt. Laufend

## Phase 15: Mehr Autos und mehr Daten

- [x] OBDb einbinden: `tools/gen_obdb.py` lädt die Signalsätze (Zwischenspeicher `.cache/obdb`, nicht im Repo) und setzt jedes Signal exakt in eine Torque-Gleichung um, geprüft gegen den Decoder von OBDb; je Modelljahrbereich ein Paket, gepackt in `data/obdb` unter CC BY-SA 4.0 (1,5 MB, die CLI wird 1,6 MB größer). Stand 11.11.2026: 810 Repositories gelesen, 545 Fahrzeuge mit Werten, 968 Pakete, 153.794 Signale übernommen, 19.587 übersprungen (erweiterte Adressierung 8.224, Diagnosesitzung 6.366, Null-Grenzen 3.510, Aufzählungen 832, funktionale Anfrage 527, sonst 128). Zuordnung zum Fahrzeugkatalog über Namen, Suchbegriffe und `data/obdb_aliases.csv` (491 von 545 Fahrzeugen). Die App bietet die Pakete für das erkannte Auto auf Live-Werte und E-Auto-Akku an, die CLI mit `custom --pack obdb:MARKE MODELL JAHR`. Offen: BMW und andere mit erweiterter Adressierung, Werte hinter einer Diagnosesitzung, Aufzählungen als Text
- [ ] opendbc einbinden: Rundfunkwerte zum Mithören für Toyota, Honda, Hyundai/Kia, GM, Ford, VW, Nissan, Subaru, Tesla Model S und X und weitere. Ehrlich dazu: viele dieser Busse liegen hinter dem Gateway und sind an der OBD-Buchse nicht zu hören. Mittel
- [ ] Mehr Steuergerätelisten über Mitschnitte: Knopf "Mitschnitt teilen", der eine Aufnahme ohne VIN-Endziffern und ohne Standortdaten als Datei für ein GitHub-Issue packt. So kommen Mercedes, Volvo, Stellantis und DoIP-Autos ins Projekt, belegt am echten Auto. Mittel
- [ ] Herstellercodes (P1, B, C, U) für weitere Marken, nur aus frei nutzbaren Quellen oder eigenen Mitschnitten. Laufend
- [ ] Lage der OBD-Buchse mit Skizze für die 200 häufigsten Modelle. Mittel
- [ ] Motorräder: OBD nach Euro 5 (ISO 14230 und 15765) an den gängigen Diagnosesteckern, mit Adapterkabel. Mittel
- [ ] Lkw und Transporter über SAE J1939 (29 Bit, 250 oder 500 kbit/s, Diagnosebuchse 9-polig): Fehlercodes (DM1, DM2) und Grundwerte. Groß
- [ ] Katalog: Motorkennbuchstaben je Generation, damit Sollwerte (Leerlauf, Temperatur) zum Motor passen. Mittel

## Phase 16: Mehr Kommunikationswege

- [x] VAG TP 2.0 über die rohen CAN-Schnittstellen (J2534, SLCAN, SocketCAN): damit alle Steuergeräte bei VW, Audi, Seat und Skoda von etwa 2004 bis 2012, was mit dem ELM327 nicht ging. Groß, Nutzen hoch (Kanalaufbau, Blockgröße, Bestätigung, Kanaltest; KWP2000 mit Kennung, Fehlerspeicher, Messwertblöcken und Grundeinstellung; Suchlauf und KKL-Seiten wählen "TP 2.0", sobald über eine CAN-Schnittstelle verbunden ist; am echten Auto noch zu bestätigen)
- [x] K-Line über J2534 (ISO 9141 und 14230 als eigene Protokolle der PassThru-Geräte), damit ältere Autos auch mit Profi-Interfaces gehen. Mittel (ByteStream mit FIVE_BAUD_INIT und FAST_INIT, die KKL-Seiten listen J2534-Geräte; am echten Gerät noch zu bestätigen)
- [ ] 32-Bit-Brücke für alte J2534-Treiber: ein kleines 32-Bit-Hilfsprogramm, das über eine Pipe mit Caruspex spricht. Mittel
- [x] GM Single-Wire-CAN und Ford MS-CAN über OBDLink (STN-Befehle), damit Karosseriesteuergeräte dieser Marken erreichbar werden. Mittel
  Stand: `Elm327::switch_stn_bus` schaltet mit STP 53/54 auf das MS-CAN (125 kBit, Pins 3 und 11) und mit STP 63/64 auf das Single-Wire-CAN (33,3 kBit, Pin 1), zurück mit STP 33 bis 36; die Nummern stammen aus dem OBDLink-Handbuch (FRPM Rev. F, Abschnitt 8.6). Welcher Chip welchen Transceiver hat, steht in dessen Gerätetabelle: STN2230 bis 2232 (OBDLink EX) nur MS-CAN, STN1150 bis 1152, 1170, 2120, 2255 und 2256 (MX+) beide, SX, LX und CX keinen. Das SW-CAN geht beim Öffnen von selbst in den Normalbetrieb (STCSWM), STP setzt den CAN-Hardwarefilter passend. Die Steuergeräteseite bietet "Ford: Karosseriegeräte am MS-CAN" und "GM: Karosseriegeräte am Single-Wire-CAN" nur an, wenn ein Ford oder GM über einen OBDLink mit dem Transceiver verbunden ist; der Worker schaltet für die Suche um und danach zurück, Löschen geht auf denselben Bus. Gefragt werden die Ford-Liste ohne Motor und Getriebe und bei GM 0x240 bis 0x24F mit Antwort auf +0x400. Die Demo hat dafür je ein Karosseriesteuergerät auf beiden Bussen und die Profile OBDLink EX und MX+.
  Offen: Test an einem echten Ford und GM. Schlafende GMLAN-Geräte mit dem Hochvolt-Wecken (STCSWM 2) aufwecken ist nicht eingebaut.
- [ ] Ältere BMW (E-Reihe) über K+DCAN-Kabel im K-Line-Modus (DS2 und KWP2000 nach BMW-Art), Protokollwissen aus EdiabasLib, Test am Auto Pflicht. Groß
- [x] Schnellere Live-Werte mit STN-Geräten: Filter und Sammelabfragen über die STN-Befehle, Rundfunkwerte mit STMA statt ATMA. Mittel
  Stand: Nach dem Verbinden schaltet Caruspex bei STN-Chips die Sammelbefehle ein (STBC 1, FRPM Abschnitt 16, flüchtig); `Elm327::request_many` schickt dann alle Sechsergruppen und Einzel-PIDs einer Runde in einer Zeile und trennt die Antworten an den `|`, ein Fehler mitten im Stapel lässt den Rest einzeln nachfragen. Die App fragt damit 24 statt 6 PIDs je Takt. Mithören läuft mit STMA n statt ATMA, das Mitschneiden einer ID mit ATCRA (beim STN der CAN-Hardwarefilter), einem Durchlassfilter STFPA (ältere Firmware STFAP) und STM n, danach ATAR; beide hören nach n Botschaften von selbst auf. Gemessen an der Demo, die ihre Adapterzeit simuliert (`DemoTransport::adapter_ms`, 24 PIDs je Runde am OBDLink EX): über USB mit 2 ms je Weg 94 statt 100 ms, über Bluetooth mit 30 ms je Weg 122 statt 212 ms, gleiche Werte. STPX ist nicht eingebaut: die Live-Anfragen passen in einen CAN-Rahmen, und die Flusssteuerung für fremde Antwort-IDs regelt schon ATFCSH, das der STN versteht.
  Offen: am echten OBDLink prüfen, ob dessen Firmware STBC kennt (ohne STBC bleibt alles wie beim ELM327). Ein Hinweis auf die Zahl der Antworten je Gruppe könnte noch das Warten nach der letzten Antwort sparen.
- [ ] CAN FD bei den rohen Schnittstellen, die es können (SocketCAN, neuere J2534-Geräte), für die neuesten Autos. Mittel
- [ ] PCAN-Basic (PEAK) und Kvaser CANlib unter Windows direkt, für Leute ohne J2534-Treiber. Mittel

## Phase 17: Optimieren

- [ ] Steuergeräte-Suchlauf parallel über rohe CAN-Schnittstellen: mehrere ISO-TP-Sitzungen gleichzeitig, aus Minuten werden Sekunden. Mittel
- [ ] Gemeinsamer ELM-Vorbau: EnetElm und CanElm teilen viel Code (AT-Befehle, Formatierung); eine Basisklasse spart Doppeltes und Fehler. Mittel
- [ ] Übersetzungskatalog kleiner: die 6 MB Texte gepackt als Ressource je Sprache und erst bei Bedarf laden; Programm und Start werden schlanker. Mittel
- [ ] Schneller verbinden: letzter funktionierender Weg (Adapter, Protokoll, Bitrate) je VIN merken und zuerst probieren. Klein
- [ ] Große Dateien aufteilen (MainWindow, Session), damit Änderungen überschaubar bleiben. Mittel
- [ ] Robustheit: Fuzzing für alle Parser (ELM-Antworten, HSFZ, DoIP, DBC, Torque-CSV, Aufnahmen), Linux-Lauf mit AddressSanitizer und UndefinedBehaviorSanitizer in der CI. Mittel
- [ ] Kurven: bei langen Aufzeichnungen ausdünnen, damit die Live-Seite auch nach einer Stunde flüssig bleibt. Klein

## Phase 18: Vertrauen und Qualität

- [ ] Testprogramm am echten Auto mit der Gemeinschaft: Rückmeldung aus der App (Aufnahme plus Modell, VIN gekürzt) als Issue-Vorlage, Liste "geprüft mit" je Marke und Schnittstelle in der README. Mittel
- [ ] Signieren unter Windows (Zertifikat) und Notarisieren unter macOS (Apple-Entwicklerkonto), damit keine Warnungen mehr kommen. Klein, braucht Geld
- [ ] Absturzberichte nur auf Wunsch, ohne Fahrzeugdaten. Mittel
- [ ] Dokumentation als Website (GitHub Pages) in allen Sprachen, mit Bildern und einer Seite je Schnittstelle. Mittel

## Phase 19: Mehr Plattformen

- [ ] Android-App mit Qt: Bluetooth LE und klassisches Bluetooth sind schon da, das Handy ist im Auto sowieso dabei. Groß, Nutzen sehr hoch
- [ ] Verteilen über winget, Homebrew, Flathub und Snap, damit Installieren und Aktualisieren ein Befehl ist. Klein bis Mittel
- [ ] iOS später: nur Bluetooth LE und WLAN, App Store braucht ein Entwicklerkonto. Groß

## Phase 20: 3D-Röntgenansicht des erkannten Autos

Das verbundene Auto als drehbares 3D-Modell, die Karosserie durchsichtig wie
Glas, darin Motor, Antrieb, Tank oder Akku, Auspuff, die Steuergeräte und die
Datenleitungen zwischen ihnen. Was Caruspex gerade liest, leuchtet an der
Stelle, an der es im Auto sitzt: ein Fehler am Katalysator am Katalysator, die
Kühlmitteltemperatur am Kühler, ein Steuergerät, das nicht antwortet, rot im
Netz. Die bestehende Seitenansicht mit Durchsicht-Modus (`VehicleDiagram`)
bleibt als Rückfall und für schwache Rechner.

Ehrlich vorweg: ein 1:1-Nachbau mit jedem einzelnen Kabel geht für kein Auto,
weil Konstruktionsdaten und Stromlaufpläne den Herstellern gehören (ElsaPro,
ISTA, XENTRY und so weiter, nur gegen Lizenz). Erreichbar ist ein Modell, das
zum erkannten Auto passt: richtige Maße und Form, richtige Lage von Motor,
Antrieb, Akku und Tank, die Steuergeräte, die der Suchlauf wirklich gefunden
hat, und die Bussysteme, über die sie verbunden sind. Für einzelne Modelle
kommt ein echtes Modell dazu, wenn es eines mit freier Lizenz gibt.

- [x] 3D-Unterbau mit Three.js in der Tauri-Oberfläche: Kamera zum Drehen, Zoomen und Schwenken mit Maus, Touch und Tastatur, Glas-Material mit Fresnel-Rand für die Karosserie, feste Teile darin, glTF-Lader schon dabei. Rückfall auf die 2D-Ansicht (SVG) ohne WebGL oder über Remote Desktop. Mittel
  Stand: Seite "Röntgenansicht" unter Diagnose, Three.js wird erst beim Öffnen geladen (eigener Teil des Pakets), Einstieg auch vom Fehlerspeicher und vom Autobild der Startseite. Der Rückfall greift, wenn kein WebGL-Kontext entsteht; Remote Desktop mit Software-WebGL zeigt weiter 3D. `XrayScene.loadShell()` lädt schon ein glTF als Hülle, eine Bedienung dafür fehlt noch.
- [x] Karosserie aus Maßen: ein Formgenerator baut aus Aufbau (`BodyType`), Länge, Breite, Höhe, Radstand und Spurweite eine glatte Karosserie mit Fenstern, Rädern und Radhäusern. Die Maße je Generation kommen aus Wikidata (CC0) in den Katalog, mit Prüfung im Datencheck aus Phase 13. Groß, Nutzen hoch
  Stand: `tools/gen_dimensions.py` holt Länge, Breite, Höhe, Radstand und Gewicht per SPARQL aus Wikidata (Cache in `.cache/wikidata/`, nicht eingecheckt), ordnet sie über Name, Generationscode, Nummer und Baujahr den Generationen zu (`data/wikidata/aliases.csv` mit Begründung je Zeile für abweichende Namen, Sperren und Breiten mit Spiegeln) und schreibt `data/wikidata/dimensions.csv` und `crates/caruspex-vehicles/src/generated/dimensions.rs`; die CI erzeugt die Tabelle offline aus der CSV und prüft Umrechnung und Zuordnung an echten, gekürzten Antworten (`crates/caruspex-vehicles/tests/fixtures/wikidata/`). 1043 von 2247 Generationen haben Werte aus Wikidata, 576 alle vier; fehlende Werte folgen den echten im Verhältnis und sind in der App mit ≈ markiert. Die Form kommt aus `app/src/xray/shape.ts`: Überhänge, Haube, Scheibenneigungen, Dachwölbung und Dachabfall, Heckform, Gurtlinie, Bodenfreiheit, Räder, Seitenfenster mit Säulen, Türfugen und Knick, Vorgaben je Aufbau, Baujahr und Antrieb und eine Tabelle von Hand für 80 gängige Generationen samt Karosserievarianten (Kombi, Coupé, Cabrio). Die Seite sagt, woher die Form kommt (Wikidata, typisch oder eigenes Modell).
  Offen: Wikidata hat keine Spurweite und für viele beliebte Generationen keine oder nur einzelne Werte (F30, F36, Focus, Octavia); dort helfen nur Ergänzungen in Wikidata selbst, dann `--fetch` und `--match`. Den Abgleich in den monatlichen Datencheck aus Phase 13 aufnehmen. Scheinwerfer und Rückleuchten als Linien fehlen noch.
- [x] Antriebslayout je Generation: Motor vorn quer, vorn längs, Mitte oder hinten, Front-, Heck- oder Allradantrieb, Elektro mit Akku im Boden und ein oder zwei Motoren, Hybrid mit beidem. Daraus liegen Motor, Getriebe, Antriebswellen, Tank oder Akku, Auspuffstrang und Kühler an der richtigen Stelle. Quelle Wikidata und der vorhandene Katalog (`powertrains`). Mittel
  Stand: aus dem Katalog mit einer kurzen Regel und einer Ausnahmetabelle je Marke und Modell (`caruspex_vehicles::anatomy::drivetrain`), Wikidata als Quelle kommt mit den Maßen.
- [ ] Steuergeräte und Datenleitungen: jedes Steuergerät aus dem Suchlauf als Baustein an seinem üblichen Einbauort (Motor im Motorraum, Airbag unter der Mittelkonsole, Gateway hinter dem Handschuhfach, Einbauorte je Marke in `data/ecus/*.csv` ergänzt). Die Busse als Leitungen dazwischen: Antriebs-CAN, Komfort-CAN, LIN, FlexRay, Ethernet und die K-Leitung, farbig nach Bus. Beim Lesen laufen Pakete sichtbar über die Leitung zum Gerät, das gerade antwortet. Klick auf ein Gerät öffnet seine Seite. Mittel bis Groß, Nutzen hoch
  Stand: Geräte aus dem UDS-Suchlauf, dem KKL-Suchlauf und Motor und Getriebe über OBD stehen an ihrem üblichen Ort je Art, Antriebs-CAN, Komfort-CAN, Diagnose-CAN und K-Leitung farbig, Pakete laufen beim Suchlauf und beim Lesen über OBD. Offen: LIN, FlexRay, Ethernet, Einbauorte je Marke in `data/ecus/*.csv`; ein Klick wählt das Gerät und fliegt hin, die Seite öffnet der Knopf darunter.
- [ ] Der Hauptkabelbaum als Strang: Batterie, Hauptsicherungen, Masse, Motorkabelbaum, Strang durch den Innenraum zum Heck und die OBD-Buchse an ihrem wirklichen Ort (`SocketLocation`), mit dem gesteckten Adapter darin. Kein Einzeldraht, aber man sieht, wo was entlangläuft und wo man suchen muss. Mittel
- [x] Fehler und Live-Werte im Modell: Fehlercodes färben ihre Zone (`locateDtc`) und das zuständige Steuergerät, Live-Werte hängen als Schild am Teil (Ladedruck am Turbo, Lambda am Krümmer, Zellspannungen am Akku), "Auto prüfen" fliegt nacheinander alle gefundenen Stellen an. Mittel
  Stand: Kühlmittel am Kühler, Drehzahl am Motor, Spannung an der Batterie, Tank, Ansaugluft, Geschwindigkeit, Hochvoltakku; Ladedruck, Lambda und Zellspannungen folgen mit den Herstellerwerten.
- [ ] Echte Modelle, wo die Lizenz es erlaubt: glTF-Modelle mit CC0 oder CC BY für einzelne Generationen, nachgeladen bei Bedarf statt im Installer, mit Herkunft und Lizenz in der App. Dazu ein eigenes glTF laden, wer selbst eins hat. Das Innenleben bleibt aus dem Layout, nur die Hülle wird ersetzt. Mittel
  Stand: "Eigenes 3D-Modell laden" auf der Röntgenseite (Dialog nur auf Klick, `.glb` oder `.gltf` mit seinen Dateien daneben, `read_model_file`): das Modell wird längs gedreht (glTF schaut nach +Z), auf die echte Länge skaliert, die Achsen aus den Aufstandspunkten der Reifen über die des Layouts gelegt (bis 8 % Streckung für den Radstand), gläsern gezeichnet, Scheiben dunkler, eigene Räder ersetzen die erzeugten; "Modell umdrehen" für Modelle, die rückwärts schauen. Pfad und Drehung stehen je Auto in der Garage (`xrayModel`). Offen: freie Modelle mit Lizenzangabe zum Nachladen.
- [ ] Röntgen-Schnitte: Ebenen einzeln ein- und ausblenden (Karosserie, Antrieb, Kühlung, Kraftstoff oder Hochvolt, Elektrik, Netz), Schnittebene durchs Auto ziehen, Explosionsansicht für den Antriebsstrang. Klein bis Mittel
  Stand: die sechs Ebenen lassen sich schalten. Offen: Schnittebene und Explosionsansicht.
- [ ] Bilder und Tests: `--shot` und `--render-cars` auch für die 3D-Ansicht, damit README und CI sie zeigen, ein Test, dass jede Katalog-Generation ein gültiges Modell ergibt (keine Lücken, Räder auf dem Boden, Teile innerhalb der Hülle). Klein
  Stand: der Test steht (`app/src/xray/layout.test.ts`, jede Generation des Katalogs mit ihren Wikidata-Maßen und jeder Karosserievariante: Räder auf dem Boden, positive Überhänge, Innenraum über dem Boden, Teile in der Hülle; dazu `shell.test.ts` für das Einpassen eigener Modelle), `--shot` geht mit `--page xray`, für Vergleichsbilder auch `--page "xray-generation:<id oder Suche>|<estate, coupe, cabrio ...>|side"` und `?xray-generation=` in der Browser-Vorschau. Offen: `--render-cars` für 3D.

## Empfohlene Reihenfolge

1. Phase 13 (aktuell bleiben): klein, schützt alles Weitere
2. Gebrauchtwagen-Check, "Was soll ich jetzt tun?" und Einsteigermodus aus Phase 14: der größte Nutzen für normale Halter
3. OBDb aus Phase 15: Herstellerwerte und Akku für hunderte Modelle
4. 3D-Röntgenansicht aus Phase 20, zuerst Unterbau, Karosserie aus Maßen und Steuergeräte mit Datenleitungen: macht sichtbar, was die Diagnose findet
5. VAG TP 2.0 und K-Line über J2534 aus Phase 16
6. Mitschnitte teilen (Phase 15) und das Testprogramm (Phase 18) zusammen: daraus kommen Mercedes, DoIP-Listen und die Bestätigung am echten Auto
7. Android (Phase 19)

## Was bewusst nicht kommt

- Schreiben in Anpassungskanäle oder Codierung: ein falscher Wert legt das Auto lahm, das bleibt nur lesend. Löschen von Fehlern ist die einzige schreibende Aktion
- Wegfahrsperre, Schlüssel anlernen, Kilometerstand verändern: rechtlich heikel und nicht der Zweck dieses Werkzeugs
- VAG TP 2.0 über ELM327 (siehe Phase 6), dafür bleibt KKL
- Konstruktionsdaten und Stromlaufpläne der Hersteller im 3D-Modell: sie sind nur gegen Lizenz zu haben, deshalb zeigt Phase 20 Lage, Geräte und Busse statt jedes einzelnen Drahts
