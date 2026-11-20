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

## Erster Test am Auto

Die Profiseite **Erster Test am Auto** führt durch die erste Sitzung mit einem neuen KKL-Kabel. Sie ist auf den Audi A2 zugeschnitten und geht an jedem VW, Audi, Seat oder Skoda mit K-Line. Es wird nur gelesen: nichts gelöscht, kein Stellgliedtest, keine Grundeinstellung. Der Port wird erst beim Start eines Tests geöffnet.

1. **Das Kabel ohne Auto prüfen.** Das Kabel nur am USB anstecken, seinen Anschluss wählen und auf **Kabel prüfen** drücken. Caruspex liest den Chip (FTDI, CH340, Prolific, CP210x), beim FTDI den Latenz-Timer, und misst die Bits einer 5-Baud-Adresse auf der Break-Leitung. Ohne Auto kommt kein Echo zurück, das ist richtig so.
2. **Den Latenz-Timer auf 1 ms stellen** (FTDI-Kabel unter Windows): Geräte-Manager, Anschlüsse (COM & LPT), den USB Serial Port doppelt anklicken, Anschlusseinstellungen, Erweitert, Latency Timer (msec) auf 1, OK, dann das Kabel einmal ab- und wieder anstecken. Mit den voreingestellten 16 ms kann die Antwort auf die Schlüsselbytes ihr Fenster von 50 ms verpassen. Caruspex liest diese Einstellung, ändert sie aber nie.
3. **Am Auto:** Zündung an, Motor aus, das Kabel fest in der OBD-Buchse und direkt am Rechner (kein USB-Hub), kein anderes Diagnoseprogramm offen. Die Liste abhaken und auf **Ersten Test starten** drücken.
4. Der Test weckt den Motor (Adresse 01) und misst jedes Bit der Adresse, das Synchronbyte 0x55, die Schlüsselbytes und die Antwort darauf. Danach liest er Kennung, Fehlerspeicher und die Messwertblöcke 001 bis 003, dann Kombiinstrument 17, ABS 03, Airbag 15, Komfort 46 und Zentralelektrik 09. Ein Steuergerät, das nicht antwortet, ist vielleicht einfach nicht verbaut.
5. **Bericht speichern** legt eine Textdatei und eine JSON-Datei in den Ordner `test-reports` im Einstellungsordner von Caruspex. **Testbericht auf GitHub öffnen** füllt das Formular für den Testbericht aus. FIN und Wegfahrsperren-Nummer sind gekürzt, solange der Haken fehlt.

Was die Ergebnisse bedeuten:

| Ergebnis | Was tun |
|---|---|
| Kein Echo | Das Kabel hört sich selbst nicht: der Stecker sitzt nicht, oder an Pin 16 fehlen die 12 V (Sicherung der Diagnosebuchse prüfen). Ohne Auto ist das normal. |
| Das Echo ist verfälscht | Ein anderes Gerät an der Buchse, der Schalter eines K+DCAN-Kabels nicht auf K-Line, oder ein defektes Kabel. |
| Die Weckadresse ist zu ungenau getaktet | Die Bits weichen mehr als 20 ms von 200 ms ab: kein USB-Hub, andere Programme schließen, Latenz-Timer 1 ms. Caruspex versucht es noch einmal mit vorgezogenen Break-Aufrufen. |
| Kabel in Ordnung, aber keine Antwort vom Auto | Ist die Zündung an? Adresse 01 ist der Motor. Autos ab etwa 2005 werden über CAN diagnostiziert. |
| Die Antwort auf die Schlüsselbytes kam zu spät | Den Latenz-Timer auf 1 ms stellen. |

## Typische Probleme

- **Keine Antwort vom Steuergerät**: Zündung an? Das Wecken mit 5 Baud braucht genaues Timing; USB-Hubs und manche virtuellen Maschinen verderben es, das Kabel direkt anschließen.
- **Bricht nach einer Weile ab**: Billige Kabel verlieren Bytes. Erst ein anderes Kabel probieren, bevor man das Auto verdächtigt.
