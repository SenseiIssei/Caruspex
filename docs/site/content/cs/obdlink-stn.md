# OBDLink a čipy STN

Adaptéry OBDLink (SX, EX, MX+, CX, LX) používají čipy STN od firmy OBD Solutions. Rozumí všemu, čemu ELM327, a ještě víc; Caruspex je sám rozpozná a využije jejich přednosti.

## Co Caruspex využívá

- **Hromadné dotazy**: Caruspex se ptá až na 24 živých hodnot najednou místo 6. Přes Bluetooth je tak jedno kolo hodnot skoro dvakrát rychlejší.
- **Odposlech**: vysílané hodnoty (například u Tesly) se nahrávají vlastními příkazy STN, které se samy zastaví po nastaveném počtu zpráv.
- **Ford MS-CAN a GM Single-Wire-CAN**: řídicí jednotky karoserie u Fordu a GM nesedí na běžné diagnostické sběrnici CAN. S adaptérem, který má přídavný transceiver, nabídne stránka **Všechny řídicí jednotky** volbu "Ford: jednotky karoserie na MS-CAN" nebo "GM: jednotky karoserie na Single-Wire-CAN".

| Čip | Adaptér | MS-CAN | SW-CAN |
|---|---|---|---|
| STN2230 až STN2232 | OBDLink EX | ano | ne |
| STN1150 až 1152, 1170, 2120, 2255, 2256 | OBDLink MX+ a další | ano | ano |
| ostatní | OBDLink SX, LX, CX | ne | ne |

## Připojení

Adaptéry OBDLink se připojují jako každý ELM327: přes USB (EX a SX), Bluetooth (MX+, LX, CX) nebo tak, jak je popsáno v [ELM327 přes USB](elm327-usb.md) a [ELM327 přes Bluetooth](elm327-bluetooth.md).

## Otestováno

OBDLink s čipem STN2232 prošel testem adaptéru: čip, firmware i příkazy byly správně rozpoznány. V autě zatím použitý nebyl, takže hromadné dotazy, MS-CAN a SW-CAN zatím nejsou na skutečném hardwaru potvrzené.
