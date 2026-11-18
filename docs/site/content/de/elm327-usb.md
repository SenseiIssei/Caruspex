# ELM327 über USB

Ein Stecker für die OBD-Buchse mit einem Kabel zum USB-Anschluss. Das ist die zuverlässigste Art von ELM327: kein Koppeln, kein Funk, keine leeren Akkus.

## Treiber

Im Stecker macht ein USB-Chip aus dem Kabel einen seriellen Anschluss (unter Windows einen COM-Port). Welcher Treiber nötig ist, hängt von diesem Chip ab:

| Chip | Treiber |
|---|---|
| FTDI FT232 | richtet Windows meist selbst ein, in Linux und macOS eingebaut |
| CH340 / CH341 | richtet Windows meist selbst ein, in Linux und macOS eingebaut |
| Silicon Labs CP210x | von der Seite des Herstellers, wenn Windows ihn nicht findet |
| Prolific PL2303 | von der Seite des Herstellers; alte Nachbauten gehen oft nur mit einem alten Treiber |

Nach dem Einstecken zeigt der Geräte-Manager von Windows den Adapter unter "Anschlüsse (COM & LPT)".

## Verbinden

1. Den Adapter ins Auto stecken, die Zündung einschalten, das Kabel in den Rechner stecken.
2. Auf der Seite **Verbinden** **USB oder COM-Port** wählen.
3. Den Anschluss auf **Automatisch suchen** lassen oder aus der Liste wählen, dann verbinden.

Die automatische Suche probiert jeden seriellen Anschluss mit den üblichen Geschwindigkeiten. Wer die Geschwindigkeit seines Adapters kennt, stellt sie in den Einstellungen unter **Baudrate des Adapters** ein.

## Linux und macOS

Unter Linux heißt der Adapter `/dev/ttyUSB0` oder `/dev/ttyACM0`. Der eigene Benutzer muss in der Gruppe `dialout` sein (siehe [Installieren](install.md)). Unter macOS heißt er `/dev/cu.usbserial-...` oder `/dev/cu.wchusbserial...`.

## Typische Probleme

- **Kein Anschluss erscheint**: Der Treiber fehlt, oder das Kabel ist ein reines Ladekabel.
- **Adapter antwortet, Auto nicht**: Zündung aus, oder der Adapter ist ein Nachbau, der das Protokoll des Autos nicht beherrscht. Der [Adaptertest](interfaces.md) sagt mehr.
- **Sehr langsame Live-Werte bei älteren Autos**: Die K-Line selbst ist langsam, das liegt nicht am Adapter.
