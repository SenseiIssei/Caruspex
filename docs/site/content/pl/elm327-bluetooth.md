# ELM327 przez Bluetooth

Małe wtyczki bez kabla. Są dwa rodzaje Bluetooth, a Caruspex obsługuje oba.

## Bluetooth LE

Nowsze adaptery, takie jak Vgate iCar Pro BLE, vLinker, OBDLink CX i Veepeak BLE, używają Bluetooth Low Energy. Nie wymagają **parowania**: na stronie **Połącz** wybierz **Bluetooth**, kliknij **Szukaj urządzeń Bluetooth** i wybierz adapter z listy. Działa to na Windows, Linux i macOS.

## Klasyczny Bluetooth

Starsze i wiele tanich adapterów używa klasycznego Bluetooth z profilem portu szeregowego.

1. Sparuj adapter raz w ustawieniach Bluetooth swojego systemu. PIN to zwykle `1234` lub `0000`.
2. W Caruspex wybierz **Bluetooth** i sparowany adapter; lista mówi, czy urządzenie jest LE, klasyczne czy jedno i drugie.

W Windows i Linuksie Caruspex łączy się ze sparowanymi klasycznymi adapterami bezpośrednio. Alternatywnie Windows tworzy dla adaptera port COM, który działa jak przy [ELM327 przez USB](elm327-usb.md). W macOS klasyczne adaptery Bluetooth działają tylko wtedy, gdy system udostępnia dla nich port szeregowy; adaptery z Bluetooth LE są tam lepszym wyborem.

## Wskazówki

- Wyjmuj adapter, gdy go nie używasz: wiele tanich modeli nie pozwala akumulatorowi auta odpocząć.
- Jeśli adapter się nie pokazuje, wyłącz i włącz Bluetooth w komputerze i upewnij się, że żadna aplikacja w telefonie nie jest z nim w tym samym czasie połączona.
- Bluetooth jest wolniejszy niż kabel. Z OBDLinkiem Caruspex pyta o kilka wartości naraz, co nadrabia większość różnicy (zobacz [OBDLink i STN](obdlink-stn.md)).
