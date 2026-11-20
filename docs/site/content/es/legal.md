# Seguridad, marcas y licencias

## Seguridad y responsabilidad

Caruspex es un proyecto libre de aficionado y se ofrece sin ninguna garantía, como dice la [licencia MIT](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE). Lo usas bajo tu propia responsabilidad.

- No lo uses nunca mientras conduces. Configúralo con el coche parado; un acompañante puede vigilar los valores en vivo.
- Borrar fallos, las pruebas de actuadores y los ajustes básicos hacen que el coche haga algo. Caruspex pregunta antes de cada uno; hazlos solo con el coche parado y asegurado, y solo en vehículos en los que puedas trabajar.
- Caruspex no sustituye a un taller. Un código de fallo indica un síntoma, no siempre su causa. Frenos, airbags, dirección y todo lo que importa para la seguridad debe quedar en manos expertas.
- Borrar no es reparar. Los fallos borrados y los monitores de disponibilidad reiniciados pueden hacer que falle una inspección de emisiones o técnica.
- Caruspex no cambia el kilometraje, el inmovilizador, las llaves, el control de emisiones ni el tacógrafo, y el proyecto no acepta contribuciones que lo hagan. Manipular cuentakilómetros, el control de emisiones o tacógrafos es ilegal en muchos países.

## Marcas

Los nombres de fabricantes de vehículos, modelos, adaptadores, programas y normas (por ejemplo Volkswagen, Audi, BMW, Mercedes-Benz, Tesla, OBDLink, ELM327, VCDS o ISTA) son marcas de sus respectivos titulares. Aparecen solo para indicar con qué coches, adaptadores e interfaces funciona Caruspex. Caruspex es un proyecto independiente y no está vinculado, respaldado ni patrocinado por ningún fabricante de vehículos, fabricante de adaptadores ni proveedor de software. La aplicación y este repositorio no usan logotipos de estas empresas.

## Licencia y créditos

El código está bajo la licencia MIT, ver [LICENSE](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE). Las bibliotecas de los programas y sus licencias están en [THIRD_PARTY_NOTICES.md](https://github.com/SenseiIssei/Caruspex/blob/main/THIRD_PARTY_NOTICES.md); la aplicación las muestra en los ajustes. Datos incluidos en los programas:

- Valores de fabricante para unos 550 vehículos, convertidos de [OBDb](https://github.com/OBDb), bajo CC BY-SA 4.0 como OBDb: separados del código en [data/obdb](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb) con la licencia y la lista de fuentes ([ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb/ATTRIBUTION.md)).
- Definiciones de señales CAN de [opendbc](https://github.com/commaai/opendbc) (comma.ai) y [model3dbc](https://github.com/joshwardell/model3dbc), licencia MIT ([data/opendbc](https://github.com/SenseiIssei/Caruspex/blob/main/data/opendbc), [data/tesla](https://github.com/SenseiIssei/Caruspex/blob/main/data/tesla)).
- Códigos de motor de artículos de Wikipedia, CC BY-SA 4.0 ([data/engines](https://github.com/SenseiIssei/Caruspex/blob/main/data/engines), con artículo y revisión en cada fila).
- Parte de los códigos de fabricante del VIN según la [lista WMI de Wikibooks](https://en.wikibooks.org/wiki/Vehicle_Identification_Numbers_%28VIN_codes%29/World_Manufacturer_Identifier_%28WMI%29), CC BY-SA 4.0.
- Dimensiones de los vehículos de [Wikidata](https://www.wikidata.org), CC0.
- Fuentes bajo la SIL Open Font License 1.1: JetBrains Mono ([OFL.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-Inter.txt)), Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-PlusJakartaSans.txt)).
