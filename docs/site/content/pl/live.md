# Wartości na żywo

Strona **Wartości na żywo** (tryb profesjonalny) pokazuje, co czujniki mierzą w tej chwili. W trybie początkującym **Tryb jazdy** pokazuje najważniejsze wartości dużymi cyframi.

![Wartości na żywo](screenshot-live.png)

## Co jest dostępne

- Każda wartość trybu 01 z SAE J1979 do PID C4: 145 PID-ów z ponad 300 odczytami, pogrupowane i z wyszukiwaniem. Które z nich oferuje twoje auto, mówi ono samo.
- Przebieg, bieg, temperatura oleju, zużycie paliwa, moment obrotowy, doładowanie, temperatury spalin, filtr cząstek stałych, AdBlue, NOx i akumulator hybrydy, o ile auto je podaje.
- Przewijany wykres kilku wartości naraz. Przy długich oknach czasowych (do godziny) wykres przerzedza stare punkty, ale zachowuje każdy szczyt i każdy dołek.
- Nagrywanie do CSV.

## Wartości producenta i własne

- **Akumulatory aut elektrycznych**: stan zdrowia, naładowanie, napięcia ogniw z rozrzutem i temperatury dla Hyundai Kona i Ioniq, Kia e-Niro, Renault Zoe, Nissan Leaf oraz Tesla Model 3 i Y (przez nasłuchiwanie na magistrali CAN), na stronie **Akumulator EV**.
- **Wartości OBDb** dla około 550 kolejnych pojazdów (ciśnienie w oponach, olej, przebieg i inne), oferowane same dla rozpoznanego auta.
- **Rozszerzona sesja diagnostyczna**: niektóre wartości diesli koncernu Volkswagen, na przykład zmierzoną masę sadzy i dystans od ostatniej regeneracji, sterownik silnika podaje tylko w swojej rozszerzonej sesji diagnostycznej. W trybie profesjonalnym Caruspex przełącza silnik w tę sesję, dopóki takie wartości są odpytywane, tylko w celu odczytu, oznacza je jako **Sesja 03** i potem przełącza z powrotem. Można to wyłączyć w ustawieniach w sekcji **Wartości producenta**. Jeśli sterownik odmówi, na przykład podczas jazdy, przy wartości widać powód.
- **Twoje własne wartości** w popularnym formacie CSV z Torque, razem z równaniami.

![Akumulator EV](screenshot-battery.png)

## Jak szybko?

Ile wartości na sekundę przychodzi, zależy od auta i adaptera. Auta z CAN odpowiadają szybko; auta na K-Line są z natury wolne. Z OBDLinkiem Caruspex może pytać o maksymalnie 24 wartości naraz (zobacz [OBDLink i STN](obdlink-stn.md)); surowy interfejs CAN to najszybsza droga.

## Nagrywanie i odtwarzanie

Całą sesję z autem można nagrać do pliku i później odtworzyć bez auta, co przydaje się przy szukaniu usterek. Przyciskiem **Udostępnij nagranie** można przygotować nagranie dla projektu bez danych osobowych, zobacz [prywatność](privacy.md).
