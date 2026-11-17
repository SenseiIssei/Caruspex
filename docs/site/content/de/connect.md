# Verbindungsassistent und OBD-Buchse

## Wo ist die Buchse?

Jedes Auto, das seit 2001 (Benziner) oder 2004 (Diesel) in Europa und seit 1996 in den USA verkauft wurde, hat eine 16-polige OBD-Buchse in Reichweite des Fahrers. Übliche Stellen:

- unter dem Armaturenbrett links oder rechts der Lenksäule,
- über den Pedalen,
- in der Seitenverkleidung des Fußraums,
- hinter dem Deckel des Sicherungskastens,
- in der Mittelkonsole, manchmal hinter einer kleinen Klappe.

Für mehr als 200 Auto-Generationen und 35 Motorräder kennt Caruspex die genaue Stelle. Der Verbindungsassistent markiert sie im Grundriss und in einer Nahansicht der Fahrerseite; bei Motorrädern zeigt er eine Seitenansicht mit dem Diagnosestecker und dem nötigen Adapterkabel.

> Die Liste der Buchsen ist eine eigene Zusammenstellung und nicht an jedem Auto geprüft. Stimmt die Stelle beim eigenen Auto nicht, bitte mit einem [Testbericht](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) Bescheid geben.

## Der Verbindungsassistent

Der Assistent öffnet sich beim ersten Start von selbst. Er fragt vier Dinge, jeder Schritt lässt sich überspringen:

1. **Welcher Adapter?** ELM327 mit USB, Bluetooth oder WLAN, ein ENET-Kabel oder ein KKL-Kabel. Bilder helfen, sie auseinanderzuhalten.
2. **Welches Auto?** Aus dem Katalog wählen, dann zeigt der Assistent, wo die Buchse sitzt.
3. **Zündung an.** Die meisten Steuergeräte antworten nur mit eingeschalteter Zündung.
4. **Verbinden.** Caruspex durchsucht die Anschlüsse und probiert die Protokolle.

Später findet man den Assistenten in den Einstellungen und auf der Seite Verbinden.

## Suchen und merken

Mit der automatischen Suche geht Caruspex alle seriellen Anschlüsse durch, fragt jeden nach einem ELM327 und probiert dann die Protokolle des Autos nacheinander. Hat ein Auto einmal verbunden, merkt sich Caruspex den Weg (Adapter, Anschluss, Protokoll, Bitrate) zu seiner VIN und probiert ihn beim nächsten Mal zuerst, das geht schneller.

## Wenn es nicht verbindet

Antwortet kein Adapter oder bleibt das Auto stumm, zeigt Caruspex eine kurze Checkliste für diese Art der Verbindung, zusammen mit dem, was es schon herausgefunden hat: Treiber installiert, Bluetooth gekoppelt, Zündung an, richtige Bitrate, ein Nachbau, der nur vorgibt, eine neuere Version zu sein. Die Seiten unter [Adapter und Schnittstellen](interfaces.md) erklären jede Art genau.
