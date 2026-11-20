# Caruspex

Caruspex is een gratis diagnoseprogramma voor auto's en motoren. Het draait op Windows, Linux en macOS, spreekt 18 talen en werkt met de adapters die mensen echt hebben: de goedkope ELM327-stekker, een OBDLink, een KKL-kabel voor oudere auto's van de Volkswagen-groep, een ENET-kabel voor BMW en professionele CAN-interfaces.

![Het overzicht met live meters](screenshot-overview.png)

## Wat het doet

- Leest en wist foutcodes, legt ze in gewone taal uit en zegt of je nog mag doorrijden.
- Toont live-waarden: temperaturen, snelheid, mengsel, turbodruk, accuspanning en nog veel meer, als getallen en als grafieken.
- Vraagt elke regeleenheid van de auto uit, niet alleen de motor, voor zover de auto en de adapter dat toelaten.
- Controleert een occasion voordat je hem koopt: klopt het VIN, komen de kilometerstanden overeen, zijn er kort tevoren fouten gewist?
- Toont de auto als glazen model waarin fouten en waarden oplichten waar ze thuishoren.
- Zegt je wat je nu moet doen: meteen stoppen, binnenkort naar de garage, of het kan wachten.

## Waar begin je

1. [Installeer Caruspex](install.md) op je computer.
2. Volg de [eerste stappen](first-steps.md): probeer de demo-auto en verbind daarna je eigen auto.
3. Zoek je adapter op onder [adapters en interfaces](interfaces.md).
4. Lees onder [privacy](privacy.md) wat Caruspex opslaat en verstuurt. Kort gezegd: er verlaat niets je computer, tenzij je het zelf deelt.

> Caruspex leest vooral. Het schrijft alleen nadat het je dat gevraagd heeft: het wissen van foutcodes en, via de VAG-KKL-kabel, de actuatortest en de basisinstelling van de gasklep. Het codeert nooit regeleenheden, verandert nooit de kilometerstand en komt nooit aan de startonderbreker.

## Eerlijke stand

Caruspex is gebouwd volgens de specificaties en getest tegen gesimuleerde regeleenheden, nagebootste gateways en nagebootste drivers. Op echte hardware is tot nu toe alleen een OBDLink met de STN2232-chip gecontroleerd, zonder auto. Probeer je Caruspex op je auto, stuur dan alsjeblieft een [testrapport](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml): elk rapport maakt de lijst met geteste auto's langer.
