# Безопасность, товарные знаки и лицензии

## Безопасность и ответственность

Caruspex является свободным любительским проектом и поставляется без каких-либо гарантий, как сказано в [лицензии MIT](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE). Вы используете его на свой страх и риск.

- Никогда не пользуйтесь им во время движения. Настраивайте его на стоящем автомобиле; за значениями в реальном времени может следить пассажир.
- Стирание ошибок, тесты исполнительных механизмов и базовые настройки заставляют автомобиль что-то делать. Caruspex спрашивает перед каждым из них; выполняйте их только на стоящем и зафиксированном автомобиле и только на тех машинах, с которыми вам разрешено работать.
- Caruspex не заменяет автосервис. Код ошибки указывает на симптом, а не всегда на причину. Тормоза, подушки безопасности, рулевое управление и всё, что важно для безопасности, доверяйте специалистам.
- Стирание не является ремонтом. Стёртые ошибки и сброшенные тесты готовности могут привести к тому, что автомобиль не пройдёт проверку выхлопа или техосмотр.
- Caruspex не меняет пробег, иммобилайзер, ключи, систему очистки выхлопа или тахограф, и проект не принимает изменения, которые это делают. Вмешательство в одометры, систему очистки выхлопа или тахографы во многих странах запрещено законом.

## Товарные знаки

Названия производителей автомобилей, моделей, адаптеров, программ и стандартов (например Volkswagen, Audi, BMW, Mercedes-Benz, Tesla, OBDLink, ELM327, VCDS или ISTA) являются товарными знаками их владельцев. Они приводятся только для того, чтобы сказать, с какими автомобилями, адаптерами и интерфейсами работает Caruspex. Caruspex является независимым проектом и не связан ни с одним производителем автомобилей, производителем адаптеров или поставщиком программ, не одобрен и не спонсируется ими. Приложение и этот репозиторий не используют логотипы этих компаний.

## Лицензия и источники

Код распространяется по лицензии MIT, см. [LICENSE](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE). Библиотеки в программах и их лицензии перечислены в [THIRD_PARTY_NOTICES.md](https://github.com/SenseiIssei/Caruspex/blob/main/THIRD_PARTY_NOTICES.md); приложение показывает их в настройках. Данные, встроенные в программы:

- Значения производителей примерно для 550 автомобилей, преобразованные из [OBDb](https://github.com/OBDb), как и OBDb, под CC BY-SA 4.0: хранятся отдельно от кода в [data/obdb](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb) вместе с лицензией и списком источников ([ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb/ATTRIBUTION.md)).
- Описания сигналов CAN из [opendbc](https://github.com/commaai/opendbc) (comma.ai) и [model3dbc](https://github.com/joshwardell/model3dbc), лицензия MIT ([data/opendbc](https://github.com/SenseiIssei/Caruspex/blob/main/data/opendbc), [data/tesla](https://github.com/SenseiIssei/Caruspex/blob/main/data/tesla)).
- Значения батареи для Hyundai Ioniq и Kona, Kia e-Niro, Nissan Leaf и Renault Zoe по OBDb (CC BY-SA 4.0, как и эти пакеты), [Open Vehicle Monitoring System](https://github.com/openvehicles/Open-Vehicle-Monitoring-System-3) и [evDash](https://github.com/nickn17/evDash) (оба MIT): [data/pids](https://github.com/SenseiIssei/Caruspex/blob/main/data/pids) ([ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/pids/ATTRIBUTION.md)).
- Коды двигателей из статей Википедии, CC BY-SA 4.0 ([data/engines](https://github.com/SenseiIssei/Caruspex/blob/main/data/engines), со статьёй и версией в каждой строке).
- Коды производителей VIN сверены с [NHTSA vPIC](https://vpic.nhtsa.dot.gov) (общественное достояние) и [Wikidata](https://www.wikidata.org) (CC0); коды, которые есть только в [списке WMI в Викиучебнике](https://en.wikibooks.org/wiki/Vehicle_Identification_Numbers_%28VIN_codes%29/World_Manufacturer_Identifier_%28WMI%29), распространяются по CC BY-SA 4.0 и хранятся отдельно в [data/wmi](https://github.com/SenseiIssei/Caruspex/blob/main/data/wmi).
- Размеры автомобилей из [Викиданных](https://www.wikidata.org), CC0.
- Шрифты по лицензии SIL Open Font License 1.1: JetBrains Mono ([OFL.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-Inter.txt)), Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-PlusJakartaSans.txt)).
