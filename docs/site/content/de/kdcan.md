# K+DCAN-Kabel

Das K+DCAN-Kabel ist das übliche Kabel für BMW von etwa 1998 bis 2017 mit den Werkstattprogrammen INPA und ISTA. Darin stecken ein serieller FTDI-Chip und ein kleiner Umsetzer für das D-CAN von BMW. Viele dieser Kabel haben einen Schalter oder eine Brücke zwischen Pin 7 und 8 für ältere Autos.

## Stand in Caruspex

> Ehrlicher Stand: Den D-CAN-Modus dieses Kabels spricht Caruspex noch nicht. Die alten BMW-Protokolle (DS2 und KWP2000 nach BMW-Art) über die K-Line sind geplant, brauchen aber zuerst Tests am echten Auto.

Was heute stattdessen geht:

- **BMW und Mini ab der F-Baureihe (etwa ab 2008)**: Ein [ENET-Kabel](enet.md) erreicht alle Steuergeräte über Ethernet, oder ein einfacher [ELM327](elm327-usb.md) mit der eigenen Adressierung von BMW über CAN.
- **Standard-OBD bei BMW mit CAN (etwa ab 2007)**: Jeder ELM327 liest Motorfehler und Live-Werte.
- **Im K-Line-Modus** reicht das Kabel die K-Line durch wie ein [KKL-Kabel](kkl.md). Das hilft bei Autos aus dem VW-Konzern; BMW-Steuergeräte an der K-Line brauchen die Protokolle, die noch fehlen.

## Hilfe gesucht

Wer ein K+DCAN-Kabel und einen älteren BMW hat, hilft mit einem [Testbericht](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) samt Mitschnitt, diesen Teil zu bauen.
