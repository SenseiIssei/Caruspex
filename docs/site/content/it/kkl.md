# Cavo KKL (VAG 409.1)

Un cavo KKL è un cavo USB con un chip seriale e un driver K-Line, senza un ELM327 dentro. Caruspex pilota la K-Line da solo, byte per byte. È proprio quello che serve alle vecchie VW, Audi, Seat e Skoda: il loro protocollo KWP1281 conferma ogni singolo byte, cosa che nessun ELM327 sa fare.

## Cosa funziona

- Tutte le centraline di VW, Audi, Seat e Skoda dal 1990 al 2004 circa: motore, cambio, ABS, airbag, quadro strumenti, comfort e altre.
- KWP1281 e KWP2000 (ISO 14230), riconosciuti automaticamente dai byte chiave dopo il risveglio.
- Memoria errori per ogni centralina, blocchi di misura e la vera impostazione di base del corpo farfallato (blocco di misura 060 o 098).
- I canali di adattamento sono di proposito in sola lettura. Il test delle uscite aziona componenti reali e parte solo dopo la tua conferma.

Queste pagine sono nella **modalità esperto**: **Centraline via KKL**, **Corpo farfallato** e **Adattamento e test delle uscite**. Ognuna apre il cavo da sola: scegli la porta seriale, la centralina, avvia.

## Quale cavo?

I cavi con un FTDI FT232RL originale o un CP2102 sono molto più affidabili sulla K-Line dei cloni più economici. Per KWP1281 si consiglia un FTDI originale. Il cavo deve far passare la K-Line così com'è: le interfacce "HEX-USB" con un firmware proprio non funzionano.

Le auto precedenti al 1996 circa spesso non hanno la presa OBD ma una spina 2x2 nel vano motore o sotto il cruscotto. Un adattatore da 2x2 alla presa a 16 poli costa poco.

## Prova il cavo

Lo strumento a riga di comando prova il cavo senza auto:

```
caruspex_cli kklcheck --port COM3
```

## Problemi tipici

- **Nessuna risposta dalla centralina**: quadro acceso? Il risveglio a 5 baud ha bisogno di tempi esatti; hub USB e alcune macchine virtuali li rovinano, collega il cavo direttamente.
- **Si interrompe dopo un po'**: i cavi economici perdono byte. Prova un altro cavo prima di sospettare dell'auto.
