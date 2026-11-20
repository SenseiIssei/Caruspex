# Sicurezza e aggiornamenti

Questa pagina raccoglie le informazioni di sicurezza di Caruspex: a cosa serve il programma, quali rischi conoscere,
come funzionano gli aggiornamenti e come rimuoverlo. Segue le informazioni per gli utenti richieste dal regolamento
europeo sulla ciberresilienza (Cyber Resilience Act, allegato II), per quanto valgono per un programma libero e non
commerciale.

## Il programma

- **Nome**: Caruspex, l'app `caruspex` e lo strumento a riga di comando `caruspex_cli`, per Windows, Linux e macOS.
  La versione è nel nome di ogni download e nell'elenco delle modifiche di ogni versione.
- **Pubblicato da**: il progetto Caruspex su [GitHub](https://github.com/SenseiIssei/Caruspex), come software libero
  con licenza MIT. Domande ed errori vanno nelle [issue](https://github.com/SenseiIssei/Caruspex/issues).
- **Scopo**: leggere e spiegare i dati di diagnosi di auto, moto e camion con il proprio adattatore, sul proprio
  computer. Nessun account, nessun cloud, nessun tracciamento ([Privacy](privacy.md)).
- **Proprietà di sicurezza**: leggere non cambia mai nulla nell'auto, e tutto ciò che fa agire l'auto chiede prima
  conferma. Un aggiornamento viene installato solo se la sua firma corrisponde alla chiave integrata nell'app.
  Caruspex non apre porte di rete e non esegue codice da file o da internet.

## Segnalare una vulnerabilità {#report}

Segnalate i problemi di sicurezza in privato con il
[modulo per un nuovo avviso di sicurezza](https://github.com/SenseiIssei/Caruspex/security/advisories/new), non in una
issue pubblica. La [politica di sicurezza](https://github.com/SenseiIssei/Caruspex/blob/main/SECURITY.md) spiega cosa
succede dopo. Le vulnerabilità corrette vengono pubblicate come
[avvisi di sicurezza](https://github.com/SenseiIssei/Caruspex/security/advisories).

## Rischi da conoscere {#risks}

- Scaricate Caruspex solo dalla [pagina delle versioni](https://github.com/SenseiIssei/Caruspex/releases). Le copie
  prese altrove possono essere modificate.
- Aprite registrazioni, pacchetti PID e modelli 3D solo se vengono da persone di fiducia. Non possono eseguire
  codice, ma un file rovinato può rallentare o chiudere Caruspex.
- Cancellate i codici guasto, avviate i test degli attuatori e le impostazioni di base solo ad auto ferma e in un
  luogo sicuro.
- Molti adattatori Bluetooth e Wi-Fi non hanno PIN o hanno una rete aperta: chiunque nelle vicinanze può collegarsi.
  Scollegate l'adattatore quando avete finito.
- Cavi ENET, DoIP e adattatori Wi-Fi non cifrano nulla: usateli direttamente o solo nella vostra rete.
- I driver J2534 sono programmi dei loro produttori e girano con i vostri permessi. Installateli solo dal produttore.

## Aggiornamenti e supporto {#support}

- A ogni installazione, anche dopo l'aggiornamento da una versione più vecchia, Caruspex chiede una volta "Cercare aggiornamenti?". Solo dopo un sì chiede a GitHub se esiste una versione più recente, all'avvio e ogni sei ore finché è in esecuzione. Una nuova versione si limita ad accendere un pulsante nella barra del titolo e non interrompe mai. L'installazione è un clic, e l'app installa solo un pacchetto con la firma corretta. L'AppImage e l'app per macOS si aggiornano da sole; il pacchetto .deb e l'app Android ricevono la pagina di download.
- La ricerca e l'installazione automatica sono interruttori nelle impostazioni, sotto **Finestra e lingua**, e si possono cambiare in qualsiasi momento. Senza un sì nessuna richiesta va a GitHub; in quel caso guardate ogni tanto la pagina delle versioni.
- Le correzioni di sicurezza vanno solo nella versione più recente. Le versioni vecchie non ricevono correzioni a
  parte: tenete Caruspex aggiornato.
- Caruspex è curato da una sola persona nel tempo libero, per quanto il tempo lo permette e senza alcuna promessa: nessun periodo di supporto, nessun tempo di risposta. Se la manutenzione finisce, il repository sarà archiviato e lo dirà.

## Verificare un download {#verify}

Ogni versione contiene un file `SHA256SUMS` con il checksum di ogni file e la prova firmata da GitHub di come i file
sono stati costruiti. Con i file nella stessa cartella:

```
sha256sum --ignore-missing -c SHA256SUMS
gh attestation verify Caruspex-<version>-windows-x64-setup.exe --repo SenseiIssei/Caruspex
```

Senza questi strumenti, `Get-FileHash <file>` in PowerShell (Windows) o `shasum -a 256 <file>` (macOS) mostra il
checksum da confrontare con la riga di `SHA256SUMS`.

## Rimuovere Caruspex e i propri dati {#remove}

1. Disinstallate il programma: su Windows in Impostazioni, App; su macOS trascinate Caruspex fuori da Applicazioni;
   su Linux cancellate l'AppImage oppure usate `sudo apt remove caruspex`.
2. Cancellate i dati: il pulsante **Apri la cartella di salvataggio** nelle impostazioni mostra le cartelle con le
   impostazioni, il garage e i rapporti di arresto anomalo. Cancellatele. Registrazioni e rapporti salvati da voi
   restano dove li avete salvati.

## Per sviluppatori e aziende {#integrators}

- Ogni versione contiene, per piattaforma, una distinta dei componenti software
  `Caruspex-<version>-sbom-<platform>.cdx.json` (CycloneDX) con ogni libreria e la sua versione.
- La valutazione dei rischi, l'elenco dei requisiti e l'ambito del Cyber Resilience Act per Caruspex sono in
  [docs/cra](https://github.com/SenseiIssei/Caruspex/tree/main/docs/cra).
- Caruspex non è immesso sul mercato come prodotto ai sensi del Cyber Resilience Act, quindi non ci sono
  dichiarazione UE di conformità né marcatura CE.
