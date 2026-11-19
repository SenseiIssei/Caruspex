# Kabel K+DCAN

Kabel K+DCAN to typowy kabel do BMW z lat około 1998 do 2017, używany z programami warsztatowymi INPA i ISTA. W środku jest szeregowy układ FTDI i mały konwerter do D-CAN BMW. Wiele z tych kabli ma przełącznik albo mostek między pinami 7 i 8 dla starszych aut.

## Stan w Caruspex

> Uczciwy stan: Caruspex odczytuje BMW serii E przez K-Line tego kabla: DS2 w starszych modelach, KWP2000 w wersji BMW i BMW-FAST w generacji E60. Na razie testowano to tylko na symulowanych autach w programie, jeszcze nie na prawdziwym BMW. Trybu D-CAN kabla Caruspex jeszcze nie obsługuje.

## W trybie K-Line

- **Przełącznik na K-Line** ustawić, zanim kabel trafi do gniazda; w większości kabli to lewa pozycja. Łączy ona piny 7 i 8, dzięki czemu odpowiadają też sterowniki na pinie 8 starszych modeli.
- **Które auta**: E36 (z okrągłą 20-pinową wtyczką przez przejściówkę), E38, E39, E46, E52, E53, E83, E85 oraz generacja E60 (E60, E61, E63, E64, E65, E87, E90) do 03/2007. Później, a także od serii F, gniazdo mówi D-CAN lub Ethernet.
- **Jak**: tryb Pro włączony, strona *BMW przez K-Line*, wybór portu, *Przeszukaj*. Caruspex odpytuje każdy znany adres przez DS2 (9600 bodów, parzystość parzysta), KWP2000 (10400 bodów z szybką inicjalizacją) i BMW-FAST (115200 bodów) i pokazuje znalezione sterowniki z identyfikacją i pamięcią błędów. Trwa to około pół minuty.
- **Kasowanie** działa dla jednego sterownika albo wszędzie, zawsze dopiero po pytaniu. Potem Caruspex odczytuje, co zostało.
- **Kody błędów**: BMW nie publikuje opisów swoich kodów, dlatego Caruspex pokazuje je jako liczbę szesnastkową.
- **Tylko odczyt**: nie ma tu kodowania, adaptacji ani testów elementów wykonawczych; to zadanie oprogramowania warsztatowego.
- **Kabel KKL** działa tak samo, ale w starszych modelach dociera tylko do sterowników na pinie 7, dopóki piny 7 i 8 nie są połączone.

Aby wypróbować bez auta, lista portów na stronie zawiera symulowane E46 (DS2) i E60 (KWP2000).

## Co jeszcze działa

- **BMW i Mini od serii F (od około 2008 roku)**: [kabel ENET](enet.md) dociera do każdego sterownika przez Ethernet, albo zwykły [ELM327](elm327-usb.md) z adresowaniem BMW przez CAN.
- **Standardowe OBD w BMW z CAN (od około 2007 roku)**: każdy ELM327 odczyta błędy silnika i wartości na żywo.

## Potrzebna pomoc

Jeśli masz kabel K+DCAN i starsze BMW, [raport z testu](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) z dziennikiem pomoże potwierdzić część K-Line na prawdziwym aucie.
