# Tuning-Simulator

Die Seite **Tuning-Simulator** (Profimodus, unter Messen) ist ein virtueller BMW N55: der 3,0-Liter-Reihensechser mit einem Twin-Scroll-Turbo aus dem F30 335i und dem F32/F36 435i, 306 PS und 400 Nm. Verändere Ladedruck, Zündung und Lambda je Drehzahlband und sieh, was das Modell dazu sagt, lange bevor irgendetwas ein Auto berührt.

> **Nur Simulation.** Caruspex schreibt nie Kennfelder und flasht nie. Es kennt kein Flash-Dateiformat, keine Prüfsumme und keine Kennfeldadresse und bildet kein kommerzielles Tuning nach. Echtes Tuning gehört zu etablierten Werkzeugen und in eine Werkstatt, und die Änderung braucht eine Abnahme (in Deutschland durch den TÜV). Ohne sie erlischt die Betriebserlaubnis.

![Tuning-Simulator](screenshot-tune.png)

## Was du ändern kannst

- Offsets in fünf Drehzahlbändern (1000 bis 2000, 2000 bis 3500, 3500 bis 5000, 5000 bis 6000, 6000 bis 7000): Ladedruck in bar, Zündung in Grad, Lambda.
- Kraftstoff (ROZ 95, 98, 100 oder 102), Außentemperatur, Luftdruck und das Getriebe (Automatik 8HP45 oder Sechsgang-Schaltgetriebe).
- **Serie** setzt die Offsets auf null zurück, **Zurücksetzen** auch die Bedingungen.

Die Seite zeigt Drehmoment und Leistung von Serie und Änderung, jede Grenze, die die Änderung berührt, mit ihrem Grund, und die Innenwerte: Ladedruck, Ladeluft, Zündung, Klopfreserve, Abgastemperatur, Turbo, Hochdruckpumpe, Mitteldruck.

## Wie das Modell rechnet

1. **Luft.** Das Ladedruckziel wird begrenzt durch das, was der Turbo bei dieser Drehzahl aufbauen kann, durch sein Druckverhältnis und durch seinen Durchsatz. Der Verdichter erwärmt die Luft, der Ladeluftkühler nimmt einen Teil der Wärme wieder heraus. Druck, Temperatur und Liefergrad ergeben die Luftmasse.
2. **Kraftstoff und Zündung.** Der Kraftstoff folgt aus Luft und Lambda. Die Zündung ist der Serienzündwinkel plus dein Offset, aber nie früher, als die Klopfgrenze erlaubt: die Klopfregelung des Motors würde sie zurücknehmen. Die Klopfgrenze verschiebt sich mit Oktanzahl, Ladelufttemperatur, Ladedruck und Lambda.
3. **Leistung.** Kraftstoffenergie mal einem Wirkungsgrad, der mit dem Abstand zum besten Zündzeitpunkt sinkt, minus Reibung ergibt Drehmoment und Leistung.
4. **Grenzen.** Turbo, Abgastemperatur, Hochdruckpumpe, Einspritzventile, Vorentflammung bei niedriger Drehzahl, Getriebe, Ladeluft und Gemisch geben jeweils eine Warnung in einfachen Worten.

Eine ganze Kurve (25 Punkte von 1000 bis 7000 1/min) dauert im Release-Build etwa 0,015 ms.

## Veröffentlichte Daten

| Wert | Zahl | Quelle |
|---|---|---|
| Hubraum, Bohrung, Hub, Verdichtung | 2979 cm³, 84,0 mm, 89,6 mm, 10,2:1 | Wikipedia "BMW N55" (CC BY-SA 4.0) |
| Drehmoment | 400 Nm von 1200 bis 5000 1/min | Wikipedia "BMW N55", BMW-Daten des 435i |
| Leistung | 225 kW (306 PS) bei 5700 bis 6000 1/min | Wikipedia (5700 bis 5800), BMW (5800 bis 6000) |
| Höchstdrehzahl | 7000 1/min | Wikipedia "BMW N55" |
| Eingangsdrehmoment des 8HP45 | 450 Nm | Wikipedia "ZF 8HP transmission" (CC BY-SA 4.0) |

