# KKL-kabel (VAG 409.1)

Een KKL-kabel is een USB-kabel met een seriële chip en een K-Line-driver, zonder ELM327 erin. Caruspex stuurt de K-Line zelf aan, byte voor byte. Dat is wat oudere VW, Audi, Seat en Skoda nodig hebben: hun protocol KWP1281 bevestigt elke afzonderlijke byte, en dat kan geen enkele ELM327.

## Wat werkt

- Elke regeleenheid van VW, Audi, Seat en Skoda van ongeveer 1990 tot 2004: motor, versnellingsbak, ABS, airbag, instrumentenpaneel, comfort en meer.
- KWP1281 en KWP2000 (ISO 14230), automatisch herkend aan de keybytes na het wekken.
- Foutgeheugen per regeleenheid, meetwaardeblokken en de echte basisinstelling van het gasklephuis (meetwaardeblok 060 of 098).
- Adaptatiekanalen en uitgangstests zijn met opzet alleen-lezen.

Deze pagina's zitten in de **expertmodus**: **Regeleenheden via KKL**, **Gasklephuis** en **Adaptatie en uitgangstests**. Elk ervan opent de kabel zelf: kies de seriële poort, de regeleenheid, starten.

## Welke kabel?

Kabels met een echte FTDI FT232RL of een CP2102 zijn op de K-Line veel betrouwbaarder dan de goedkoopste klonen. Voor KWP1281 wordt een echte FTDI aanbevolen. De kabel moet de K-Line ruw doorgeven: "HEX-USB"-interfaces met eigen firmware werken niet.

Auto's van vóór ongeveer 1996 hebben vaak geen OBD-aansluiting, maar een 2x2-stekker in de motorruimte of onder het dashboard. Een adapter van 2x2 naar de 16-polige aansluiting is goedkoop.

## De kabel controleren

Het opdrachtregelprogramma test de kabel zonder auto:

```
caruspex_cli kklcheck --port COM3
```

## Typische problemen

- **Geen antwoord van de regeleenheid**: contact aan? Het wekken op 5 baud vraagt precieze timing; USB-hubs en sommige virtuele machines verpesten die, sluit de kabel direct aan.
- **Valt na een tijdje weg**: goedkope kabels verliezen bytes. Probeer een andere kabel voordat je de auto verdenkt.
