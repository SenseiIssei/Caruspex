# ELM327 przez Wi-Fi

Niektóre adaptery tworzą własną sieć Wi-Fi. Komputer się z nią łączy i rozmawia z adapterem przez TCP.

## Łączenie

1. Włóż adapter i włącz zapłon. Po kilku sekundach pojawia się sieć Wi-Fi, często o nazwie `WiFi_OBDII`, `V-LINK` albo nazwie producenta.
2. Połącz komputer z tą siecią. Nie ma ona internetu, więc system może ostrzec, że połączenie jest ograniczone: to w porządku.
3. W Caruspex na stronie **Połącz** wybierz **Wi-Fi**. Większość adapterów używa adresu `192.168.0.10` i portu `35000`; jest to ustawione domyślnie. Jeśli twój używa innego (na przykład `192.168.4.1`), wpisz go jako `address:port`.

## Wskazówki

- Dopóki komputer jest w sieci adaptera, nie ma internetu. Dla Caruspex nie ma to znaczenia.
- Adaptery Wi-Fi działają w każdym systemie bez sterowników.
- Jeśli połączenie się zrywa, przysuń komputer bliżej auta: radio tych wtyczek jest słabe.
