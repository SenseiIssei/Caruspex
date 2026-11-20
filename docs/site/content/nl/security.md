# Veiligheid en updates

Deze pagina bevat de veiligheidsinformatie over Caruspex: waarvoor het programma bedoeld is, welke risico's je moet
kennen, hoe updates werken en hoe je het weer verwijdert. Ze volgt de informatie voor gebruikers die de Europese
Cyber Resilience Act vraagt (bijlage II), voor zover die geldt voor een vrij, niet-commercieel programma.

## Het programma

- **Naam**: Caruspex, de app `caruspex` en het opdrachtregelprogramma `caruspex_cli`, voor Windows, Linux en macOS.
  De versie staat in de naam van elke download en in de wijzigingslijst van elke release.
- **Uitgegeven door**: het Caruspex-project op [GitHub](https://github.com/SenseiIssei/Caruspex), als vrije software
  onder de MIT-licentie. Vragen en fouten horen in de [issues](https://github.com/SenseiIssei/Caruspex/issues).
- **Doel**: diagnosegegevens van auto's, motoren en vrachtwagens lezen en uitleggen via je eigen adapter, op je eigen
  computer. Geen account, geen cloud en geen tracking ([Privacy](privacy.md)).
- **Veiligheidseigenschappen**: lezen verandert nooit iets aan de auto, en alles wat de auto iets laat doen vraagt
  eerst om bevestiging. Een update wordt alleen geïnstalleerd als de handtekening past bij de sleutel in de app.
  Caruspex opent geen netwerkpoort en voert geen code uit bestanden of van internet uit.

## Een kwetsbaarheid melden {#report}

Meld beveiligingsproblemen privé met het
[formulier voor een nieuwe beveiligingsmelding](https://github.com/SenseiIssei/Caruspex/security/advisories/new), niet
in een openbare issue. Het [beveiligingsbeleid](https://github.com/SenseiIssei/Caruspex/blob/main/SECURITY.md) legt
uit wat er daarna gebeurt. Opgeloste kwetsbaarheden worden gepubliceerd als
[beveiligingsadviezen](https://github.com/SenseiIssei/Caruspex/security/advisories).

## Risico's om te kennen {#risks}

- Download Caruspex alleen van de [releasepagina](https://github.com/SenseiIssei/Caruspex/releases). Kopieën van
  elders kunnen veranderd zijn.
- Open opnames, PID-pakketten en 3D-modellen alleen van mensen die je vertrouwt. Ze kunnen geen code uitvoeren, maar
  een kapot bestand kan Caruspex traag maken of sluiten.
- Wis foutcodes, start actuatortests en basisinstellingen alleen als de auto stilstaat, op een veilige plek.
- Veel Bluetooth- en wifi-adapters hebben geen pincode of een open netwerk: iedereen in de buurt kan verbinden.
  Trek de adapter eruit als je klaar bent.
- ENET-kabels, DoIP en wifi-adapters versleutelen niets: gebruik ze direct of alleen in je eigen netwerk.
- J2534-stuurprogramma's zijn programma's van hun makers en draaien met jouw rechten. Installeer ze alleen van de
  maker.

## Updates en ondersteuning {#support}

- Bij het starten vraagt Caruspex aan GitHub of er een nieuwere versie is en toont die. Installeren is één klik, en de
  app installeert alleen een pakket met een kloppende handtekening. De AppImage en de macOS-app werken zichzelf bij;
  het .deb-pakket en de Android-app krijgen de downloadpagina.
- Het zoeken en het automatisch installeren zijn schakelaars in de instellingen, onder **Venster en taal**. Kijk
  zonder het zoeken af en toe op de releasepagina.
- Beveiligingsoplossingen komen alleen in de nieuwste release. Oudere versies krijgen geen aparte oplossingen, dus
  houd Caruspex bijgewerkt.
- Caruspex wordt in de vrije tijd onderhouden en heeft geen vaste einddatum. Stopt het onderhoud, dan kondigen de
  repository en een laatste release dat vooraf aan.

## Een download controleren {#verify}

Elke release heeft een bestand `SHA256SUMS` met de controlesom van elk bestand, en het ondertekende bewijs van GitHub
van hoe de bestanden gebouwd zijn. Met de bestanden in één map:

```
sha256sum --ignore-missing -c SHA256SUMS
gh attestation verify Caruspex-<version>-windows-x64-setup.exe --repo SenseiIssei/Caruspex
```

Zonder deze hulpmiddelen toont `Get-FileHash <bestand>` in PowerShell (Windows) of `shasum -a 256 <bestand>` (macOS)
de controlesom om te vergelijken met de regel in `SHA256SUMS`.

## Caruspex en je gegevens verwijderen {#remove}

1. Verwijder het programma: op Windows via Instellingen, Apps; op macOS sleep je Caruspex uit Programma's; op Linux
   verwijder je de AppImage of gebruik je `sudo apt remove caruspex`.
2. Verwijder je gegevens: de knop **Opslaglocatie openen** in de instellingen toont de mappen met de instellingen,
   de garage en de crashrapporten. Verwijder ze. Opnames en rapporten die je zelf hebt opgeslagen, staan waar je ze
   hebt opgeslagen.

## Voor ontwikkelaars en bedrijven {#integrators}

- Elke release heeft per platform een softwarestuklijst `Caruspex-<version>-sbom-<platform>.cdx.json` (CycloneDX)
  met elke bibliotheek en haar versie.
- De risicobeoordeling, de lijst met eisen en de reikwijdte van de Cyber Resilience Act voor Caruspex staan in
  [docs/cra](https://github.com/SenseiIssei/Caruspex/tree/main/docs/cra).
- Caruspex wordt niet als product in de zin van de Cyber Resilience Act in de handel gebracht, daarom is er geen
  EU-conformiteitsverklaring en geen CE-markering.
