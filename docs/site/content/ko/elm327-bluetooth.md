# Bluetooth로 연결하는 ELM327

케이블 없는 작은 플러그예요. Bluetooth에는 두 종류가 있고, Caruspex는 둘 다 지원해요.

## Bluetooth LE

Vgate iCar Pro BLE, vLinker, OBDLink CX, Veepeak BLE 같은 최신 어댑터는 Bluetooth Low Energy를 써요. 이런 어댑터는 **페어링이 필요 없어요**: **연결** 페이지에서 **Bluetooth**를 고르고 **Bluetooth 장치 검색**을 누른 다음 목록에서 어댑터를 고르면 돼요. Windows, Linux, macOS, Android에서 모두 작동해요.

## 클래식 Bluetooth

오래된 어댑터와 저렴한 어댑터 상당수는 시리얼 프로필을 쓰는 클래식 Bluetooth를 사용해요.

1. 시스템의 Bluetooth 설정에서 어댑터를 한 번 페어링해요. PIN은 보통 `1234` 또는 `0000`이에요.
2. Caruspex에서 **Bluetooth**를 고르고 페어링한 어댑터를 선택해요. 목록에 그 장치가 LE인지, 클래식인지, 둘 다인지 표시돼요.

Windows와 Linux에서는 Caruspex가 페어링된 클래식 어댑터에 바로 연결해요. 또는 Windows가 어댑터용 COM 포트를 만들어 주는데, 이 경우 [USB로 연결하는 ELM327](elm327-usb.md)과 똑같이 쓰면 돼요. macOS에서는 시스템이 시리얼 포트를 제공할 때만 클래식 Bluetooth 어댑터가 작동해요. macOS에서는 Bluetooth LE 어댑터가 더 나은 선택이에요.

## Android에서

- Bluetooth LE 어댑터는 여기서도 페어링이 필요 없어요: **Bluetooth**를 고르고 검색하면 목록에 각 장치의 신호 세기가 나와요.
- 클래식 어댑터는 Android 설정에서 한 번 페어링하면 목록에 나타나요.
- Android 12부터는 근처 기기 접근 권한을 한 번 물어봐요. Android 11까지는 Caruspex가 위치를 사용할 수 있고 위치가 켜져 있을 때만 검색에서 Bluetooth LE 어댑터를 찾아요. Caruspex는 위치를 읽지 않아요. 페어링된 클래식 어댑터는 위치 없이도 작동해요.

## 팁

- 쓰지 않을 때는 어댑터를 뽑아 두세요: 저렴한 어댑터 상당수는 차 배터리를 계속 소모해요.
- 어댑터가 보이지 않으면 컴퓨터의 Bluetooth를 껐다가 다시 켜고, 휴대폰 앱이 동시에 연결되어 있지 않은지 확인하세요.
- Bluetooth는 케이블보다 느려요. OBDLink를 쓰면 Caruspex가 여러 값을 한 번에 요청하기 때문에 그 차이를 대부분 메워요([OBDLink와 STN](obdlink-stn.md) 참고).
