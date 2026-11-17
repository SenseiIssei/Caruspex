# BMW ENET-Kabel

BMW und Mini ab der F-Baureihe (etwa ab 2008) haben Ethernet an der OBD-Buchse. Ein ENET-Kabel geht von der OBD-Buchse zum Netzwerkanschluss des Rechners, ohne Adapterchip dazwischen. Caruspex spricht HSFZ mit dem Gateway des Autos (Ports 6811 und 6801) und erreicht alle Steuergeräte.

## Verbinden

1. Das Kabel ins Auto und in den Netzwerkanschluss des Rechners stecken (oder in einen USB-Netzwerkadapter). Die Zündung einschalten.
2. Bis zu einer Minute warten: Ohne DHCP-Server geben Windows, macOS und Linux dem Rechner von selbst eine automatische Adresse (169.254.x.x).
3. Auf der Seite **Verbinden** **Netzwerkkabel** wählen und das Feld leer lassen: Caruspex sucht das Auto im Netz. Findet die Suche nichts, die Adresse des Autos eintragen.

## Was geht

- Alle Steuergeräte über UDS mit der Adressierung von BMW, Fehlercodes im BMW-Format, wie ISTA sie zeigt (zum Beispiel 480A10).
- Fehlerspeicher, Kennung und Löschen je Steuergerät, Live-Werte über OBD.
- Caruspex liest auch hier nur. Codieren und Flashen bleiben bei den Werkstattprogrammen.

## Welches Kabel?

Ein einfaches ENET-Kabel hat einen Widerstand zwischen Pin 8 und 16, der das Ethernet des Autos einschaltet. Kabel ohne ihn bleiben dunkel. Netzwerkanschlüsse mit 1 Gbit/s gehen, das Auto spricht 100 Mbit/s.

## Kommandozeile

```
caruspex_cli ecus --enet auto
```
