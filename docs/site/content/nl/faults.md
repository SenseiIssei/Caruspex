# Fouten

De pagina **Foutgeheugen** leest wat de regeleenheden hebben opgeslagen en legt het in gewone taal uit.

![Foutgeheugen](screenshot-faults.png)

## Wat je ziet

- **Opgeslagen, wachtende en permanente** codes. Bij meerdere regeleenheden toont elke code wie hem gemeld heeft (motor, versnellingsbak).
- Ongeveer 9.400 standaardcodes (P, C, B, U) en de fabriekscodes van VAG, Ford, GM, Toyota, Honda, Nissan, Hyundai en Kia, Subaru en meer. Een P1-code wordt gelezen zoals het herkende merk hem bedoelt.
- Een **stoplicht** per code en de vraag die iedereen als eerste stelt: mag ik doorrijden?
- De gebruikelijke oorzaken, dingen die je zelf kunt controleren en een grove kostenklasse.
- Waar het onderdeel zit: de auto in doorkijkweergave, of de [röntgenweergave](xray.md).

## Alle regeleenheden

In de expertmodus vraagt de pagina **Alle regeleenheden** elke regeleenheid uit, niet alleen de motor: ABS, airbag, instrumentenpaneel, airco en meer. Elke fout komt met zijn soort storing (kortsluiting, onderbreking, onlogisch signaal), onderdeelnummer en softwareversie. Oudere auto's van de VW-groep worden uitgelezen via een [KKL-kabel](kkl.md).

## Wat moet ik nu doen?

De pagina **Wat te doen?** verdeelt alles wat Caruspex gevonden heeft over drie stappen: meteen stoppen, binnenkort naar de garage, kan wachten. Elk punt heeft één eenvoudige zin waarom, een grove kostenindicatie en een notitie om in de garage te laten zien. OBD kan niet zien of het motorlampje knippert of een rood waarschuwingslampje brandt, daarom vragen twee vakjes het aan jou; een knipperend lampje met misfires verplaatst het punt naar "meteen stoppen".

![Wat moet ik nu doen](screenshot-todo.png)

## Codes wissen

Wissen kan per regeleenheid, met een duidelijke waarschuwing, en Caruspex leest het geheugen direct daarna opnieuw uit.

> Wissen is geen reparatie. Het zet ook de gereedheidsmonitoren terug: zolang de auto niet een aantal volledige rijcycli heeft gereden, zakt hij voor een emissietest, en een code waarvan de oorzaak er nog is, komt na een korte rit terug.

## Exporteren

De lijst kun je als CSV opslaan, en het keuringsrapport als PDF.
