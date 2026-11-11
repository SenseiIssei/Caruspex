# Caruspex

<!-- languages -->
[English](README.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Français](README.fr.md) · [Português](README.pt.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Čeština](README.cs.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [Türkçe](README.tr.md) · [العربية](README.ar.md) · [中文](README.zh.md) · [日本語](README.ja.md) · **한국어** · [हिन्दी](README.hi.md) · [Bahasa Indonesia](README.id.md)
<!-- /languages -->

[![Build](https://github.com/SenseiIssei/Caruspex/actions/workflows/build.yml/badge.svg)](https://github.com/SenseiIssei/Caruspex/actions/workflows/build.yml)
![Rust](https://img.shields.io/badge/Rust-2024-orange)
![Tauri 2](https://img.shields.io/badge/Tauri-2-24c8db)
![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)

Windows, Linux, macOS용 오픈 소스 차량 진단 프로그램입니다. Rust로 작성했고 화면은 Tauri 2로 만들었습니다.
케이블이나 동글 하나, 앱 하나로 고장 코드를 읽고 지우고, 센서의 실시간 데이터를 보고,
배출가스 검사 준비 상태를 확인하고, K-Line 케이블로 VW/Audi 제어 장치와 바이트 단위로 통신할 수 있습니다.

이름은 *haruspex*에서 왔습니다. 동물의 내장을 보고 징조를 읽던 고대 로마의 점술가입니다. Caruspex는 자동차의 내장을 읽습니다.

앱은 18개 언어를 지원합니다. 화면과 모든 고장 코드 문구가 똑같이 번역되어 있습니다. 영어, 독일어, 스페인어,
프랑스어, 포르투갈어, 이탈리아어, 네덜란드어, 폴란드어, 체코어, 러시아어, 우크라이나어, 터키어, 아랍어(오른쪽에서 왼쪽으로), 중국어,
일본어, 한국어, 힌디어, 인도네시아어입니다. 시스템 언어를 따르고, 설정에서 바꿀 수 있으며,
`--language fr`처럼 실행할 때마다 지정할 수도 있습니다. 번역을 어떻게 만들고 검사하는지는
[translations/TRANSLATING.md](translations/TRANSLATING.md)에 있습니다.

![실시간 게이지가 있는 개요](docs/screenshot-overview.png)

## 주요 특징

- **완전한 진단 체계 두 가지**: 아무 ELM327 어댑터로 하는 표준 OBD-II/EOBD, 그리고 순수 KKL K-Line 케이블로 하는 VW/Audi
  제조사 진단(KWP1281과 KWP2000).
- **프로토콜을 처음부터 직접 구현**: KWP1281은 모든 바이트에 그 보수(one's complement)로 응답합니다.
  ELM327 펌웨어는 이것을 할 수 없습니다. Caruspex는 K-Line을 직접 구동하며, 5보 깨우기
  주소도 시리얼 브레이크 신호를 켜고 끄는 방식으로 한 비트씩 보내고 마감 시각 기반 타이밍을 써서, OS
  스케줄링 지터가 열 비트에 걸쳐 쌓이지 않게 합니다.
- **자동 프로토콜 감지**: 깨운 뒤 ECU가 보내는 두 개의 키 바이트로
  KWP1281인지 KWP2000(ISO 14230)인지 알 수 있고, Caruspex가 그에 맞게 전환합니다.
- **모듈식 Rust 워크스페이스**: 영역마다 crate 하나(어댑터 연결, 프로토콜, 네트워크와 CAN, 차량, 데이터,
  녹화, 분석, 데모 차량, 명령줄 도구)가 있고 그 위에 Tauri 앱이 올라갑니다. 코어는 화면에 대해 아무것도 모르므로
  앱과 명령줄 도구가 똑같은 코드로 동작합니다.
- **차 없이도 테스트**: 모의 ECU(`crates/caruspex-protocols/tests/common/`)가
  바이트 단위 핸드셰이크를 재현하고, 가짜 게이트웨이가 ENET과 DoIP를 대신하고, 가짜 J2534 드라이버가 PassThru 경로를 다루며,
  Python ELM327 시뮬레이터로 앱을 처음부터 끝까지 실행해 볼 수 있습니다.
- **번역은 한 벌**: 코어와 화면이 `translations/<code>/`에 있는 같은 번역 파일을 함께 씁니다.
  18개 언어입니다. 코어는 영어 문구로 문구를 찾고, 화면은
  같은 파일로 자기 카탈로그를 만듭니다.
- **모든 차를 알아봅니다**: 제조사 코드 365개를 갖춘 VIN 디코더와 1980년 이후
  2,100개가 넘는 모델 세대의 카탈로그가 어떤 차가 연결되어 있고 무엇이 되는지 알려 줍니다.
- **내장 데모 차량**: 프로세스 안에서 동작하는 ELM327로, CAN 위에 엔진과 변속기가 있어
  하드웨어 없이 모든 것을 써 볼 수 있습니다.
- **손으로 입력하지 않고 생성한 데이터**: 약 1,700개의 고장 코드이며, 상당수는 `tools/gen_dtc.py`가 규칙적인 코드 계열
  (실린더별, 뱅크별, 산소 센서별)로부터 계산해 만듭니다.

## 기능

### 차를 압니다
- 연결하면 Caruspex가 VIN을 읽고 어떤 차인지 알려 줍니다. 제조사, 모델, 세대, 연식입니다.
- 94개 브랜드, 2,100개가 넘는 모델 세대의 카탈로그로, 1980년부터 지금까지이며 더 이상 존재하지 않는 브랜드도 포함합니다.
- 세대마다 어떤 진단 커넥터인지, 어디에 있는지, 어떤 프로토콜인지, 그리고 OBD-II, KKL 케이블, 제조사 전용 도구 중 무엇이 되는지 알려 줍니다.
- 아무 VIN이나 확인하는 VIN 검사기: 제조사, 국가, 연식, 체크 디지트, 모델 코드.
- 차고는 VIN으로 모든 차를 알아보고 점검 기록과 주행 거리계 값을 보관합니다.

![차량](docs/screenshot-vehicles.png)

### 버튼 하나로 상태 점검
- 고장 메모리, 준비 상태 모니터, 부품 시험을 읽은 다음 30초 동안 혼합기, 온도, 산소 센서, 충전 전압을 지켜봅니다.
- 0에서 100까지의 점수이며, 깎인 점수마다 쉬운 말로 이유와 다음 할 일을 설명합니다.
- 배출가스 검사를 받을 준비가 됐을까요? 어떤 모니터가 아직 남았는지, 끝내려면 어떻게 달려야 하는지 알려 줍니다.
- PDF 진단 보고서로, 인쇄하거나 정비소에 보여 줄 수 있습니다.

![상태 점검](docs/screenshot-health.png)

### 고장 메모리
- 저장된 코드, 보류 중인 코드, 영구 코드를 보여 주며, 제어 장치가 여러 개면 각 코드를 누가 보고했는지(엔진, 변속기) 표시합니다.
- 표준 코드(P, C, B, U) 약 1,400개와 VAG, 포드, GM, 토요타, 혼다, 닛산, 현대/기아, 스바루 등의 제조사 코드. P1 코드는 감지된 브랜드에서 쓰는 의미대로 읽습니다.
- 코드마다 신호등이 있고, 누구나 가장 먼저 묻는 질문에 답합니다. 계속 운전해도 될까요?
- 흔한 원인, 직접 확인해 볼 수 있는 것, 대략적인 비용 등급.
- 투시 보기의 자동차가 해당 부품의 위치를 보여 줍니다.
- 분명한 경고 후에 지우고 자동으로 다시 읽습니다. CSV 내보내기.

![고장 메모리](docs/screenshot-faults.png)

### 실시간 데이터
- SAE J1979의 모든 모드 01 값을 PID C4까지: PID 145개, 읽기 값 300개 이상을 그룹별로 묶고 검색할 수 있습니다.
- 주행 거리계, 기어, 오일 온도, 연료 소비율, 토크, 부스트, 배기 온도, 미립자 필터, AdBlue, NOx, 하이브리드 배터리. 차가 보고하는 범위에서 보여 줍니다.
- 흐르는 차트와 CSV 녹화.

![실시간 데이터](docs/screenshot-live.png)

### 측정
- 트립 컴퓨터: 실시간 및 평균 연비, 비용, CO2, 거리, 그리고 온화한 운전 스타일 점수. 자체 트립 컴퓨터가 없는 차에서도 쓸 수 있습니다.
- 성능: 속도 신호로 0에서 100 km/h, 80에서 120, 쿼터 마일, 제동 거리를 잽니다(폐쇄된 도로에서만).
- 배터리와 발전기: 휴지 전압, 시동 시 전압 강하, 부하가 있을 때와 없을 때의 충전 전압을 네 단계로 안내합니다.

### 점검
- 준비 상태 모니터, 프리즈 프레임, VIN을 쉬운 말로.
- 값과 한계가 있는 모드 06 부품 모니터링, 실린더별 실화 횟수.
- 모드 09의 ECU 이름, 캘리브레이션 ID, 체크섬을 제어 장치별로.
- 원시 ELM327 터미널.

### 어댑터 시험
- 진짜 ELM327일까요, 복제품일까요? 버전마다 다른 명령을 물어보고 응답 시간을 재서, 그로부터 무엇을 알 수 있는지 솔직하게 알려 줍니다.
- OBDLink와 그 밖의 STN 어댑터를 알아봅니다.
- 어댑터에는 아무것도 쓰지 않습니다.

![어댑터 시험](docs/screenshot-adapter.png)

### 모든 케이블, 차로 들어가는 모든 길
Caruspex는 무엇이 꽂혀 있고 차가 어떤 프로토콜로 말하는지 스스로 알아냅니다.

| 인터페이스 | 예 | 방식 |
|---|---|---|
| USB, 블루투스, 블루투스 LE, Wi-Fi 방식의 ELM327 | 저렴한 ELM327, Vgate, vLinker, Veepeak | 시리얼 포트, 페어링했거나 직접 연결한 BLE, TCP |
| STN 칩 | OBDLink SX, EX, MX+, CX | ELM327과 같고 더 빠른 타이밍 |
| SAE J2534 PassThru | Tactrix OpenPort 2.0, Mongoose, VCX Nano, Scanmatik, Godiag, J2534 모드의 VAS 5054A | 제조사의 64비트 드라이버, ISO-TP를 쓰는 CAN은 Caruspex가 처리 |
| SLCAN / Lawicel | CANable, USBtin, CANUSB | 시리얼 포트, 자동 포트 검색으로 찾음 |
| SocketCAN(Linux) | PEAK PCAN-USB, Kvaser, candleLight를 쓴 CANable, Raspberry Pi CAN HAT | can0 등 |
| KKL 케이블(VAG 409.1) | FTDI 또는 CP2102 케이블 | K-Line, KWP1281과 KWP2000 |
| ENET 케이블 | F 시리즈 이후의 BMW와 Mini | 이더넷 위의 HSFZ |
| DoIP 케이블 | 2016년경 이후의 많은 차 | 이더넷 위의 ISO 13400 |

어느 인터페이스에서든 같은 기능이 동작합니다. OBD-II와 UDS 기반 OBD, UDS로 모든 제어 장치,
BMW 주소 지정, 브로드캐스트 값 수신(테슬라), 녹화와 재생입니다.

![연결](docs/screenshot-connect.png)

### CAN으로 모든 제어 장치(UDS)
- 평범한 ELM327로도 Caruspex는 제어 장치를 하나씩 모두 조회합니다. 엔진, 변속기, ABS, 에어백, 계기판, 공조 장치 등이며 2008년경 이후의 대부분의 차에서 됩니다.
- 장치별 고장 메모리와 고장 유형(단락, 단선, 타당하지 않음 ...), 부품 번호, 소프트웨어와 하드웨어 버전.
- VAG, BMW와 Mini, 포드, 마쓰다, GM, 토요타, 닛산, 현대/기아, 르노, PSA의 주소 목록. 나머지는 흔히 쓰는 주소를 훑어 찾습니다.
- F 시리즈 이후의 BMW와 Mini는 6F1을 통한 BMW 고유의 확장 주소 지정과 128개 진단 주소 전체 탐색을 지원하며, 고장 코드는 ISTA가 보여 주는 BMW 형식(예: 480A10)입니다.
- 장치별로 지우기. 경고가 표시됩니다.

![모든 제어 장치](docs/screenshot-ecus.png)

### KKL 케이블로 VW와 Audi(KWP1281과 KWP2000)
- 스로틀 바디 적응: 측정 블록 060 또는 098에서 하는 진짜 기본 설정.
- ABS, 에어백, 계기판, 편의 장치를 포함한 모든 제어 장치를 스캔하고 인쇄할 수 있는 보고서를 만듭니다.
- 적응 채널과 출력 시험. 의도적으로 읽기 전용입니다.

### 제조사 값과 사용자 지정 값
- 전기차 배터리: 건강 상태, 충전량, 편차가 있는 셀 전압, 온도. 현대 코나와 아이오닉, 기아 e-Niro, 르노 Zoe, 닛산 Leaf, 테슬라 Model 3와 Y(CAN 버스 수신 방식)를 지원합니다. 모든 값에는 문서화된 출처가 있습니다.
- 널리 쓰이는 Torque CSV 형식으로 자신의 값을 불러올 수 있고, Torque 수식을 완전히 계산합니다.

![EV 배터리](docs/screenshot-battery.png)

### 녹화와 재생
- 차와 한 세션을 파일에 녹화하고 나중에 차 없이 재생할 수 있어, 문제를 찾을 때 편리합니다.

### 차나 어댑터 없이
- 내장 데모 차량: 달리고, 예열되고, 고장이 몇 개 저장되어 있으며 진짜 ELM327처럼 CAN으로 말합니다.
- 모든 코드의 오프라인 사전으로, 브랜드별로 걸러 볼 수 있습니다.

### 보기와 사용감
- Odysync 스타일의 새 화면: 차분하고 어둡고 부드럽게 전환됩니다.
- 모든 포트를 자동으로 찾는 단계별 연결.
- 18개 언어. 설정은 바로 적용됩니다.
- 새 버전이 나오면 알려 줍니다. 시작할 때 GitHub에 최신 릴리스만 묻습니다(당신이나 차에 대한
  데이터는 보내지 않습니다). 설정에서 끌 수 있습니다.

## 지원하는 차량

OBD-II 또는 EOBD가 있는 모든 차량: 유럽에서는 2001년 이후의 가솔린 차와 2004년 이후의 디젤 차, 미국에서는 1996년 이후입니다. 1990년경 이후의 오래된 VW, 아우디, 세아트, 슈코다는 KKL 케이블로 됩니다. 차량 페이지에서 해당 차에 어떤 방법이 되는지 알려 줍니다.

| 시대 | 버스 | 예 |
|---|---|---|
| 1995년경까지 | 제조사 고유 | 2x2 플러그의 KKL을 쓰는 VAG, 그 외에는 정비소 도구만 |
| 1996년에서 2003년경 | K-Line(ISO 9141-2, KWP2000) | 아우디 A2, VW 골프 IV, 토요타, 르노 |
| 2004년경 이후 | CAN(ISO 15765-4) | 아우디 A4 B7, VW 골프 V, 최근의 일본 차 |
| 미국 2005년경까지 | SAE J1850 PWM과 VPW | 포드, 쉐보레 |
| 최신 차 | UDS 기반 OBD(SAE J1979-2) | 미국에서는 2027년식부터 의무, 일부 최신 차에는 이미 적용. 자동으로 감지해 변환 |
| F 시리즈 이후의 BMW와 Mini | ENET(이더넷 위의 HSFZ) 또는 6F1 주소 지정의 D-CAN | 모든 제어 장치 |
| 2016년경 이후의 많은 차 | DoIP 케이블로 DoIP(ISO 13400) | 이더넷 위의 OBD, 자동으로 찾음 |
| 테슬라 Model 3와 Y | OBD 없음: 브로드캐스트 CAN 프레임 | 배터리, 셀, 온도, 속도, 주행 거리계를 수신으로 얻음(진단 커넥터에 연결하는 어댑터 케이블 필요) |

순수 전기차는 OBD-II 배출가스 데이터를 제공하지 않아도 됩니다. 일부는 그래도 부분적으로 응답합니다. 응답이 전혀 없지만 버스에 통신이 있는 차는 Caruspex가 수신 전용으로 연결해 내장 팩이 아는 것을 읽습니다(지금까지는 테슬라 Model 3와 Y).

## 지원하는 어댑터

- **ELM327** 호환 어댑터 전부: USB, 클래식 블루투스(Windows와 Linux에서는 페어링 후 RFCOMM으로 직접, 또는 COM 포트로), **블루투스 LE**
  (Vgate iCar Pro BLE, vLinker, OBDLink CX, Veepeak BLE 등 일반적인 GATT 서비스를 가진 어댑터.
  스캔으로 찾으며 페어링은 필요 없음), 또는 TCP를 통한 Wi-Fi
- F 시리즈 이후의 BMW와 Mini용 **ENET 케이블**(OBD에서 RJ45): Caruspex가 네트워크에서 차를 찾고
  (HSFZ, 포트 6811과 6801) 어댑터 칩 없이 이더넷으로 모든 제어 장치를 읽습니다. 컴퓨터에는
  자동 주소(169.254.x.x)가 필요하며, DHCP 응답이 없으면 Windows, macOS, Linux가 알아서 배정합니다.
  여기서도 Caruspex는 읽기만 합니다. 코딩과 플래싱은 E-Sys, ISTA, 튜닝 도구에 맡깁니다.
- 모든 **KKL 케이블(VAG 409.1)**. FTDI 또는 CP2102 기반 케이블은 K-Line에서 가장 싼
  복제품보다 훨씬 믿을 만합니다. KWP1281에는 정품 FTDI FT232RL을 권장합니다.

Delphi DS150E, Autocom, VAS 5054A 같은 전용 인터페이스는 ELM327로 말하지 않으므로 지원하지 않습니다.

| 기능 | ELM327 | KKL 409.1 |
|---|---|---|
| 엔진 고장 읽기와 지우기 | 예 | 예 |
| 실시간 데이터, 프리즈 프레임, 준비 상태 | 예 | 예 |
| ABS, 에어백, 계기판, 편의 장치 | **아니요** | 예 |
| 제조사 측정 블록 | **아니요** | 예 |
| **스로틀 바디 적응** | **아니요** | 예 |

이것은 소프트웨어가 아니라 하드웨어의 한계입니다. KWP1281은 블록 프로토콜이고 모든 바이트를
상대편이 응답합니다. ELM327 펌웨어는 요청과 응답의 한 쌍 전체만 처리할 수 있습니다.
KKL 케이블은 K-Line을 그대로 통과시키므로 Caruspex가 프로토콜을 직접 말할 수 있습니다.

### 두 가지 VAG 프로토콜, 자동 감지

| 프로토콜 | 주로 쓰이는 곳 | 기본 설정 |
|---|---|---|
| KWP1281(블록 프로토콜) | 1.4 MPI 엔진 | 전용 명령(블록 0x28) |
| KWP2000(ISO 14230) | 보쉬 MED의 1.6 FSI | 전용 진단 세션 후 블록 읽기 |

적응 채널과 출력 시험은 현재 KWP1281 경로에서 구현되어 있습니다. KWP2000에서는
앱이 조용히 실패하는 대신 그렇다고 알려 줍니다.

아직 구현하지 않은 것: CAN에서 VW TP 2.0을 쓰는 제조사 진단(2004년경 이후의 차량).

## 기술 스택

| 영역 | 기술 |
|---|---|
| 언어 | Rust(edition 2024) |
| 화면 | Tauri 2, React 19, TypeScript, 빌드는 Vite |
| 라이브러리 | `serialport`(시리얼 포트), `btleplug`(블루투스 LE), `libloading`(J2534 드라이버) |
| 현지화 | `translations/`에 있는 코어와 화면 공용 JSON 파일 한 벌, 18개 언어 |
| 빌드 | Cargo 워크스페이스, 화면은 npm |
| 테스트 | 모의 ECU, 가짜 게이트웨이, 가짜 J2534 드라이버를 대상으로 한 `cargo test` |
| 도구 | Python 3(코드와 표 생성기, 번역 검사, ELM327 시뮬레이터) |
| 플랫폼 | Windows 10과 11, Linux, macOS |

## 다운로드

바로 쓸 수 있는 패키지는 [릴리스 페이지](https://github.com/SenseiIssei/Caruspex/releases)에 있습니다.

| 시스템 | 파일 | 비고 |
|---|---|---|
| Windows 10과 11, 64비트 | `Caruspex-<version>-windows-x64-setup.exe` | 관리자 권한 없이 설치되고 `caruspex_cli`를 검색 경로에 넣을 수 있습니다. 설치된 0.8을 그 자리에서 업데이트하며, WebView2 런타임은 없을 때만 설치합니다 |
| Windows 10과 11, 64비트 | `Caruspex-<version>-windows-x64.zip` | 휴대용. 압축을 풀고 실행하면 됩니다. WebView2 런타임이 필요하지만 Windows 10과 11에는 이미 있습니다 |
| macOS 13 이상, Apple Silicon | `Caruspex-<version>-macos.dmg` | 앱을 응용 프로그램으로 끌어다 놓기 |
| Linux, x86_64 | `Caruspex-<version>-linux-x86_64.AppImage` | `chmod +x` 후 실행 |
| Debian과 Ubuntu, 64비트 | `Caruspex-<version>-linux-amd64.deb` | `sudo apt install ./Caruspex-<version>-linux-amd64.deb`로 설치 |
| 명령줄 도구만 | `caruspex_cli-<version>-windows-x64.zip`, `caruspex_cli-<version>-linux-x86_64.tar.gz`, `caruspex_cli-<version>-macos.tar.gz` | 스크립트와 서버용, 화면 없음 |

패키지는 아직 서명되어 있지 않습니다. Windows SmartScreen이 처음 실행할 때 물어봅니다("추가 정보", "실행"). macOS에서는
처음 한 번 마우스 오른쪽 버튼으로 눌러 "열기"를 선택해 앱을 여세요.

## 빌드

필요한 것: 안정 버전 Rust 툴체인(edition 2024, Rust 1.85 이상), npm이 포함된 Node 24, 그리고 생성기와 번역 검사용
Python 3. Linux에서는 화면과 어댑터 연결에 필요한 시스템 패키지도 필요합니다
(Debian과 Ubuntu에서의 이름).

```bash
sudo apt-get install libwebkit2gtk-4.1-dev librsvg2-dev libdbus-1-dev libudev-dev pkg-config
```

Windows에는 WebView2 런타임이 필요하고(Windows 10과 11에 포함), macOS는 따로 필요한 것이 없습니다.

화면의 의존성을 한 번 설치한 다음 명령줄 도구와 앱을 빌드합니다.

```bash
npm --prefix app ci
```

```bash
cargo build --release -p caruspex-cli
```

```bash
npm --prefix app run tauri -- build
```

`npm --prefix app run tauri -- dev`는 개발하는 동안 라이브 리로드와 함께 앱을 실행합니다. Tauri 창 없이
화면만 일반 브라우저에서 내장 데모 차량과 함께 실행할 수도 있습니다.

```bash
npm --prefix app run dev
```

`http://localhost:5287/?demo`를 열면 곧바로 데모 차량에 연결됩니다. `?lang=de`는 언어를,
`?theme=light`는 밝은 테마를 고릅니다. 예를 들어 `?demo&lang=de&theme=light`입니다. Tauri 셸이 없으면 어댑터에
접근할 수 없으므로 브라우저 버전은 데모 차량만 압니다.

### Linux의 시리얼 포트 권한

`sudo` 없이 어댑터에 접근하려면:

```bash
sudo usermod -aG dialout $USER
```

그런 다음 로그아웃했다가 다시 로그인하세요.

## 빌드 결과물

| 결과물 | 설명 |
|---|---|
| `target/release/caruspex` / `caruspex.exe` | Tauri 화면이 있는 앱(`tauri build`는 설치 프로그램과 번들도 `target/release/bundle/`에 씁니다) |
| `target/release/caruspex_cli` / `caruspex_cli.exe` | 같은 코어 위에서 동작하며 스크립트로 쓸 수 있는 명령줄 도구, [명령줄 도구](#명령줄-도구) 참고 |

### 패키지와 릴리스

`bash tools/package.sh`는 현재 시스템용 패키지를 `dist/`에 만듭니다. Windows에서는 설치 프로그램과
zip 두 개(설치 프로그램에는 [Inno Setup 6](https://jrsoftware.org/isinfo.php)이 필요), macOS에서는 dmg와 명령줄 압축 파일,
Linux에서는 AppImage, `.deb`, 명령줄 압축 파일입니다. 그다음 `bash tools/smoke_test.sh`가 사용자처럼 설치하거나
열어 보고, 두 프로그램을 실행한 뒤 다시 제거합니다. CI는 매번 두 가지를 모두 실행합니다.

릴리스는 태그에서 나옵니다. `bash tools/release.sh <version>`은 아무것도 푸시하지 않고 별도 브랜치에서 준비합니다.
`Cargo.toml`과 `app/package.json`의 버전을 설정하고, `changelog/<version>.md`(독일어와 영어)를 만들고,
검사, 테스트, 패키징, 스모크 테스트를 실행하며, README 스크린샷(`tools/screenshots.sh de`와
`tools/screenshots.sh en`)을 새로 만듭니다. 풀 리퀘스트가 병합되면 `v<version>` 태그를 붙이고 태그를 푸시합니다. CI가 세 시스템
모두를 빌드하고 패키지와 변경 목록을 담아 릴리스를 게시합니다.

CI(`.github/workflows/build.yml`)는 먼저 Linux에서 포맷 검사, 생성된 표 검사, 번역 검사,
화면 빌드, clippy를 실행한 다음 Windows, Linux, macOS에서 테스트, 패키징, 스모크 테스트를 합니다.
두 번째 워크플로(`.github/workflows/data-check.yml`)는 한 달에 한 번 상위 저장소에 새 VIN 코드, 차량, DBC 파일이 있는지 살펴보고
새로운 내용을 담은 이슈를 엽니다.

## 테스트

```bash
cargo test --workspace
```

약 370개의 테스트가 몇 초 만에 실행되며, 모두 차 없이 돌아갑니다. ELM327 응답 파서, 고장 코드
디코딩, PID 변환, 차량 프로필과 VIN 디코더, 데모 차량을 대상으로 한 OBD-II와 UDS 서비스,
모의 제어 장치를 대상으로 한 KWP1281과 KWP2000 전체 세션, 가짜 게이트웨이를 대상으로 한 ENET(HSFZ)과 DoIP,
가짜 드라이버를 대상으로 한 SLCAN과 J2534 경로, 녹화와 재생, 그리고 분석(상태 점검, 배터리 시험,
트립 컴퓨터, 성능 타이머)을 다룹니다. 명령줄 테스트는 빌드한 `caruspex_cli`를 실행하고 그 출력을
`crates/caruspex-cli/tests/golden/`의 골든 파일과 비교합니다. 이 파일에는 이전 C++ 버전이 같은
명령에 대해 출력한 내용이 들어 있어, 옛 도구와 달라진 점이 있으면 바로 드러납니다.

CI가 테스트 외에 실행하는 검사:

```bash
cargo fmt --all --check
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
python tools/i18n_rust.py --strict
```

```bash
npm --prefix app run build
```

처음 두 개는 포맷과 린트입니다. `tools/i18n_rust.py --strict`는 코어의 문구가 번역 파일에 없거나
18개 언어 중 하나에 번역이 빠져 있으면 실패합니다. 화면 빌드는 `I18N_STRICT=1`을 설정하면 자기
문구를 같은 방식으로 검사합니다. 생성된 표를 새로 만들려면 `python tools/gen_dtc.py`, `gen_vehicles.py`, `gen_ecus.py`,
`gen_pids.py`를 실행하세요. 커밋된 내용과 다르면 CI가 실패합니다.

## 차 없이 써 보기

가장 빠른 방법은 내장 데모 차량입니다. 연결 페이지에서 "데모 차량"을 고르거나 다음과 같이 시작하세요.

```bash
target/release/caruspex --demo --language en
```

`caruspex --help`는 앱의 모든 옵션을 보여 줍니다: `--tcp`, `--port`, `--bluetooth`, `--enet`, `--can`,
`--replay`, `--report`, `--language`, `--theme`, `--page`, `--shot`, `--size`.

Python 시뮬레이터는 K-Line 위의 아우디 A2 엔진을 흉내 내거나, `--can`을 주면 엔진과
변속기가 있는 CAN 차를 흉내 냅니다(헤더, ISO-TP, 제어 장치별 모드 06과 09).

```bash
python tools/mock_elm327.py --can
```

그런 다음 앱을 연결합니다(Windows에서는 실행 파일이 `target/release/caruspex.exe`입니다).

```bash
target/release/caruspex --tcp 127.0.0.1:35000 --language en
```

명령줄 도구도 시뮬레이터에 대해 쓸 수 있습니다.

```bash
target/release/caruspex_cli read --tcp 127.0.0.1:35000
```

### 명령줄 도구

`caruspex_cli <command>`에 `--demo`, `--tcp HOST[:PORT]`, `--port NAME [--baud N]`, `--enet HOST|auto`,
`--can SPEC` 또는 `--replay FILE`을 함께 씁니다.

| 명령 | 하는 일 |
|---|---|
| `read` | 경고등, 준비 상태, 고장 코드. VIN으로 브랜드를 알 수 있으면 그 브랜드에서의 의미도 함께 |
| `vin` | VIN, 그것이 말해 주는 내용(제조사, 국가, 연식, 체크 디지트), 카탈로그에서 일치하는 차 |
| `freeze` | 모든 프리즈 프레임, 저장된 고장마다 하나씩 |
| `health` | 상태 점검: 고장, 준비 상태, 모드 06, 실시간 측정 계열(`--seconds N`, 기본값 15) |
| `probe` | 어댑터 시험: ELM327의 정체, 아는 명령, 왕복 시간 |
| `uds` | CAN 위의 UDS: 모든 제어 장치의 식별 정보와 고장 메모리(자신의 주소는 `--ids A,B,...`) |
| `custom` | 내장 팩(`--pack NAME`) 또는 Torque CSV(`--csv FILE`)의 제조사 값과 사용자 지정 값, 한 번 |
| `live`, `info`, `clear`, `ecus`, `ports`, `kklcheck` | 실시간 값, VIN과 첫 번째 프리즈 프레임, 지우기(모두 제어 장치별), 시리얼 포트, KKL 케이블 자가 시험 |
| `enet`, `interfaces` | 네트워크에서 BMW와 Mini 차량 및 DoIP 차량을 찾고, 이 컴퓨터의 CAN 인터페이스(J2534 드라이버, SocketCAN)를 나열 |
| `version`, `help` | 프로그램 버전, 전체 명령과 옵션 목록 |

`--json`은 `read`, `vin`, `health`, `probe`, `freeze`, `uds`, `custom`을 스크립트용 JSON으로 출력하고, `--record FILE`은
어댑터와의 세션 전체를 `--replay`용으로 저장하며, `--fast-timing`은 어댑터를 공격적인 적응형
타이밍으로 바꾸고, `-v`는 어댑터 명령을 모두 기록합니다(`--json`과 함께 쓰면 stderr로). `caruspex_cli help`에서 전부 볼 수 있습니다.

```bash
target/release/caruspex_cli health --demo --json > health.json
```

이 페이지의 스크린샷은 데모 차량에서 가져온 것이며 앱이 직접 찍었습니다.
`--demo --page faults --shot shot.png --size 1440x920`은 페이지를 열고, 몇 초 뒤 창 이미지를 저장한 다음
종료합니다(`tools/screenshots.sh`는 전체 세트를 찍습니다).

## 프로젝트 구조

```
crates/caruspex-i18n/       언어: 독일어와 영어는 코드 안에, 나머지 16개는 translations/에서
crates/caruspex-vehicles/   VIN 디코더와 차량 카탈로그(브랜드, 모델, 세대, 커넥터, 버스)
crates/caruspex-data/       고장 코드, PID, 제어 장치 주소, 코드와 부품의 대응
crates/caruspex-link/       어댑터로 가는 길(시리얼, TCP, 블루투스 LE), ELM327 드라이버, 어댑터 시험
crates/caruspex-protocols/  OBD-II, UDS, KWP2000, KWP1281(K-Line, 바이트 단위)
crates/caruspex-net/        BMW ENET(HSFZ), DoIP, SLCAN, SocketCAN, J2534 PassThru
crates/caruspex-record/     녹화와 재생, 사용자 지정 값(Torque CSV, PID 팩), JSON 작성기
crates/caruspex-analysis/   상태 점검, 주행 사이클 안내, 트립 컴퓨터, 성능 타이머, 배터리 시험
crates/caruspex-demo/       내장 데모 차량: 모의 제어 장치가 있는 ELM327
crates/caruspex-cli/        명령줄 도구와 골든 파일 테스트
app/src/                    화면: React 19와 TypeScript(페이지, 셸, 상태, 보고서, i18n)
app/src-tauri/              Tauri 앱: 창, 명령, 업데이트 확인, 명령줄 옵션
translations/               코어와 화면 공용 번역 파일, 18개 언어
data/                       손으로 관리하는 CSV 차량 카탈로그, 제어 장치와 PID 소스
tools/                      생성기, 번역 검사, 패키징, 릴리스와 스크린샷 스크립트, ELM327 시뮬레이터
docs/                       README의 스크린샷
packaging/                  Windows 설치 프로그램 스크립트, Linux desktop 파일, macOS Info.plist
changelog/                  버전별 릴리스 노트, 독일어와 영어
```

고장 코드 데이터베이스는 생성됩니다. `tools/gen_dtc.py`가 규칙적인 EOBD 코드 계열과
`tools/dtc/`에 있는 엄선한 목록을 `crates/caruspex-data/src/generated/*.rs`로 펼칩니다. 데이터를 바꾼 뒤에는 다시 실행하세요.

```bash
python tools/gen_dtc.py
```

차량 카탈로그도 같은 방식입니다. `data/vehicles/`의 CSV 파일을 고친 다음 다음을 실행하세요.

```bash
python tools/gen_vehicles.py
```

제어 장치 주소와 PID 팩에는 `tools/gen_ecus.py`와 `tools/gen_pids.py`가 있습니다. 번역은
[translations/TRANSLATING.md](translations/TRANSLATING.md)에 설명되어 있습니다. 코드에 문구를 추가한 뒤에는
`python tools/i18n_rust.py`가 아직 번역해야 할 것을 나열해 줍니다.

## 코드 지우기에 대한 참고

OBD 모드 04는 고장 메모리, 프리즈 프레임, 준비 상태 모니터를 지웁니다. 지운 직후에는
차가 완전한 주행 사이클을 여러 번 마칠 때까지 배출가스 검사에 통과하지 못하며, 원인이 아직
남아 있는 코드는 잠깐 달리면 다시 나타납니다. 지우는 것은 수리가 아닙니다.

## 면책 조항

Caruspex는 취미 프로젝트이며 어떠한 보증도 없이 제공됩니다. 사용에 따른 책임은 본인에게 있으며,
작업해도 되는 차량에만, 그리고 운전 중에는 절대 사용하지 마세요. 모든 제품명과 브랜드명은 각
소유자의 상표입니다. Caruspex는 어떤 자동차 제조사와도 제휴하거나 보증을 받지 않았습니다.

## 라이선스

MIT, [LICENSE](LICENSE) 참고. 함께 들어 있는 글꼴은 SIL Open Font License 1.1로 배포됩니다.
JetBrains Mono([OFL.txt](resources/fonts/OFL.txt)), Inter([OFL-Inter.txt](resources/fonts/OFL-Inter.txt)),
Plus Jakarta Sans([OFL-PlusJakartaSans.txt](resources/fonts/OFL-PlusJakartaSans.txt)).
