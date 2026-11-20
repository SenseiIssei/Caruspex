# Datenschutz

Caruspex arbeitet auf dem eigenen Rechner. Es gibt kein Konto, keine Cloud und kein Tracking.

## Was gespeichert wird

Alles bleibt auf dem eigenen Rechner:

| Was | Wann | Wo |
|---|---|---|
| Einstellungen | immer | im Einstellungsordner des Systems (der Knopf **Ablageort öffnen** in den Einstellungen zeigt ihn) |
| Garage: VIN, Auto, Prüfungen, Kilometerstände, der letzte funktionierende Weg zum Auto | nach dem Verbinden mit einem Auto | neben den Einstellungen im Datenordner von Caruspex |
| Aufnahmen und CSV-Dateien | nur, wenn man sie startet | wo man sie speichert |
| Protokolldatei | nur, wenn man sie in den Einstellungen einschaltet | am gewählten Pfad |
| Absturzberichte | nur, wenn Caruspex abstürzt | im Ordner `crash-reports` neben der Garage |
| Testberichte des ersten KKL-Tests | nur, wenn man einen speichert | im Ordner `test-reports` im Einstellungsordner; FIN und Wegfahrsperrennummer werden gekürzt, außer man setzt das Häkchen |

Jede dieser Dateien lässt sich jederzeit löschen.

## Was gesendet wird

Beim Start fragt Caruspex bei GitHub nach, ob es eine neuere Version gibt. Dabei wird nichts über einen selbst oder das Auto mitgeschickt. Die Suche lässt sich in den Einstellungen abschalten. Sonst geht Caruspex nicht ins Internet. Das Gespräch mit einem WLAN-Adapter, einem ENET-Kabel oder einem DoIP-Auto bleibt im lokalen Netz.

Nach Updates suchen nur die auf GitHub veröffentlichten Release-Builds; eine selbst gebaute Kopie tut das nie. Wie bei jeder Anfrage im Internet sieht GitHub bei der Suche und beim Herunterladen eines Updates die IP-Adresse und die Version von Caruspex; dafür gilt die Datenschutzerklärung von GitHub. Updates kommen nur aus den Releases des Caruspex-Repositorys. Knöpfe wie **Auf GitHub melden** öffnen nur den Browser; was man dort absendet, entscheidet man selbst.

## Android

Bis Android 11 lässt das System eine App Bluetooth-LE-Adapter nur mit der Standortberechtigung finden. Caruspex fragt nur deshalb danach und liest den Standort nie. Ab Android 12 fragt es stattdessen nach der Bluetooth-Berechtigung, die als nie für den Standort genutzt gekennzeichnet ist.

## Absturzberichte nur auf Wunsch

Stürzt Caruspex ab, schreibt es einen kurzen Bericht auf den Rechner: Version, System, die Stelle im Code und die Aufrufkette. VINs, die Bytes des Verkehrs mit dem Auto und der Heimordner werden herausgenommen. Verschickt wird nichts. Caruspex zeigt den Bericht und bietet zwei Knöpfe: **Auf GitHub melden** öffnet im Browser ein vorausgefülltes Issue, das man vor dem Absenden liest und ändert, **Nicht melden** legt ihn ab.

## Mitschnitte ohne persönliche Daten teilen

Mitschnitte echter Autos helfen, Caruspex für mehr Autos besser zu machen. **Mitschnitt teilen** bereitet eine Kopie vor und zeigt genau, was bleibt und was rausgeht:

- **Entfernt**: die letzten Stellen der VIN (überall in der Datei), der Name des Anschlusses und die Bluetooth- oder Netzwerkadresse des Adapters, die Uhrzeit.
- **Nie in einer Aufnahme**: Standort und Nutzerdaten.
- **Bleibt**: die Anfragen und Antworten, die Art des Adapters, das Protokoll und das Datum.

Die Datei bleibt auf dem Rechner, bis man sie selbst speichert und an ein GitHub-Issue hängt. Hochgeladen wird nichts von selbst.

## Das Testprogramm

Das [Formular für Testberichte](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml) fragt nach Auto, Baujahr, Motor und Adapter. Es fragt nur nach einer gekürzten VIN; nie die ganze VIN, das Kennzeichen oder den eigenen Namen dort eintragen.
