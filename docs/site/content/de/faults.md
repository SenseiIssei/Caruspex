# Fehler

Die Seite **Fehlerspeicher** liest, was die Steuergeräte gespeichert haben, und erklärt es in einfachen Worten.

![Fehlerspeicher](screenshot-faults.png)

## Was man sieht

- **Gespeicherte, sporadische und permanente** Codes. Bei mehreren Steuergeräten steht an jedem Code, wer ihn gemeldet hat (Motor, Getriebe).
- Etwa 1.400 Standardcodes (P, C, B, U) und die Herstellercodes von VAG, Ford, GM, Toyota, Honda, Nissan, Hyundai und Kia, Subaru und mehr. Ein P1-Code wird so gelesen, wie die erkannte Marke ihn meint.
- Eine **Ampel** je Code und die Frage, die jeder zuerst stellt: Darf ich weiterfahren?
- Die üblichen Ursachen, was man selbst prüfen kann, und eine grobe Kostenklasse.
- Wo das Teil sitzt: das Auto in der Durchsicht oder die [Röntgenansicht](xray.md).

## Alle Steuergeräte

Im Profimodus fragt die Seite **Alle Steuergeräte** jedes Steuergerät ab, nicht nur den Motor: ABS, Airbag, Kombiinstrument, Klima und mehr. Zu jedem Fehler kommen die Fehlerart (Kurzschluss, Unterbrechung, unplausibles Signal), die Teilenummer und der Softwarestand. Ältere Autos aus dem VW-Konzern werden über ein [KKL-Kabel](kkl.md) gelesen.

## Was soll ich jetzt tun?

Die Seite **Was tun?** sortiert alles, was Caruspex gefunden hat, in drei Stufen: sofort anhalten, bald in die Werkstatt, kann warten. Jeder Punkt hat einen einfachen Satz zum Warum, eine grobe Kostenspanne und einen Zettel für die Werkstatt. Ob die Motorleuchte blinkt oder eine rote Warnleuchte an ist, sieht OBD nicht, deshalb fragen zwei Kästchen danach; eine blinkende Leuchte mit Zündaussetzern schiebt den Punkt auf "sofort anhalten".

![Was soll ich jetzt tun?](screenshot-todo.png)

## Codes löschen

Löschen geht je Steuergerät, mit einer deutlichen Warnung, und Caruspex liest den Speicher gleich danach noch einmal.

> Löschen ist keine Reparatur. Es setzt auch die Bereitschaftstests zurück: Bis das Auto mehrere vollständige Fahrzyklen gefahren ist, fällt es bei der Abgasuntersuchung durch, und ein Code, dessen Ursache noch da ist, kommt nach einer kurzen Fahrt wieder.

## Export

Die Liste lässt sich als CSV speichern und der Prüfbericht als PDF.
