# SocketCAN (Linux)

SocketCAN은 Linux 커널의 CAN 계층이에요. 커널 드라이버가 있는 CAN 어댑터라면 무엇이든 이것을 통해 쓸 수 있어요: PEAK PCAN-USB, Kvaser, candleLight 펌웨어를 올린 CANable, Raspberry Pi CAN HAT 등.

## 설정하기

차의 비트레이트(CAN을 통한 OBD는 500 kbit/s)로 인터페이스를 올려요:

```
sudo ip link set can0 up type can bitrate 500000
```

최신 차의 CAN FD라면 데이터 비트레이트도 함께 지정해요:

```
sudo ip link set can0 up type can bitrate 500000 dbitrate 2000000 fd on
```

## 연결하기

**연결** 페이지에서 **CAN 인터페이스**를 골라요. 컴퓨터의 모든 CAN 인터페이스가 목록에 있어요(`can0`, `can1` 등). 그것을 고르고 비트레이트를 정한 다음, CAN FD라면 CAN FD 상자를 체크해요.

## 되는 것

OBD, UDS를 통한 모든 제어 장치, 병렬 제어 장치 검색, VW TP 2.0, 브로드캐스트 값 수신, CAN FD.

## 명령줄

```
caruspex_cli interfaces
caruspex_cli read --can socketcan:can0@500000
caruspex_cli read --can socketcan:can0+fd@500000
```
