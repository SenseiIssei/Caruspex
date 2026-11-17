# OBDLink en STN-chips

De OBDLink-adapters (SX, EX, MX+, CX, LX) gebruiken STN-chips van OBD Solutions. Ze begrijpen alles wat een ELM327 begrijpt en meer; Caruspex herkent ze zelf en gebruikt de extra's.

## Wat Caruspex gebruikt

- **Gebundelde aanvragen**: Caruspex vraagt tot 24 live-waarden in één keer op in plaats van 6. Via Bluetooth maakt dat een ronde waarden bijna twee keer zo snel.
- **Meeluisteren**: uitgezonden waarden (bijvoorbeeld bij een Tesla) worden opgenomen met de eigen commando's van de STN, die vanzelf stoppen na een ingesteld aantal berichten.
- **Ford MS-CAN en GM Single-Wire-CAN**: carrosserie-regeleenheden van Ford en GM zitten niet op de normale diagnose-CAN. Met een adapter die de extra transceiver heeft, biedt de pagina **Alle regeleenheden** "Ford: carrosserie-eenheden op MS-CAN" of "GM: carrosserie-eenheden op Single-Wire-CAN" aan.

| Chip | Adapter | MS-CAN | SW-CAN |
|---|---|---|---|
| STN2230 tot STN2232 | OBDLink EX | ja | nee |
| STN1150 tot 1152, 1170, 2120, 2255, 2256 | OBDLink MX+ en andere | ja | ja |
| andere | OBDLink SX, LX, CX | nee | nee |

## Verbinden

OBDLink-adapters verbind je zoals elke ELM327: via USB (de EX en SX), Bluetooth (MX+, LX, CX) of zoals beschreven bij [ELM327 via USB](elm327-usb.md) en [ELM327 via Bluetooth](elm327-bluetooth.md).

## Getest

Een OBDLink met de STN2232-chip is door de adaptertest gekomen: chip, firmware en commando's werden correct herkend. Hij is nog niet op een auto gebruikt, dus gebundelde aanvragen, MS-CAN en SW-CAN zijn op echte hardware nog niet bevestigd.
