# Instalacja

Każde wydanie jest na [stronie wydań na GitHubie](https://github.com/SenseiIssei/Caruspex/releases/latest). Paczki nie są jeszcze podpisane, dlatego Windows i macOS pytają raz przed pierwszym uruchomieniem.

## Windows 10 i 11

| Plik | Co to jest |
|---|---|
| `Caruspex-<version>-windows-x64-setup.exe` | Instalator. Nie potrzebuje uprawnień administratora, może dodać narzędzie wiersza poleceń `caruspex_cli` do ścieżki wyszukiwania i instaluje środowisko WebView2 tylko wtedy, gdy go brakuje. |
| `Caruspex-<version>-windows-x64.zip` | Wersja przenośna: rozpakuj i uruchom `caruspex.exe`. |

Przy pierwszym uruchomieniu Windows SmartScreen może powiedzieć, że aplikacja jest nieznana: kliknij "Więcej informacji", a potem "Uruchom mimo to".

Adaptery USB potrzebują sterownika do swojego układu USB. Windows 10 i 11 zwykle znajdują go same; jeśli nie, zajrzyj do [ELM327 przez USB](elm327-usb.md).

## macOS 13 i nowszy (Apple Silicon)

Pobierz `Caruspex-<version>-macos.dmg`, otwórz go i przeciągnij Caruspex do Aplikacji. Ponieważ aplikacja nie jest jeszcze notaryzowana, za pierwszym razem otwórz ją prawym przyciskiem myszy i "Otwórz".

## Linux (x86_64)

| Plik | Co to jest |
|---|---|
| `Caruspex-<version>-linux-x86_64.AppImage` | Działa na prawie każdej dystrybucji: nadaj mu prawo wykonywania przez `chmod +x` i uruchom. Sam się też aktualizuje. |
| `Caruspex-<version>-linux-amd64.deb` | Dla Debiana i Ubuntu: `sudo apt install ./Caruspex-<version>-linux-amd64.deb` |

Aby używać adapterów szeregowych bez `sudo`, dodaj się raz do grupy `dialout`, a potem wyloguj i zaloguj ponownie:

```
sudo usermod -aG dialout $USER
```

## Tylko narzędzie wiersza poleceń

Do skryptów i serwerów jest `caruspex_cli` bez interfejsu, dla Windows, Linux i macOS. `caruspex_cli help` wyświetla polecenia.

`caruspex_cli report` zapisuje raport diagnostyczny jako plik PDF, bez aplikacji i przez każde połączenie, które znają pozostałe polecenia (`--demo`, `--port`, `--tcp`, `--enet`, `--can`, `--replay`):

```
caruspex_cli report --demo report.pdf
caruspex_cli report --port COM5 --lang de report.pdf
```

Raport może być po niemiecku, angielsku, hiszpańsku, francusku, portugalsku, włosku, niderlandzku i indonezyjsku (`--lang`). W pozostałych językach, także po polsku, powstaje po angielsku, bo czcionkom wbudowanym w każdą przeglądarkę PDF brakuje ich liter.

## Aktualizacje

Przy starcie Caruspex pyta GitHuba, czy jest nowsza wersja, i nic poza tym. Nowa wersja jest instalowana tylko wtedy, gdy jej podpis pasuje do klucza wbudowanego w aplikację. Zarówno sprawdzanie, jak i automatyczną instalację można wyłączyć w ustawieniach.

## Menedżery pakietów

Pakiety dla winget, Homebrew, Flathub i Snap są w przygotowaniu. Dopóki nie zostaną opublikowane, Caruspex instaluje się z plików powyżej.
