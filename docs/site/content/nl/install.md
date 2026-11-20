# Installeren

Elke versie staat op de [releasepagina op GitHub](https://github.com/SenseiIssei/Caruspex/releases/latest). De pakketten zijn nog niet ondertekend, daarom vragen Windows en macOS één keer om bevestiging voor de eerste start.

## Windows 10 en 11

| Bestand | Wat het is |
|---|---|
| `Caruspex-<version>-windows-x64-setup.exe` | Het installatieprogramma. Het heeft geen beheerdersrechten nodig, kan het opdrachtregelprogramma `caruspex_cli` aan het zoekpad toevoegen en installeert de WebView2-runtime alleen als die ontbreekt. |
| `Caruspex-<version>-windows-x64.zip` | Draagbaar: uitpakken en `caruspex.exe` starten. |

Bij de eerste start kan Windows SmartScreen melden dat de app onbekend is: klik op "Meer informatie" en dan op "Toch uitvoeren".

USB-adapters hebben een driver nodig voor hun USB-chip. Windows 10 en 11 vinden die meestal zelf; zo niet, kijk dan bij [ELM327 via USB](elm327-usb.md).

## macOS 13 en nieuwer (Apple Silicon)

Download `Caruspex-<version>-macos.dmg`, open het en sleep Caruspex naar Apps. Omdat de app nog niet genotariseerd is, open je hem de eerste keer met een rechtsklik en "Open".

## Linux (x86_64)

| Bestand | Wat het is |
|---|---|
| `Caruspex-<version>-linux-x86_64.AppImage` | Draait op bijna elke distributie: maak het uitvoerbaar met `chmod +x` en start het. Het werkt zichzelf ook bij. |
| `Caruspex-<version>-linux-amd64.deb` | Voor Debian en Ubuntu: `sudo apt install ./Caruspex-<version>-linux-amd64.deb` |

Om seriële adapters zonder `sudo` te gebruiken, voeg je jezelf één keer toe aan de groep `dialout` en meld je je daarna af en weer aan:

```
sudo usermod -aG dialout $USER
```

## Alleen het opdrachtregelprogramma

Voor scripts en servers is er `caruspex_cli` zonder interface, voor Windows, Linux en macOS. `caruspex_cli help` toont de opdrachten.

`caruspex_cli report` schrijft het diagnoserapport als pdf-bestand, zonder de app en via elke verbinding die de andere opdrachten kennen (`--demo`, `--port`, `--tcp`, `--enet`, `--can`, `--replay`):

```
caruspex_cli report --demo report.pdf
caruspex_cli report --port COM5 --lang de report.pdf
```

Het rapport kan in het Duits, Engels, Spaans, Frans, Portugees, Italiaans, Nederlands en Indonesisch (`--lang`). Voor de andere talen komt het in het Engels, omdat de lettertypen die elke pdf-lezer ingebouwd heeft hun letters missen.

## Updates

Bij het starten vraagt Caruspex aan GitHub of er een nieuwere versie is, en verder niets. Een nieuwe versie wordt alleen geïnstalleerd als de handtekening past bij de sleutel die in de app is ingebouwd. Zowel de controle als de automatische installatie kun je in de instellingen uitschakelen.

## Pakketbeheerders

Pakketten voor winget, Homebrew, Flathub en Snap worden voorbereid. Tot ze gepubliceerd zijn, installeer je Caruspex met de bestanden hierboven.
