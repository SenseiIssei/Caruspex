# ELM327 via Bluetooth

Piccole spine senza cavo. Ci sono due tipi di Bluetooth, e Caruspex li parla entrambi.

## Bluetooth LE

Gli adattatori più recenti come Vgate iCar Pro BLE, vLinker, OBDLink CX e Veepeak BLE usano il Bluetooth Low Energy. **Non serve l'associazione**: nella pagina **Collega** scegli **Bluetooth**, clicca su **Cerca dispositivi Bluetooth** e scegli l'adattatore dall'elenco. Funziona su Windows, Linux, macOS e Android.

## Bluetooth classico

Gli adattatori più vecchi e molti di quelli economici usano il Bluetooth classico con un profilo seriale.

1. Associa una volta l'adattatore nelle impostazioni Bluetooth del tuo sistema. Il PIN di solito è `1234` o `0000`.
2. In Caruspex scegli **Bluetooth** e seleziona l'adattatore associato; l'elenco indica se un dispositivo è LE, classico o entrambi.

Su Windows e Linux Caruspex si collega direttamente agli adattatori classici associati. In alternativa Windows crea una porta COM per l'adattatore, che funziona come [ELM327 via USB](elm327-usb.md). Su macOS gli adattatori Bluetooth classici funzionano solo se il sistema offre una porta seriale per loro; lì gli adattatori Bluetooth LE sono la scelta migliore.

## Su Android

- Anche qui gli adattatori Bluetooth LE non richiedono l'associazione: scegli **Bluetooth** e avvia la ricerca; l'elenco mostra la potenza del segnale di ogni dispositivo.
- Gli adattatori classici si associano una volta nelle impostazioni di Android, poi compaiono nell'elenco.
- Da Android 12 il sistema chiede una volta l'accesso ai dispositivi nelle vicinanze. Fino ad Android 11 la ricerca trova gli adattatori Bluetooth LE solo se Caruspex può usare la posizione e la posizione è attiva; Caruspex non la legge. Gli adattatori classici associati funzionano anche senza.

## Consigli

- Stacca l'adattatore quando non lo usi: molti di quelli economici tengono sveglia la batteria dell'auto.
- Se l'adattatore non compare, spegni e riaccendi il Bluetooth del computer e assicurati che nessuna app del telefono ci sia collegata nello stesso momento.
- Il Bluetooth è più lento di un cavo. Con un OBDLink, Caruspex chiede più valori insieme, il che recupera quasi tutta la differenza (vedi [OBDLink e STN](obdlink-stn.md)).
