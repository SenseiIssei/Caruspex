# DoIP (Diagnose über IP)

Viele Autos ab etwa 2016 lassen sich über Ethernet nach ISO 13400 diagnostizieren, kurz DoIP. Ein DoIP-Kabel verbindet die OBD-Buchse mit dem Netzwerkanschluss des Rechners.

## Verbinden

1. Das Kabel einstecken, die Zündung einschalten und warten, bis der Rechner seine automatische Adresse (169.254.x.x) hat, das kann bis zu einer Minute dauern.
2. Auf der Seite **Verbinden** **Netzwerkkabel** wählen und das Feld leer lassen. Caruspex sucht DoIP-Fahrzeuge im Netz und erkennt selbst, ob das Auto DoIP oder das HSFZ von BMW spricht.

## Was geht

OBD über DoIP und die Steuergeräte über UDS, deren Adressen Caruspex für die Marke kennt. Die Adresslisten für DoIP-Autos sind noch kurz; Mitschnitte echter Autos füllen sie (was ein geteilter Mitschnitt enthält, steht unter [Datenschutz](privacy.md)).

## Welches Kabel?

DoIP braucht die Ethernet-Leitungen an Pin 3, 11, 12 und 13 der OBD-Buchse und eine Aktivierungsleitung an Pin 8. Kabel, die für eine bestimmte Marke verkauft werden, sind meist genau so beschaltet; das ENET-Kabel von BMW ist eines davon.

## Kommandozeile

```
caruspex_cli enet
```

listet die BMW- und DoIP-Fahrzeuge, die im Netz gefunden werden.
