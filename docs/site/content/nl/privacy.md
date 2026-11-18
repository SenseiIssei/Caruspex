# Privacy

Caruspex werkt op je computer. Het heeft geen account, geen cloud en geen tracking.

## Wat wordt opgeslagen

Alles blijft op je computer:

| Wat | Wanneer | Waar |
|---|---|---|
| Instellingen | altijd | de instellingenmap van je systeem (de knop **Opslaglocatie openen** in de instellingen laat die zien) |
| Garage: VIN, auto, controles, kilometerstanden, de laatste werkende verbinding | na het verbinden van een auto | naast de instellingen, in de gegevensmap van Caruspex |
| Opnames en CSV-bestanden | alleen als je ze start | waar je ze opslaat |
| Logbestand | alleen als je het in de instellingen aanzet | het pad dat je kiest |
| Crashrapporten | alleen als Caruspex crasht | de map `crash-reports` naast de garage |

Je kunt elk van deze bestanden op elk moment verwijderen.

## Wat wordt verstuurd

Bij het starten vraagt Caruspex aan GitHub of er een nieuwere versie is. Daarbij wordt niets over jou of de auto meegestuurd. De controle kun je in de instellingen uitzetten. Verder maakt Caruspex geen verbinding met internet. Het contact met een wifi-adapter, een ENET-kabel of een DoIP-auto blijft in je lokale netwerk.

## Crashrapporten alleen op verzoek

Als Caruspex crasht, schrijft het een kort rapport op je computer: versie, systeem, de plek in de code en de aanroepketen. VIN's, de bytes van het verkeer met de auto en je thuismap worden eruit geknipt. Er wordt niets verstuurd. Caruspex laat je het rapport zien en biedt twee knoppen: **Melden op GitHub** opent een vooraf ingevulde issue in je browser die je leest en aanpast voordat je hem plaatst, **Niet melden** legt het weg.

## Opnames delen zonder persoonlijke gegevens

Opnames van echte auto's helpen om Caruspex beter te maken voor meer auto's. **Opname delen** maakt een kopie en laat je precies zien wat blijft en wat verdwijnt:

- **Verwijderd**: de laatste cijfers van het VIN (overal in het bestand), de poortnaam en het Bluetooth- of netwerkadres van de adapter, het tijdstip.
- **Nooit in een opname**: je locatie en gebruikersgegevens.
- **Blijft**: de aanvragen en antwoorden, het adaptertype, het protocol en de datum.

Het bestand blijft op je computer tot je het opslaat en zelf aan een GitHub-issue toevoegt. Er wordt niets automatisch geüpload.

## Het testprogramma

Het [formulier voor testrapporten](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) vraagt naar de auto, het bouwjaar, de motor en de adapter. Het vraagt alleen om een ingekort VIN; plaats nooit het volledige VIN, je kenteken of je naam.