## Annahmen

Die inneren Kennfelder des N55 veröffentlicht niemand frei lizenziert. Diese Werte sind meine Annahmen; jede ist in [spec.rs](https://github.com/SenseiIssei/Caruspex/blob/main/crates/caruspex-tune/src/spec.rs) begründet.

- Der **Serienladedruck** ist mit dem Modell aus der veröffentlichten Kurve zurückgerechnet (etwa 0,7 bis 0,8 bar bis 5000 1/min, darüber weniger). Dass die Serienkurve passt, zeigt deshalb Stimmigkeit, nicht Wahrheit.
- Liefergrad 0,86 bis 0,97, innerer Wirkungsgrad 0,39, Reibung quadratisch über der Drehzahl.
- Klopfbegrenzte Zündung 3 bis 15 Grad vor dem oberen Totpunkt mit 1,5 Grad Reserve; sie verschiebt sich um 1 Grad je Oktanzahl, 0,1 Grad je Kelvin Ladeluft, 6 Grad je bar und 2,5 Grad je 0,1 Lambda.
- Turbo: Druckverhältnis bis 2,55, korrigierter Durchsatz bis 0,32 kg/s, Wirkungsgrad 0,72. Ladeluftkühler-Wirkungsgrad 0,85, weniger bei hohem Durchsatz.
- Grenze der Abgastemperatur 980 °C, Hochdruckpumpe 0,027 kg/s, Einspritzfenster 40 % des Arbeitsspiels, Risiko der Vorentflammung über 18 bar Mitteldruck unter 2500 1/min, Sechsgang-Schaltgetriebe etwa 450 Nm.
- Kraftstoff E5: Luftbedarf 14,4, 42,4 MJ/kg.

## Wie genau ist das?

- Serie gegen die veröffentlichte Kurve: 400 Nm auf 1 % genau von 1250 bis 5000 1/min, 306 PS bei 6000 1/min und 304 PS bei 5750 1/min. Die Tests lassen 3 % zu.
- Das Modell ist noch an keinem echten N55 geprüft. Jede Zahl ist eine Schätzung: etwa ±8 % bei Drehmoment und Leistung, ±0,1 bar Ladedruck, ±10 K Ladeluft, ±3 Grad Klopfreserve, ±60 K Abgastemperatur.

## Vergleich mit deinem Auto

1. Verbinden, **Live-Werte** mit Drehzahl, Last, Saugrohrdruck, Ansauglufttemperatur, Zündzeitpunkt, Lambda und Luftmasse öffnen und eine Aufnahme starten.
2. Einmal mit Vollgas durch das Drehzahlband beschleunigen, nur wo das erlaubt und sicher ist.
3. Die Aufnahme auf der Simulator-Seite öffnen. Caruspex nimmt die Volllast-Punkte, fasst sie in Bändern von 500 1/min zusammen und zeigt gemessen gegen simuliert: Saugrohrdruck und Ladeluft mit deinen Kennfeldern, Luftmasse und Zündung mit dem gemessenen Druck und der gemessenen Temperatur.

OBD meldet den Saugrohrdruck nur bis 255 kPa, und welchen Sensor das Steuergerät für die Ansaugtemperatur nimmt, ist nicht dokumentiert. **Beispiel ansehen** füllt die Tabelle mit erfundenen Werten, damit du siehst, wie sie sich liest.

## Gut zu wissen

- Mehr Drehmoment als Serie bei niedriger Drehzahl erhöht das Risiko der Vorentflammung, die Kolben auf einen Schlag zerstören kann.
- Frühere Zündung als die Klopfgrenze bringt nichts: die Klopfregelung nimmt sie zurück.
- Besserer Kraftstoff allein bringt Reserve, keine Leistung: das Serienkennfeld der Zündung ist die Obergrenze.
