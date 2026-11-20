# Häufige Fragen

## Kostet Caruspex etwas?

Nein. Caruspex ist freie Software unter der MIT-Lizenz, ohne Werbung, Konto oder bezahlte Extras.

## Welchen Adapter soll ich kaufen?

Für die meisten ist ein USB-ELM327 mit FTDI- oder CH340-Chip oder ein OBDLink (Bluetooth oder USB) der beste Anfang. Für VW, Audi, Seat und Skoda vor etwa 2004 kommt ein [KKL-Kabel](kkl.md) dazu, für BMW und Mini ab der F-Baureihe ein [ENET-Kabel](enet.md). Die [Übersicht der Schnittstellen](interfaces.md) vergleicht sie.

## Geht es mit meinem Auto?

Jedes Auto mit OBD-II oder EOBD geht für den Motor: in Europa Benziner ab 2001 und Diesel ab 2004, in den USA ab 1996. Ältere Autos aus dem VW-Konzern ab etwa 1990 gehen über das KKL-Kabel. Die Seite **Fahrzeuge** sagt, welcher Weg bei einem bestimmten Auto geht. Motorräder ab Euro 5 gehen mit einem Adapterkabel für ihren Diagnosestecker.

## Wurde es an meinem Auto getestet?

Ehrlich: noch nicht. Caruspex wird gegen nachgebildete Steuergeräte getestet; an echter Hardware wurde bisher nur ein OBDLink mit dem Chip STN2232 geprüft, ohne Auto. Bitte mit einem [Testbericht](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) helfen.

## Kann Caruspex mein Auto kaputt machen?

Caruspex liest überwiegend. Geschrieben wird nur nach Rückfrage: Fehlercodes löschen und, über das VAG-KKL-Kabel, der Stellglieddiagnose-Test und die Grundeinstellung der Drosselklappe. Es codiert nie, flasht nie und ändert keine Anpassungswerte. Nicht während der Fahrt benutzen.

## Kann es den Kilometerstand, die Wegfahrsperre oder Codierungen ändern?

Nein, und das wird es auch nicht. Das ist rechtlich heikel und nicht der Zweck dieses Werkzeugs.

## Warum meldet der Adapter Version 2.1, aber manches geht nicht?

Viele billige Adapter geben sich als neuere ELM327-Version aus, als sie sind. Der **Adaptertest** findet heraus, welche Befehle der Adapter wirklich kennt.

## Warum sind die Werte bei meinem älteren Auto langsam?

Autos an der K-Line (in Europa etwa 2001 bis 2007) antworten von Haus aus langsam. CAN-Autos sind viel schneller.

## Schickt Caruspex meine Daten irgendwohin?

Nein. Nur die Update-Suche fragt GitHub nach der neuesten Version, und erst nach einem Ja; das lässt sich jederzeit ändern. Siehe [Datenschutz](privacy.md).

## Wie kann ich helfen?

Einen Testbericht vom eigenen Auto schicken, einen Mitschnitt teilen, Fehler melden oder eine Übersetzung verbessern: Die Anleitung ist [TRANSLATING.md](https://github.com/SenseiIssei/Caruspex/blob/main/translations/TRANSLATING.md), und der Knopf **Übersetzung verbessern** in den Einstellungen öffnet sie.

Auch Code ist willkommen: [CONTRIBUTING.md](https://github.com/SenseiIssei/Caruspex/blob/main/CONTRIBUTING.md) erklärt auf Englisch, wie man Caruspex baut und was ein Pull Request braucht, mit einem kurzen deutschen Teil am Ende.
