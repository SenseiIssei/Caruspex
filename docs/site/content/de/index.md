# Caruspex

Caruspex ist ein freies Diagnoseprogramm für Autos und Motorräder. Es läuft unter Windows, Linux und macOS, spricht 18 Sprachen und arbeitet mit den Adaptern, die man wirklich hat: dem billigen ELM327-Stecker, einem OBDLink, einem KKL-Kabel für ältere Autos aus dem VW-Konzern, einem ENET-Kabel für BMW und professionellen CAN-Interfaces.

![Die Übersicht mit Live-Anzeigen](screenshot-overview.png)

## Was es kann

- Liest und löscht Fehlercodes, erklärt sie in einfachen Worten und sagt, ob man weiterfahren darf.
- Zeigt Live-Werte: Temperaturen, Geschwindigkeit, Gemisch, Ladedruck, Batteriespannung und vieles mehr, als Zahl und als Kurve.
- Fragt alle Steuergeräte des Autos ab, nicht nur den Motor, soweit Auto und Adapter es zulassen.
- Prüft einen Gebrauchtwagen vor dem Kauf: Passt die VIN, stimmen die Kilometerstände überein, wurden kurz vorher Fehler gelöscht?
- Zeigt das Auto als Glasmodell, in dem Fehler und Werte dort aufleuchten, wo sie hingehören.
- Sagt, was als Nächstes zu tun ist: sofort anhalten, bald in die Werkstatt oder es kann warten.

## Wo anfangen

1. [Caruspex installieren](install.md).
2. Den [ersten Schritten](first-steps.md) folgen: erst das Demo-Auto ausprobieren, dann das eigene verbinden.
3. Den eigenen Adapter unter [Adapter und Schnittstellen](interfaces.md) nachschlagen.
4. Unter [Datenschutz](privacy.md) lesen, was Caruspex speichert und sendet. Kurz gesagt: Nichts verlässt den Rechner, außer man teilt es selbst.

> Caruspex liest nur. Es codiert keine Steuergeräte, verändert nie den Kilometerstand und fasst die Wegfahrsperre nicht an. Fehlercodes löschen ist das Einzige, was es ins Auto schreibt.

## Ehrlicher Stand

Caruspex ist nach den Normen gebaut und gegen nachgebildete Steuergeräte, falsche Gateways und falsche Treiber getestet. An echter Hardware wurde bisher nur ein OBDLink mit dem Chip STN2232 geprüft, ohne Auto. Wer Caruspex am eigenen Auto ausprobiert, hilft mit einem [Testbericht](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml): Jeder Bericht macht die Liste der geprüften Autos länger.
