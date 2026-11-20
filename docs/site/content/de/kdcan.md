# K+DCAN-Kabel

Das K+DCAN-Kabel ist das übliche Kabel für BMW von etwa 1998 bis 2017 mit den Werkstattprogrammen INPA und ISTA. Darin stecken ein serieller FTDI-Chip und ein kleiner Umsetzer für das D-CAN von BMW. Viele dieser Kabel haben einen Schalter oder eine Brücke zwischen Pin 7 und 8 für ältere Autos.

## Stand in Caruspex

> Ehrlicher Stand: Caruspex liest BMW der E-Reihe über die K-Line dieses Kabels: DS2 bei den älteren Modellen, KWP2000 nach BMW-Art und BMW-FAST bei der E60-Generation. Getestet ist das bisher nur an den nachgebauten Autos im Programm, noch nicht an einem echten BMW. Den D-CAN-Modus des Kabels spricht Caruspex noch nicht.

## Im K-Line-Modus

- **Schalter auf K-Line** stellen, bevor das Kabel in die Buchse kommt; bei den meisten Kabeln ist das die linke Stellung. Sie verbindet Pin 7 und 8, damit auch die Steuergeräte an Pin 8 der älteren Modelle antworten.
- **Welche Autos**: E36 (mit dem 20-poligen Rundstecker über einen Adapter), E38, E39, E46, E52, E53, E83, E85 und die E60-Generation (E60, E61, E63, E64, E65, E87, E90) bis 03/2007. Danach und ab der F-Reihe spricht die Buchse D-CAN oder Ethernet.
- **So geht's**: Profimodus an, Seite *BMW über K-Line*, Anschluss wählen, *Durchsuchen*. Caruspex fragt jede bekannte Adresse über DS2 (9600 Baud, gerade Parität), KWP2000 (10400 Baud mit Schnellinitialisierung) und BMW-FAST (115200 Baud) und zeigt die gefundenen Steuergeräte mit Kennung und Fehlerspeicher. Das dauert etwa eine halbe Minute.
- **Löschen** geht je Steuergerät oder überall, beides erst nach Rückfrage. Danach liest Caruspex nach, was geblieben ist.
- **Fehlercodes**: BMW veröffentlicht keine Texte zu seinen Codes, Caruspex zeigt sie deshalb als Hexzahl.
- **Nur lesen**: Codieren, Anpassen und Stellglieder gibt es hier nicht, die gehören in die Werkstattsoftware.
- **Ein KKL-Kabel** geht genauso, erreicht bei den älteren Modellen aber nur die Steuergeräte an Pin 7, solange Pin 7 und 8 nicht verbunden sind.

Zum Ausprobieren ohne Auto stehen in der Anschlussliste der Seite ein nachgebauter E46 (DS2) und ein E60 (KWP2000).

## Was sonst geht

- **BMW und Mini ab der F-Baureihe (etwa ab 2008)**: Ein [ENET-Kabel](enet.md) erreicht alle Steuergeräte über Ethernet, oder ein einfacher [ELM327](elm327-usb.md) mit der eigenen Adressierung von BMW über CAN.
- **Standard-OBD bei BMW mit CAN (etwa ab 2007)**: Jeder ELM327 liest Motorfehler und Live-Werte.

## Hilfe gesucht

Wer ein K+DCAN-Kabel und einen älteren BMW hat, hilft mit einem [Testbericht](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) samt Protokoll, den K-Line-Teil am echten Auto zu bestätigen.
