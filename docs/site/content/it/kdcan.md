# Cavo K+DCAN

Il cavo K+DCAN è il cavo abituale per le BMW dal 1998 al 2017 circa con i programmi d'officina INPA e ISTA. Dentro c'è un chip seriale FTDI e un piccolo convertitore per il D-CAN di BMW. Molti di questi cavi hanno un interruttore o un ponticello tra i pin 7 e 8 per le auto più vecchie.

## Stato in Caruspex

> Stato reale: Caruspex legge le BMW serie E tramite la K-Line di questo cavo: DS2 sui modelli più vecchi, KWP2000 alla maniera BMW e BMW-FAST sulla generazione E60. Finora è stato provato solo con le auto simulate del programma, non ancora su una BMW vera. Caruspex non parla ancora la modalità D-CAN del cavo.

## In modalità K-Line

- **Interruttore su K-Line** prima di inserire il cavo nella presa; sulla maggior parte dei cavi è la posizione sinistra. Collega i pin 7 e 8, così rispondono anche le centraline sul pin 8 dei modelli più vecchi.
- **Quali auto**: E36 (con la presa rotonda a 20 pin tramite un adattatore), E38, E39, E46, E52, E53, E83, E85 e la generazione E60 (E60, E61, E63, E64, E65, E87, E90) fino al 03/2007. Dopo, e dalla serie F in poi, la presa parla D-CAN o Ethernet.
- **Come**: modalità Pro attiva, pagina *BMW tramite K-Line*, scegliere la porta, *Cerca*. Caruspex interroga ogni indirizzo noto tramite DS2 (9600 baud, parità pari), KWP2000 (10400 baud con inizializzazione rapida) e BMW-FAST (115200 baud) e mostra le centraline trovate con identificazione e memoria guasti. Ci vuole circa mezzo minuto.
- **Cancellare** si può per centralina o ovunque, sempre dopo una domanda. Poi Caruspex rilegge ciò che è rimasto.
- **Codici guasto**: BMW non pubblica testi per i suoi codici, perciò Caruspex li mostra come numero esadecimale.
- **Solo lettura**: qui non ci sono codifiche, adattamenti né test degli attuatori; spettano al software d'officina.
- **Un cavo KKL** funziona allo stesso modo, ma sui modelli più vecchi raggiunge solo le centraline sul pin 7 finché i pin 7 e 8 non sono collegati.

Per provare senza auto, l'elenco delle porte della pagina contiene una E46 (DS2) e una E60 (KWP2000) simulate.

## Cos'altro funziona

- **BMW e Mini dalla serie F (dal 2008 circa)**: un [cavo ENET](enet.md) raggiunge tutte le centraline via Ethernet, oppure un semplice [ELM327](elm327-usb.md) con l'indirizzamento proprio di BMW su CAN.
- **OBD standard su BMW con CAN (dal 2007 circa)**: qualsiasi ELM327 legge gli errori motore e i valori live.

## Cerchiamo aiuto

Se hai un cavo K+DCAN e una BMW più vecchia, un [rapporto di prova](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) con il registro aiuta a confermare la parte K-Line su un'auto vera.
