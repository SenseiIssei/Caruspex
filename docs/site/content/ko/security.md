# 보안과 업데이트

이 페이지는 Caruspex의 보안 정보입니다. 프로그램의 용도, 알아 두어야 할 위험, 업데이트 방식, 삭제 방법을 담았습니다. EU 사이버 복원력법(Cyber Resilience Act, 부속서 II)이 사용자에게 요구하는 정보 가운데 무료이며 비상업적인 프로그램에 해당하는 부분을 따릅니다.

## 프로그램

- **이름**: Caruspex. 앱 `caruspex`와 명령줄 도구 `caruspex_cli`가 있으며 Windows, Linux, macOS에서 동작합니다. 버전은 모든 다운로드 파일 이름과 각 릴리스의 변경 목록에 적혀 있습니다.
- **배포**: [GitHub](https://github.com/SenseiIssei/Caruspex)의 Caruspex 프로젝트이며, MIT 라이선스의 자유 소프트웨어입니다. 질문과 버그는 [issues](https://github.com/SenseiIssei/Caruspex/issues)에 남겨 주세요.
- **용도**: 자신의 어댑터로, 자신의 컴퓨터에서 자동차, 오토바이, 트럭의 진단 데이터를 읽고 쉽게 설명합니다. 계정도, 클라우드도, 추적도 없습니다([개인정보](privacy.md)).
- **보안 특성**: 읽기는 차량의 어떤 것도 바꾸지 않으며, 차량이 무언가를 하게 만드는 모든 작업은 먼저 묻습니다. 업데이트는 서명이 앱에 내장된 키와 일치할 때만 설치됩니다. Caruspex는 네트워크 포트를 열지 않고, 파일이나 인터넷에서 온 코드를 실행하지 않습니다.

## 취약점 신고 {#report}

보안 문제는 공개 issue가 아니라 [새 보안 권고 양식](https://github.com/SenseiIssei/Caruspex/security/advisories/new)으로 비공개로 신고해 주세요. 이후 절차는 [보안 정책](https://github.com/SenseiIssei/Caruspex/blob/main/SECURITY.md)에 있습니다. 수정된 취약점은 [보안 권고](https://github.com/SenseiIssei/Caruspex/security/advisories)로 공개됩니다.

## 알아 두어야 할 위험 {#risks}

- Caruspex는 [릴리스 페이지](https://github.com/SenseiIssei/Caruspex/releases)에서만 내려받으세요. 다른 곳의 사본은 변조되었을 수 있습니다.
- 기록 파일, PID 팩, 3D 모델은 믿을 수 있는 사람에게 받은 것만 여세요. 코드를 실행할 수는 없지만, 손상된 파일 때문에 Caruspex가 느려지거나 닫힐 수 있습니다.
- 고장 코드 삭제, 액추에이터 테스트, 기본 설정은 차량이 멈춰 있고 안전한 곳에 있을 때만 하세요.
- 많은 블루투스 및 Wi-Fi 어댑터에는 PIN이 없거나 네트워크가 열려 있어 근처의 누구나 연결할 수 있습니다. 작업이 끝나면 어댑터를 뽑으세요.
- ENET 케이블, DoIP, Wi-Fi 어댑터는 아무것도 암호화하지 않습니다. 직접 연결하거나 자신의 네트워크에서만 사용하세요.
- J2534 드라이버는 제조사의 프로그램이며 사용자의 권한으로 실행됩니다. 제조사에서 받은 것만 설치하세요.

## 업데이트와 지원 {#support}

- 설치할 때마다, 이전 버전에서 업데이트한 뒤에도, Caruspex는 한 번 "업데이트를 확인할까요?"라고 묻습니다. "예"라고 답한 뒤에야 시작할 때와 실행 중 여섯 시간마다 GitHub에 새 버전이 있는지 묻습니다. 새 버전이 있어도 제목 표시줄의 버튼에 불이 들어올 뿐 작업을 방해하지 않습니다. 설치는 한 번의 클릭이며, 서명이 일치하는 패키지만 설치됩니다. AppImage와 macOS 앱은 스스로 업데이트되고, .deb 패키지와 Android 앱에는 다운로드 페이지가 열립니다.
- 확인과 자동 설치는 설정의 **창과 언어**에 있는 스위치이며 언제든 바꿀 수 있습니다. "예"라고 답하지 않으면 GitHub로 어떤 요청도 가지 않으니, 그럴 때는 가끔 릴리스 페이지를 확인하세요.
- 보안 수정은 최신 릴리스에만 들어갑니다. 이전 버전은 별도의 수정을 받지 않으므로 Caruspex를 최신으로 유지하세요.
- Caruspex는 한 사람이 여가 시간에, 시간이 허락하는 만큼 관리하며 어떤 약속도 하지 않습니다. 지원 기간도 없고 답변 시간도 없습니다. 관리가 끝나면 저장소를 보관(아카이브) 처리하고 그렇게 밝힙니다.

## 다운로드 확인 {#verify}

각 릴리스에는 모든 파일의 체크섬이 담긴 `SHA256SUMS` 파일과, 파일이 어떻게 빌드되었는지에 대한 GitHub의 서명된 기록이 있습니다. 파일을 한 폴더에 두고:

```
sha256sum --ignore-missing -c SHA256SUMS
gh attestation verify Caruspex-<version>-windows-x64-setup.exe --repo SenseiIssei/Caruspex
```

이 도구가 없다면 PowerShell의 `Get-FileHash <파일>`(Windows) 또는 `shasum -a 256 <파일>`(macOS)로 체크섬을 보고 `SHA256SUMS`의 줄과 비교하세요.

## Caruspex와 데이터 삭제 {#remove}

1. 프로그램을 제거합니다. Windows에서는 설정의 앱에서, macOS에서는 Caruspex를 응용 프로그램 폴더에서 빼내고, Linux에서는 AppImage를 지우거나 `sudo apt remove caruspex`를 실행합니다.
2. 데이터를 삭제합니다. 설정의 **저장 위치 열기** 버튼이 설정, 차고, 충돌 보고서가 있는 폴더를 보여 줍니다. 그 폴더를 지우세요. 직접 저장한 기록과 보고서는 저장한 곳에 있습니다.

## 개발자와 기업을 위해 {#integrators}

- 각 릴리스에는 플랫폼별 소프트웨어 자재 명세서 `Caruspex-<version>-sbom-<platform>.cdx.json`(CycloneDX)가 있으며, 모든 라이브러리와 그 버전이 들어 있습니다.
- Caruspex에 대한 위험 평가, 요구 사항 목록, Cyber Resilience Act의 적용 범위는 [docs/cra](https://github.com/SenseiIssei/Caruspex/tree/main/docs/cra)에 있습니다.
- Caruspex는 Cyber Resilience Act에서 말하는 제품으로 시장에 출시되지 않으므로 EU 적합성 선언과 CE 마크가 없습니다.
