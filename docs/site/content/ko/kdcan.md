# K+DCAN 케이블

K+DCAN 케이블은 1998년부터 2017년경까지의 BMW를 정비소 프로그램 INPA와 ISTA로 진단할 때 흔히 쓰는 케이블이에요. 안에는 FTDI 시리얼 칩과 BMW D-CAN용 작은 변환기가 들어 있어요. 이런 케이블 상당수에는 오래된 차를 위해 7번과 8번 핀 사이에 스위치나 브리지가 있어요.

## Caruspex에서의 현황

> 솔직한 현황: Caruspex는 아직 이 케이블의 D-CAN 모드를 지원하지 않아요. K-Line을 통한 오래된 BMW 프로토콜(DS2와 BMW 방식의 KWP2000)은 계획되어 있지만, 먼저 실제 차에서 테스트가 필요해요.

지금 대신 쓸 수 있는 것:

- **F 시리즈 이후의 BMW와 Mini (2008년경 이후)**: [ENET 케이블](enet.md)이면 이더넷으로 모든 제어 장치에 닿고, 또는 일반 [ELM327](elm327-usb.md)로 CAN에서 BMW 고유의 주소 방식을 쓸 수 있어요.
- **CAN이 있는 BMW의 표준 OBD (2007년경 이후)**: 어떤 ELM327이든 엔진 고장과 실시간 값을 읽어요.
- **K-Line 모드에서는**이 케이블이 [KKL 케이블](kkl.md)처럼 K-Line을 그대로 통과시켜요. VW 그룹 차에는 유용하지만, K-Line에 있는 BMW 제어 장치에는 아직 없는 프로토콜이 필요해요.

## 도움을 구해요

K+DCAN 케이블과 오래된 BMW가 있다면, 기록을 첨부한 [테스트 보고서](https://github.com/SenseiIssei/Caruspex/issues/new?template=test-report.yml)가 이 부분을 만드는 데 도움이 돼요.
