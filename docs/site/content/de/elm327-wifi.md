# ELM327 über WLAN

Manche Adapter öffnen ein eigenes WLAN. Der Rechner tritt ihm bei und spricht über TCP mit dem Adapter.

## Verbinden

1. Den Adapter einstecken und die Zündung einschalten. Nach ein paar Sekunden erscheint ein WLAN, oft mit dem Namen `WiFi_OBDII`, `V-LINK` oder nach dem Hersteller.
2. Den Rechner mit diesem WLAN verbinden. Es hat kein Internet, deshalb warnt das System vielleicht, dass die Verbindung eingeschränkt ist: Das ist in Ordnung.
3. In Caruspex auf der Seite **Verbinden** **WLAN** wählen. Die meisten Adapter nutzen die Adresse `192.168.0.10` und den Port `35000`; das ist voreingestellt. Nutzt der eigene eine andere (zum Beispiel `192.168.4.1`), sie als `address:port` eintragen.

## Tipps

- Solange der Rechner im WLAN des Adapters ist, hat er kein Internet. Für Caruspex spielt das keine Rolle.
- WLAN-Adapter gehen auf jedem System ohne Treiber.
- Bricht die Verbindung ab, mit dem Rechner näher ans Auto gehen: Der Funk dieser Stecker ist schwach.
