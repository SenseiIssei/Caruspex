# Безпека, торговельні марки та ліцензії

## Безпека та відповідальність

Caruspex є вільним аматорським проєктом і постачається без жодних гарантій, як зазначено в [ліцензії MIT](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE). Ви користуєтеся ним на власний ризик.

- Ніколи не користуйтеся ним під час руху. Налаштовуйте його на нерухомому авто; за значеннями в реальному часі може стежити пасажир.
- Стирання помилок, тести виконавчих механізмів і базові налаштування змушують авто щось робити. Caruspex питає перед кожним із них; виконуйте їх лише на нерухомому й зафіксованому авто і лише на тих машинах, з якими вам дозволено працювати.
- Caruspex не замінює автосервіс. Код помилки вказує на симптом, а не завжди на причину. Гальма, подушки безпеки, кермо і все, що важливо для безпеки, довіряйте фахівцям.
- Стирання не є ремонтом. Стерті помилки та скинуті тести готовності можуть призвести до того, що авто не пройде перевірку викидів або техогляд.
- Caruspex не змінює пробіг, іммобілайзер, ключі, систему очищення вихлопу чи тахограф, і проєкт не приймає змін, які це роблять. Втручання в одометри, систему очищення вихлопу чи тахографи в багатьох країнах заборонене законом.

## Торговельні марки

Назви виробників автомобілів, моделей, адаптерів, програм і стандартів (наприклад Volkswagen, Audi, BMW, Mercedes-Benz, Tesla, OBDLink, ELM327, VCDS чи ISTA) є торговельними марками їхніх власників. Вони наведені лише для того, щоб сказати, з якими автомобілями, адаптерами та інтерфейсами працює Caruspex. Caruspex є незалежним проєктом і не пов'язаний з жодним виробником автомобілів, виробником адаптерів чи постачальником програм, не схвалений і не спонсорований ними. Застосунок і цей репозиторій не використовують логотипів цих компаній.

## Ліцензія та джерела

Код поширюється за ліцензією MIT, див. [LICENSE](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE). Бібліотеки в програмах та їхні ліцензії перелічено в [THIRD_PARTY_NOTICES.md](https://github.com/SenseiIssei/Caruspex/blob/main/THIRD_PARTY_NOTICES.md); застосунок показує їх у налаштуваннях. Дані, вбудовані в програми:

- Значення виробників приблизно для 550 автомобілів, перетворені з [OBDb](https://github.com/OBDb), як і OBDb, під CC BY-SA 4.0: зберігаються окремо від коду в [data/obdb](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb) разом з ліцензією та списком джерел ([ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb/ATTRIBUTION.md)).
- Описи сигналів CAN з [opendbc](https://github.com/commaai/opendbc) (comma.ai) і [model3dbc](https://github.com/joshwardell/model3dbc), ліцензія MIT ([data/opendbc](https://github.com/SenseiIssei/Caruspex/blob/main/data/opendbc), [data/tesla](https://github.com/SenseiIssei/Caruspex/blob/main/data/tesla)).
- Коди двигунів зі статей Вікіпедії, CC BY-SA 4.0 ([data/engines](https://github.com/SenseiIssei/Caruspex/blob/main/data/engines), зі статтею та версією в кожному рядку).
- Частина кодів виробників VIN за [списком WMI у Вікіпідручнику](https://en.wikibooks.org/wiki/Vehicle_Identification_Numbers_%28VIN_codes%29/World_Manufacturer_Identifier_%28WMI%29), CC BY-SA 4.0.
- Розміри автомобілів з [Вікіданих](https://www.wikidata.org), CC0.
- Шрифти за ліцензією SIL Open Font License 1.1: JetBrains Mono ([OFL.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-Inter.txt)), Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-PlusJakartaSans.txt)).
