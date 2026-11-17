# OBDLink e chip STN

Gli adattatori OBDLink (SX, EX, MX+, CX, LX) usano i chip STN di OBD Solutions. Capiscono tutto quello che capisce un ELM327 e anche di più; Caruspex li riconosce da solo e sfrutta le funzioni in più.

## Cosa usa Caruspex

- **Richieste raggruppate**: Caruspex chiede fino a 24 valori live in un colpo solo invece di 6. Via Bluetooth questo rende un giro di valori quasi due volte più veloce.
- **Ascolto**: i valori trasmessi (per esempio su una Tesla) vengono registrati con i comandi propri dell'STN, che si fermano da soli dopo un certo numero di messaggi.
- **Ford MS-CAN e GM Single-Wire-CAN**: le centraline della carrozzeria di Ford e GM non stanno sul normale CAN diagnostico. Con un adattatore che ha il transceiver in più, la pagina **Tutte le centraline** offre "Ford: centraline carrozzeria su MS-CAN" o "GM: centraline carrozzeria su Single-Wire-CAN".

| Chip | Adattatore | MS-CAN | SW-CAN |
|---|---|---|---|
| da STN2230 a STN2232 | OBDLink EX | sì | no |
| da STN1150 a 1152, 1170, 2120, 2255, 2256 | OBDLink MX+ e altri | sì | sì |
| altri | OBDLink SX, LX, CX | no | no |

## Collegamento

Gli adattatori OBDLink si collegano come qualsiasi ELM327: via USB (EX e SX), Bluetooth (MX+, LX, CX) o come descritto in [ELM327 via USB](elm327-usb.md) e [ELM327 via Bluetooth](elm327-bluetooth.md).

## Provato

Un OBDLink con il chip STN2232 ha superato il test dell'adattatore: chip, firmware e comandi sono stati riconosciuti correttamente. Non è ancora stato usato su un'auto, quindi richieste raggruppate, MS-CAN e SW-CAN non sono ancora confermati su hardware reale.
