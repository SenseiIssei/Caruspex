# Privacy

Caruspex lavora sul tuo computer. Non ha account, né cloud, né tracciamento.

## Cosa viene salvato

Tutto resta sul tuo computer:

| Cosa | Quando | Dove |
|---|---|---|
| Impostazioni | sempre | la cartella delle impostazioni del tuo sistema (il pulsante **Apri la cartella di salvataggio** nelle impostazioni la mostra) |
| Garage: VIN, auto, controlli, letture del contachilometri, l'ultimo collegamento riuscito | dopo aver collegato un'auto | accanto alle impostazioni, nella cartella dati di Caruspex |
| Registrazioni e file CSV | solo quando le avvii tu | dove le salvi |
| File di log | solo se lo attivi nelle impostazioni | il percorso che scegli |
| Rapporti di crash | solo se Caruspex si blocca | la cartella `crash-reports` accanto al garage |

Puoi cancellare uno qualsiasi di questi file in qualsiasi momento.

## Cosa viene inviato

All'avvio Caruspex chiede a GitHub se c'è una versione più recente. Non viene inviato niente su di te o sull'auto. Il controllo si può disattivare nelle impostazioni. A parte questo Caruspex non si collega a internet. La comunicazione con un adattatore Wi-Fi, un cavo ENET o un'auto DoIP resta nella tua rete locale.

## Rapporti di crash solo su richiesta

Se Caruspex si blocca, scrive un breve rapporto sul tuo computer: versione, sistema, il punto nel codice e la catena delle chiamate. I VIN, i byte del traffico con l'auto e la tua cartella utente vengono tagliati via. Non viene inviato niente. Caruspex ti mostra il rapporto e offre due pulsanti: **Segnala su GitHub** apre nel browser una segnalazione già compilata che leggi e modifichi prima di pubblicarla, **Non segnalare** la mette da parte.

## Condividere registrazioni senza dati personali

Le registrazioni di auto vere aiutano a rendere Caruspex migliore per più auto. **Condividi registrazione** prepara una copia e ti mostra esattamente cosa resta e cosa sparisce:

- **Rimosso**: le ultime cifre del VIN (ovunque nel file), il nome della porta e l'indirizzo Bluetooth o di rete dell'adattatore, l'ora del giorno.
- **Mai in una registrazione**: la tua posizione e i dati utente.
- **Resta**: le richieste e le risposte, il tipo di adattatore, il protocollo e la data.

Il file resta sul tuo computer finché non lo salvi e lo alleghi tu stesso a una segnalazione su GitHub. Niente viene caricato automaticamente.

## Il programma di prova

Il [modulo del rapporto di prova](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) chiede auto, anno, motore e adattatore. Chiede solo un VIN abbreviato; non pubblicare mai il VIN completo, la tua targa o il tuo nome.
