# Segurança, marcas e licenças

## Segurança e responsabilidade

O Caruspex é um projeto livre de hobby e vem sem qualquer garantia, como diz a [licença MIT](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE). Usa-o por sua conta e risco.

- Nunca o utilize durante a condução. Configure-o com o carro parado; um passageiro pode acompanhar os valores ao vivo.
- Apagar falhas, testes de atuadores e ajustes básicos fazem o carro fazer algo. O Caruspex pergunta antes de cada um; faça-os só com o carro parado e imobilizado, e só em veículos em que pode trabalhar.
- O Caruspex não substitui uma oficina. Um código de falha indica um sintoma, nem sempre a causa. Travões, airbags, direção e tudo o que importa para a segurança devem ficar em mãos qualificadas.
- Apagar não é reparar. Falhas apagadas e monitores de prontidão repostos podem fazer falhar uma inspeção de emissões ou periódica.
- O Caruspex não altera a quilometragem, o imobilizador, as chaves, o controlo de emissões nem o tacógrafo, e o projeto não aceita contribuições que o façam. Manipular conta-quilómetros, o controlo de emissões ou tacógrafos é ilegal em muitos países.

## Marcas

Os nomes de fabricantes de veículos, modelos, adaptadores, programas e normas (por exemplo Volkswagen, Audi, BMW, Mercedes-Benz, Tesla, OBDLink, ELM327, VCDS ou ISTA) são marcas dos seus respetivos titulares. Aparecem apenas para dizer com que carros, adaptadores e interfaces o Caruspex funciona. O Caruspex é um projeto independente e não está ligado, aprovado nem patrocinado por nenhum fabricante de veículos, fabricante de adaptadores ou fornecedor de software. A aplicação e este repositório não usam logótipos destas empresas.

## Licença e créditos

O código está sob a licença MIT, veja [LICENSE](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE). As bibliotecas dos programas e as suas licenças estão em [THIRD_PARTY_NOTICES.md](https://github.com/SenseiIssei/Caruspex/blob/main/THIRD_PARTY_NOTICES.md); a aplicação mostra-as nas definições. Dados incluídos nos programas:

- Valores de fabricante para cerca de 550 veículos, convertidos do [OBDb](https://github.com/OBDb), sob CC BY-SA 4.0 como o OBDb: separados do código em [data/obdb](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb) com a licença e a lista de fontes ([ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb/ATTRIBUTION.md)).
- Definições de sinais CAN do [opendbc](https://github.com/commaai/opendbc) (comma.ai) e do [model3dbc](https://github.com/joshwardell/model3dbc), licença MIT ([data/opendbc](https://github.com/SenseiIssei/Caruspex/blob/main/data/opendbc), [data/tesla](https://github.com/SenseiIssei/Caruspex/blob/main/data/tesla)).
- Valores de bateria do Hyundai Ioniq e Kona, do Kia e-Niro, do Nissan Leaf e do Renault Zoe segundo o OBDb (CC BY-SA 4.0, como estes pacotes), o [Open Vehicle Monitoring System](https://github.com/openvehicles/Open-Vehicle-Monitoring-System-3) e o [evDash](https://github.com/nickn17/evDash) (ambos MIT): [data/pids](https://github.com/SenseiIssei/Caruspex/blob/main/data/pids) ([ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/pids/ATTRIBUTION.md)).
- Códigos de motor de artigos da Wikipédia, CC BY-SA 4.0 ([data/engines](https://github.com/SenseiIssei/Caruspex/blob/main/data/engines), com artigo e revisão em cada linha).
- Códigos de fabricante do VIN verificados no [NHTSA vPIC](https://vpic.nhtsa.dot.gov) (domínio público) e no [Wikidata](https://www.wikidata.org) (CC0); os códigos que só a [lista WMI do Wikibooks](https://en.wikibooks.org/wiki/Vehicle_Identification_Numbers_%28VIN_codes%29/World_Manufacturer_Identifier_%28WMI%29) tem estão sob CC BY-SA 4.0 e ficam à parte em [data/wmi](https://github.com/SenseiIssei/Caruspex/blob/main/data/wmi).
- Dimensões dos veículos do [Wikidata](https://www.wikidata.org), CC0.
- Fontes sob a SIL Open Font License 1.1: JetBrains Mono ([OFL.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-Inter.txt)), Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-PlusJakartaSans.txt)).
