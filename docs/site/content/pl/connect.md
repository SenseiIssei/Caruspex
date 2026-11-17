# Asystent połączenia i gniazdo OBD

## Gdzie jest gniazdo?

Każde auto sprzedane w Europie od 2001 (benzyna) lub 2004 roku (diesel), a w USA od 1996 roku, ma 16-pinowe gniazdo OBD w zasięgu ręki kierowcy. Typowe miejsca:

- pod deską rozdzielczą, po lewej lub prawej stronie kolumny kierownicy,
- nad pedałami,
- w bocznej osłonie przestrzeni na nogi,
- za pokrywą skrzynki bezpieczników,
- w konsoli środkowej, czasem za małą klapką.

Dla ponad 200 generacji samochodów i 35 motocykli Caruspex zna dokładne miejsce. Asystent połączenia zaznacza je na rzucie z góry i na zbliżeniu strony kierowcy; dla motocykli pokazuje widok z boku z wtyczką diagnostyczną i potrzebnym kablem adaptera.

> Lista miejsc gniazda to nasz własny zbiór i nie została sprawdzona w każdym aucie. Jeśli miejsce w twoim aucie się nie zgadza, daj nam znać [raportem z testu](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml).

## Asystent połączenia

Asystent otwiera się sam przy pierwszym uruchomieniu. Pyta o cztery rzeczy, a każdy krok można pominąć:

1. **Jaki adapter?** ELM327 przez USB, Bluetooth lub Wi-Fi, kabel ENET albo kabel KKL. Zdjęcia pomagają je rozróżnić.
2. **Jakie auto?** Wybierz je z katalogu, a asystent pokaże, gdzie jest gniazdo.
3. **Zapłon włączony.** Większość sterowników odpowiada tylko przy włączonym zapłonie.
4. **Połącz.** Caruspex przeszukuje porty i próbuje protokołów.

Później asystenta znajdziesz w ustawieniach i na stronie Połącz.

## Wyszukiwanie i zapamiętywanie

Przy automatycznym wyszukiwaniu Caruspex przechodzi przez wszystkie porty szeregowe, w każdym pyta o ELM327, a potem po kolei próbuje protokołów auta. Gdy auto raz się połączy, Caruspex zapamiętuje drogę (adapter, port, protokół, prędkość transmisji) dla jego numeru VIN i następnym razem próbuje jej najpierw, dzięki czemu łączenie idzie szybciej.

## Gdy nie chce się połączyć

Jeśli żaden adapter nie odpowiada albo auto milczy, Caruspex pokazuje krótką listę kontrolną dla tego rodzaju połączenia, razem z tym, co już ustalił: sterownik zainstalowany, Bluetooth sparowany, zapłon włączony, właściwa prędkość transmisji, klon adaptera, który tylko udaje nowszą wersję. Strony w dziale [adaptery i interfejsy](interfaces.md) szczegółowo opisują każdy rodzaj.
