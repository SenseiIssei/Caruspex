# OBDLink와 STN 칩

OBDLink 어댑터(SX, EX, MX+, CX, LX)는 OBD Solutions의 STN 칩을 써요. ELM327이 하는 모든 것을 이해하고 그 이상도 해요. Caruspex는 이 칩을 스스로 알아보고 추가 기능을 사용해요.

## Caruspex가 쓰는 기능

- **묶음 요청**: Caruspex는 실시간 값을 6개가 아니라 한 번에 최대 24개까지 요청해요. Bluetooth에서는 이렇게 하면 값 한 바퀴를 읽는 속도가 거의 두 배가 돼요.
- **수신**: 브로드캐스트 값(예: Tesla)은 STN 고유 명령으로 기록하는데, 이 명령은 정해진 개수의 메시지를 받으면 스스로 멈춰요.
- **Ford MS-CAN과 GM Single-Wire-CAN**: Ford와 GM의 차체 제어 장치는 일반 진단 CAN에 있지 않아요. 추가 트랜시버가 있는 어댑터라면 **모든 제어 장치** 페이지에 "Ford: MS-CAN의 차체 장치" 또는 "GM: Single-Wire-CAN의 차체 장치"가 나타나요.

| 칩 | 어댑터 | MS-CAN | SW-CAN |
|---|---|---|---|
| STN2230에서 STN2232 | OBDLink EX | 예 | 아니요 |
| STN1150에서 1152, 1170, 2120, 2255, 2256 | OBDLink MX+ 등 | 예 | 예 |
| 그 외 | OBDLink SX, LX, CX | 아니요 | 아니요 |

## 연결하기

OBDLink 어댑터는 다른 ELM327과 똑같이 연결해요: USB(EX와 SX), Bluetooth(MX+, LX, CX), 또는 [USB로 연결하는 ELM327](elm327-usb.md)과 [Bluetooth로 연결하는 ELM327](elm327-bluetooth.md)의 설명대로요.

## 테스트 결과

STN2232 칩이 들어간 OBDLink가 어댑터 시험을 통과했어요: 칩, 펌웨어, 명령이 올바르게 인식됐어요. 아직 차에서는 써 보지 않았기 때문에, 묶음 요청, MS-CAN, SW-CAN은 실제 하드웨어에서 아직 확인되지 않았어요.
