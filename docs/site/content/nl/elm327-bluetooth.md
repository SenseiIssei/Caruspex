# ELM327 via Bluetooth

Kleine stekkers zonder kabel. Er zijn twee soorten Bluetooth, en Caruspex spreekt ze allebei.

## Bluetooth LE

Nieuwere adapters zoals de Vgate iCar Pro BLE, vLinker, OBDLink CX en Veepeak BLE gebruiken Bluetooth Low Energy. Ze hoeven **niet gekoppeld** te worden: kies op de pagina **Verbinden** voor **Bluetooth**, klik op **Zoeken naar Bluetooth-apparaten** en kies de adapter uit de lijst. Dit werkt op Windows, Linux, macOS en Android.

## Klassiek Bluetooth

Oudere en veel goedkope adapters gebruiken klassiek Bluetooth met een serieel profiel.

1. Koppel de adapter één keer in de Bluetooth-instellingen van je systeem. De pincode is meestal `1234` of `0000`.
2. Kies in Caruspex voor **Bluetooth** en kies de gekoppelde adapter; de lijst zegt of een apparaat LE, klassiek of allebei is.

Op Windows en Linux verbindt Caruspex direct met gekoppelde klassieke adapters. Als alternatief maakt Windows een COM-poort voor de adapter aan, die werkt als bij [ELM327 via USB](elm327-usb.md). Op macOS werken klassieke Bluetooth-adapters alleen als het systeem er een seriële poort voor aanbiedt; adapters met Bluetooth LE zijn daar de betere keuze.

## Op Android

- Ook hier hoeven Bluetooth LE-adapters niet gekoppeld te worden: kies **Bluetooth** en zoek; de lijst toont de signaalsterkte van elk apparaat.
- Klassieke adapters koppel je één keer in de Android-instellingen, daarna staan ze in de lijst.
- Vanaf Android 12 vraagt het systeem één keer om toegang tot apparaten in de buurt. Tot en met Android 11 vindt de zoekopdracht Bluetooth LE-adapters alleen als Caruspex de locatie mag gebruiken en de locatie aanstaat; Caruspex leest die niet. Gekoppelde klassieke adapters werken ook zonder.

## Tips

- Trek de adapter eruit als je hem niet gebruikt: veel goedkope exemplaren houden de accu van de auto bezig.
- Als de adapter niet verschijnt, zet Bluetooth op de computer uit en weer aan en zorg dat er niet tegelijk een app op je telefoon mee verbonden is.
- Bluetooth is trager dan een kabel. Met een OBDLink vraagt Caruspex meerdere waarden tegelijk op, wat het meeste goedmaakt (zie [OBDLink en STN](obdlink-stn.md)).
