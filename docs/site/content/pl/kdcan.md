# Kabel K+DCAN

Kabel K+DCAN to typowy kabel do BMW z lat około 1998 do 2017, używany z programami warsztatowymi INPA i ISTA. W środku jest szeregowy układ FTDI i mały konwerter do D-CAN BMW. Wiele z tych kabli ma przełącznik albo mostek między pinami 7 i 8 dla starszych aut.

## Stan w Caruspex

> Uczciwy stan: Caruspex nie obsługuje jeszcze trybu D-CAN tego kabla. Stare protokoły BMW (DS2 i KWP2000 w wersji BMW) przez K-Line są w planach, ale najpierw wymagają testów na prawdziwym aucie.

Czego możesz użyć zamiast tego już dziś:

- **BMW i Mini od serii F (od około 2008 roku)**: [kabel ENET](enet.md) dociera do każdego sterownika przez Ethernet, albo zwykły [ELM327](elm327-usb.md) z adresowaniem BMW przez CAN.
- **Standardowe OBD w BMW z CAN (od około 2007 roku)**: każdy ELM327 odczyta błędy silnika i wartości na żywo.
- **W trybie K-Line** kabel przepuszcza linię K-Line jak [kabel KKL](kkl.md). Przydaje się to w autach koncernu VW; sterowniki BMW na K-Line potrzebują protokołów, których jeszcze brakuje.

## Potrzebna pomoc

Jeśli masz kabel K+DCAN i starsze BMW, [raport z testu](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) z nagraniem pomoże zbudować tę część.
