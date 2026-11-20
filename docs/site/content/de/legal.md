# Sicherheit, Marken und Lizenzen

## Sicherheit und Haftung

Caruspex ist ein freies Hobbyprojekt und kommt ohne jede Gewährleistung, so wie es die [MIT-Lizenz](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE) sagt. Die Nutzung geschieht auf eigene Gefahr.

- Nie während der Fahrt bedienen. Eingerichtet wird im Stand; die Live-Werte kann ein Beifahrer im Blick behalten.
- Fehler löschen, Stellglieddiagnose und Grundeinstellungen lassen das Auto etwas tun. Caruspex fragt vor jedem davon nach; nur im Stand mit gesichertem Auto und nur an Fahrzeugen, an denen man arbeiten darf.
- Caruspex ersetzt keine Werkstatt. Ein Fehlercode nennt ein Symptom, nicht immer die Ursache. Bremsen, Airbags, Lenkung und alles, was für die Sicherheit zählt, gehören in fachkundige Hände.
- Löschen ist keine Reparatur. Gelöschte Fehler und zurückgesetzte Bereitschaftstests können eine Abgasuntersuchung oder Hauptuntersuchung scheitern lassen.
- Caruspex ändert weder Kilometerstand noch Wegfahrsperre, Schlüssel, Abgasreinigung oder Tachograph, und das Projekt nimmt keine Beiträge an, die das tun. Manipulationen an Kilometerzählern, Abgasreinigung oder Tachographen sind in vielen Ländern verboten.

## Marken

Namen von Fahrzeugherstellern, Modellen, Adaptern, Programmen und Normen (zum Beispiel Volkswagen, Audi, BMW, Mercedes-Benz, Tesla, OBDLink, ELM327, VCDS oder ISTA) sind Marken ihrer jeweiligen Inhaber. Sie stehen hier nur, um zu sagen, mit welchen Autos, Adaptern und Schnittstellen Caruspex arbeitet. Caruspex ist ein unabhängiges Projekt und mit keinem Fahrzeughersteller, Adapterhersteller oder Softwareanbieter verbunden, von keinem empfohlen und von keinem unterstützt. Die App und dieses Repository verwenden keine Logos dieser Firmen.

## Lizenz und Quellen

Der Code steht unter der MIT-Lizenz, siehe [LICENSE](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE). Die Bibliotheken in den Programmen und ihre Lizenzen stehen in [THIRD_PARTY_NOTICES.md](https://github.com/SenseiIssei/Caruspex/blob/main/THIRD_PARTY_NOTICES.md); die App zeigt sie in den Einstellungen. In die Programme eingebaute Daten:

- Herstellerwerte für rund 550 Fahrzeuge, umgewandelt aus [OBDb](https://github.com/OBDb), wie OBDb unter CC BY-SA 4.0: getrennt vom Code in [data/obdb](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb) mit der Lizenz und der Liste der Quellen ([ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb/ATTRIBUTION.md)).
- CAN-Signale aus [opendbc](https://github.com/commaai/opendbc) (comma.ai) und [model3dbc](https://github.com/joshwardell/model3dbc), MIT-Lizenz ([data/opendbc](https://github.com/SenseiIssei/Caruspex/blob/main/data/opendbc), [data/tesla](https://github.com/SenseiIssei/Caruspex/blob/main/data/tesla)).
- Motorkennbuchstaben aus Wikipedia-Artikeln, CC BY-SA 4.0 ([data/engines](https://github.com/SenseiIssei/Caruspex/blob/main/data/engines), mit Artikel und Version in jeder Zeile).
- Ein Teil der Herstellerkennungen der FIN nach der [WMI-Liste auf Wikibooks](https://en.wikibooks.org/wiki/Vehicle_Identification_Numbers_%28VIN_codes%29/World_Manufacturer_Identifier_%28WMI%29), CC BY-SA 4.0.
- Fahrzeugmaße aus [Wikidata](https://www.wikidata.org), CC0.
- Schriften unter der SIL Open Font License 1.1: JetBrains Mono ([OFL.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-Inter.txt)), Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-PlusJakartaSans.txt)).
