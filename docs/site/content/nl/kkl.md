# KKL-kabel (VAG 409.1)

Een KKL-kabel is een USB-kabel met een seriële chip en een K-Line-driver, zonder ELM327 erin. Caruspex stuurt de K-Line zelf aan, byte voor byte. Dat is wat oudere VW, Audi, Seat en Skoda nodig hebben: hun protocol KWP1281 bevestigt elke afzonderlijke byte, en dat kan geen enkele ELM327.

## Wat werkt

- Elke regeleenheid van VW, Audi, Seat en Skoda van ongeveer 1990 tot 2004: motor, versnellingsbak, ABS, airbag, instrumentenpaneel, comfort en meer.
- KWP1281 en KWP2000 (ISO 14230), automatisch herkend aan de keybytes na het wekken.
- Foutgeheugen per regeleenheid, meetwaardeblokken en de echte basisinstelling van het gasklephuis (meetwaardeblok 060 of 098).
- Adaptatiekanalen zijn met opzet alleen-lezen. De uitgangstest schakelt echte onderdelen en start pas na je bevestiging.

Deze pagina's zitten in de **expertmodus**: **Regeleenheden via KKL**, **Gasklephuis** en **Adaptatie en uitgangstests**. Elk ervan opent de kabel zelf: kies de seriële poort, de regeleenheid, starten.

## Welke kabel?

Kabels met een echte FTDI FT232RL of een CP2102 zijn op de K-Line veel betrouwbaarder dan de goedkoopste klonen. Voor KWP1281 wordt een echte FTDI aanbevolen. De kabel moet de K-Line ruw doorgeven: "HEX-USB"-interfaces met eigen firmware werken niet.

Auto's van vóór ongeveer 1996 hebben vaak geen OBD-aansluiting, maar een 2x2-stekker in de motorruimte of onder het dashboard. Een adapter van 2x2 naar de 16-polige aansluiting is goedkoop.

## De kabel controleren

Het opdrachtregelprogramma test de kabel zonder auto:

```
caruspex_cli kklcheck --port COM3
```

## Eerste test aan de auto

De pagina **Eerste test aan de auto** in de expertmodus begeleidt je door de eerste sessie met een nieuwe KKL-kabel. Ze is afgestemd op de Audi A2 en werkt op elke VW, Audi, Seat of Skoda met K-Line. Er wordt alleen gelezen: niets gewist, geen uitgangstest, geen basisinstelling. De poort wordt pas geopend als een test start.

1. **Controleer de kabel zonder auto.** Sluit de kabel alleen op USB aan, kies zijn poort en druk op **Kabel controleren**. Caruspex leest de chip (FTDI, CH340, Prolific, CP210x), bij FTDI de latentietimer, en meet de bits van een adres op 5 baud op de break-lijn. Zonder auto komt er geen echo terug, zo hoort het.
2. **Zet de latentietimer op 1 ms** (FTDI-kabels onder Windows): Apparaatbeheer, Poorten (COM & LPT), dubbelklik op de USB Serial Port, Poortinstellingen, Geavanceerd, Latency Timer (msec) op 1, OK, haal daarna de kabel er één keer uit en steek hem weer in. Met de standaardwaarde van 16 ms kan het antwoord op de sleutelbytes zijn venster van 50 ms missen. Caruspex leest deze instelling, maar verandert haar nooit.
3. **Aan de auto:** contact aan, motor uit, de kabel stevig in de OBD-aansluiting en direct aan de computer (geen USB-hub), geen ander diagnoseprogramma open. Vink de lijst af en druk op **Eerste test starten**.
4. De test wekt de motor (adres 01) en meet elke bit van het adres, de synchronisatiebyte 0x55, de sleutelbytes en het antwoord daarop. Daarna leest hij de identificatie, het foutgeheugen en de meetblokken 001 tot 003, en vervolgens het instrumentenpaneel 17, ABS 03, airbag 15, comfort 46 en centrale elektronica 09. Een regeleenheid die niet antwoordt, is misschien gewoon niet ingebouwd.
5. **Rapport opslaan** zet een tekstbestand en een JSON-bestand in de map `test-reports` in de instellingenmap van Caruspex. **Testrapport openen op GitHub** vult het formulier voor het testrapport voor je in. Het VIN en het nummer van de wegrijbeveiliging worden ingekort, tenzij je het vakje aanvinkt.

Wat de resultaten betekenen:

| Resultaat | Wat te doen |
|---|---|
| Geen echo | De kabel hoort zichzelf niet: de stekker zit niet goed, of op pin 16 ontbreekt de 12 V (controleer de zekering van de diagnoseaansluiting). Zonder auto is dit normaal. |
| De echo is verminkt | Een ander apparaat in de aansluiting, de schakelaar van een K+DCAN-kabel niet op K-Line, of een defecte kabel. |
| Het wekadres is te onregelmatig getimed | De bits wijken meer dan 20 ms af van 200 ms: geen USB-hub, sluit andere programma's, latentietimer 1 ms. Caruspex probeert het nog één keer met vervroegde break-aanroepen. |
| Kabel in orde, maar geen antwoord van de auto | Staat het contact aan? Adres 01 is de motor. Auto's vanaf ongeveer 2005 worden via CAN gediagnosticeerd. |
| Het antwoord op de sleutelbytes kwam te laat | Zet de latentietimer op 1 ms. |

## Typische problemen

- **Geen antwoord van de regeleenheid**: contact aan? Het wekken op 5 baud vraagt precieze timing; USB-hubs en sommige virtuele machines verpesten die, sluit de kabel direct aan.
- **Valt na een tijdje weg**: goedkope kabels verliezen bytes. Probeer een andere kabel voordat je de auto verdenkt.
