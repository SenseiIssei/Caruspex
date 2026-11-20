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
| Rapporti del primo test KKL | solo quando ne salvi uno | la cartella `test-reports` nella cartella delle impostazioni; il VIN e il numero dell'immobilizzatore vengono accorciati, a meno che tu non spunti la casella |

Puoi cancellare uno qualsiasi di questi file in qualsiasi momento.

## Cosa viene inviato

All'avvio Caruspex chiede a GitHub se c'è una versione più recente. Non viene inviato niente su di te o sull'auto. Il controllo si può disattivare nelle impostazioni. A parte questo Caruspex non si collega a internet. La comunicazione con un adattatore Wi-Fi, un cavo ENET o un'auto DoIP resta nella tua rete locale.

Solo le versioni pubblicate su GitHub cercano aggiornamenti; una copia che compili da te non lo fa mai. Come per ogni richiesta su internet, la ricerca e il download di un aggiornamento mostrano a GitHub il tuo indirizzo IP e la versione di Caruspex, e vale l'informativa sulla privacy di GitHub. Gli aggiornamenti vengono scaricati solo dalle release del repository di Caruspex. Pulsanti come **Segnala su GitHub** aprono solo il browser; cosa pubblichi lì lo decidi tu.

## Android

Fino ad Android 11 il sistema permette a un'app di trovare adattatori Bluetooth LE solo con il permesso di posizione. Caruspex lo chiede solo per questo e non legge mai la tua posizione. Da Android 12 chiede invece il permesso Bluetooth, dichiarato come mai usato per la posizione.

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
