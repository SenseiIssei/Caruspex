# Röntgenweergave

De **Röntgenweergave** toont de verbonden auto als glazen model dat je kunt draaien. Binnenin zie je motor of elektromotoren, versnellingsbak, tank of hoogvoltaccu, uitlaat, radiateur, de 12V-accu en de OBD-aansluiting, ingedeeld naar het carrosserietype en de aandrijving van de herkende auto.

![Röntgenweergave](screenshot-xray.png)

## Wat er oplicht

- **Fouten** kleuren hun gebied, en de verantwoordelijke regeleenheid wordt rood.
- **Live-waarden** staan als labels op het onderdeel waar ze bij horen: koelvloeistof bij de radiateur, toerental bij de motor, spanning bij de accu.
- **Regeleenheden** die bij het zoeken gevonden zijn, zitten waar ze meestal zitten, verbonden door hun datalijnen (aandrijvings-CAN, comfort-CAN, diagnose-CAN, K-Line). Terwijl Caruspex leest, reizen kleine pakketjes naar de eenheid die wordt uitgevraagd.
- **Auto controleren** laat de camera naar elke bevinding vliegen, de een na de ander.

Draai met de muis of een vinger, zoom met het wieltje of twee vingers; het toetsenbord werkt ook. Lagen (carrosserie, aandrijving, koeling, brandstof of hoogvolt, elektrisch, netwerk) kun je een voor een uitzetten.

## Hoe precies is het model?

De vorm komt van de echte afmetingen van de generatie waar die bekend zijn (uit Wikidata of de technische gegevens van de fabrikant), en anders van typische waarden voor het carrosserietype en de periode. Het is geen kopie van het precieze model: de ontwerpgegevens en bedradingsschema's van de fabrikanten zijn niet vrij beschikbaar. De pagina zegt waar de vorm vandaan komt.

Heb je een 3D-model van je auto als glTF (`.glb` of `.gltf`), dan kun je het laden met **Eigen 3D-model laden**; de binnenkant blijft van de indeling.

## Zonder 3D

Op computers zonder WebGL valt de pagina terug op een plat zijaanzicht met dezelfde informatie.
