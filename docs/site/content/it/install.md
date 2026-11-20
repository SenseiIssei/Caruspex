# Installazione

Ogni versione è nella [pagina delle release su GitHub](https://github.com/SenseiIssei/Caruspex/releases/latest). I pacchetti non sono ancora firmati, quindi Windows e macOS chiedono conferma una volta prima del primo avvio.

## Windows 10 e 11

| File | Cos'è |
|---|---|
| `Caruspex-<version>-windows-x64-setup.exe` | Il programma di installazione. Non richiede diritti di amministratore, può mettere lo strumento a riga di comando `caruspex_cli` nel percorso di ricerca e installa il runtime WebView2 solo se manca. |
| `Caruspex-<version>-windows-x64.zip` | Portatile: estrailo e avvia `caruspex.exe`. |

Al primo avvio Windows SmartScreen potrebbe dire che l'app è sconosciuta: clicca su "Ulteriori informazioni", poi su "Esegui comunque".

Gli adattatori USB hanno bisogno di un driver per il loro chip USB. Windows 10 e 11 di solito lo trovano da soli; altrimenti vedi [ELM327 via USB](elm327-usb.md).

## macOS 13 e successivi (Apple Silicon)

Scarica `Caruspex-<version>-macos.dmg`, aprilo e trascina Caruspex in Applicazioni. Poiché l'app non è ancora notarizzata, la prima volta aprila con il clic destro e "Apri".

## Linux (x86_64)

| File | Cos'è |
|---|---|
| `Caruspex-<version>-linux-x86_64.AppImage` | Gira su quasi tutte le distribuzioni: rendilo eseguibile con `chmod +x` e avvialo. Si aggiorna anche da solo. |
| `Caruspex-<version>-linux-amd64.deb` | Per Debian e Ubuntu: `sudo apt install ./Caruspex-<version>-linux-amd64.deb` |

Per usare gli adattatori seriali senza `sudo`, aggiungiti una volta al gruppo `dialout`, poi esci e rientra:

```
sudo usermod -aG dialout $USER
```

## Solo lo strumento a riga di comando

Per script e server c'è `caruspex_cli` senza interfaccia, per Windows, Linux e macOS. `caruspex_cli help` elenca i comandi.

`caruspex_cli report` scrive il rapporto di diagnosi in un file PDF, senza l'app e con qualsiasi collegamento che conoscono gli altri comandi (`--demo`, `--port`, `--tcp`, `--enet`, `--can`, `--replay`):

```
caruspex_cli report --demo report.pdf
caruspex_cli report --port COM5 --lang de report.pdf
```

Il rapporto può essere in tedesco, inglese, spagnolo, francese, portoghese, italiano, olandese e indonesiano (`--lang`). Per le altre lingue esce in inglese, perché ai caratteri che ogni lettore PDF ha già incorporati mancano le loro lettere.

## Aggiornamenti

All'avvio Caruspex chiede a GitHub se c'è una versione più recente, e nient'altro. Una nuova versione viene installata solo se la sua firma corrisponde alla chiave integrata nell'app. Sia il controllo sia l'installazione automatica si possono disattivare nelle impostazioni.

## Gestori di pacchetti

I pacchetti per winget, Homebrew, Flathub e Snap sono in preparazione. Finché non saranno pubblicati, i file qui sopra sono il modo per installare Caruspex.
