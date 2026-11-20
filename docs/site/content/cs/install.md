# Instalace

Každé vydání je na [stránce vydání na GitHubu](https://github.com/SenseiIssei/Caruspex/releases/latest). Balíčky zatím nejsou podepsané, proto se Windows a macOS jednou zeptají před prvním spuštěním.

## Windows 10 a 11

| Soubor | Co to je |
|---|---|
| `Caruspex-<version>-windows-x64-setup.exe` | Instalátor. Nepotřebuje práva správce, umí přidat nástroj pro příkazový řádek `caruspex_cli` do vyhledávací cesty a runtime WebView2 instaluje jen tehdy, když chybí. |
| `Caruspex-<version>-windows-x64.zip` | Přenosná verze: rozbalte ji a spusťte `caruspex.exe`. |

Při prvním spuštění může Windows SmartScreen hlásit, že aplikace je neznámá: klikněte na "Další informace" a pak na "Přesto spustit".

Adaptéry USB potřebují ovladač pro svůj čip USB. Windows 10 a 11 ho obvykle najdou samy; pokud ne, podívejte se na [ELM327 přes USB](elm327-usb.md).

## macOS 13 a novější (Apple Silicon)

Stáhněte `Caruspex-<version>-macos.dmg`, otevřete ho a přetáhněte Caruspex do Aplikací. Protože aplikace zatím není notarizovaná, otevřete ji poprvé pravým tlačítkem a volbou "Otevřít".

## Linux (x86_64)

| Soubor | Co to je |
|---|---|
| `Caruspex-<version>-linux-x86_64.AppImage` | Běží skoro na každé distribuci: nastavte mu spustitelnost přes `chmod +x` a spusťte ho. Také se sám aktualizuje. |
| `Caruspex-<version>-linux-amd64.deb` | Pro Debian a Ubuntu: `sudo apt install ./Caruspex-<version>-linux-amd64.deb` |

Abyste mohli používat sériové adaptéry bez `sudo`, přidejte se jednou do skupiny `dialout`, pak se odhlaste a znovu přihlaste:

```
sudo usermod -aG dialout $USER
```

## Jen nástroj pro příkazový řádek

Pro skripty a servery je tu `caruspex_cli` bez rozhraní, pro Windows, Linux i macOS. `caruspex_cli help` vypíše příkazy.

`caruspex_cli report` zapíše diagnostickou zprávu do souboru PDF, bez aplikace a přes každé připojení, které znají ostatní příkazy (`--demo`, `--port`, `--tcp`, `--enet`, `--can`, `--replay`):

```
caruspex_cli report --demo report.pdf
caruspex_cli report --port COM5 --lang de report.pdf
```

Zpráva může být německy, anglicky, španělsky, francouzsky, portugalsky, italsky, nizozemsky a indonésky (`--lang`). V ostatních jazycích, i v češtině, vyjde anglicky, protože písmům, která má každý prohlížeč PDF zabudovaná, chybějí jejich písmena.

## Aktualizace

Při každé instalaci se Caruspex jednou zeptá, zda má hledat aktualizace. Teprve po odpovědi "ano" se zeptá GitHubu, zda existuje novější verze, a nic dalšího. Nová verze se nainstaluje jen tehdy, když její podpis odpovídá klíči zabudovanému v aplikaci. Kontrolu i automatickou instalaci lze kdykoli změnit v nastavení.

## Správci balíčků

Balíčky pro winget, Homebrew, Flathub a Snap se připravují. Dokud nebudou zveřejněné, instaluje se Caruspex ze souborů výše.
