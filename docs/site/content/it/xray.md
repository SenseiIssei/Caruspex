# Vista a raggi X

La **Vista a raggi X** mostra l'auto collegata come un modello di vetro che puoi ruotare. Dentro vedi il motore o i motori elettrici, il cambio, il serbatoio o la batteria ad alta tensione, lo scarico, il radiatore, la batteria a 12 V e la presa OBD, disposti secondo il tipo di carrozzeria e la trazione dell'auto riconosciuta.

![Vista a raggi X](screenshot-xray.png)

## Cosa si accende

- Gli **errori** colorano la loro zona, e la centralina responsabile diventa rossa.
- I **valori live** stanno come etichette sul pezzo a cui appartengono: liquido di raffreddamento sul radiatore, giri motore sul motore, tensione sulla batteria.
- Le **centraline** trovate dalla ricerca stanno dove di solito si trovano, unite dalle loro linee dati (CAN motopropulsore, CAN comfort, CAN diagnostico, K-Line). Mentre Caruspex legge, piccoli pacchetti viaggiano verso la centralina interrogata.
- **Controlla l'auto** porta la telecamera su ogni risultato, uno dopo l'altro.

Ruota con il mouse o con un dito, fai zoom con la rotellina o con due dita; funziona anche la tastiera. Gli strati (carrozzeria, trasmissione, raffreddamento, carburante o alta tensione, impianto elettrico, rete) si possono spegnere uno per uno.

## Quanto è preciso il modello?

La forma viene dalle dimensioni reali della generazione, dove sono note (da Wikidata o dai dati tecnici del produttore), e altrimenti da valori tipici per il tipo di carrozzeria e l'epoca. Non è una copia del modello esatto: i dati di progetto e gli schemi elettrici dei produttori non sono liberamente disponibili. La pagina dice da dove viene la forma.

Se hai un modello 3D della tua auto in glTF (`.glb` o `.gltf`), puoi caricarlo con **Carica il tuo modello 3D**; l'interno resta quello della disposizione.

## Senza 3D

Sui computer senza WebGL la pagina ripiega su una vista laterale piatta con le stesse informazioni.
