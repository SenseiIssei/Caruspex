# 안전, 상표, 라이선스

## 안전과 책임

Caruspex는 자유로운 취미 프로젝트이며 [MIT 라이선스](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE)에 적힌 대로 어떠한 보증도 없이 제공됩니다. 사용에 따른 위험은 사용자에게 있습니다.

- 주행 중에는 절대 조작하지 마세요. 설정은 차를 세운 상태에서 하고, 실시간 값은 동승자가 지켜볼 수 있습니다.
- 고장 삭제, 액추에이터 테스트, 기본 설정은 차가 무언가를 하게 만듭니다. Caruspex는 각각을 실행하기 전에 묻습니다. 차를 세우고 고정한 상태에서, 작업이 허락된 차량에서만 하세요.
- Caruspex는 정비소를 대신하지 않습니다. 고장 코드는 증상을 알려 줄 뿐 항상 원인을 알려 주지는 않습니다. 브레이크, 에어백, 조향 장치 등 안전에 중요한 모든 것은 전문가에게 맡기세요.
- 삭제는 수리가 아닙니다. 삭제한 고장과 초기화된 준비 상태 모니터 때문에 배출가스 검사나 자동차 검사에 불합격할 수 있습니다.
- Caruspex는 주행 거리, 이모빌라이저, 키, 배출가스 제어 장치, 타코그래프를 바꾸지 않으며, 프로젝트는 이런 기능을 넣는 기여를 받지 않습니다. 주행 거리계, 배출가스 제어 장치, 타코그래프를 조작하는 것은 많은 나라에서 불법입니다.

## 상표

자동차 제조사, 모델, 어댑터, 프로그램, 표준의 이름(예: Volkswagen, Audi, BMW, Mercedes-Benz, Tesla, OBDLink, ELM327, VCDS, ISTA)은 각 소유자의 상표입니다. Caruspex가 어떤 차, 어댑터, 인터페이스와 함께 동작하는지 알리기 위해서만 쓰입니다. Caruspex는 독립 프로젝트이며 어떤 자동차 제조사, 어댑터 제조사, 소프트웨어 공급자와도 제휴하거나 승인 또는 후원을 받지 않았습니다. 앱과 이 저장소는 이들 회사의 로고를 사용하지 않습니다.

## 라이선스와 출처

코드는 MIT 라이선스를 따릅니다. [LICENSE](https://github.com/SenseiIssei/Caruspex/blob/main/LICENSE)를 참고하세요. 프로그램에 들어 있는 라이브러리와 그 라이선스는 [THIRD_PARTY_NOTICES.md](https://github.com/SenseiIssei/Caruspex/blob/main/THIRD_PARTY_NOTICES.md)에 있으며, 앱은 설정에서 보여 줍니다. 프로그램에 들어 있는 데이터:

- 약 550개 차량의 제조사 값. [OBDb](https://github.com/OBDb)에서 변환했으며 OBDb와 마찬가지로 CC BY-SA 4.0입니다. 코드와 분리되어 [data/obdb](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb)에 라이선스와 출처 목록([ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/obdb/ATTRIBUTION.md))과 함께 있습니다.
- CAN 신호 정의는 [opendbc](https://github.com/commaai/opendbc)(comma.ai)와 [model3dbc](https://github.com/joshwardell/model3dbc)에서, MIT 라이선스([data/opendbc](https://github.com/SenseiIssei/Caruspex/blob/main/data/opendbc), [data/tesla](https://github.com/SenseiIssei/Caruspex/blob/main/data/tesla)).
- 현대 아이오닉과 코나, 기아 e-니로, 닛산 리프, 르노 조에의 배터리 값은 OBDb(CC BY-SA 4.0, 이 팩들도 같음), [Open Vehicle Monitoring System](https://github.com/openvehicles/Open-Vehicle-Monitoring-System-3), [evDash](https://github.com/nickn17/evDash)(둘 다 MIT)를 따릅니다: [data/pids](https://github.com/SenseiIssei/Caruspex/blob/main/data/pids) ([ATTRIBUTION.md](https://github.com/SenseiIssei/Caruspex/blob/main/data/pids/ATTRIBUTION.md)).
- 엔진 코드는 위키백과 문서에서, CC BY-SA 4.0([data/engines](https://github.com/SenseiIssei/Caruspex/blob/main/data/engines), 각 행에 문서와 판 표시).
- VIN 제조사 코드는 [NHTSA vPIC](https://vpic.nhtsa.dot.gov)(퍼블릭 도메인)와 [Wikidata](https://www.wikidata.org)(CC0)에서 확인했습니다. [위키책의 WMI 목록](https://en.wikibooks.org/wiki/Vehicle_Identification_Numbers_%28VIN_codes%29/World_Manufacturer_Identifier_%28WMI%29)에만 있는 코드는 CC BY-SA 4.0이며 [data/wmi](https://github.com/SenseiIssei/Caruspex/blob/main/data/wmi)에 따로 둡니다.
- 차량 치수는 [위키데이터](https://www.wikidata.org)에서, CC0.
- SIL Open Font License 1.1 글꼴: JetBrains Mono ([OFL.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL.txt)), Inter ([OFL-Inter.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-Inter.txt)), Plus Jakarta Sans ([OFL-PlusJakartaSans.txt](https://github.com/SenseiIssei/Caruspex/blob/main/resources/fonts/OFL-PlusJakartaSans.txt)).
