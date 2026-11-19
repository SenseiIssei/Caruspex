# K+DCAN-kabel

De K+DCAN-kabel is de gebruikelijke kabel voor BMW van ongeveer 1998 tot 2017 met de werkplaatsprogramma's INPA en ISTA. Erin zit een seriële FTDI-chip en een kleine omzetter voor de D-CAN van BMW. Veel van deze kabels hebben een schakelaar of een brug tussen pin 7 en 8 voor oudere auto's.

## Stand in Caruspex

> Eerlijke stand: Caruspex leest BMW's uit de E-serie via de K-Line van deze kabel: DS2 bij de oudere modellen, KWP2000 op de manier van BMW en BMW-FAST bij de E60-generatie. Tot nu toe is dat alleen getest met de gesimuleerde auto's in het programma, nog niet aan een echte BMW. De D-CAN-modus van de kabel spreekt Caruspex nog niet.

## In K-Line-modus

- **Schakelaar op K-Line** zetten voordat de kabel in de aansluiting gaat; bij de meeste kabels is dat de linker stand. Die verbindt pin 7 en 8, zodat ook de regeleenheden op pin 8 van de oudere modellen antwoorden.
- **Welke auto's**: E36 (met de ronde 20-polige stekker via een adapter), E38, E39, E46, E52, E53, E83, E85 en de E60-generatie (E60, E61, E63, E64, E65, E87, E90) tot 03/2007. Daarna, en vanaf de F-serie, spreekt de aansluiting D-CAN of Ethernet.
- **Zo gaat het**: profimodus aan, pagina *BMW via K-Line*, poort kiezen, *Doorzoeken*. Caruspex vraagt elk bekend adres via DS2 (9600 baud, even pariteit), KWP2000 (10400 baud met snelle initialisatie) en BMW-FAST (115200 baud) en toont de gevonden regeleenheden met identificatie en foutgeheugen. Dat duurt ongeveer een halve minuut.
- **Wissen** kan per regeleenheid of overal, allebei pas na een vraag. Daarna leest Caruspex terug wat er over is.
- **Foutcodes**: BMW publiceert geen teksten bij zijn codes, daarom toont Caruspex ze als hexadecimaal getal.
- **Alleen lezen**: coderen, aanpassen en actuatortests zijn er hier niet; die horen in de werkplaatssoftware.
- **Een KKL-kabel** werkt net zo, maar bereikt bij de oudere modellen alleen de regeleenheden op pin 7 zolang pin 7 en 8 niet verbonden zijn.

Om het zonder auto te proberen, staan in de poortlijst van de pagina een gesimuleerde E46 (DS2) en E60 (KWP2000).

## Wat er verder kan

- **BMW en Mini vanaf de F-serie (vanaf ongeveer 2008)**: een [ENET-kabel](enet.md) bereikt elke regeleenheid via Ethernet, of een gewone [ELM327](elm327-usb.md) met BMW's eigen adressering via CAN.
- **Standaard-OBD bij BMW met CAN (vanaf ongeveer 2007)**: elke ELM327 leest motorfouten en live-waarden.

## Hulp gezocht

Wie een K+DCAN-kabel en een oudere BMW heeft, helpt met een [testrapport](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) met het logboek om het K-Line-deel aan een echte auto te bevestigen.
