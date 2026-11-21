# Live-waarden

De pagina **Live-waarden** (expertmodus) toont wat de sensoren op dit moment meten. In de beginnersmodus toont de **Rijmodus** de belangrijkste waarden in grote letters.

![Live-waarden](screenshot-live.png)

## Wat er is

- Elke mode 01-waarde van SAE J1979 tot en met PID C4: 145 PID's met meer dan 300 meetwaarden, gegroepeerd en doorzoekbaar. Welke jouw auto aanbiedt, zegt hij zelf.
- Kilometerstand, versnelling, olietemperatuur, brandstofverbruik, koppel, turbodruk, uitlaatgastemperaturen, roetfilter, AdBlue, NOx en hybride-accu, voor zover de auto ze meldt.
- Een doorlopende grafiek voor meerdere waarden tegelijk. Over lange vensters (tot een uur) dunt de grafiek oude punten uit, maar houdt elke piek en elk dal.
- Opnemen naar CSV.

## Fabrieks- en eigen waarden

- **Accu's van elektrische auto's**: gezondheidstoestand, lading, celspanningen met spreiding en temperaturen voor Hyundai Kona en Ioniq, Kia e-Niro, Renault Zoe, Nissan Leaf en Tesla Model 3 en Y (door mee te luisteren op de CAN-bus), op de pagina **EV-accu**.
- **OBDb-waarden** voor ongeveer 550 extra voertuigen (bandenspanning, olie, kilometerstand en meer), vanzelf aangeboden voor de herkende auto.
- **Uitgebreide diagnosesessie**: sommige waarden van diesels van de Volkswagen-groep, zoals de gemeten roetmassa en de afstand sinds de laatste regeneratie, geeft de motorregeleenheid alleen in haar uitgebreide diagnosesessie. In de expertmodus zet Caruspex de motor in die sessie zolang zulke waarden worden opgevraagd, alleen om te lezen, markeert ze met **Sessie 03** en zet hem daarna terug. Uitzetten kan in de instellingen onder **Fabriekswaarden**. Weigert de regeleenheid, bijvoorbeeld tijdens het rijden, dan staat de reden bij de waarde.
- **Je eigen waarden** in het veelgebruikte CSV-formaat van Torque, inclusief formules.

![EV-accu](screenshot-battery.png)

## Hoe snel?

Hoeveel waarden er per seconde binnenkomen, hangt af van de auto en de adapter. CAN-auto's antwoorden snel; auto's op de K-Line zijn van nature traag. Met een OBDLink kan Caruspex tot 24 waarden in één keer opvragen (zie [OBDLink en STN](obdlink-stn.md)); een ruwe CAN-interface is de snelste weg.

## Opnemen en afspelen

Een hele sessie met de auto kun je in een bestand opnemen en later zonder de auto afspelen, handig bij het zoeken naar storingen. Met **Opname delen** kun je een opname zonder persoonlijke gegevens voor het project klaarmaken, zie [privacy](privacy.md).
