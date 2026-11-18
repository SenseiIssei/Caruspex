# ELM327 przez USB

Wtyczka do gniazda OBD z kablem do portu USB. To najbardziej niezawodny rodzaj ELM327: bez parowania, bez radia, bez rozładowanych baterii.

## Sterownik

W środku wtyczki układ USB zamienia kabel w port szeregowy (port COM w Windows). To, jaki sterownik jest potrzebny, zależy od tego układu:

| Układ | Sterownik |
|---|---|
| FTDI FT232 | zwykle instaluje go sam Windows, wbudowany w Linux i macOS |
| CH340 / CH341 | zwykle instaluje go sam Windows, wbudowany w Linux i macOS |
| Silicon Labs CP210x | ze strony producenta, jeśli Windows go nie znajdzie |
| Prolific PL2303 | ze strony producenta; stare klony często działają tylko ze starym sterownikiem |

Po podłączeniu Menedżer urządzeń w Windows pokazuje adapter w sekcji "Porty (COM i LPT)".

## Łączenie

1. Włóż adapter do auta, włącz zapłon, podłącz kabel do komputera.
2. Na stronie **Połącz** wybierz **USB lub port COM**.
3. Zostaw port na **Szukaj automatycznie** albo wybierz go z listy, potem połącz.

Automatyczne wyszukiwanie próbuje każdego portu szeregowego przy typowych prędkościach. Jeśli znasz prędkość swojego adaptera, możesz ją ustawić w ustawieniach w polu **Szybkość transmisji adaptera**.

## Linux i macOS

W Linuksie adapter pojawia się jako `/dev/ttyUSB0` lub `/dev/ttyACM0`. Twój użytkownik musi należeć do grupy `dialout` (zobacz [Instalacja](install.md)). W macOS pojawia się jako `/dev/cu.usbserial-...` lub `/dev/cu.wchusbserial...`.

## Typowe problemy

- **Nie pojawia się żaden port**: brakuje sterownika albo kabel służy tylko do ładowania.
- **Adapter odpowiada, auto nie**: zapłon wyłączony albo adapter to klon, który nie radzi sobie z protokołem auta. Więcej powie ci [test adaptera](interfaces.md).
- **Bardzo wolne wartości na żywo w starszych autach**: sama linia K-Line jest wolna, to nie wina adaptera.
