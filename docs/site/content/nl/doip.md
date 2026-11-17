# DoIP (diagnose via IP)

Veel auto's vanaf ongeveer 2016 kun je via Ethernet uitlezen volgens ISO 13400, DoIP genoemd. Een DoIP-kabel verbindt de OBD-aansluiting met de netwerkpoort van de computer.

## Verbinden

1. Steek de kabel erin, zet het contact aan en wacht tot de computer zijn automatische adres (169.254.x.x) heeft, wat tot een minuut kan duren.
2. Kies op de pagina **Verbinden** voor **Netwerkkabel** en laat het veld leeg. Caruspex zoekt DoIP-voertuigen in het netwerk en herkent zelf of de auto DoIP of BMW's HSFZ spreekt.

## Wat werkt

OBD via DoIP, en de regeleenheden via UDS waarvan Caruspex de adressen voor het merk kent. De adreslijsten voor DoIP-auto's zijn nog kort; opnames van echte auto's vullen ze aan (zie [privacy](privacy.md) voor wat een gedeelde opname bevat).

## Welke kabel?

DoIP heeft de Ethernet-lijnen op pin 3, 11, 12 en 13 van de OBD-aansluiting nodig en een activeringslijn op pin 8. Kabels die voor een bepaald merk verkocht worden, zijn meestal precies zo bedraad; de BMW ENET-kabel is er een van.

## Opdrachtregel

```
caruspex_cli enet
```

toont de BMW- en DoIP-voertuigen die in het netwerk gevonden zijn.
