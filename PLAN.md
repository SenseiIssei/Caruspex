# Caruspex: der große Ausbau

Stand: 09.11.2026. Zweig `grosser-ausbau`.

Ziel: Man steckt einen Adapter ein, drückt auf einen Knopf, und Caruspex weiß, welches Auto da hängt, was es kann und was ihm fehlt. Für jedes Fahrzeug, vom Golf 2 mit KKL-Stecker bis zum E-Auto von heute. Die Oberfläche soll sich dabei anfühlen wie Odysync: ruhig, dunkel, mit Bewegung an den richtigen Stellen, und so erklärt, dass auch jemand ohne Werkstatterfahrung versteht, was los ist.

Ehrlich bleibt es trotzdem. Was ein Auto oder ein Adapter technisch nicht hergibt, sagt Caruspex klar, statt es vorzutäuschen.

## Phase 0: Grundlage

- [x] Arbeitszweig, Build und Tests laufen (48 Testfälle grün)
- [ ] Echte Umlaute in allen Texten statt ae/oe/ue, die englische Übersetzung wandert mit
- [ ] Neue Dateien sauber in CMake und CI

## Phase 1: Jedes Fahrzeug erkennen

**Fahrgestellnummer deuten (VIN-Decoder)**
- Herstellerkennung (WMI) für alle großen und viele kleine Marken weltweit, auch historische
- Baujahr aus Stelle 10, mit der 30-Jahres-Zweideutigkeit sauber aufgelöst
- Prüfziffer nach nordamerikanischer Norm, Herkunftsland und Werk
- Modellcodes, wo der Hersteller sie in die VIN schreibt (VAG Stelle 7 und 8: 8Z ist der A2, 1K der Golf 5 und so weiter)

**Fahrzeugkatalog**
- Marken, Modelle und Generationen mit Bauzeit, Karosserieform und Antrieb
- Je Generation: welcher Diagnosestecker (keiner, herstellereigen, OBD-II), welches Protokoll, was Caruspex damit kann
- Wo die OBD-Buchse sitzt, damit niemand unterm Armaturenbrett suchen muss
- Ältere Autos ohne OBD-II bekommen eine klare Auskunft statt einer Fehlermeldung, VAG ab etwa 1990 geht über KKL
- Gepflegt als CSV, ein Generator macht C++ daraus (wie bei den Fehlercodes)

**Automatisch statt Auswahlliste**
- Verbinden ohne Vorwahl, danach VIN lesen, deuten, im Katalog finden
- Kraftstoffart (PID 51) und OBD-Norm (PID 1C) ergänzen das Bild
- Die Handauswahl bleibt, aber durchsuchbar: Marke, Modell, Generation

## Phase 2: Viel mehr OBD-II

**Messwerte**
- Alle Modus-01-Messwerte nach SAE J1979 bis PID C4, auch die mehrteiligen
- Neu darunter: Kilometerstand (PID A6, neuere Autos), Gang, Öltemperatur, Kraftstoffdurchfluss, Drehmoment, Ladedruck, Abgastemperatur, Partikelfilter, AdBlue, NOx, Hybrid- und Akkuwerte

**Fehlercodes**
- Von 462 auf mehrere tausend: P0, P2, P3, U0, dazu B und C
- Herstellereigene Codes für die großen Gruppen, die Bedeutung richtet sich nach der erkannten Marke
- Zu jedem Code: wie dringend (Ampel), darf ich weiterfahren, typische Ursachen, was man selbst prüfen kann

**Mehrere Steuergeräte über CAN**
- Welches Steuergerät meldet welchen Fehler (Motor, Getriebe, Hybrid)
- Steuergerätenamen aus Modus 09

**Prüfen und Auswerten**
- Gesundheitscheck mit einem Knopf: Fehler, Bereitschaft, Batterie, Kühlmittel, Gemischregelung, Lambdasonden, Zündaussetzer. Ergebnis als Punktzahl mit Erklärung in einfachen Worten
- AU-Vorabcheck mit Fahrzyklus-Hilfe: welcher Test fehlt noch und wie man ihn fährt
- Batterie- und Ladeprüfung als geführter Test: Ruhespannung, Startspannungseinbruch, Ladespannung
- Bordcomputer: Verbrauch live und im Schnitt, Kosten, CO2, Strecke, Fahrstil
- Leistungsmessung: 0 bis 100, 80 bis 120, Viertelmeile
- Zündaussetzer je Zylinder aus Modus 06
- In-Use-Performance-Zähler aus Modus 09

**Garage und Verlauf**
- Jedes Auto wird an seiner VIN wiedererkannt
- Verlauf: welche Fehler wann, Kilometerstand, Prüfergebnisse
- Bericht als PDF zum Ausdrucken oder Mitnehmen in die Werkstatt

**Adapter**
- Adaptertest: echter ELM327 oder Klon, was er wirklich kann, wie schnell er antwortet
- OBDLink und andere STN-Adapter erkennen und schneller fahren
- Eingebautes Demo-Fahrzeug, damit man alles ohne Auto und ohne Python ausprobieren kann

## Phase 3: Neue Oberfläche

- Gestaltungsgrundlage: Farben, Abstände, Rundungen, Schriftgrößen an einer Stelle
- Eigene Symbole für die Navigation, sauber in jeder Größe (SVG)
- Seitenleiste in Gruppen (Diagnose, Messen, Werkstatt, Wissen) mit gleitender Markierung
- Verbindung oben als Statuskapsel, Protokoll als ausklappbare Leiste unten statt in der Seitenleiste
- Weiche Übergänge zwischen den Seiten
- Startseite neu: großes Fahrzeugbild, Name des erkannten Autos, animierter Gesundheitsring wie der Pflegering in Odysync, große Kacheln für die häufigsten Aufgaben
- Das Fahrzeugbild neu gezeichnet: richtige Karosserieformen (Kleinwagen, Limousine, Kombi, SUV, Transporter, Coupé, Pick-up), Lack mit Verlauf, Felgen, Licht. Fehlerorte als leuchtende Punkte mit Beschriftung statt abgeschnittener Kästen
- Rundanzeigen mit Verlauf, Skala und Leuchten
- Einrichtungshilfe für den ersten Start: Buchse finden, Zündung an, Adapter wählen, los
- Kurze Hinweise unten rechts statt stummer Statuszeilen, Ladeanimationen statt leerer Flächen

## Phase 4: Abschluss

- Tests für VIN, Katalog, neue Messwerte, Gesundheitscheck und Bordcomputer
- Englische Übersetzung vollständig
- README deutsch und englisch, neue Bildschirmfotos
- CI auf Windows, Linux und macOS grün

## Was bewusst nicht kommt

- Schreiben in Anpassungskanäle oder Codierung: ein falscher Wert legt das Auto lahm, das bleibt nur lesend
- Herstellerdiagnose über CAN (UDS, TP 2.0) für alle Marken: braucht je Hersteller eigene Datensätze, die frei nicht zu haben sind. Für VAG kann das später ein eigener Schritt werden
- Bluetooth Low Energy: Qt Bluetooth fehlt in der Qt-Installation. Klassisches Bluetooth läuft weiter über den COM-Anschluss, WLAN-Adapter über TCP
