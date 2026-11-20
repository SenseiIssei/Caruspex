# Najczęściej zadawane pytania

## Czy Caruspex coś kosztuje?

Nie. Caruspex to wolne oprogramowanie na licencji MIT, bez reklam, kont i płatnych dodatków.

## Jaki adapter kupić?

Dla większości osób najlepszym startem jest ELM327 na USB z układem FTDI lub CH340 albo OBDLink (Bluetooth lub USB). Do VW, Audi, Seata i Skody sprzed około 2004 roku dokup [kabel KKL](kkl.md); do BMW i Mini od serii F [kabel ENET](enet.md). [Przegląd interfejsów](interfaces.md) je porównuje.

## Czy zadziała z moim autem?

Każde auto z OBD-II lub EOBD działa w zakresie silnika: w Europie benzynowe od 2001 roku i diesle od 2004, w USA od 1996. Starsze auta koncernu VW od około 1990 roku działają przez kabel KKL. Strona **Pojazdy** mówi, która droga działa dla danego auta. Motocykle od Euro 5 działają z kablem adaptera do ich wtyczki diagnostycznej.

## Czy przetestowano go na moim aucie?

Szczerze: jeszcze nie. Caruspex jest testowany na symulowanych sterownikach; na prawdziwym sprzęcie sprawdzono do tej pory tylko OBDLinka z układem STN2232, bez samochodu. Pomóż proszę [raportem z testu](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml).

## Czy Caruspex może zepsuć mi auto?

Caruspex tylko odczytuje. Jedyna rzecz, którą zapisuje, to kasowanie kodów błędów, i ostrzega, zanim to zrobi. Nigdy nie koduje, nigdy nie programuje i nigdy nie zmienia wartości adaptacji. Nie używaj go podczas jazdy.

## Czy może zmienić przebieg, immobilizer albo dokodować funkcje?

Nie i nie będzie tego robić. To sprawa prawnie drażliwa i nie do tego służy to narzędzie.

## Dlaczego adapter podaje wersję 2.1, a niektóre rzeczy nie działają?

Wiele tanich adapterów podaje nowszą wersję ELM327, niż naprawdę mają. **Test adaptera** sprawdza, jakie polecenia adapter naprawdę zna.

## Dlaczego wartości w moim starszym aucie są wolne?

Auta na K-Line (w Europie mniej więcej od 2001 do 2007 roku) odpowiadają z natury wolno. Auta z CAN są dużo szybsze.

## Czy Caruspex gdzieś wysyła moje dane?

Nie. Tylko sprawdzanie aktualizacji pyta GitHuba o najnowszą wersję i można je wyłączyć. Zobacz [prywatność](privacy.md).

## Jak mogę pomóc?

Wyślij raport z testu swojego auta, udostępnij nagranie, zgłaszaj błędy albo popraw tłumaczenie: przewodnik to [TRANSLATING.md](https://github.com/SenseiIssei/Caruspex/blob/main/translations/TRANSLATING.md), a przycisk **Popraw tłumaczenie** w ustawieniach go otwiera.

Kod też jest mile widziany: [CONTRIBUTING.md](https://github.com/SenseiIssei/Caruspex/blob/main/CONTRIBUTING.md) wyjaśnia (po angielsku), jak zbudować Caruspex i czego potrzebuje pull request.
