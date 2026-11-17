# K+DCAN-kabel

De K+DCAN-kabel is de gebruikelijke kabel voor BMW van ongeveer 1998 tot 2017 met de werkplaatsprogramma's INPA en ISTA. Erin zit een seriële FTDI-chip en een kleine omzetter voor de D-CAN van BMW. Veel van deze kabels hebben een schakelaar of een brug tussen pin 7 en 8 voor oudere auto's.

## Stand in Caruspex

> Eerlijke stand: Caruspex spreekt de D-CAN-modus van deze kabel nog niet. De oude BMW-protocollen (DS2 en KWP2000 in BMW's eigen stijl) via de K-Line zijn gepland, maar moeten eerst op een echte auto getest worden.

Wat je vandaag in plaats daarvan kunt gebruiken:

- **BMW en Mini vanaf de F-serie (vanaf ongeveer 2008)**: een [ENET-kabel](enet.md) bereikt elke regeleenheid via Ethernet, of een gewone [ELM327](elm327-usb.md) met BMW's eigen adressering via CAN.
- **Standaard-OBD bij BMW met CAN (vanaf ongeveer 2007)**: elke ELM327 leest motorfouten en live-waarden.
- **In K-Line-modus** geeft de kabel de K-Line door zoals een [KKL-kabel](kkl.md). Dat is handig voor auto's van de VW-groep; BMW-regeleenheden op de K-Line hebben de protocollen nodig die nog ontbreken.

## Hulp gezocht

Heb je een K+DCAN-kabel en een oudere BMW, dan helpt een [testrapport](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) met een opname om dit deel te bouwen.
