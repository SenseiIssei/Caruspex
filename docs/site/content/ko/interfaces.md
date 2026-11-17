# 어댑터와 인터페이스

Caruspex는 무엇이 꽂혀 있는지, 차가 어떤 프로토콜을 쓰는지 스스로 알아내요. 어떤 방법으로 차에 연결하든 같은 기능을 쓸 수 있어요: OBD-II, UDS를 통한 모든 제어 장치, 브로드캐스트 값 수신, 기록과 재생.

| 인터페이스 | 예 | 페이지 |
|---|---|---|
| USB로 연결하는 ELM327 | 저렴한 ELM327 케이블, Vgate, Veepeak | [USB로 연결하는 ELM327](elm327-usb.md) |
| Bluetooth로 연결하는 ELM327 | 클래식 Bluetooth와 Bluetooth LE 플러그, vLinker, Vgate iCar Pro | [Bluetooth로 연결하는 ELM327](elm327-bluetooth.md) |
| Wi-Fi로 연결하는 ELM327 | 자체 Wi-Fi를 여는 플러그 | [Wi-Fi로 연결하는 ELM327](elm327-wifi.md) |
| STN 칩 | OBDLink SX, EX, MX+, CX | [OBDLink와 STN](obdlink-stn.md) |
| KKL 케이블 (VAG 409.1) | FTDI 또는 CP2102 케이블 | [KKL 케이블](kkl.md) |
| K+DCAN 케이블 | 스위치가 달린 BMW용 케이블 | [K+DCAN 케이블](kdcan.md) |
| ENET 케이블 | F 시리즈 이후의 BMW와 Mini | [BMW ENET](enet.md) |
| SAE J2534 PassThru | Tactrix OpenPort 2.0, Mongoose, VCX Nano, Scanmatik | [J2534](j2534.md) |
| SLCAN / Lawicel | CANable, USBtin, CANUSB | [SLCAN](slcan.md) |
| SocketCAN (Linux) | PEAK, Kvaser, candleLight를 올린 CANable, Raspberry Pi CAN HAT | [SocketCAN](socketcan.md) |
| PEAK PCAN (Windows) | PCAN-USB, PCAN-USB FD | [PCAN](pcan.md) |
| Kvaser (Windows) | Kvaser Leaf, U100 | [Kvaser](kvaser.md) |
| DoIP 케이블 | 2016년경 이후의 많은 차 | [DoIP](doip.md) |

## 각각 무엇을 할 수 있나요?

일반 ELM327이면 엔진, 변속기, 그리고 2008년경 이후 대부분의 차에서는 UDS를 통한 나머지 모든 제어 장치까지 충분해요. 하지만 바이트 하나하나를 확인 응답하는 오래된 Volkswagen 그룹 프로토콜 KWP1281은 처리하지 못해요: 2004년경 이전의 VW, Audi, Seat, Skoda에는 KKL 케이블이 필요해요.

원시 CAN 인터페이스(J2534, SLCAN, SocketCAN, PCAN, Kvaser)는 Caruspex에 날것의 CAN 프레임을 넘겨줘요. 그러면 Caruspex가 전송 프로토콜을 직접 처리하므로, 제어 장치를 빠르게 병렬 검색할 수 있고, 2004년부터 2012년경까지의 Volkswagen 그룹 차에 VW TP 2.0을 쓸 수 있어요.

![어댑터 시험](screenshot-adapter.png)

## 내 어댑터는 정품인가요?

**어댑터 시험**은 어댑터에 ELM327 버전별 명령을 물어보고, 응답 시간을 재고, 그 결과가 무엇을 뜻하는지 솔직하게 알려 줘요: 저렴한 어댑터 상당수는 버전 1.5나 2.1이라고 보고하지만 1.0의 명령밖에 몰라요. Caruspex는 어댑터에 아무것도 쓰지 않아요.

> 지금까지 테스트한 것: STN2232 칩이 들어간 OBDLink가 차 없이 어댑터 시험을 통과했어요. 다른 모든 어댑터와 인터페이스는 문서를 따라 만들었지만 아직 실제 차에서 테스트하지 않았어요.
