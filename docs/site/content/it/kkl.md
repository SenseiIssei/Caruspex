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

## Primo test sull'auto

La pagina della modalità esperto **Primo test sull'auto** ti guida nella prima sessione con un nuovo cavo KKL. È pensata per l'Audi A2 e funziona su qualsiasi VW, Audi, Seat o Skoda con K-Line. Legge soltanto: non cancella nulla, nessun test delle uscite, nessuna impostazione di base. La porta si apre solo all'avvio di un test.

1. **Verifica il cavo senza l'auto.** Collega il cavo solo all'USB, scegli la sua porta e premi **Verifica cavo**. Caruspex legge il chip (FTDI, CH340, Prolific, CP210x), con un FTDI il timer di latenza, e misura i bit di un indirizzo a 5 baud sulla linea di break. Senza auto non torna nessun eco, ed è giusto così.
2. **Imposta il timer di latenza a 1 ms** (cavi FTDI su Windows): Gestione dispositivi, Porte (COM e LPT), doppio clic su USB Serial Port, Impostazioni della porta, Avanzate, Latency Timer (msec) a 1, OK, poi scollega e ricollega il cavo una volta. Con i 16 ms predefiniti la risposta ai byte chiave può mancare la sua finestra di 50 ms. Caruspex legge questa impostazione, ma non la cambia mai.
3. **Sull'auto:** quadro acceso, motore spento, il cavo ben inserito nella presa OBD e collegato direttamente al computer (niente hub USB), nessun altro programma di diagnosi aperto. Spunta l'elenco e premi **Avvia il primo test**.
4. Il test sveglia il motore (indirizzo 01) e misura ogni bit dell'indirizzo, il byte di sincronizzazione 0x55, i byte chiave e la risposta a questi. Poi legge l'identificazione, la memoria errori e i blocchi di misura da 001 a 003, e dopo il quadro strumenti 17, ABS 03, airbag 15, comfort 46 e impianto elettrico centrale 09. Una centralina che non risponde forse semplicemente non è montata.
5. **Salva rapporto** scrive un file di testo e un file JSON nella cartella `test-reports` dentro la cartella delle impostazioni di Caruspex. **Apri il rapporto di test su GitHub** compila per te il modulo del rapporto. Il VIN e il numero dell'immobilizzatore vengono accorciati finché non spunti la casella.

Cosa significano i risultati:

| Risultato | Cosa fare |
|---|---|
| Nessun eco | Il cavo non sente se stesso: la spina non è ben inserita, oppure al pin 16 mancano i 12 V (controlla il fusibile della presa di diagnosi). Senza auto è normale. |
| L'eco è alterato | Un altro dispositivo nella presa, l'interruttore di un cavo K+DCAN non su K-Line, oppure un cavo difettoso. |
| L'indirizzo di risveglio ha una temporizzazione troppo irregolare | I bit si scostano di più di 20 ms da 200 ms: niente hub USB, chiudi gli altri programmi, timer di latenza a 1 ms. Caruspex riprova ancora una volta con le chiamate break anticipate. |
| Cavo a posto, ma nessuna risposta dall'auto | Il quadro è acceso? L'indirizzo 01 è il motore. Le auto dal 2005 circa si diagnosticano via CAN. |
| La risposta ai byte chiave è arrivata troppo tardi | Imposta il timer di latenza a 1 ms. |

## Problemi tipici

- **Nessuna risposta dalla centralina**: quadro acceso? Il risveglio a 5 baud ha bisogno di tempi esatti; hub USB e alcune macchine virtuali li rovinano, collega il cavo direttamente.
- **Si interrompe dopo un po'**: i cavi economici perdono byte. Prova un altro cavo prima di sospettare dell'auto.
