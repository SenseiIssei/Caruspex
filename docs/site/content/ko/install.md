# 설치

모든 릴리스는 [GitHub의 릴리스 페이지](https://github.com/SenseiIssei/Caruspex/releases/latest)에 있어요. 패키지에 아직 서명이 없어서 Windows와 macOS가 처음 실행하기 전에 한 번 확인을 요청해요.

## Windows 10과 11

| 파일 | 설명 |
|---|---|
| `Caruspex-<version>-windows-x64-setup.exe` | 설치 프로그램. 관리자 권한이 필요 없고, 명령줄 도구 `caruspex_cli`를 검색 경로에 넣을 수 있으며, WebView2 런타임은 없을 때만 설치해요. |
| `Caruspex-<version>-windows-x64.zip` | 포터블 버전: 압축을 풀고 `caruspex.exe`를 실행해요. |

처음 실행할 때 Windows SmartScreen이 알 수 없는 앱이라고 할 수 있어요: "추가 정보"를 누른 다음 "실행"을 누르세요.

USB 어댑터에는 USB 칩용 드라이버가 필요해요. Windows 10과 11은 보통 알아서 찾아요. 찾지 못하면 [USB로 연결하는 ELM327](elm327-usb.md)을 참고하세요.

## macOS 13 이상 (Apple Silicon)

`Caruspex-<version>-macos.dmg`를 내려받아 열고, Caruspex를 응용 프로그램으로 끌어다 놓으세요. 앱이 아직 공증을 받지 않았기 때문에, 처음에는 오른쪽 클릭 후 "열기"로 여세요.

## Linux (x86_64)

| 파일 | 설명 |
|---|---|
| `Caruspex-<version>-linux-x86_64.AppImage` | 거의 모든 배포판에서 실행돼요: `chmod +x`로 실행 가능하게 만든 뒤 실행하세요. 스스로 업데이트도 해요. |
| `Caruspex-<version>-linux-amd64.deb` | Debian과 Ubuntu용: `sudo apt install ./Caruspex-<version>-linux-amd64.deb` |

`sudo` 없이 시리얼 어댑터를 쓰려면 한 번만 자신을 `dialout` 그룹에 추가한 다음, 로그아웃했다가 다시 로그인하세요:

```
sudo usermod -aG dialout $USER
```

## 명령줄 도구만 쓰기

스크립트와 서버용으로 화면이 없는 `caruspex_cli`가 Windows, Linux, macOS용으로 있어요. `caruspex_cli help`로 명령 목록을 볼 수 있어요.

## 업데이트

실행할 때 Caruspex는 GitHub에 새 버전이 있는지 묻고, 그 외에는 아무것도 하지 않아요. 새 버전은 서명이 앱에 내장된 키와 일치할 때만 설치돼요. 확인과 자동 설치 모두 설정에서 끌 수 있어요.

## 패키지 관리자

winget, Homebrew, Flathub, Snap용 패키지를 준비하고 있어요. 공개되기 전까지는 위의 파일로 Caruspex를 설치하세요.
