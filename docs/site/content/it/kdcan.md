# Cavo K+DCAN

Il cavo K+DCAN è il cavo abituale per le BMW dal 1998 al 2017 circa con i programmi d'officina INPA e ISTA. Dentro c'è un chip seriale FTDI e un piccolo convertitore per il D-CAN di BMW. Molti di questi cavi hanno un interruttore o un ponticello tra i pin 7 e 8 per le auto più vecchie.

## Stato in Caruspex

> Stato reale: Caruspex non parla ancora la modalità D-CAN di questo cavo. I vecchi protocolli BMW (DS2 e KWP2000 nello stile proprio di BMW) su K-Line sono previsti, ma prima servono prove su un'auto vera.

Cosa puoi usare oggi al suo posto:

- **BMW e Mini dalla serie F (dal 2008 circa)**: un [cavo ENET](enet.md) raggiunge tutte le centraline via Ethernet, oppure un semplice [ELM327](elm327-usb.md) con l'indirizzamento proprio di BMW su CAN.
- **OBD standard su BMW con CAN (dal 2007 circa)**: qualsiasi ELM327 legge gli errori motore e i valori live.
- **In modalità K-Line** il cavo fa passare la K-Line come un [cavo KKL](kkl.md). È utile per le auto del gruppo VW; le centraline BMW su K-Line hanno bisogno dei protocolli che ancora mancano.

## Cerchiamo aiuto

Se hai un cavo K+DCAN e una BMW più vecchia, un [rapporto di prova](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) con una registrazione aiuta a costruire questa parte.
