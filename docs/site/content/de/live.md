# Live-Werte

Die Seite **Live-Werte** (Profimodus) zeigt, was die Sensoren gerade messen. Im Einsteigermodus zeigt der **Fahrmodus** die wichtigsten Werte in großer Schrift.

![Live-Werte](screenshot-live.png)

## Was es gibt

- Jeder Wert aus Mode 01 von SAE J1979 bis PID C4: 145 PIDs mit mehr als 300 Messwerten, gruppiert und durchsuchbar. Welche das eigene Auto anbietet, sagt es selbst.
- Kilometerstand, Gang, Öltemperatur, Kraftstoffverbrauch, Drehmoment, Ladedruck, Abgastemperaturen, Partikelfilter, AdBlue, NOx und Hybridbatterie, soweit das Auto sie meldet.
- Eine laufende Kurve für mehrere Werte zugleich. Bei langen Zeitfenstern (bis zu einer Stunde) dünnt die Kurve alte Punkte aus, behält aber jede Spitze und jedes Tal.
- Aufzeichnen als CSV.

## Hersteller- und eigene Werte

- **Akkus von Elektroautos**: Gesundheitszustand, Ladestand, Zellspannungen mit Streuung und Temperaturen für Hyundai Kona und Ioniq, Kia e-Niro, Renault Zoe, Nissan Leaf und Tesla Model 3 und Y (durch Mithören am CAN-Bus), auf der Seite **E-Auto-Akku**.
- **OBDb-Werte** für etwa 550 weitere Fahrzeuge (Reifendruck, Öl, Kilometerstand und mehr), von selbst für das erkannte Auto angeboten.
- **Erweiterte Diagnosesitzung**: Manche Werte von Dieseln aus dem VW-Konzern, etwa die gemessene Rußmasse und die Strecke seit der letzten Regeneration, gibt das Motorsteuergerät nur in seiner erweiterten Diagnosesitzung heraus. Im Profimodus schaltet Caruspex das Steuergerät dafür in diese Sitzung, solange solche Werte abgefragt werden, nur zum Lesen, markiert sie mit **Sitzung 03** und schaltet danach zurück. Abschalten lässt sich das in den Einstellungen unter **Herstellerwerte**. Lehnt das Steuergerät ab, etwa während der Fahrt, steht der Grund am Wert.
- **Eigene Werte** im verbreiteten Torque-CSV-Format, samt Gleichungen.

![E-Auto-Akku](screenshot-battery.png)

## Wie schnell?

Wie viele Werte je Sekunde kommen, hängt von Auto und Adapter ab. CAN-Autos antworten schnell, Autos an der K-Line sind von Haus aus langsam. Mit einem OBDLink fragt Caruspex bis zu 24 Werte auf einmal (siehe [OBDLink und STN](obdlink-stn.md)); eine rohe CAN-Schnittstelle ist der schnellste Weg.

## Aufnehmen und Abspielen

Eine ganze Sitzung mit dem Auto lässt sich in eine Datei aufnehmen und später ohne Auto abspielen, praktisch bei der Fehlersuche. Mit **Mitschnitt teilen** lässt sich eine Aufnahme ohne persönliche Daten für das Projekt vorbereiten, siehe [Datenschutz](privacy.md).
