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
| Testrapporten van de eerste KKL-test | alleen als je er een opslaat | de map `test-reports` in de map met instellingen; het VIN en het nummer van de startonderbreker worden ingekort, tenzij je het vakje aanvinkt |

Je kunt elk van deze bestanden op elk moment verwijderen.

## Wat wordt verstuurd

Bij elke installatie, ook na een update vanaf een oudere versie, vraagt Caruspex één keer of het naar updates moet zoeken. Pas na een ja vraagt het aan GitHub of er een nieuwere versie is, bij het starten en daarna elke zes uur zolang het draait. Daarbij wordt niets over jou of de auto meegestuurd. Zonder ja gaat er geen verzoek naar GitHub. Je kunt dit altijd wijzigen in de instellingen, onder **Venster en taal**. Verder maakt Caruspex geen verbinding met internet. Het contact met een wifi-adapter, een ENET-kabel of een DoIP-auto blijft in je lokale netwerk.

Alleen de op GitHub gepubliceerde releases zoeken naar updates; een kopie die je zelf bouwt doet dat nooit. Zoekt Caruspex naar een update of downloadt het er een, dan ziet GitHub, zoals bij elke download, het IP-adres van de computer en de versie van Caruspex; Caruspex zelf verstuurt en bewaart verder niets, en niemand anders, het project inbegrepen, ziet het. Daarvoor geldt de privacyverklaring van GitHub. Updates komen alleen uit de releases van de Caruspex-repository. Knoppen als **Melden op GitHub** openen alleen je browser; wat je daar plaatst, bepaal je zelf.

## Android

Tot en met Android 11 laat het systeem een app Bluetooth LE-adapters alleen vinden met de locatietoestemming. Caruspex vraagt daar alleen om die reden om en leest je locatie nooit. Vanaf Android 12 vraagt het in plaats daarvan de Bluetooth-toestemming, gemarkeerd als nooit gebruikt voor locatie.

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
