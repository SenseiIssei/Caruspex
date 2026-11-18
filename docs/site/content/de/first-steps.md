# Erste Schritte

## 1. Das Demo-Auto ausprobieren

Zum Umschauen braucht man kein Auto. Auf der Seite **Verbinden** **Demo-Auto** wählen: Es fährt, wird warm, hat ein paar Fehler gespeichert und antwortet wie ein echter ELM327. Jede Seite funktioniert damit.

## 2. Sprache und Modus wählen

Caruspex folgt der Sprache des Systems. In den **Einstellungen** lässt sich eine der 18 Sprachen und das helle oder dunkle Aussehen wählen; beides wechselt beim nächsten Start.

Der **Einsteigermodus** zeigt die Seiten für den Alltag. Der **Profimodus** zeigt dazu die Werkstattwerkzeuge: die KKL-Seiten, Anpassung, rohe Adapterbefehle und Steuergeräteadressen.

## 3. Das Auto verbinden

1. Die Diagnosebuchse finden: Der [Verbindungsassistent](connect.md) zeigt, wo sie im eigenen Auto sitzt.
2. Den Adapter einstecken und die Zündung einschalten. Der Motor darf laufen, muss aber nicht.
3. Auf der Seite **Verbinden** die Art der Verbindung wählen oder Caruspex alle Anschlüsse selbst durchsuchen lassen.

Beim ersten Start öffnet sich der Verbindungsassistent von selbst. Jeder Schritt lässt sich überspringen, und der Assistent kommt aus den Einstellungen und von der Seite Verbinden zurück.

![Die Seite Verbinden](screenshot-connect.png)

## 4. Ein erster Blick aufs Auto

Nach dem Verbinden liest Caruspex die VIN und sagt, welches Auto es ist. Gute Stellen zum Anfangen:

- **Gesundheitscheck**: ein Knopf, eine halbe Minute, eine Punktzahl von 0 bis 100, jeder Abzug erklärt.
- **Was soll ich jetzt tun?**: alles Gefundene, in drei Stufen sortiert.
- Der [Fehlerspeicher](faults.md) und die [Live-Werte](live.md).

![Gesundheitscheck](screenshot-health.png)

## 5. Garage und Berichte

Jedes Auto wird an seiner VIN erkannt und bekommt einen eigenen Eintrag in der Garage, mit dem Verlauf seiner Prüfungen und Kilometerstände. Der Prüfbericht lässt sich als PDF für die Werkstatt speichern.
