# Caruspex

<!-- languages -->
[English](README.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Français](README.fr.md) · [Português](README.pt.md) · **Italiano** · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Čeština](README.cs.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [Türkçe](README.tr.md) · [العربية](README.ar.md) · [中文](README.zh.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [हिन्दी](README.hi.md) · [Bahasa Indonesia](README.id.md)
<!-- /languages -->

[![Build](https://github.com/SenseiIssei/Caruspex/actions/workflows/build.yml/badge.svg)](https://github.com/SenseiIssei/Caruspex/actions/workflows/build.yml)
![Rust](https://img.shields.io/badge/Rust-2024-orange)
![Tauri 2](https://img.shields.io/badge/Tauri-2-24c8db)
![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)

Diagnostica per veicoli open source per Windows, Linux e macOS, scritta in Rust con un'interfaccia Tauri 2.
Un cavo o un adattatore, un'applicazione: leggere e cancellare i codici errore, vedere i dati dei sensori in tempo reale,
controllare la prontezza per le emissioni e dialogare byte per byte con le centraline VW/Audi tramite un cavo K-Line.

Il nome viene dall'*aruspice*, l'indovino romano che leggeva i presagi nelle viscere. Caruspex legge quelle dell'auto.

L'applicazione parla 18 lingue, sia l'interfaccia sia ogni testo dei codici errore: inglese, tedesco, spagnolo,
francese, portoghese, italiano, olandese, polacco, ceco, russo, ucraino, turco, arabo (da destra a sinistra), cinese,
giapponese, coreano, hindi e indonesiano. Segue la lingua del sistema, si può cambiare nelle impostazioni oppure
forzare a ogni avvio con `--language fr` e simili. Come vengono fatte e controllate le traduzioni è descritto in
[translations/TRANSLATING.md](translations/TRANSLATING.md).

![Panoramica con indicatori in tempo reale](docs/screenshot-overview.png)

## In evidenza

- **Due stack diagnostici completi**: OBD-II/EOBD standard con qualsiasi adattatore ELM327 e diagnostica
  del costruttore VW/Audi (KWP1281 e KWP2000) tramite un cavo KKL K-Line diretto.
- **Protocolli implementati da zero**: KWP1281 conferma ogni singolo byte con il suo complemento a uno,
  cosa che il firmware dell'ELM327 non sa fare. Caruspex pilota la K-Line da sé, compreso l'indirizzo di
  risveglio a 5 baud, generato bit per bit commutando il segnale di break della porta seriale con una temporizzazione basata su scadenze, così che
  il jitter dello scheduling del sistema operativo non si accumuli sui dieci bit.
- **Riconoscimento automatico del protocollo**: dopo il risveglio, i due byte chiave della centralina indicano se parla
  KWP1281 o KWP2000 (ISO 14230); Caruspex passa di conseguenza.
- **Workspace Rust modulare**: un crate per ogni area (collegamenti con l'adattatore, protocolli, rete e CAN, veicoli, dati,
  registrazioni, analisi, auto demo, strumento da riga di comando) e sopra l'applicazione Tauri. Il nucleo non sa nulla
  dell'interfaccia, quindi l'applicazione e lo strumento da riga di comando eseguono esattamente lo stesso codice.
- **Testato senza auto**: centraline simulate (`crates/caruspex-protocols/tests/common/`) riproducono lo
  scambio byte per byte, gateway finti fanno le veci di ENET e DoIP, un driver J2534 finto copre il percorso PassThru
  e un simulatore ELM327 in Python permette esecuzioni complete dell'applicazione.
- **Un unico insieme di traduzioni**: il nucleo e l'interfaccia condividono gli stessi file di traduzione in
  `translations/<code>/`, in 18 lingue. Il nucleo cerca i suoi testi tramite il testo inglese, l'interfaccia costruisce
  i suoi cataloghi dagli stessi file.
- **Riconosce ogni auto**: un decodificatore VIN con 365 codici costruttore e un catalogo di oltre 2.100
  generazioni di modelli dal 1980 dicono quale auto è collegata e che cosa funziona con essa.
- **Auto demo integrata**: un ELM327 nello stesso processo con un motore e un cambio su CAN, per provare tutto
  senza hardware.
- **Dati generati, non scritti a mano**: circa 1.700 codici errore, molti dei quali calcolati da famiglie di codici regolari
  (per cilindro, per bancata, per sonda lambda) con `tools/gen_dtc.py`.

## Funzioni

### Conosce l'auto
- Dopo il collegamento, Caruspex legge il VIN e ti dice di che auto si tratta: marca, modello, generazione, anno del modello.
- Un catalogo di oltre 2.100 generazioni di modelli di 94 marchi, dal 1980 a oggi, compresi marchi che non esistono più.
- Per ogni generazione: quale connettore di diagnosi, dove si trova, quale protocollo e se funzionano OBD-II, un cavo KKL o solo gli strumenti del costruttore.
- Un controllo VIN per qualsiasi VIN: costruttore, paese, anno del modello, cifra di controllo, codice del modello.
- Un garage che riconosce ogni auto dal suo VIN e conserva lo storico dei suoi controlli e delle letture del contachilometri.

![Veicoli](docs/screenshot-vehicles.png)

### Controllo della salute con un pulsante
- Legge la memoria errori, i monitor di prontezza e i test dei componenti, poi osserva miscela, temperature, sonde lambda e tensione di carica per mezzo minuto.
- Un punteggio da 0 a 100 in cui ogni punto perso viene spiegato in parole semplici, con il passo successivo.
- Pronta per il test delle emissioni? Quale monitor è ancora aperto e come guidare per completarlo.
- Un rapporto di ispezione in PDF, da stampare o da portare in officina.

![Controllo della salute](docs/screenshot-health.png)

### Memoria errori
- Codici memorizzati, in sospeso e permanenti; con più centraline, ogni codice mostra chi lo ha segnalato (motore, cambio).
- Circa 1.400 codici standard (P, C, B, U) più codici del costruttore per VAG, Ford, GM, Toyota, Honda, Nissan, Hyundai/Kia, Subaru e altri. Un codice P1 viene letto nel modo in cui lo intende la marca rilevata.
- Un semaforo per ogni codice e la domanda che tutti fanno per prima: posso continuare a guidare?
- Cause abituali, cose che puoi controllare da solo e una classe di costo approssimativa.
- L'auto in vista trasparente mostra dove si trova il componente interessato.
- Cancellazione con un avviso chiaro e rilettura automatica; esportazione in CSV.

![Memoria errori](docs/screenshot-faults.png)

### Cosa devo fare adesso?
- Una pagina che ordina tutto ciò che Caruspex ha trovato in tre passi: fermarsi subito, presto in officina, può aspettare.
- Ogni punto con una frase semplice che spiega il perché, una fascia di costo indicativa e una nota da mostrare in officina (codice, centralina, zona, cause abituali).
- L'OBD non può vedere se la spia motore lampeggia o se è accesa una spia rossa: due caselle lo chiedono, e una spia che lampeggia con mancate accensioni sposta il punto su "fermarsi subito".

![Cosa devo fare adesso?](docs/screenshot-todo.png)

### Controlla un'auto usata
- Per la visita prima dell'acquisto: il VIN dell'auto corrisponde ai documenti e al catalogo, i chilometri concordano tra le centraline, gli errori sono stati cancellati poco prima (distanza e tempo dalla cancellazione)?
- Monitor di prontezza, errori memorizzati, il controllo della salute e, sulle auto elettriche, lo stato della batteria.
- Un semaforo, le domande da fare al venditore e un PDF per l'acquisto.

![Controlla un'auto usata](docs/screenshot-used-car.png)

### Vista a raggi X in 3D
- L'auto collegata come un modello di vetro che puoi ruotare: motore o motori, cambio, serbatoio o batteria ad alta tensione, scarico, radiatore, batteria da 12 V e presa OBD, disposti secondo la carrozzeria e la trazione dell'auto riconosciuta.
- Ogni centralina trovata dalla scansione sta dove di solito si trova, collegata dalle sue linee dati (CAN del gruppo propulsore e del comfort); mentre Caruspex legge, piccoli pacchetti viaggiano verso la centralina interrogata.
- Gli errori illuminano la loro zona e colorano di rosso la centralina responsabile, i valori live stanno sul componente a cui appartengono e "Controlla auto" porta la telecamera su ogni risultato. I livelli si possono disattivare uno a uno.
- Dimensioni tipiche per la carrozzeria e l'epoca, non una replica del modello esatto: i dati di progetto e gli schemi elettrici dei costruttori non sono liberamente disponibili.

![Vista a raggi X](docs/screenshot-xray.png)

### Valori live
- Tutti i valori del modo 01 della SAE J1979 fino al PID C4: 145 PID con oltre 300 letture, raggruppate e ricercabili.
- Contachilometri, marcia, temperatura dell'olio, consumo di carburante, coppia, pressione del turbo, temperature di scarico, filtro antiparticolato, AdBlue, NOx e batteria ibrida, nella misura in cui l'auto li comunica.
- Grafico scorrevole e registrazione in CSV.

![Valori live](docs/screenshot-live.png)

### Misurare
- Computer di viaggio: consumo in tempo reale e medio, costo, CO2, distanza e un punteggio di guida dolce, anche per le auto senza un computer di viaggio proprio.
- Prestazioni: da 0 a 100 km/h, da 80 a 120, quarto di miglio e spazio di frenata dal segnale di velocità (solo su strade chiuse).
- Batteria e alternatore: tensione a riposo, calo all'avviamento, tensione di carica con e senza carico, guidato in quattro passi.

### Ispezione
- Monitor di prontezza in parole semplici, fotogramma congelato, VIN.
- Monitoraggio dei componenti del modo 06 con valore e limiti, conteggio delle mancate accensioni per cilindro.
- Nomi delle centraline, ID di calibrazione e checksum dal modo 09, per centralina.
- Terminale ELM327 grezzo.

### Controllo dell'adattatore
- ELM327 originale o clone? Chiede i comandi di ciascuna versione, misura il tempo di risposta e dice onestamente che cosa se ne deduce.
- Riconosce OBDLink e altri adattatori STN.
- Non scrive mai nulla nell'adattatore.

![Controllo dell'adattatore](docs/screenshot-adapter.png)

### Ogni cavo, ogni via d'accesso all'auto
Caruspex scopre da solo che cosa è collegato e che cosa parla l'auto.

| Interfaccia | Esempi | Come |
|---|---|---|
| ELM327 via USB, Bluetooth, Bluetooth LE, Wi-Fi | ELM327 economico, Vgate, vLinker, Veepeak | porta seriale, associato o BLE diretto, TCP |
| Chip STN | OBDLink SX, EX, MX+, CX | come ELM327, con temporizzazione più rapida |
| SAE J2534 PassThru | Tactrix OpenPort 2.0, Mongoose, VCX Nano, Scanmatik, Godiag, VAS 5054A in modalità J2534 | il driver a 64 bit del costruttore, CAN con ISO-TP gestito da Caruspex |
| SLCAN / Lawicel | CANable, USBtin, CANUSB | porta seriale, trovata dalla ricerca automatica delle porte |
| SocketCAN (Linux) | PEAK PCAN-USB, Kvaser, CANable con candleLight, schede CAN per Raspberry Pi | can0 e simili |
| Cavo KKL (VAG 409.1) | cavi FTDI o CP2102 | K-Line, KWP1281 e KWP2000 |
| Cavo ENET | BMW e Mini dalla serie F | HSFZ su Ethernet |
| Cavo DoIP | molte auto dal 2016 circa | ISO 13400 su Ethernet |

Su ognuna funzionano le stesse funzioni: OBD-II e OBD su UDS, tutte le centraline su UDS, indirizzamento
BMW, ascolto dei valori in broadcast (Tesla), registrazione e riproduzione.

![Collega](docs/screenshot-connect.png)

### Tutte le centraline via CAN (UDS)
- Con un normale ELM327, Caruspex interroga una per una tutte le centraline: motore, cambio, ABS, airbag, quadro strumenti, climatizzazione e altre, sulla maggior parte delle auto dal 2008 circa.
- Memoria errori per centralina con il tipo di guasto (cortocircuito, circuito aperto, non plausibile ...), codice del componente, versione software e hardware.
- Elenchi di indirizzi per VAG, BMW e Mini, Ford, Mazda, GM, Toyota, Nissan, Hyundai/Kia, Renault e PSA; per tutto il resto, una scansione degli indirizzi usuali.
- BMW e Mini dalla serie F con l'indirizzamento esteso proprio di BMW su 6F1, più una scansione di tutti i 128 indirizzi di diagnosi; codici errore nel formato BMW come li mostra ISTA (ad es. 480A10).
- Cancellazione per centralina, con un avviso.

![Tutte le centraline](docs/screenshot-ecus.png)

### VW e Audi tramite un cavo KKL (KWP1281 e KWP2000)
- Adattamento del corpo farfallato: la vera regolazione di base nel blocco di misura 060 o 098.
- Scansione di tutte le centraline, compresi ABS, airbag, quadro strumenti e comfort, con un rapporto stampabile.
- Canali di adattamento e test delle uscite, volutamente di sola lettura.

### Valori del costruttore e personalizzati
- Batterie delle auto elettriche: stato di salute, carica, tensioni delle celle con dispersione, temperature, per Hyundai Kona e Ioniq, Kia e-Niro, Renault Zoe, Nissan Leaf e Tesla Model 3 e Y (ascoltando il bus CAN); ogni valore con una fonte documentata.
- Valori del costruttore per circa 550 veicoli in più dal progetto [OBDb](https://github.com/OBDb) (batteria, pressione degli pneumatici, olio, contachilometri e altro), proposti automaticamente per l'auto riconosciuta; ogni equazione è verificata con il decodificatore di OBDb stesso.
- Carica i tuoi valori nel diffuso formato CSV di Torque, con un valutatore completo delle equazioni di Torque.

![Batteria EV](docs/screenshot-battery.png)

### Registrare e riprodurre
- Registra una sessione con l'auto in un file e riproducila più tardi senza l'auto, utile per cercare i guasti.

### Senza auto né adattatore
- Un'auto demo integrata: guida, si scalda, ha qualche errore memorizzato e parla come un vero ELM327 su CAN.
- Un lessico offline di tutti i codici, filtrabile per marca.

### Aspetto e uso
- Una nuova interfaccia nello stile di Odysync: calma, scura, con transizioni fluide.
- Collegamento passo dopo passo con una ricerca automatica su tutte le porte e, se fallisce, una breve lista di controllo per quel tipo di collegamento (driver, associazione, quadro, velocità in bit, adattatori cloni) con ciò che Caruspex ha già scoperto.
- Modalità principiante ed esperto: i principianti vedono le pagine di ogni giorno, gli strumenti da officina (KKL, adattamento, comandi grezzi, indirizzi) compaiono in modalità esperto.
- 18 lingue; le impostazioni si applicano subito.
- Ti avvisa quando esce una nuova versione: all'avvio chiede a GitHub l'ultima versione e nient'altro (nessun dato
  su di te o sull'auto); si può disattivare nelle impostazioni.

## Veicoli supportati

Qualsiasi veicolo con OBD-II o EOBD: in Europa auto a benzina dal 2001 e diesel dal 2004, negli USA dal 1996. VW, Audi, Seat e Skoda più vecchie, dal 1990 circa, funzionano con il cavo KKL. La pagina Veicoli dice quale via funziona per una determinata auto.

| Epoca | Bus | Esempi |
|---|---|---|
| fino al 1995 circa | specifico del costruttore | VAG con KKL tramite i connettori 2x2, altrimenti solo strumenti da officina |
| dal 1996 al 2003 circa | K-Line (ISO 9141-2, KWP2000) | Audi A2, VW Golf IV, Toyota, Renault |
| dal 2004 circa | CAN (ISO 15765-4) | Audi A4 B7, VW Golf V, auto giapponesi moderne |
| USA fino al 2005 circa | SAE J1850 PWM e VPW | Ford, Chevrolet |
| auto più recenti | OBD su UDS (SAE J1979-2) | obbligatorio negli USA dall'anno modello 2027, già presente in alcune auto recenti; riconosciuto e tradotto automaticamente |
| BMW e Mini dalla serie F | ENET (HSFZ su Ethernet) o D-CAN con indirizzamento 6F1 | tutte le centraline |
| molte auto dal 2016 circa | DoIP (ISO 13400) con un cavo DoIP | OBD su Ethernet, trovato automaticamente |
| Tesla Model 3 e Y | niente OBD: frame CAN in broadcast | batteria, celle, temperature, velocità, contachilometri tramite ascolto (cavo adattatore al connettore di diagnosi) |

Le auto puramente elettriche non sono tenute a fornire dati sulle emissioni OBD-II. Alcune rispondono comunque in parte; per le auto che non rispondono ma hanno traffico sul bus, Caruspex si collega in ascolto e legge ciò che conoscono i pacchetti integrati (finora Tesla Model 3 e Y).

## Adattatori supportati

- Qualsiasi adattatore compatibile **ELM327**: USB, Bluetooth classico (direttamente via RFCOMM su Windows e Linux una volta associato, oppure tramite la sua porta COM), **Bluetooth LE**
  (Vgate iCar Pro BLE, vLinker, OBDLink CX, Veepeak BLE e altri adattatori con i consueti servizi GATT; trovati con una
  scansione, senza associazione) o Wi-Fi via TCP
- Un **cavo ENET** (da OBD a RJ45) per BMW e Mini dalla serie F: Caruspex trova l'auto nella rete
  (HSFZ, porte 6811 e 6801) e legge tutte le centraline via Ethernet, senza chip adattatore in mezzo. Il computer
  ha bisogno di un indirizzo automatico (169.254.x.x), che Windows, macOS e Linux assegnano da soli quando nessun DHCP risponde.
  Anche qui Caruspex si limita a leggere; codifica e flash restano a E-Sys, ISTA o agli strumenti di tuning.
- Qualsiasi **cavo KKL (VAG 409.1)**. I cavi basati su FTDI o CP2102 sono molto più affidabili sulla K-Line dei
  cloni più economici; per KWP1281 si consiglia un FTDI FT232RL originale.

Le interfacce proprietarie come Delphi DS150E, Autocom o VAS 5054A non sono supportate, non parlano ELM327.

| Funzione | ELM327 | KKL 409.1 |
|---|---|---|
| Leggere e cancellare gli errori del motore | sì | sì |
| Valori live, fotogramma congelato, prontezza | sì | sì |
| ABS, airbag, quadro strumenti, comfort | **no** | sì |
| Blocchi di misura del costruttore | **no** | sì |
| **Adattamento del corpo farfallato** | **no** | sì |

È un limite dell'hardware, non del software: KWP1281 è un protocollo a blocchi in cui ogni byte viene
confermato dall'altra parte. Il firmware dell'ELM327 gestisce solo coppie complete di richiesta e risposta.
Un cavo KKL fa passare la K-Line grezza, quindi Caruspex può parlare il protocollo da sé.

### Due protocolli VAG, riconosciuti automaticamente

| Protocollo | Tipico di | Impostazioni di base |
|---|---|---|
| KWP1281 (protocollo a blocchi) | motori 1.4 MPI | comando dedicato (blocco 0x28) |
| KWP2000 (ISO 14230) | 1.6 FSI con Bosch MED | sessione di diagnosi dedicata, poi lettura del blocco |

I canali di adattamento e i test delle uscite sono per ora implementati sul percorso KWP1281; su KWP2000 l'
applicazione lo segnala invece di fallire in silenzio.

Non ancora implementata: diagnostica del costruttore su CAN con VW TP 2.0 (veicoli dal 2004 circa).

## Stack tecnologico

| Area | Tecnologia |
|---|---|
| Linguaggio | Rust (edizione 2024) |
| Interfaccia | Tauri 2 con React 19 e TypeScript, compilata con Vite |
| Librerie | `serialport` (porte seriali), `btleplug` (Bluetooth LE), `libloading` (driver J2534) |
| Localizzazione | Un unico insieme di file JSON in `translations/` per nucleo e interfaccia, 18 lingue |
| Compilazione | Workspace Cargo, npm per l'interfaccia |
| Test | `cargo test` contro centraline simulate, gateway finti e un driver J2534 finto |
| Strumenti | Python 3 (generatori di codice e tabelle, controlli delle traduzioni, simulatore ELM327) |
| Piattaforme | Windows 10 e 11, Linux, macOS |

## Download

I pacchetti pronti sono nella [pagina delle release](https://github.com/SenseiIssei/Caruspex/releases):

| Sistema | File | Note |
|---|---|---|
| Windows 10 e 11, 64 bit | `Caruspex-<version>-windows-x64-setup.exe` | Si installa senza diritti di amministratore e può aggiungere `caruspex_cli` al percorso di ricerca. Aggiorna sul posto una 0.8 già installata e installa il runtime WebView2 solo se manca |
| Windows 10 e 11, 64 bit | `Caruspex-<version>-windows-x64.zip` | Portatile, decomprimi e avvia. Richiede il runtime WebView2, che Windows 10 e 11 hanno già |
| macOS 13 e successivi, Apple Silicon | `Caruspex-<version>-macos.dmg` | Trascina l'app in Applicazioni |
| Linux, x86_64 | `Caruspex-<version>-linux-x86_64.AppImage` | `chmod +x` e avvia |
| Debian e Ubuntu, 64 bit | `Caruspex-<version>-linux-amd64.deb` | Installa con `sudo apt install ./Caruspex-<version>-linux-amd64.deb` |
| Solo lo strumento da riga di comando | `caruspex_cli-<version>-windows-x64.zip`, `caruspex_cli-<version>-linux-x86_64.tar.gz`, `caruspex_cli-<version>-macos.tar.gz` | Per script e server, senza interfaccia |

I pacchetti non sono ancora firmati. Windows SmartScreen chiede conferma al primo avvio ("Ulteriori informazioni", "Esegui comunque"); su macOS
apri l'app la prima volta con il clic destro e "Apri".

## Compilazione

Requisiti: la toolchain Rust stabile (edizione 2024, Rust 1.85 o successivo), Node 24 con npm e Python 3 per
i generatori e i controlli delle traduzioni. Su Linux i pacchetti di sistema di cui hanno bisogno l'interfaccia e i collegamenti con l'adattatore
(nomi Debian e Ubuntu):

```bash
sudo apt-get install libwebkit2gtk-4.1-dev librsvg2-dev libdbus-1-dev libudev-dev pkg-config
```

Windows richiede il runtime WebView2 (incluso in Windows 10 e 11), macOS non richiede nulla di più.

Installa una volta le dipendenze dell'interfaccia, poi compila lo strumento da riga di comando e l'applicazione:

```bash
npm --prefix app ci
```

```bash
cargo build --release -p caruspex-cli
```

```bash
npm --prefix app run tauri -- build
```

`npm --prefix app run tauri -- dev` avvia l'applicazione con ricarica dal vivo mentre ci lavori. L'
interfaccia da sola, senza la finestra Tauri, gira in un normale browser con l'auto demo integrata:

```bash
npm --prefix app run dev
```

Apri `http://localhost:5287/?demo` per collegarti subito all'auto demo. `?lang=de` sceglie una lingua e
`?theme=light` il tema chiaro, per esempio `?demo&lang=de&theme=light`. Senza la shell Tauri non c'è
accesso all'adattatore, quindi la versione per browser conosce solo l'auto demo.

### Permessi della porta seriale su Linux

Per accedere all'adattatore senza `sudo`:

```bash
sudo usermod -aG dialout $USER
```

Poi esci dalla sessione e rientra.

## Risultati della compilazione

| Risultato | Descrizione |
|---|---|
| `target/release/caruspex` / `caruspex.exe` | L'applicazione con l'interfaccia Tauri (`tauri build` scrive anche gli installer e i bundle in `target/release/bundle/`) |
| `target/release/caruspex_cli` / `caruspex_cli.exe` | Strumento da riga di comando automatizzabile sullo stesso nucleo, vedi [Strumento da riga di comando](#strumento-da-riga-di-comando) |

### Pacchetti e release

`bash tools/package.sh` costruisce in `dist/` i pacchetti del sistema attuale: l'installer e due zip su
Windows (l'installer richiede [Inno Setup 6](https://jrsoftware.org/isinfo.php)), il dmg e l'archivio da riga di comando
su macOS, l'AppImage, il `.deb` e l'archivio da riga di comando su Linux. Poi `bash tools/smoke_test.sh` li installa o
li apre come farebbe un utente, avvia entrambi i programmi e li rimuove di nuovo. La CI fa entrambe le cose a ogni esecuzione.

Una release nasce da un tag. `bash tools/release.sh <version>` la prepara su un proprio branch senza inviare nulla:
imposta la versione in `Cargo.toml` e `app/package.json`, crea `changelog/<version>.md` (tedesco e inglese),
esegue i controlli, i test, i pacchetti e lo smoke test, e aggiorna gli screenshot del README (`tools/screenshots.sh de` e
`tools/screenshots.sh en`). Dopo l'unione della pull request, crea il tag `v<version>` e invialo. La CI compila tutti e
tre i sistemi e pubblica la release con i pacchetti e l'elenco delle modifiche.

La CI (`.github/workflows/build.yml`) esegue prima formattazione, controllo delle tabelle generate, controllo delle traduzioni,
compilazione dell'interfaccia e clippy su Linux, poi testa, impacchetta e sottopone a smoke test Windows, Linux e macOS.
Un secondo workflow (`.github/workflows/data-check.yml`) cerca una volta al mese nuovi codici VIN, veicoli e file DBC
a monte e apre una issue con le novità.

## Test

```bash
cargo test --workspace
```

Circa 370 test girano in pochi secondi, tutti senza auto. Coprono il parser delle risposte ELM327, la decodifica
dei codici errore, la conversione dei PID, i profili dei veicoli e il decodificatore VIN, i servizi OBD-II e UDS contro l'auto demo,
sessioni complete KWP1281 e KWP2000 contro centraline simulate, ENET (HSFZ) e DoIP contro gateway finti,
SLCAN e il percorso J2534 contro un driver finto, registrazioni e riproduzione, e l'analisi (controllo della salute, test della batteria,
computer di viaggio, cronometro delle prestazioni). I test della riga di comando eseguono il `caruspex_cli` compilato e confrontano il suo output con
i file di riferimento in `crates/caruspex-cli/tests/golden/`, che contengono ciò che la vecchia versione in C++ stampava per gli stessi
comandi, così ogni differenza rispetto al vecchio strumento salta fuori.

I controlli che la CI esegue oltre ai test:

```bash
cargo fmt --all --check
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
python tools/i18n_rust.py --strict
```

```bash
npm --prefix app run build
```

I primi due sono formattazione e avvisi del linter. `tools/i18n_rust.py --strict` fallisce quando un testo del nucleo manca nei
file di traduzione o una traduzione manca in una delle 18 lingue; la compilazione dell'interfaccia controlla i propri
testi allo stesso modo quando è impostato `I18N_STRICT=1`. Esegui `python tools/gen_dtc.py`, `gen_vehicles.py`, `gen_ecus.py` e
`gen_pids.py` per aggiornare le tabelle generate; la CI fallisce se differiscono da quanto è nel repository.

## Provalo senza auto

Il modo più rapido è l'auto demo integrata: scegli "Auto demo" nella pagina Collega, oppure avvia con

```bash
target/release/caruspex --demo --language en
```

`caruspex --help` elenca tutte le opzioni dell'applicazione: `--tcp`, `--port`, `--bluetooth`, `--enet`, `--can`,
`--replay`, `--report`, `--language`, `--theme`, `--page`, `--shot` e `--size`.

Il simulatore Python interpreta il motore dell'Audi A2 sulla K-Line oppure, con `--can`, un'auto CAN con motore
e cambio (intestazioni, ISO-TP, modi 06 e 09 per centralina):

```bash
python tools/mock_elm327.py --can
```

Poi collega l'applicazione (su Windows il binario è `target/release/caruspex.exe`):

```bash
target/release/caruspex --tcp 127.0.0.1:35000 --language en
```

Anche lo strumento da riga di comando funziona contro il simulatore:

```bash
target/release/caruspex_cli read --tcp 127.0.0.1:35000
```

### Strumento da riga di comando

`caruspex_cli <command>` con `--demo`, `--tcp HOST[:PORT]`, `--port NAME [--baud N]`, `--enet HOST|auto`,
`--can SPEC` o `--replay FILE`:

| Comando | Che cosa fa |
|---|---|
| `read` | Spia di avaria, prontezza e codici errore, con il significato per la marca quando il VIN la rivela |
| `vin` | VIN, che cosa dice (costruttore, paese, anno del modello, cifra di controllo) e l'auto corrispondente del catalogo |
| `freeze` | Tutti i fotogrammi congelati, uno per ogni errore memorizzato |
| `health` | Controllo della salute: errori, prontezza, modo 06 e una serie live (`--seconds N`, 15 per impostazione predefinita) |
| `probe` | Test dell'adattatore: che cos'è davvero l'ELM327, quali comandi conosce, tempi di andata e ritorno |
| `uds` | UDS su CAN: identificazione e memoria errori di ogni centralina (`--ids A,B,...` per i tuoi indirizzi) |
| `custom` | Valori del costruttore e personalizzati di un pacchetto integrato (`--pack NAME`) o di un CSV di Torque (`--csv FILE`), una volta |
| `live`, `info`, `clear`, `ecus`, `ports`, `kklcheck` | Valori live, VIN e primo fotogramma congelato, cancellazione, tutto per centralina, porte seriali, autotest del cavo KKL |
| `enet`, `interfaces` | Cerca nella rete auto BMW e Mini e auto DoIP; elenca le interfacce CAN di questo computer (driver J2534, SocketCAN) |
| `version`, `help` | Versione del programma, l'elenco completo di comandi e opzioni |

`--json` stampa `read`, `vin`, `health`, `probe`, `freeze`, `uds` e `custom` come JSON per gli script, `--record FILE`
salva l'intera sessione con l'adattatore per `--replay`, `--fast-timing` porta l'adattatore in una
temporizzazione adattiva aggressiva, `-v` registra ogni comando dell'adattatore (su stderr insieme a `--json`). `caruspex_cli help` elenca tutto.

```bash
target/release/caruspex_cli health --demo --json > health.json
```

Gli screenshot di questa pagina vengono dall'auto demo, scattati dall'applicazione stessa:
`--demo --page faults --shot shot.png --size 1440x920` apre una pagina, salva un'immagine della finestra dopo qualche
secondo ed esce (`tools/screenshots.sh` scatta l'intero set).

## Struttura del progetto

```
crates/caruspex-i18n/       Lingue: tedesco e inglese nel codice, le altre 16 da translations/
crates/caruspex-vehicles/   Decodificatore VIN e catalogo dei veicoli (marchi, modelli, generazioni, connettori, bus)
crates/caruspex-data/       Codici errore, PID, indirizzi delle centraline, associazione codice-componente
crates/caruspex-link/       Vie verso l'adattatore (seriale, TCP, Bluetooth LE), driver ELM327, controllo dell'adattatore
crates/caruspex-protocols/  OBD-II, UDS, KWP2000 e KWP1281 (K-Line, byte per byte)
crates/caruspex-net/        BMW ENET (HSFZ), DoIP, SLCAN, SocketCAN, J2534 PassThru
crates/caruspex-record/     Registrazioni e riproduzione, valori personalizzati (CSV di Torque, pacchetti PID), scrittore JSON
crates/caruspex-analysis/   Controllo della salute, guida al ciclo di guida, computer di viaggio, cronometro delle prestazioni, test della batteria
crates/caruspex-demo/       L'auto demo integrata: un ELM327 con centraline simulate
crates/caruspex-cli/        Lo strumento da riga di comando, con i suoi test su file di riferimento
app/src/                    L'interfaccia: React 19 e TypeScript (pagine, struttura, stato, rapporto, i18n)
app/src-tauri/              L'applicazione Tauri: finestra, comandi, controllo degli aggiornamenti, opzioni da riga di comando
translations/               File di traduzione condivisi per nucleo e interfaccia, 18 lingue
data/                       Il catalogo dei veicoli come CSV curati a mano, fonti di centraline e PID
tools/                      Generatori, controlli delle traduzioni, impacchettamento, script di release e screenshot, simulatore ELM327
docs/                       Screenshot del README
packaging/                  Script dell'installer Windows, file desktop Linux, Info.plist macOS
changelog/                  Note di rilascio per versione, tedesco e inglese
```

Il database dei codici errore è generato: `tools/gen_dtc.py` espande le famiglie regolari di codici EOBD e gli
elenchi curati in `tools/dtc/` in `crates/caruspex-data/src/generated/*.rs`. Eseguilo di nuovo dopo aver cambiato i dati:

```bash
python tools/gen_dtc.py
```

Il catalogo dei veicoli funziona allo stesso modo: modifica i file CSV in `data/vehicles/`, poi esegui

```bash
python tools/gen_vehicles.py
```

Gli indirizzi delle centraline e i pacchetti PID hanno `tools/gen_ecus.py` e `tools/gen_pids.py`. Le traduzioni sono descritte
in [translations/TRANSLATING.md](translations/TRANSLATING.md); dopo aver aggiunto un testo nel codice,
`python tools/i18n_rust.py` elenca che cosa resta da tradurre.

## Una nota sulla cancellazione dei codici

Il modo 04 dell'OBD cancella la memoria errori, il fotogramma congelato e i monitor di prontezza. Subito dopo
l'auto non supererà un test delle emissioni finché non avrà completato diversi cicli di guida completi, e un codice la cui causa
è ancora presente tornerà dopo un breve tragitto. Cancellare non significa riparare.

## Avvertenza

Caruspex è un progetto per hobby ed è fornito senza garanzia. Usalo a tuo rischio, solo su veicoli
su cui sei autorizzato a lavorare e mai durante la guida. Tutti i nomi di prodotti e marchi sono marchi dei rispettivi
proprietari; Caruspex non è affiliato né approvato da alcun costruttore di veicoli.

## Licenza

MIT, vedi [LICENSE](LICENSE). I valori del costruttore per centinaia di modelli sono convertiti da [OBDb](https://github.com/OBDb) e, come OBDb, concessi in licenza CC BY-SA 4.0: si trovano separati dal codice in [data/obdb](data/obdb) con la licenza e l'elenco delle fonti ([ATTRIBUTION.md](data/obdb/ATTRIBUTION.md)). I font inclusi sono concessi in licenza SIL Open Font License 1.1:
JetBrains Mono ([OFL.txt](resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](resources/fonts/OFL-Inter.txt)) e
Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](resources/fonts/OFL-PlusJakartaSans.txt)).
