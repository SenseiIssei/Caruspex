# ELM327 via wifi

Sommige adapters openen hun eigen wifi-netwerk. De computer maakt daar verbinding mee en praat via TCP met de adapter.

## Verbinden

1. Steek de adapter erin en zet het contact aan. Na een paar seconden verschijnt een wifi-netwerk, vaak met de naam `WiFi_OBDII`, `V-LINK` of de naam van de fabrikant.
2. Verbind je computer met dat netwerk. Het heeft geen internet, dus je systeem waarschuwt misschien dat de verbinding beperkt is: dat is in orde.
3. Kies in Caruspex op de pagina **Verbinden** voor **Wifi**. De meeste adapters gebruiken het adres `192.168.0.10` en poort `35000`; dat staat al ingevuld. Gebruikt de jouwe een ander adres (bijvoorbeeld `192.168.4.1`), typ het dan in als `address:port`.

## Tips

- Zolang de computer in het netwerk van de adapter zit, heeft hij geen internet. Voor Caruspex maakt dat niet uit.
- Wifi-adapters werken op elk systeem zonder drivers.
- Als de verbinding wegvalt, ga dan met de computer dichter bij de auto zitten: de radio van deze stekkers is zwak.
