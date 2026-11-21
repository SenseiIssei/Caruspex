# Valori live

La pagina **Valori live** (modalità esperto) mostra cosa misurano i sensori in questo momento. In modalità principiante la **Modalità guida** mostra i valori più importanti a caratteri grandi.

![Valori live](screenshot-live.png)

## Cosa c'è

- Tutti i valori del modo 01 di SAE J1979 fino al PID C4: 145 PID con più di 300 letture, raggruppati e ricercabili. Quali offre la tua auto, lo dice lei stessa.
- Contachilometri, marcia, temperatura dell'olio, consumo di carburante, coppia, pressione turbo, temperature dei gas di scarico, filtro antiparticolato, AdBlue, NOx e batteria ibrida, per quanto l'auto li comunica.
- Un grafico scorrevole per più valori insieme. Su finestre lunghe (fino a un'ora) il grafico sfoltisce i punti vecchi ma mantiene ogni picco e ogni avvallamento.
- Registrazione in CSV.

## Valori del costruttore e valori propri

- **Batterie delle auto elettriche**: stato di salute, carica, tensioni delle celle con dispersione e temperature per Hyundai Kona e Ioniq, Kia e-Niro, Renault Zoe, Nissan Leaf e Tesla Model 3 e Y (ascoltando sul bus CAN), nella pagina **Batteria EV**.
- **Valori OBDb** per circa 550 veicoli in più (pressione degli pneumatici, olio, contachilometri e altro), proposti da soli per l'auto riconosciuta.
- **Sessione diagnostica estesa**: alcuni valori dei diesel del gruppo Volkswagen, come la massa di fuliggine misurata e la distanza dall'ultima rigenerazione, la centralina motore li fornisce solo nella sua sessione diagnostica estesa. In modalità esperto Caruspex porta il motore in questa sessione finché questi valori vengono richiesti, solo per leggerli, li segna con **Sessione 03** e poi lo riporta indietro. Si può disattivare nelle impostazioni, in **Valori del costruttore**. Se la centralina rifiuta, per esempio durante la marcia, il valore mostra il motivo.
- **I tuoi valori** nel diffuso formato CSV di Torque, equazioni comprese.

![Batteria EV](screenshot-battery.png)

## Quanto è veloce?

Quanti valori arrivano al secondo dipende dall'auto e dall'adattatore. Le auto CAN rispondono in fretta; le auto su K-Line sono lente per come sono fatte. Un OBDLink permette a Caruspex di chiedere fino a 24 valori in un colpo solo (vedi [OBDLink e STN](obdlink-stn.md)); un'interfaccia CAN diretta è la strada più veloce.

## Registrazione e riproduzione

Un'intera sessione con l'auto si può registrare in un file e riprodurre più tardi senza auto, comodo per la ricerca guasti. Con **Condividi registrazione** una registrazione può essere preparata per il progetto senza dati personali, vedi [privacy](privacy.md).
