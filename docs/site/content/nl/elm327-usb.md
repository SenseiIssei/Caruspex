# ELM327 via USB

Een stekker voor de OBD-aansluiting met een kabel naar de USB-poort. Het is de betrouwbaarste soort ELM327: niet koppelen, geen radio, geen lege batterijen.

## Driver

In de stekker maakt een USB-chip van de kabel een seriële poort (COM-poort op Windows). Welke driver nodig is, hangt af van die chip:

| Chip | Driver |
|---|---|
| FTDI FT232 | meestal door Windows zelf geïnstalleerd, ingebouwd in Linux en macOS |
| CH340 / CH341 | meestal door Windows zelf geïnstalleerd, ingebouwd in Linux en macOS |
| Silicon Labs CP210x | van de website van de fabrikant als Windows hem niet vindt |
| Prolific PL2303 | van de website van de fabrikant; oude klonen werken vaak alleen met een oude driver |

Na het insteken toont Apparaatbeheer van Windows de adapter onder "Poorten (COM & LPT)".

## Verbinden

1. Steek de adapter in de auto, zet het contact aan en steek de kabel in de computer.
2. Kies op de pagina **Verbinden** voor **USB- of COM-poort**.
3. Laat de poort op **Automatisch zoeken** staan of kies hem uit de lijst, en verbind dan.

Het automatisch zoeken probeert elke seriële poort op de gebruikelijke snelheden. Als je de snelheid van je adapter kent, kun je die in de instellingen instellen onder **Baudrate van de adapter**.

## Linux en macOS

Op Linux verschijnt de adapter als `/dev/ttyUSB0` of `/dev/ttyACM0`. Je gebruiker moet in de groep `dialout` zitten (zie [Installeren](install.md)). Op macOS verschijnt hij als `/dev/cu.usbserial-...` of `/dev/cu.wchusbserial...`.

## Typische problemen

- **Er verschijnt geen poort**: de driver ontbreekt of de kabel is alleen een oplaadkabel.
- **Adapter antwoordt, auto niet**: contact uit, of de adapter is een kloon die het protocol van de auto niet aankan. De [adaptertest](interfaces.md) vertelt je meer.
- **Erg trage live-waarden bij oudere auto's**: de K-Line zelf is traag, dat ligt niet aan de adapter.
