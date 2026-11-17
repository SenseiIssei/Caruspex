# Röntgenansicht

Die **Röntgenansicht** zeigt das verbundene Auto als drehbares Glasmodell. Darin sieht man Motor oder Elektromotoren, Getriebe, Tank oder Hochvoltakku, Auspuff, Kühler, die 12-V-Batterie und die OBD-Buchse, angeordnet nach Aufbau und Antrieb des erkannten Autos.

![Röntgenansicht](screenshot-xray.png)

## Was aufleuchtet

- **Fehler** färben ihre Zone, und das zuständige Steuergerät wird rot.
- **Live-Werte** hängen als Schild an ihrem Teil: Kühlmittel am Kühler, Drehzahl am Motor, Spannung an der Batterie.
- **Steuergeräte** aus der Suche sitzen an ihrem üblichen Ort, verbunden durch ihre Datenleitungen (Antriebs-CAN, Komfort-CAN, Diagnose-CAN, K-Leitung). Während Caruspex liest, laufen kleine Pakete zum gefragten Gerät.
- **Auto prüfen** fliegt die Kamera nacheinander zu jedem Befund.

Drehen mit der Maus oder dem Finger, zoomen mit dem Mausrad oder zwei Fingern; die Tastatur geht auch. Die Ebenen (Karosserie, Antrieb, Kühlung, Kraftstoff oder Hochvolt, Elektrik, Netz) lassen sich einzeln ausblenden.

## Wie genau ist das Modell?

Die Form kommt aus den echten Maßen der Generation, wo sie bekannt sind (aus Wikidata oder den technischen Daten des Herstellers), sonst aus typischen Werten für Aufbau und Baujahr. Ein Nachbau des genauen Modells ist es nicht: Konstruktionsdaten und Stromlaufpläne der Hersteller sind nicht frei zu haben. Die Seite sagt, woher die Form kommt.

Wer ein 3D-Modell seines Autos als glTF hat (`.glb` oder `.gltf`), lädt es mit **Eigenes 3D-Modell laden**; das Innenleben bleibt aus dem Layout.

## Ohne 3D

Auf Rechnern ohne WebGL zeigt die Seite stattdessen eine flache Seitenansicht mit denselben Angaben.
