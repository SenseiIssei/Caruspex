# Kabel BMW ENET

BMW i Mini od serii F (od około 2008 roku) mają Ethernet w gnieździe OBD. Kabel ENET biegnie od gniazda OBD do portu sieciowego komputera, bez żadnego układu adaptera po drodze. Caruspex rozmawia przez HSFZ z bramką auta (porty 6811 i 6801) i dociera do każdego sterownika.

## Łączenie

1. Podłącz kabel do auta i do portu sieciowego komputera (albo do sieciowej karty USB). Włącz zapłon.
2. Poczekaj do minuty: bez serwera DHCP Windows, macOS i Linux same nadają komputerowi automatyczny adres (169.254.x.x).
3. Na stronie **Połącz** wybierz **Kabel sieciowy** i zostaw pole puste: Caruspex szuka auta w sieci. Jeśli wyszukiwanie nic nie znajdzie, wpisz adres auta.

## Co działa

- Każdy sterownik przez UDS z adresowaniem BMW, kody błędów w formacie BMW tak, jak pokazuje je ISTA (na przykład 480A10).
- Pamięć usterek, identyfikacja i kasowanie dla każdego sterownika, wartości na żywo przez OBD.
- Tutaj też Caruspex tylko odczytuje. Kodowanie i programowanie zostają przy programach warsztatowych.

## Jaki kabel?

Zwykły kabel ENET ma rezystor między pinami 8 i 16, który włącza Ethernet w aucie. Kable bez niego pozostają martwe. Porty sieciowe 1 Gbit/s działają, auto mówi z prędkością 100 Mbit/s.

## Wiersz poleceń

```
caruspex_cli ecus --enet auto
```
