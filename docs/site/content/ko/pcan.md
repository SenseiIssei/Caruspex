# PEAK PCAN (Windows)

PEAK-System의 CAN 어댑터(PCAN-USB, PCAN-USB FD, PCAN-USB Pro 등)는 J2534 드라이버 없이 Windows에서 바로 쓸 수 있어요. Linux에서는 [SocketCAN](socketcan.md) 인터페이스로 쓰여요.

## 연결하기

1. PEAK의 장치 드라이버를 설치해요. 드라이버와 함께 라이브러리 `PCANBasic.dll`이 설치되고, Caruspex가 이것을 불러와요.
2. **연결** 페이지에서 **CAN 인터페이스**를 골라요. 장비가 연결된 모든 PCAN 채널이 저절로 목록에 나타나고, 다른 프로그램이 이미 쓰고 있는 채널은 빠져요.
3. 비트레이트(CAN을 통한 OBD는 500 kbit/s)를 고르고 연결해요.

## 되는 것

OBD, UDS를 통한 모든 제어 장치, 병렬 제어 장치 검색, VW TP 2.0, 브로드캐스트 값 수신. PCAN-Basic을 통한 CAN FD는 아직 들어가 있지 않아요.

## 명령줄

```
caruspex_cli read --can pcan:USB1@500000
```
