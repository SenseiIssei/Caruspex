# ELM327 via USB

Una spina per la presa OBD con un cavo verso la porta USB. È il tipo di ELM327 più affidabile: niente associazione, niente radio, niente batterie scariche.

## Driver

Dentro la spina un chip USB trasforma il cavo in una porta seriale (porta COM su Windows). Il driver necessario dipende da quel chip:

| Chip | Driver |
|---|---|
| FTDI FT232 | di solito lo installa Windows da solo, già incluso in Linux e macOS |
| CH340 / CH341 | di solito lo installa Windows da solo, già incluso in Linux e macOS |
| Silicon Labs CP210x | dal sito del produttore se Windows non lo trova |
| Prolific PL2303 | dal sito del produttore; i vecchi cloni spesso funzionano solo con un driver vecchio |

Dopo averlo collegato, Gestione dispositivi di Windows mostra l'adattatore sotto "Porte (COM e LPT)".

## Collegamento

1. Inserisci l'adattatore nell'auto, accendi il quadro, collega il cavo al computer.
2. Nella pagina **Collega** scegli **Porta USB o COM**.
3. Lascia la porta su **Ricerca automatica** oppure sceglila dall'elenco, poi collegati.

La ricerca automatica prova ogni porta seriale alle velocità abituali. Se conosci la velocità del tuo adattatore, puoi impostarla nelle impostazioni sotto **Baud rate dell'adattatore**.

## Linux e macOS

Su Linux l'adattatore compare come `/dev/ttyUSB0` o `/dev/ttyACM0`. Il tuo utente deve far parte del gruppo `dialout` (vedi [Installazione](install.md)). Su macOS compare come `/dev/cu.usbserial-...` o `/dev/cu.wchusbserial...`.

## Problemi tipici

- **Non compare nessuna porta**: manca il driver oppure il cavo è solo di ricarica.
- **L'adattatore risponde, l'auto no**: quadro spento, oppure l'adattatore è un clone che non gestisce il protocollo dell'auto. Il [test dell'adattatore](interfaces.md) ti dice di più.
- **Valori live molto lenti sulle auto più vecchie**: è la K-Line stessa a essere lenta, non è colpa dell'adattatore.
