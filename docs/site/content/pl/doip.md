# DoIP (diagnostyka przez IP)

Wiele aut od około 2016 roku można diagnozować przez Ethernet według ISO 13400, co nazywa się DoIP. Kabel DoIP łączy gniazdo OBD z portem sieciowym komputera.

## Łączenie

1. Podłącz kabel, włącz zapłon i poczekaj, aż komputer dostanie automatyczny adres (169.254.x.x), co może potrwać do minuty.
2. Na stronie **Połącz** wybierz **Kabel sieciowy** i zostaw pole puste. Caruspex szuka pojazdów DoIP w sieci i sam rozpoznaje, czy auto mówi DoIP, czy HSFZ BMW.

## Co działa

OBD przez DoIP oraz sterowniki przez UDS, których adresy Caruspex zna dla danej marki. Listy adresów dla aut z DoIP są jeszcze krótkie; nagrania z prawdziwych aut je uzupełniają (zobacz [prywatność](privacy.md), co zawiera udostępnione nagranie).

## Jaki kabel?

DoIP potrzebuje linii Ethernet na pinach 3, 11, 12 i 13 gniazda OBD oraz linii aktywacji na pinie 8. Kable sprzedawane do konkretnej marki zwykle są połączone dokładnie tak; kabel BMW ENET jest jednym z nich.

## Wiersz poleceń

```
caruspex_cli enet
```

wypisuje pojazdy BMW i DoIP znalezione w sieci.
