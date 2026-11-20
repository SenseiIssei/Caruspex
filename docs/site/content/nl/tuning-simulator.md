# Tuningsimulator

De pagina **Tuningsimulator** (pro-modus, onder Meten) is een virtuele BMW N55: de 3,0 liter zes-in-lijn met één twin-scroll-turbo uit de F30 335i en de F32/F36 435i, 306 pk en 400 Nm. Verander laaddruk, ontsteking en lambda per toerengebied en zie wat het model ervan zegt, lang voordat er iets aan een auto komt.

> **Alleen simulatie.** Caruspex schrijft nooit motorkennvelden en flasht nooit. Het kent geen flash-bestandsformaat, geen controlesom en geen kennveldadres, en kopieert geen commerciële tuning. Echt tunen hoort bij gevestigde gereedschappen en in een werkplaats, en de wijziging moet gekeurd worden (in Duitsland door de TÜV). Zonder keuring vervalt de typegoedkeuring.

![Tuningsimulator](screenshot-tune.png)

## Wat je kunt veranderen

- Afwijkingen in vijf toerengebieden (1000 tot 2000, 2000 tot 3500, 3500 tot 5000, 5000 tot 6000, 6000 tot 7000): laaddruk in bar, ontsteking in graden, lambda.
- Brandstof (RON 95, 98, 100 of 102), buitentemperatuur, luchtdruk en de versnellingsbak (automaat 8HP45 of handbak met zes versnellingen).
- **Standaard** zet de afwijkingen terug op nul, **Herstellen** ook de omstandigheden.

De pagina toont koppel en vermogen standaard tegenover gewijzigd, elke grens die de wijziging raakt met de reden, en de interne waarden: laaddruk, inlaatlucht, ontsteking, pingelreserve, uitlaatgastemperatuur, turbo, hogedrukpomp, gemiddelde effectieve druk.

## Hoe het model rekent

1. **Lucht.** Het laaddrukdoel wordt begrensd door wat de turbo bij dat toerental kan opbouwen, door zijn drukverhouding en door zijn debiet. De compressor verwarmt de lucht, de intercooler haalt een deel van de warmte er weer uit. Druk, temperatuur en vullingsgraad geven de luchtmassa.
2. **Brandstof en ontsteking.** De brandstof volgt uit lucht en lambda. De ontsteking is het standaardtijdstip plus jouw afwijking, maar nooit vroeger dan de pingelgrens toelaat: de klopregeling van de motor zou haar terugnemen. De pingelgrens verschuift met octaangetal, inlaatluchttemperatuur, laaddruk en lambda.
3. **Vermogen.** Brandstofenergie maal een rendement dat daalt met de afstand tot het beste ontstekingstijdstip, min wrijving, geeft koppel en vermogen.
4. **Grenzen.** Turbo, uitlaatgastemperatuur, hogedrukpomp, injectoren, voorontsteking bij laag toerental, versnellingsbak, inlaatlucht en mengsel geven elk een waarschuwing in gewone woorden.

Een hele curve (25 punten van 1000 tot 7000 tpm) duurt in de releaseversie ongeveer 0,015 ms.

## Gepubliceerde gegevens

| Waarde | Getal | Bron |
|---|---|---|
| Cilinderinhoud, boring, slag, compressie | 2979 cm³, 84,0 mm, 89,6 mm, 10,2:1 | Wikipedia "BMW N55" (CC BY-SA 4.0) |
| Koppel | 400 Nm van 1200 tot 5000 tpm | Wikipedia "BMW N55", BMW-gegevens van de 435i |
| Vermogen | 225 kW (306 pk) bij 5700 tot 6000 tpm | Wikipedia (5700 tot 5800), BMW (5800 tot 6000) |
| Maximaal toerental | 7000 tpm | Wikipedia "BMW N55" |
| Nominaal ingangskoppel van de 8HP45 | 450 Nm | Wikipedia "ZF 8HP transmission" (CC BY-SA 4.0) |

## Aannames

Niemand publiceert de interne kennvelden van de N55 onder een vrije licentie. Deze waarden zijn mijn aannames; elke staat toegelicht in [spec.rs](https://github.com/SenseiIssei/Caruspex/blob/main/crates/caruspex-tune/src/spec.rs).

- De **standaardlaaddruk** is met het model teruggerekend uit de gepubliceerde curve (ongeveer 0,7 tot 0,8 bar tot 5000 tpm, daarboven minder). Dat de standaardcurve klopt, toont dus samenhang, geen waarheid.
- Vullingsgraad 0,86 tot 0,97, geïndiceerd rendement 0,39, wrijving kwadratisch met het toerental.
- Door pingelen begrensde ontsteking 3 tot 15 graden voor het bovenste dode punt met 1,5 graden reserve; die verschuift 1 graad per octaangetal, 0,1 graad per kelvin inlaatlucht, 6 graden per bar en 2,5 graden per 0,1 lambda.
- Turbo: drukverhouding tot 2,55, gecorrigeerd debiet tot 0,32 kg/s, rendement 0,72. Effectiviteit van de intercooler 0,85, minder bij hoog debiet.
- Grens van de uitlaatgastemperatuur 980 °C, hogedrukpomp 0,027 kg/s, inspuitvenster 40 % van de cyclus, risico op voorontsteking boven 18 bar gemiddelde effectieve druk onder 2500 tpm, handbak ongeveer 450 Nm.
- Brandstof E5: lucht-brandstofverhouding 14,4, 42,4 MJ/kg.

## Hoe nauwkeurig is het?

- Standaard tegenover de gepubliceerde curve: 400 Nm binnen 1 % van 1250 tot 5000 tpm, 306 pk bij 6000 tpm en 304 pk bij 5750 tpm. De tests laten 3 % toe.
- Het model is nog aan geen echte N55 getoetst. Elk getal is een schatting: ongeveer ±8 % bij koppel en vermogen, ±0,1 bar laaddruk, ±10 K inlaatlucht, ±3 graden pingelreserve, ±60 K uitlaatgastemperatuur.

## Vergelijken met je auto

1. Maak verbinding, open **Live-waarden** met toerental, belasting, inlaatdruk, inlaatluchttemperatuur, ontstekingstijdstip, lambda en luchtmassa, en start een opname.
2. Accelereer één keer vol door het toerengebied, alleen waar dat mag en veilig is.
3. Open de opname op de simulatorpagina. Caruspex houdt de punten bij vol gas, groepeert ze in gebieden van 500 tpm en toont gemeten tegenover gesimuleerd: inlaatdruk en inlaatlucht met jouw kennvelden, luchtmassa en ontsteking met de gemeten druk en temperatuur.

OBD meldt de inlaatdruk maar tot 255 kPa, en welke sensor de motorregeling voor de inlaattemperatuur gebruikt, is niet gedocumenteerd. **Voorbeeld bekijken** vult de tabel met verzonnen waarden, zodat je ziet hoe die leest.

## Goed om te weten

- Meer koppel dan standaard bij laag toerental verhoogt het risico op voorontsteking, die zuigers in één klap kan breken.
- Een vroegere ontsteking dan de pingelgrens levert niets op: de klopregeling neemt haar terug.
- Betere brandstof alleen geeft reserve, geen vermogen: het standaard ontstekingskennveld is het plafond.
