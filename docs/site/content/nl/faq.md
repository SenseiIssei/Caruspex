# Veelgestelde vragen

## Kost Caruspex iets?

Nee. Caruspex is vrije software onder de MIT-licentie, zonder reclame, accounts of betaalde extra's.

## Welke adapter moet ik kopen?

Voor de meeste mensen is een USB-ELM327 met een FTDI- of CH340-chip, of een OBDLink (Bluetooth of USB), de beste start. Voor VW, Audi, Seat en Skoda van vóór ongeveer 2004 neem je er een [KKL-kabel](kkl.md) bij; voor BMW en Mini vanaf de F-serie een [ENET-kabel](enet.md). Het [overzicht van de interfaces](interfaces.md) vergelijkt ze.

## Werkt het met mijn auto?

Elke auto met OBD-II of EOBD werkt voor de motor: in Europa benzineauto's vanaf 2001 en diesels vanaf 2004, in de VS vanaf 1996. Oudere auto's van de VW-groep vanaf ongeveer 1990 werken via de KKL-kabel. De pagina **Voertuigen** zegt welke weg werkt voor een bepaalde auto. Motoren vanaf Euro 5 werken met een adapterkabel voor hun diagnosestekker.

## Is het op mijn auto getest?

Eerlijk gezegd: nog niet. Caruspex wordt getest tegen gesimuleerde regeleenheden; op echte hardware is tot nu toe alleen een OBDLink met de STN2232-chip gecontroleerd, zonder auto. Help alsjeblieft mee met een [testrapport](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml).

## Kan Caruspex mijn auto kapotmaken?

Caruspex leest alleen. Het enige wat het schrijft, is het wissen van foutcodes, en het waarschuwt voordat het dat doet. Het codeert nooit, flasht nooit en verandert nooit adaptatiewaarden. Gebruik het niet tijdens het rijden.

## Kan het de kilometerstand of de startonderbreker veranderen of functies vrijcoderen?

Nee, en dat gaat het ook niet doen. Dat ligt juridisch gevoelig en is niet het doel van dit programma.

## Waarom meldt de adapter versie 2.1, maar mislukken sommige dingen?

Veel goedkope adapters melden een nieuwere ELM327-versie dan ze echt zijn. De **Adaptertest** zoekt uit welke commando's de adapter echt kent.

## Waarom zijn de waarden traag bij mijn oudere auto?

Auto's op de K-Line (in Europa ongeveer 2001 tot 2007) antwoorden van nature traag. CAN-auto's zijn veel sneller.

## Stuurt Caruspex mijn gegevens ergens heen?

Nee. Alleen de updatecontrole vraagt GitHub naar de nieuwste versie, en die kun je uitzetten. Zie [privacy](privacy.md).

## Hoe kan ik helpen?

Stuur een testrapport van je auto, deel een opname, meld bugs of verbeter een vertaling: de handleiding is [TRANSLATING.md](https://github.com/SenseiIssei/Caruspex/blob/main/translations/TRANSLATING.md), en de knop **Vertaling verbeteren** in de instellingen opent die.

Code is ook welkom: [CONTRIBUTING.md](https://github.com/SenseiIssei/Caruspex/blob/main/CONTRIBUTING.md) legt (in het Engels) uit hoe je Caruspex bouwt en wat een pull request nodig heeft.
