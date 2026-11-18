# Errori

La pagina **Memoria errori** legge quello che le centraline hanno memorizzato e lo spiega con parole semplici.

![Memoria errori](screenshot-faults.png)

## Cosa vedi

- Codici **memorizzati, in sospeso e permanenti**. Con più centraline, ogni codice mostra chi l'ha segnalato (motore, cambio).
- Circa 1.400 codici standard (P, C, B, U) e i codici costruttore di VAG, Ford, GM, Toyota, Honda, Nissan, Hyundai e Kia, Subaru e altri. Un codice P1 viene letto nel senso che gli dà la marca riconosciuta.
- Un **semaforo** per ogni codice e la domanda che tutti fanno per prima: posso continuare a guidare?
- Le cause tipiche, i controlli che puoi fare da solo e una fascia di costo indicativa.
- Dove si trova il pezzo: l'auto in vista trasparente, oppure la [vista a raggi X](xray.md).

## Tutte le centraline

In modalità esperto la pagina **Tutte le centraline** interroga ogni centralina, non solo il motore: ABS, airbag, quadro strumenti, clima e altre. Ogni errore arriva con il suo tipo di guasto (cortocircuito, circuito interrotto, segnale non plausibile), codice ricambio e versione software. Le auto più vecchie del gruppo VW si leggono con un [cavo KKL](kkl.md).

## Cosa devo fare adesso?

La pagina **Cosa fare?** ordina tutto ciò che Caruspex ha trovato in tre passi: fermati subito, presto in officina, può aspettare. Ogni punto ha una frase semplice sul perché, una fascia di costo indicativa e una nota da mostrare in officina. L'OBD non vede se la spia motore lampeggia o se è accesa una spia rossa, quindi due caselle te lo chiedono; una spia lampeggiante con mancate accensioni sposta il punto su "fermati subito".

![Cosa devo fare adesso](screenshot-todo.png)

## Cancellare i codici

La cancellazione si fa per singola centralina, con un avviso chiaro, e subito dopo Caruspex rilegge la memoria.

> Cancellare non è riparare. Azzera anche i monitor di prontezza: finché l'auto non ha fatto diversi cicli di guida completi, non passa la revisione delle emissioni, e un codice la cui causa c'è ancora torna dopo un breve tragitto.

## Esportazione

L'elenco si può salvare come CSV, e il rapporto di controllo come PDF.
