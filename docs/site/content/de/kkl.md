# KKL-Kabel (VAG 409.1)

Ein KKL-Kabel ist ein USB-Kabel mit einem seriellen Chip und einem K-Line-Treiber, ohne ELM327 darin. Caruspex steuert die K-Line selbst, Byte für Byte. Genau das brauchen ältere VW, Audi, Seat und Skoda: Ihr Protokoll KWP1281 bestätigt jedes einzelne Byte, und das kann kein ELM327.

## Was geht

- Alle Steuergeräte von VW, Audi, Seat und Skoda von etwa 1990 bis 2004: Motor, Getriebe, ABS, Airbag, Kombiinstrument, Komfort und mehr.
- KWP1281 und KWP2000 (ISO 14230), automatisch erkannt an den Schlüsselbytes nach dem Wecken.
- Fehlerspeicher je Steuergerät, Messwertblöcke und die echte Grundeinstellung der Drosselklappe (Messwertblock 060 oder 098).
- Anpassungskanäle werden mit Absicht nur gelesen. Die Stellglieddiagnose schaltet echte Teile und startet erst nach einer Bestätigung.

Diese Seiten gibt es im **Profimodus**: **Steuergeräte über KKL**, **Drosselklappe** und **Anpassung und Stellglieder**. Jede öffnet das Kabel selbst: seriellen Anschluss wählen, Steuergerät wählen, starten.

## Welches Kabel?

Kabel mit einem echten FTDI FT232RL oder einem CP2102 sind an der K-Line viel zuverlässiger als die billigsten Nachbauten. Für KWP1281 ist ein echter FTDI zu empfehlen. Das Kabel muss die K-Line roh durchreichen: "HEX-USB"-Interfaces mit eigener Firmware gehen nicht.

Autos vor etwa 1996 haben oft keine OBD-Buchse, sondern einen 2x2-Stecker im Motorraum oder unter dem Armaturenbrett. Ein Adapter von 2x2 auf die 16-polige Buchse kostet wenig.

## Das Kabel prüfen

Das Kommandozeilenwerkzeug prüft das Kabel ohne Auto:

```
caruspex_cli kklcheck --port COM3
```

## Typische Probleme

- **Keine Antwort vom Steuergerät**: Zündung an? Das Wecken mit 5 Baud braucht genaues Timing; USB-Hubs und manche virtuellen Maschinen verderben es, das Kabel direkt anschließen.
- **Bricht nach einer Weile ab**: Billige Kabel verlieren Bytes. Erst ein anderes Kabel probieren, bevor man das Auto verdächtigt.
