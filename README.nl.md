# Caruspex

<!-- languages -->
[English](README.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Français](README.fr.md) · [Português](README.pt.md) · [Italiano](README.it.md) · **Nederlands** · [Polski](README.pl.md) · [Čeština](README.cs.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [Türkçe](README.tr.md) · [العربية](README.ar.md) · [中文](README.zh.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [हिन्दी](README.hi.md) · [Bahasa Indonesia](README.id.md)
<!-- /languages -->

[![Build](https://github.com/SenseiIssei/Caruspex/actions/workflows/build.yml/badge.svg)](https://github.com/SenseiIssei/Caruspex/actions/workflows/build.yml)
![Rust](https://img.shields.io/badge/Rust-2024-orange)
![Tauri 2](https://img.shields.io/badge/Tauri-2-24c8db)
![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)

Open-source voertuigdiagnose voor Windows, Linux en macOS, geschreven in Rust met een Tauri 2-interface.
Eén kabel of dongle, één applicatie: foutcodes lezen en wissen, live sensorgegevens bekijken,
de gereedheid voor de uitlaatgassenkeuring controleren en met VW/Audi-regeleenheden praten op byteniveau via een K-Line-kabel.

De naam komt van de *haruspex*, de Romeinse waarzegger die voortekenen las in ingewanden. Caruspex leest die van de auto.

De applicatie spreekt 18 talen, zowel de interface als elke foutcodetekst: Engels, Duits, Spaans,
Frans, Portugees, Italiaans, Nederlands, Pools, Tsjechisch, Russisch, Oekraïens, Turks, Arabisch (van rechts naar links), Chinees,
Japans, Koreaans, Hindi en Indonesisch. De taal volgt de systeemtaal, kan in de instellingen worden gewijzigd of
per sessie worden afgedwongen met `--language fr` enzovoort. Hoe vertalingen worden gemaakt en gecontroleerd, staat in
[translations/TRANSLATING.md](translations/TRANSLATING.md).

![Overzicht met live meters](docs/screenshot-overview.png)

## Highlights

- **Twee complete diagnosestacks**: standaard OBD-II/EOBD via elke ELM327-adapter, en VW/Audi
  fabrikantdiagnose (KWP1281 en KWP2000) via een ruwe KKL K-Line-kabel.
- **Protocollen van nul af zelf gebouwd**: KWP1281 bevestigt elke afzonderlijke byte met zijn complement,
  iets wat ELM327-firmware niet kan. Caruspex stuurt de K-Line zelf aan, inclusief het 5-baud-wekadres
  dat bit voor bit wordt verzonden door het seriële break-signaal te schakelen, met timing op basis van deadlines, zodat
  de jitter van de planner van het besturingssysteem zich niet over de tien bits ophoopt.
- **Automatische protocolherkenning**: na het wekken laten de twee sleutelbytes van de ECU zien of hij
  KWP1281 of KWP2000 (ISO 14230) spreekt; Caruspex schakelt daarop om.
- **Modulaire Rust-workspace**: één crate per gebied (adapterverbindingen, protocollen, netwerk en CAN, voertuigen, gegevens,
  opnamen, analyse, demo-auto, opdrachtregelprogramma), met daarbovenop de Tauri-applicatie. De kern weet niets
  van de interface, dus de app en het opdrachtregelprogramma draaien op precies dezelfde code.
- **Getest zonder auto**: gesimuleerde ECU's (`crates/caruspex-protocols/tests/common/`) bootsen de
  byte-voor-byte-handshake na, nepgateways vervangen ENET en DoIP, een nep-J2534-driver dekt het PassThru-pad
  en een Python-ELM327-simulator maakt end-to-end-runs van de app mogelijk.
- **Eén set vertalingen**: de kern en de interface delen dezelfde vertaalbestanden in
  `translations/<code>/`, in 18 talen. De kern zoekt zijn teksten op via de Engelse tekst, de interface bouwt
  zijn catalogi uit dezelfde bestanden.
- **Herkent elke auto**: een VIN-decoder met 365 fabrikantcodes en een catalogus van meer dan 2.100
  modelgeneraties sinds 1980 laten zien welke auto is aangesloten en wat ermee werkt.
- **Ingebouwde demo-auto**: een ELM327 in hetzelfde proces met een motor en een versnellingsbak op CAN, zodat alles
  zonder hardware kan worden uitgeprobeerd.
- **Gegenereerde gegevens, niet met de hand getypt**: ongeveer 1.700 foutcodes, veel ervan berekend uit regelmatige codefamilies
  (per cilinder, per bank, per zuurstofsensor) door `tools/gen_dtc.py`.

## Functies

### Kent de auto
- Na het verbinden leest Caruspex het VIN en vertelt het welke auto het is: merk, model, generatie, bouwjaar.
- Een catalogus van meer dan 2.100 modelgeneraties van 94 merken, van 1980 tot nu, inclusief merken die niet meer bestaan.
- Per generatie: welke diagnoseaansluiting, waar die zit, welk protocol, en of OBD-II, een KKL-kabel of alleen het eigen gereedschap van de fabrikant werkt.
- Een VIN-controle voor elk VIN: fabrikant, land, bouwjaar, controlecijfer, modelcode.
- Een garage die elke auto herkent aan zijn VIN en de geschiedenis van de controles en kilometerstanden bijhoudt.

![Voertuigen](docs/screenshot-vehicles.png)

### Gezondheidscheck met één knop
- Leest het foutgeheugen, de gereedheidsmonitoren en de componenttests en bewaakt daarna een halve minuut lang mengsel, temperaturen, zuurstofsensoren en laadspanning.
- Een score van 0 tot 100 waarbij elk verloren punt in gewone woorden wordt uitgelegd, met de volgende stap.
- Klaar voor de uitlaatgassenkeuring? Welke monitor nog openstaat en hoe je rijdt om hem af te ronden.
- Een keuringsrapport als PDF, om af te drukken of mee te nemen naar de garage.

![Gezondheidscheck](docs/screenshot-health.png)

### Foutgeheugen
- Opgeslagen, in afwachting zijnde en permanente codes; bij meerdere regeleenheden toont elke code wie hem heeft gemeld (motor, versnellingsbak).
- Ongeveer 1.400 standaardcodes (P, C, B, U) plus fabrikantcodes voor VAG, Ford, GM, Toyota, Honda, Nissan, Hyundai/Kia, Subaru en meer. Een P1-code wordt gelezen zoals het herkende merk hem bedoelt.
- Een stoplicht per code en de vraag die iedereen als eerste stelt: mag ik doorrijden?
- Gebruikelijke oorzaken, dingen die je zelf kunt controleren en een grove kostenklasse.
- De auto in doorzichtige weergave laat zien waar het betreffende onderdeel zit.
- Wissen met een duidelijke waarschuwing en automatisch opnieuw uitlezen; CSV-export.

![Foutgeheugen](docs/screenshot-faults.png)

### Wat moet ik nu doen?
- Eén pagina die alles wat Caruspex vond in drie stappen sorteert: nu stoppen, binnenkort naar de garage, kan wachten.
- Elk punt met één duidelijke zin waarom, een grove kostenindicatie en een briefje om in de garage te laten zien (code, regeleenheid, gebied, gebruikelijke oorzaken).
- OBD kan niet zien of de motorstoringslamp knippert of dat er een rode waarschuwingslamp brandt: twee vinkjes vragen ernaar, en een knipperende lamp met misfires zet het punt op "nu stoppen".

![Wat moet ik nu doen](docs/screenshot-todo.png)

### Tweedehandsauto controleren
- Voor de bezichtiging voor je koopt: komt het VIN in de auto overeen met de papieren en de catalogus, klopt de kilometerstand tussen de regeleenheden, zijn de fouten kort daarvoor gewist (afstand en tijd sinds het wissen)?
- Gereedheidsmonitoren, opgeslagen fouten, de gezondheidscheck en bij elektrische auto's de toestand van de accu.
- Een stoplicht, de vragen voor de verkoper en een pdf voor de aankoop.

![Tweedehandsauto controleren](docs/screenshot-used-car.png)

### Röntgenweergave in 3D
- De verbonden auto als glazen model dat je kunt draaien: motor of motoren, versnellingsbak, tank of hoogspanningsaccu, uitlaat, radiateur, de 12 V-accu en de OBD-aansluiting, ingedeeld naar de carrosserie en aandrijving van de herkende auto.
- Elke regeleenheid die de scan vond zit waar hij meestal zit, verbonden met zijn datalijnen (aandrijflijn- en comfort-CAN); terwijl Caruspex leest, reizen kleine pakketjes naar de regeleenheid waaraan wordt gevraagd.
- Fouten laten hun gebied oplichten en kleuren de verantwoordelijke regeleenheid rood, live-waarden staan op het onderdeel waar ze bij horen en "Auto controleren" laat de camera naar elke bevinding vliegen. Lagen kunnen een voor een worden uitgezet.
- Typische afmetingen voor de carrosserie en het tijdperk, geen replica van het exacte model: de ontwerpgegevens en bedradingsschema's van de fabrikanten zijn niet vrij beschikbaar.

![Röntgenweergave](docs/screenshot-xray.png)

### Live-waarden
- Elke mode 01-waarde van SAE J1979 tot en met PID C4: 145 PID's met meer dan 300 uitlezingen, gegroepeerd en doorzoekbaar.
- Kilometerstand, versnelling, olietemperatuur, brandstofverbruik, koppel, turbodruk, uitlaattemperaturen, roetfilter, AdBlue, NOx en hybride accu, voor zover de auto ze doorgeeft.
- Doorlopende grafiek en CSV-opname.

![Live-waarden](docs/screenshot-live.png)

### Meten
- Boordcomputer: verbruik live en gemiddeld, kosten, CO2, afstand en een score voor een rustige rijstijl, ook voor auto's zonder eigen boordcomputer.
- Prestaties: 0 tot 100 km/h, 80 tot 120, kwartmijl en remweg uit het snelheidssignaal (alleen op afgesloten wegen).
- Accu en dynamo: rustspanning, spanningsdip bij het starten, laadspanning met en zonder belasting, begeleid in vier stappen.

### Controle
- Gereedheidsmonitoren in gewone woorden, freeze frame, VIN.
- Componentbewaking in mode 06 met waarde en grenzen, misfire-tellers per cilinder.
- ECU-namen, kalibratie-ID's en checksums uit mode 09, per regeleenheid.
- Ruwe ELM327-terminal.

### Adaptertest
- Echte ELM327 of namaak? Het vraagt de commando's van elke versie op, meet de reactietijd en zegt eerlijk wat daaruit volgt.
- Herkent OBDLink en andere STN-adapters.
- Schrijft nooit iets naar de adapter.

![Adaptertest](docs/screenshot-adapter.png)

### Elke kabel, elke weg naar de auto
Caruspex ontdekt zelf wat is aangesloten en welke taal de auto spreekt.

| Interface | Voorbeelden | Hoe |
|---|---|---|
| ELM327 via USB, Bluetooth, Bluetooth LE, Wi-Fi | goedkope ELM327, Vgate, vLinker, Veepeak | seriële poort, gekoppeld of direct BLE, TCP |
| STN-chips | OBDLink SX, EX, MX+, CX | als ELM327, met snellere timing |
| SAE J2534 PassThru | Tactrix OpenPort 2.0, Mongoose, VCX Nano, Scanmatik, Godiag, VAS 5054A in J2534-modus | het 64-bit-stuurprogramma van de fabrikant, CAN met ISO-TP door Caruspex |
| SLCAN / Lawicel | CANable, USBtin, CANUSB | seriële poort, gevonden door de automatische poortzoektocht |
| SocketCAN (Linux) | PEAK PCAN-USB, Kvaser, CANable met candleLight, Raspberry Pi CAN-hats | can0 en verwanten |
| KKL-kabel (VAG 409.1) | FTDI- of CP2102-kabels | K-Line, KWP1281 en KWP2000 |
| ENET-kabel | BMW en Mini vanaf de F-serie | HSFZ via Ethernet |
| DoIP-kabel | veel auto's vanaf ongeveer 2016 | ISO 13400 via Ethernet |

Op elk daarvan werken dezelfde functies: OBD-II en OBD op UDS, elke regeleenheid via UDS, BMW-
adressering, luisteren naar broadcastwaarden (Tesla), opnemen en afspelen.

![Verbinden](docs/screenshot-connect.png)

### Alle regeleenheden via CAN (UDS)
- Met een gewone ELM327 vraagt Caruspex alle regeleenheden één voor één: motor, versnellingsbak, ABS, airbag, instrumentenpaneel, klimaat en meer, bij de meeste auto's vanaf ongeveer 2008.
- Foutgeheugen per eenheid met het soort storing (kortsluiting, onderbreking, onwaarschijnlijk ...), onderdeelnummer, software- en hardwareversie.
- Adreslijsten voor VAG, BMW en Mini, Ford, Mazda, GM, Toyota, Nissan, Hyundai/Kia, Renault en PSA; voor al het andere een zoektocht over de gebruikelijke adressen.
- BMW en Mini vanaf de F-serie met de eigen uitgebreide adressering van BMW via 6F1, plus een zoektocht over alle 128 diagnoseadressen; foutcodes in het BMW-formaat zoals ISTA ze toont (bijv. 480A10).
- Wissen per eenheid, met een waarschuwing.

![Alle regeleenheden](docs/screenshot-ecus.png)

### VW en Audi via een KKL-kabel (KWP1281 en KWP2000)
- Gasklephuis aanleren: de echte basisinstelling in meetblok 060 of 098.
- Alle regeleenheden scannen, inclusief ABS, airbag, instrumentenpaneel en comfort, met een afdrukbaar rapport.
- Adaptatiekanalen en uitgangstests, bewust alleen-lezen.

### Fabrikantwaarden en eigen waarden
- Accu's van elektrische auto's: conditie, lading, celspanningen met spreiding, temperaturen, voor Hyundai Kona en Ioniq, Kia e-Niro, Renault Zoe, Nissan Leaf en Tesla Model 3 en Y (door te luisteren op de CAN-bus); elke waarde met een gedocumenteerde bron.
- Fabrikantwaarden voor ongeveer 550 extra voertuigen uit het [OBDb](https://github.com/OBDb)-project (accu, bandenspanning, olie, kilometerteller en meer), automatisch aangeboden voor de herkende auto; elke formule wordt gecontroleerd met de eigen decoder van OBDb.
- Laad je eigen waarden in het veelgebruikte Torque-CSV-formaat, met een complete evaluator voor Torque-formules.

![EV-accu](docs/screenshot-battery.png)

### Opnemen en afspelen
- Neem een sessie met de auto op in een bestand en speel hem later zonder auto af, handig bij het zoeken naar fouten.

### Zonder auto of adapter
- Een ingebouwde demo-auto: hij rijdt, warmt op, heeft een paar fouten opgeslagen en praat als een echte ELM327 via CAN.
- Een offline lexicon van alle codes, te filteren op merk.

### Uiterlijk en gevoel
- Een nieuwe interface in de stijl van Odysync: rustig, donker, met soepele overgangen.
- Stap voor stap verbinden met een automatische zoektocht over alle poorten en, als het mislukt, een korte checklist voor dat soort verbinding (stuurprogramma, koppelen, contact, bitsnelheid, kloonadapters) met wat Caruspex al heeft uitgezocht.
- Beginner- en expertmodus: beginners zien de pagina's voor elke dag, de werkplaatstools (KKL, aanpassing, ruwe commando's, adressen) verschijnen in de expertmodus.
- 18 talen; instellingen gelden direct.
- Laat het weten als er een nieuwe versie is: bij het starten vraagt het GitHub om de nieuwste release en verder niets (geen gegevens
  over jou of de auto); kan in de instellingen worden uitgezet.

## Ondersteunde voertuigen

Elk voertuig met OBD-II of EOBD: in Europa benzineauto's vanaf 2001 en diesels vanaf 2004, in de VS vanaf 1996. Oudere VW, Audi, Seat en Skoda vanaf ongeveer 1990 werken via de KKL-kabel. De pagina Voertuigen vertelt welke weg voor een bepaalde auto werkt.

| Tijdperk | Bus | Voorbeelden |
|---|---|---|
| tot ongeveer 1995 | fabrikantspecifiek | VAG met KKL via de 2x2-stekkers, verder alleen werkplaatsgereedschap |
| 1996 tot ongeveer 2003 | K-Line (ISO 9141-2, KWP2000) | Audi A2, VW Golf IV, Toyota, Renault |
| vanaf ongeveer 2004 | CAN (ISO 15765-4) | Audi A4 B7, VW Golf V, moderne Japanse auto's |
| VS tot ongeveer 2005 | SAE J1850 PWM en VPW | Ford, Chevrolet |
| nieuwste auto's | OBD op UDS (SAE J1979-2) | in de VS verplicht vanaf modeljaar 2027, nu al in enkele nieuwere auto's; automatisch herkend en vertaald |
| BMW en Mini vanaf de F-serie | ENET (HSFZ via Ethernet) of D-CAN met 6F1-adressering | elke regeleenheid |
| veel auto's vanaf ongeveer 2016 | DoIP (ISO 13400) met een DoIP-kabel | OBD via Ethernet, automatisch gevonden |
| Tesla Model 3 en Y | geen OBD: broadcast-CAN-frames | accu, cellen, temperaturen, snelheid, kilometerstand door te luisteren (adapterkabel naar de diagnoseaansluiting) |

Zuiver elektrische auto's hoeven geen OBD-II-emissiegegevens te leveren. Sommige antwoorden toch gedeeltelijk; voor auto's zonder enig antwoord maar met verkeer op de bus maakt Caruspex verbinding om te luisteren en leest het wat de ingebouwde pakketten kennen (tot nu toe Tesla Model 3 en Y).

## Ondersteunde adapters

- Elke **ELM327**-compatibele adapter: USB, klassieke Bluetooth (direct via RFCOMM op Windows en Linux zodra gekoppeld, of via zijn COM-poort), **Bluetooth LE**
  (Vgate iCar Pro BLE, vLinker, OBDLink CX, Veepeak BLE en andere adapters met de gebruikelijke GATT-diensten; gevonden door een
  scan, zonder koppelen) of Wi-Fi via TCP
- Een **ENET-kabel** (OBD naar RJ45) voor BMW en Mini vanaf de F-serie: Caruspex vindt de auto op het netwerk
  (HSFZ, poorten 6811 en 6801) en leest elke regeleenheid via Ethernet, zonder adapterchip ertussen. De computer
  heeft een automatisch adres (169.254.x.x) nodig, dat Windows, macOS en Linux zelf toewijzen als geen DHCP antwoordt.
  Ook hier leest Caruspex alleen; coderen en flashen blijven bij E-Sys, ISTA of de tuningtools.
- Elke **KKL-kabel (VAG 409.1)**. Kabels op basis van FTDI of CP2102 zijn veel betrouwbaarder op de K-Line dan de
  goedkoopste namaak; voor KWP1281 wordt een echte FTDI FT232RL aanbevolen.

Eigen interfaces zoals Delphi DS150E, Autocom of VAS 5054A worden niet ondersteund, ze spreken geen ELM327.

| Functie | ELM327 | KKL 409.1 |
|---|---|---|
| Motorfouten lezen en wissen | ja | ja |
| Live-waarden, freeze frame, gereedheid | ja | ja |
| ABS, airbag, instrumentenpaneel, comfort | **nee** | ja |
| Meetblokken van de fabrikant | **nee** | ja |
| **Gasklephuis aanleren** | **nee** | ja |

Dit is een hardwarebeperking, geen softwarebeperking: KWP1281 is een blokprotocol waarin elke byte door
de andere kant wordt bevestigd. De ELM327-firmware verwerkt alleen hele verzoek/antwoord-paren.
Een KKL-kabel voert de K-Line ruw door, zodat Caruspex het protocol zelf kan spreken.

### Twee VAG-protocollen, automatisch herkend

| Protocol | Typisch voor | Basisinstellingen |
|---|---|---|
| KWP1281 (blokprotocol) | 1.4 MPI-motoren | eigen commando (blok 0x28) |
| KWP2000 (ISO 14230) | 1.6 FSI met Bosch MED | eigen diagnosesessie, daarna blok lezen |

Adaptatiekanalen en uitgangstests zijn momenteel geïmplementeerd op het KWP1281-pad; bij KWP2000 meldt de
applicatie dat in plaats van stilzwijgend te falen.

Nog niet geïmplementeerd: fabrikantdiagnose via CAN met VW TP 2.0 (voertuigen vanaf ongeveer 2004).

## Techniek

| Gebied | Technologie |
|---|---|
| Taal | Rust (edition 2024) |
| Interface | Tauri 2 met React 19 en TypeScript, gebouwd met Vite |
| Bibliotheken | `serialport` (seriële poorten), `btleplug` (Bluetooth LE), `libloading` (J2534-drivers) |
| Lokalisatie | Eén set JSON-bestanden in `translations/` voor kern en interface, 18 talen |
| Build | Cargo-workspace, npm voor de interface |
| Tests | `cargo test` tegen gesimuleerde ECU's, nepgateways en een nep-J2534-driver |
| Tooling | Python 3 (code- en tabelgeneratoren, vertaalcontroles, ELM327-simulator) |
| Platforms | Windows 10 en 11, Linux, macOS |

## Downloaden

Kant-en-klare pakketten staan op de [releasepagina](https://github.com/SenseiIssei/Caruspex/releases):

| Systeem | Bestand | Opmerkingen |
|---|---|---|
| Windows 10 en 11, 64-bit | `Caruspex-<version>-windows-x64-setup.exe` | Installeert zonder beheerdersrechten en kan `caruspex_cli` aan het zoekpad toevoegen. Werkt een geïnstalleerde 0.8 ter plekke bij en installeert de WebView2-runtime alleen als die ontbreekt |
| Windows 10 en 11, 64-bit | `Caruspex-<version>-windows-x64.zip` | Draagbaar, uitpakken en starten. Heeft de WebView2-runtime nodig, die Windows 10 en 11 al hebben |
| macOS 13 en nieuwer, Apple Silicon | `Caruspex-<version>-macos.dmg` | Sleep de app naar Programma's |
| Linux, x86_64 | `Caruspex-<version>-linux-x86_64.AppImage` | `chmod +x` en starten |
| Debian en Ubuntu, 64-bit | `Caruspex-<version>-linux-amd64.deb` | Installeren met `sudo apt install ./Caruspex-<version>-linux-amd64.deb` |
| Alleen opdrachtregelprogramma | `caruspex_cli-<version>-windows-x64.zip`, `caruspex_cli-<version>-linux-x86_64.tar.gz`, `caruspex_cli-<version>-macos.tar.gz` | Voor scripts en servers, zonder interface |

De pakketten zijn nog niet ondertekend. Windows SmartScreen vraagt bij de eerste start om bevestiging ("Meer informatie", "Toch uitvoeren"); open de app op macOS
de eerste keer met een rechtsklik en "Open".

## Bouwen

Vereisten: de stabiele Rust-toolchain (edition 2024, Rust 1.85 of nieuwer), Node 24 met npm en Python 3 voor
de generatoren en vertaalcontroles. Op Linux de systeempakketten die de interface en de adapterverbindingen nodig hebben
(namen voor Debian en Ubuntu):

```bash
sudo apt-get install libwebkit2gtk-4.1-dev librsvg2-dev libdbus-1-dev libudev-dev pkg-config
```

Windows heeft de WebView2-runtime nodig (onderdeel van Windows 10 en 11), macOS heeft niets extra's nodig.

Installeer de afhankelijkheden van de interface één keer en bouw dan het opdrachtregelprogramma en de applicatie:

```bash
npm --prefix app ci
```

```bash
cargo build --release -p caruspex-cli
```

```bash
npm --prefix app run tauri -- build
```

`npm --prefix app run tauri -- dev` start de applicatie met live herladen terwijl je eraan werkt. De
interface alleen, zonder het Tauri-venster, draait in een gewone browser met de ingebouwde demo-auto:

```bash
npm --prefix app run dev
```

Open `http://localhost:5287/?demo` om meteen met de demo-auto te verbinden. `?lang=de` kiest een taal en
`?theme=light` het lichte thema, bijvoorbeeld `?demo&lang=de&theme=light`. Zonder de Tauri-shell is er geen
toegang tot adapters, dus de browserversie kent alleen de demo-auto.

### Rechten voor de seriële poort op Linux

Om de adapter zonder `sudo` te gebruiken:

```bash
sudo usermod -aG dialout $USER
```

Meld je daarna af en weer aan.

## Build-uitvoer

| Uitvoer | Beschrijving |
|---|---|
| `target/release/caruspex` / `caruspex.exe` | De applicatie met de Tauri-interface (`tauri build` schrijft ook de installatieprogramma's en bundels naar `target/release/bundle/`) |
| `target/release/caruspex_cli` / `caruspex_cli.exe` | Scriptbaar opdrachtregelprogramma op dezelfde kern, zie [Opdrachtregelprogramma](#opdrachtregelprogramma) |

### Pakketten en releases

`bash tools/package.sh` bouwt de pakketten van het huidige systeem in `dist/`: het installatieprogramma en twee zips op
Windows (het installatieprogramma heeft [Inno Setup 6](https://jrsoftware.org/isinfo.php) nodig), de dmg en het opdrachtregelarchief
op macOS, de AppImage, de `.deb` en het opdrachtregelarchief op Linux. `bash tools/smoke_test.sh` installeert of
opent ze daarna zoals een gebruiker dat zou doen, start beide programma's en verwijdert ze weer. De CI doet beide bij elke run.

Een release komt van een tag. `bash tools/release.sh <version>` bereidt hem voor op een eigen branch zonder iets te pushen:
het zet de versie in `Cargo.toml` en `app/package.json`, maakt `changelog/<version>.md` (Duits en Engels),
voert de controles, tests, het verpakken en de smoke-test uit en ververst de README-schermafbeeldingen (`tools/screenshots.sh de` en
`tools/screenshots.sh en`). Nadat de pull request is samengevoegd, tag je `v<version>` en push je de tag. De CI bouwt alle
drie de systemen en publiceert de release met de pakketten en de lijst met wijzigingen.

De CI (`.github/workflows/build.yml`) voert eerst opmaak, de controle van de gegenereerde tabellen, de vertaalcontrole, de
interfacebuild en clippy uit op Linux, en test, verpakt en smoke-test daarna Windows, Linux en macOS.
Een tweede workflow (`.github/workflows/data-check.yml`) zoekt eens per maand naar nieuwe VIN-codes, voertuigen en DBC-bestanden
upstream en opent een issue met wat nieuw is.

## Tests

```bash
cargo test --workspace
```

Ongeveer 370 tests draaien in enkele seconden, allemaal zonder auto. Ze dekken de ELM327-antwoordparser, de decodering van foutcodes,
PID-conversie, voertuigprofielen en de VIN-decoder, de OBD-II- en UDS-diensten tegen de demo-auto,
volledige KWP1281- en KWP2000-sessies tegen gesimuleerde regeleenheden, ENET (HSFZ) en DoIP tegen nepgateways,
SLCAN en het J2534-pad tegen een nepdriver, opnamen en afspelen, en de analyse (gezondheidscheck, accutest,
boordcomputer, prestatietimer). De tests van het opdrachtregelprogramma draaien de gebouwde `caruspex_cli` en vergelijken de uitvoer met
de golden files in `crates/caruspex-cli/tests/golden/`, die bevatten wat de eerdere C++-versie voor dezelfde
commando's afdrukte, zodat elk verschil met het oude programma zichtbaar wordt.

De controles die de CI naast de tests uitvoert:

```bash
cargo fmt --all --check
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
python tools/i18n_rust.py --strict
```

```bash
npm --prefix app run build
```

De eerste twee zijn opmaak en lints. `tools/i18n_rust.py --strict` faalt als een tekst in de kern ontbreekt in
de vertaalbestanden of als een vertaling ontbreekt in een van de 18 talen; de interfacebuild controleert zijn eigen
teksten op dezelfde manier als `I18N_STRICT=1` is gezet. Voer `python tools/gen_dtc.py`, `gen_vehicles.py`, `gen_ecus.py` en
`gen_pids.py` uit om de gegenereerde tabellen te verversen; de CI faalt als ze afwijken van wat is ingecheckt.

## Uitproberen zonder auto

De snelste manier is de ingebouwde demo-auto: kies "Demo-auto" op de pagina Verbinden, of start met

```bash
target/release/caruspex --demo --language en
```

`caruspex --help` toont elke optie van de applicatie: `--tcp`, `--port`, `--bluetooth`, `--enet`, `--can`,
`--replay`, `--report`, `--language`, `--theme`, `--page`, `--shot` en `--size`.

De Python-simulator speelt ofwel de motor van de Audi A2 op de K-Line, ofwel, met `--can`, een CAN-auto met motor
en versnellingsbak (headers, ISO-TP, mode 06 en 09 per regeleenheid):

```bash
python tools/mock_elm327.py --can
```

Verbind daarna de applicatie (op Windows is het programma `target/release/caruspex.exe`):

```bash
target/release/caruspex --tcp 127.0.0.1:35000 --language en
```

Het opdrachtregelprogramma werkt ook tegen de simulator:

```bash
target/release/caruspex_cli read --tcp 127.0.0.1:35000
```

### Opdrachtregelprogramma

`caruspex_cli <command>` met `--demo`, `--tcp HOST[:PORT]`, `--port NAME [--baud N]`, `--enet HOST|auto`,
`--can SPEC` of `--replay FILE`:

| Commando | Wat het doet |
|---|---|
| `read` | Waarschuwingslampje, gereedheid en foutcodes, met de betekenis voor het merk als het VIN die verraadt |
| `vin` | VIN, wat het zegt (fabrikant, land, bouwjaar, controlecijfer) en de passende auto uit de catalogus |
| `freeze` | Elk freeze frame, één per opgeslagen fout |
| `health` | Gezondheidscheck: fouten, gereedheid, mode 06 en een live reeks (`--seconds N`, standaard 15) |
| `probe` | Adaptertest: wat de ELM327 echt is, welke commando's hij kent, heen-en-terugtijden |
| `uds` | UDS op CAN: identificatie en foutgeheugen van elke regeleenheid (`--ids A,B,...` voor je eigen adressen) |
| `custom` | Fabrikant- en eigen waarden van een ingebouwd pakket (`--pack NAME`) of een Torque-CSV (`--csv FILE`), eenmalig |
| `live`, `info`, `clear`, `ecus`, `ports`, `kklcheck` | Live-waarden, VIN en eerste freeze frame, wissen, alles per regeleenheid, seriële poorten, zelftest van de KKL-kabel |
| `enet`, `interfaces` | Zoekt BMW- en Mini-auto's en DoIP-auto's op het netwerk; toont de CAN-interfaces van deze computer (J2534-drivers, SocketCAN) |
| `version`, `help` | Versie van het programma, de volledige lijst met commando's en opties |

`--json` drukt `read`, `vin`, `health`, `probe`, `freeze`, `uds` en `custom` af als JSON voor scripts, `--record FILE`
slaat de hele sessie met de adapter op voor `--replay`, `--fast-timing` zet de adapter op agressieve adaptieve
timing, `-v` logt elk adaptercommando (naar stderr samen met `--json`). `caruspex_cli help` toont alles.

```bash
target/release/caruspex_cli health --demo --json > health.json
```

De schermafbeeldingen op deze pagina komen van de demo-auto, gemaakt door de app zelf:
`--demo --page faults --shot shot.png --size 1440x920` opent een pagina, slaat na een paar seconden een afbeelding van het venster op
en sluit af (`tools/screenshots.sh` maakt de hele set).

## Projectstructuur

```
crates/caruspex-i18n/       Talen: Duits en Engels in de code, de andere 16 uit translations/
crates/caruspex-vehicles/   VIN-decoder en de voertuigcatalogus (merken, modellen, generaties, aansluitingen, bussen)
crates/caruspex-data/       Foutcodes, PID's, adressen van regeleenheden, koppeling van code aan component
crates/caruspex-link/       Wegen naar de adapter (serieel, TCP, Bluetooth LE), ELM327-driver, adaptertest
crates/caruspex-protocols/  OBD-II, UDS, KWP2000 en KWP1281 (K-Line, byte voor byte)
crates/caruspex-net/        BMW ENET (HSFZ), DoIP, SLCAN, SocketCAN, J2534 PassThru
crates/caruspex-record/     Opnamen en afspelen, eigen waarden (Torque-CSV, PID-pakketten), JSON-schrijver
crates/caruspex-analysis/   Gezondheidscheck, rijcyclusgids, boordcomputer, prestatietimer, accutest
crates/caruspex-demo/       De ingebouwde demo-auto: een ELM327 met gesimuleerde regeleenheden
crates/caruspex-cli/        Het opdrachtregelprogramma, met zijn golden-file-tests
app/src/                    De interface: React 19 en TypeScript (pagina's, shell, state, rapport, i18n)
app/src-tauri/              De Tauri-applicatie: venster, commando's, updatecontrole, opdrachtregelopties
translations/               Gedeelde vertaalbestanden voor kern en interface, 18 talen
data/                       De voertuigcatalogus als met de hand onderhouden CSV, bronnen voor regeleenheden en PID's
tools/                      Generatoren, vertaalcontroles, scripts voor verpakken, release en schermafbeeldingen, ELM327-simulator
docs/                       Schermafbeeldingen van de README
packaging/                  Windows-installatiescript, Linux-desktopbestand, macOS-Info.plist
changelog/                  Releasenotities per versie, Duits en Engels
```

De database met foutcodes wordt gegenereerd: `tools/gen_dtc.py` breidt de regelmatige EOBD-codefamilies en de
samengestelde lijsten in `tools/dtc/` uit tot `crates/caruspex-data/src/generated/*.rs`. Voer het opnieuw uit na het wijzigen van de gegevens:

```bash
python tools/gen_dtc.py
```

De voertuigcatalogus werkt op dezelfde manier: bewerk de CSV-bestanden in `data/vehicles/` en voer dan uit

```bash
python tools/gen_vehicles.py
```

Adressen van regeleenheden en PID-pakketten hebben `tools/gen_ecus.py` en `tools/gen_pids.py`. Vertalingen worden beschreven
in [translations/TRANSLATING.md](translations/TRANSLATING.md); na het toevoegen van een tekst in de code
toont `python tools/i18n_rust.py` wat nog vertaald moet worden.

## Een opmerking over het wissen van codes

OBD mode 04 wist het foutgeheugen, het freeze frame en de gereedheidsmonitoren. Direct daarna
valt de auto door een uitlaatgassenkeuring totdat hij meerdere volledige rijcycli heeft voltooid, en een code waarvan de oorzaak
nog aanwezig is, komt na een korte rit terug. Wissen is geen reparatie.

## Disclaimer

Caruspex is een hobbyproject en wordt zonder garantie geleverd. Gebruik het op eigen risico, alleen op voertuigen
waaraan je mag werken, en nooit tijdens het rijden. Alle product- en merknamen zijn handelsmerken van hun
respectieve eigenaren; Caruspex is niet verbonden aan of goedgekeurd door een autofabrikant.

## Licentie

MIT, zie [LICENSE](LICENSE). De fabrikantwaarden voor honderden modellen zijn omgezet vanuit [OBDb](https://github.com/OBDb) en vallen, net als OBDb, onder CC BY-SA 4.0: ze staan los van de code in [data/obdb](data/obdb) met de licentie en de lijst met bronnen ([ATTRIBUTION.md](data/obdb/ATTRIBUTION.md)). De meegeleverde lettertypen vallen onder de SIL Open Font License 1.1:
JetBrains Mono ([OFL.txt](resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](resources/fonts/OFL-Inter.txt)) en
Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](resources/fonts/OFL-PlusJakartaSans.txt)).
