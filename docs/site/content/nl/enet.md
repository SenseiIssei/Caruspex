# BMW ENET-kabel

BMW en Mini vanaf de F-serie (vanaf ongeveer 2008) hebben Ethernet op de OBD-aansluiting. Een ENET-kabel loopt van de OBD-aansluiting naar de netwerkpoort van de computer, zonder adapterchip ertussen. Caruspex praat HSFZ met de gateway van de auto (poorten 6811 en 6801) en bereikt elke regeleenheid.

## Verbinden

1. Steek de kabel in de auto en in de netwerkpoort van de computer (of een USB-netwerkadapter). Zet het contact aan.
2. Wacht tot een minuut: zonder DHCP-server geven Windows, macOS en Linux de computer zelf een automatisch adres (169.254.x.x).
3. Kies op de pagina **Verbinden** voor **Netwerkkabel** en laat het veld leeg: Caruspex zoekt de auto in het netwerk. Vindt het zoeken niets, typ dan het adres van de auto in.

## Wat werkt

- Elke regeleenheid via UDS met BMW's adressering, foutcodes in BMW's formaat zoals ISTA ze toont (bijvoorbeeld 480A10).
- Foutgeheugen, identificatie en wissen per regeleenheid, live-waarden via OBD.
- Ook hier leest Caruspex alleen. Coderen en flashen blijven voor de werkplaatsprogramma's.

## Welke kabel?

Een gewone ENET-kabel heeft een weerstand tussen pin 8 en 16 die het Ethernet van de auto inschakelt. Kabels zonder die weerstand blijven donker. Netwerkpoorten met 1 Gbit/s werken, de auto spreekt 100 Mbit/s.

## Opdrachtregel

```
caruspex_cli ecus --enet auto
```
