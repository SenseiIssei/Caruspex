# SLCAN (Lawicel)

SLCAN은 CAN 스틱을 위한, 시리얼 포트를 통한 간단한 텍스트 프로토콜이에요: slcan 펌웨어를 올린 CANable, USBtin, Lawicel CANUSB, 그리고 많은 자작 장치. Windows, Linux, macOS에서 작동해요.

## 연결하기

1. 스틱을 컴퓨터에 연결하고, 케이블로 OBD 단자에 연결해요: CAN high는 6번 핀, CAN low는 14번 핀, 접지는 4번과 5번 핀이에요.
2. **연결** 페이지에서 **CAN 인터페이스**를 골라요. SLCAN 스틱은 자동 포트 검색으로 찾아져서 목록에 나타나요.
3. 비트레이트(CAN을 통한 OBD는 500 kbit/s)를 고르고 연결해요.

## 되는 것

다른 원시 CAN 인터페이스와 같아요: OBD, UDS를 통한 모든 제어 장치, 병렬 제어 장치 검색, VW TP 2.0, 브로드캐스트 값 수신. SLCAN으로는 CAN FD를 쓸 수 없어요.

## 팁

- candleLight 펌웨어를 올린 CANable은 SLCAN 장치가 아니에요: Linux에서는 [SocketCAN](socketcan.md) 인터페이스로 나타나요.
- OBD 단자 쪽에 스틱의 종단 저항은 필요 없어요. 차에 이미 있어요.

## 명령줄

```
caruspex_cli read --can slcan:COM5@500000
caruspex_cli read --can slcan:/dev/ttyACM0@500000
```
