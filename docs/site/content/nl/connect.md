# Verbindingsassistent en OBD-aansluiting

## Waar zit de aansluiting?

Elke auto die sinds 2001 (benzine) of 2004 (diesel) in Europa is verkocht, en in de VS sinds 1996, heeft een 16-polige OBD-aansluiting binnen handbereik van de bestuurder. Gebruikelijke plekken:

- onder het dashboard, links of rechts van de stuurkolom,
- boven de pedalen,
- in de zijbekleding van de voetenruimte,
- achter het deksel van de zekeringkast,
- in de middenconsole, soms achter een klein klepje.

Voor meer dan 200 autogeneraties en 35 motoren kent Caruspex de precieze plek. De verbindingsassistent markeert die in een plattegrond en in een close-up van de bestuurderskant; voor motoren toont hij een zijaanzicht met de diagnosestekker en de adapterkabel die nodig is.

> De lijst met plekken van de aansluiting is onze eigen verzameling en is niet bij elke auto gecontroleerd. Klopt de plek niet voor jouw auto, laat het ons dan weten met een [testrapport](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml).

## De verbindingsassistent

De assistent opent vanzelf bij de eerste start. Hij vraagt vier dingen, en elke stap kun je overslaan:

1. **Welke adapter?** ELM327 met USB, Bluetooth of wifi, een ENET-kabel of een KKL-kabel. Afbeeldingen helpen om ze uit elkaar te houden.
2. **Welke auto?** Kies hem uit de catalogus, dan laat de assistent zien waar de aansluiting zit.
3. **Contact aan.** De meeste regeleenheden antwoorden alleen als het contact aan staat.
4. **Verbinden.** Caruspex doorzoekt de poorten en probeert de protocollen.

Later vind je de assistent in de instellingen en op de pagina Verbinden.

## Zoeken en onthouden

Bij het automatisch zoeken loopt Caruspex alle seriële poorten langs, vraagt bij elke poort of er een ELM327 is en probeert dan één voor één de protocollen van de auto. Zodra een auto verbonden is geweest, onthoudt Caruspex de weg (adapter, poort, protocol, bitrate) bij zijn VIN en probeert die de volgende keer als eerste, zodat het verbinden sneller gaat.

## Als het niet verbindt

Als geen adapter antwoordt of de auto stil blijft, toont Caruspex een korte checklist voor dat soort verbinding, samen met wat het al heeft uitgevonden: driver geïnstalleerd, Bluetooth gekoppeld, contact aan, juiste bitrate, een kloonadapter die alleen doet alsof hij een nieuwere versie is. De pagina's onder [adapters en interfaces](interfaces.md) leggen elk soort uitgebreid uit.
