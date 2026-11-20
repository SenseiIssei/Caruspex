# السلامة والعلامات التجارية والتراخيص

## السلامة والمسؤولية

Caruspex مشروع هواية حر ويُقدَّم بلا أي ضمان، كما ينص [ترخيص MIT](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE). تستخدمه على مسؤوليتك الخاصة.

- لا تستخدمه أبدًا أثناء القيادة. اضبطه والسيارة متوقفة؛ ويمكن لراكب أن يتابع القيم المباشرة.
- مسح الأعطال واختبارات المشغّلات والإعدادات الأساسية تجعل السيارة تقوم بشيء ما. يسأل Caruspex قبل كل منها؛ نفّذها فقط والسيارة متوقفة ومؤمّنة، وفقط على مركبات يحق لك العمل عليها.
- لا يغني Caruspex عن الورشة. رمز العطل يدل على عَرَض، وليس دائمًا على سببه. المكابح والوسائد الهوائية والتوجيه وكل ما يهم السلامة يجب أن يتولاه مختصون.
- المسح ليس إصلاحًا. قد يؤدي مسح الأعطال وإعادة ضبط اختبارات الجاهزية إلى الرسوب في فحص الانبعاثات أو الفحص الفني.
- لا يغيّر Caruspex عداد المسافة ولا نظام منع التشغيل ولا المفاتيح ولا نظام التحكم في الانبعاثات ولا جهاز التاكوغراف، ولا يقبل المشروع مساهمات تفعل ذلك. التلاعب بعدادات المسافة أو أنظمة الانبعاثات أو أجهزة التاكوغراف مخالف للقانون في كثير من الدول.

## العلامات التجارية

أسماء الشركات المصنّعة للمركبات والطرازات والمحوّلات والبرامج والمعايير (مثل Volkswagen وAudi وBMW وMercedes-Benz وTesla وOBDLink وELM327 وVCDS وISTA) علامات تجارية لأصحابها. وهي مذكورة فقط لبيان السيارات والمحوّلات والواجهات التي يعمل معها Caruspex. Caruspex مشروع مستقل لا يرتبط بأي شركة مصنّعة للمركبات أو للمحوّلات أو مزوّد برمجيات، ولا تعتمده أو ترعاه أي منها. لا يستخدم التطبيق ولا هذا المستودع شعارات هذه الشركات.

## الترخيص والمصادر

الشيفرة مرخّصة بموجب ترخيص MIT، انظر [LICENSE](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE). المكتبات الموجودة في البرامج وتراخيصها مدرجة في [THIRD_PARTY_NOTICES.md](https://github.com/SenseiIssei/Caruspex/blob/main/THIRD_PARTY_NOTICES.md)؛ ويعرضها التطبيق في الإعدادات. البيانات المضمّنة في البرامج:

- قيم الشركات المصنّعة لنحو 550 مركبة، محوّلة من [OBDb](https://github.com/OBDb)، ومرخّصة مثل OBDb بموجب CC BY-SA 4.0: منفصلة عن الشيفرة في [data/obdb](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb) مع الترخيص وقائمة المصادر ([ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb/ATTRIBUTION.md)).
- تعريفات إشارات CAN من [opendbc](https://github.com/commaai/opendbc) (comma.ai) و[model3dbc](https://github.com/joshwardell/model3dbc)، بترخيص MIT ([data/opendbc](https://github.com/SenseiIssei/Caruspex/blob/main/data/opendbc)، [data/tesla](https://github.com/SenseiIssei/Caruspex/blob/main/data/tesla)).
- قيم البطارية لسيارات Hyundai Ioniq وKona وKia e-Niro وNissan Leaf وRenault Zoe وفق OBDb (بترخيص CC BY-SA 4.0 مثل هذه الحزم) و[Open Vehicle Monitoring System](https://github.com/openvehicles/Open-Vehicle-Monitoring-System-3) و[evDash](https://github.com/nickn17/evDash) (كلاهما بترخيص MIT): [data/pids](https://github.com/SenseiIssei/Caruspex/blob/main/data/pids) ([ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/pids/ATTRIBUTION.md)).
- رموز المحركات من مقالات ويكيبيديا، CC BY-SA 4.0 ([data/engines](https://github.com/SenseiIssei/Caruspex/blob/main/data/engines)، مع المقالة والمراجعة في كل سطر).
- رموز الشركات المصنّعة في رقم VIN مُتحقَّق منها في [NHTSA vPIC](https://vpic.nhtsa.dot.gov) (ملكية عامة) و[Wikidata](https://www.wikidata.org) (CC0)؛ أما الرموز التي لا توجد إلا في [قائمة WMI في ويكي الكتب](https://en.wikibooks.org/wiki/Vehicle_Identification_Numbers_%28VIN_codes%29/World_Manufacturer_Identifier_%28WMI%29) فهي بترخيص CC BY-SA 4.0 ومحفوظة على حدة في [data/wmi](https://github.com/SenseiIssei/Caruspex/blob/main/data/wmi).
- أبعاد المركبات من [ويكي بيانات](https://www.wikidata.org)، CC0.
- خطوط مرخّصة بموجب SIL Open Font License 1.1: JetBrains Mono ([OFL.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-Inter.txt)), Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-PlusJakartaSans.txt)).
