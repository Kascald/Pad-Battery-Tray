# Pad Battery Tray

Xbox 계열(XInput) 패드의 배터리 잔량을 작업 표시줄 트레이 아이콘으로 보여 주는 Windows 앱이다. 단일 exe라 설치나 런타임이 필요 없다.

## 지원 환경

- Windows 10/11 x64
- 패드 최대 4대

| 연결 방식 | 지원 |
|---|---|
| Xbox 무선 어댑터(USB 동글) | 지원 |
| USB 유선 | 지원(잔량 대신 `유선`으로 표시) |
| Xbox 360 무선 수신기 | 지원 |
| 블루투스(Series X\|S) | 미지원(Windows가 잔량을 제대로 보고하지 않음) |
| 블루투스(One S) | 동작하면 표시하지만 보장하지 않음 |
| DualSense 등 비XInput 패드 | 미지원 |

## 메모리 사용량

> **주의**: 아래 수치는 한 대의 PC에서 잰 참고값이다. Windows 버전, 드라이버, 연결한 패드와 어댑터 등 PC 환경에 따라 다른 결과가 나올 수 있다.

패드를 연결하지 않은 대기 상태에서 같은 PC로 잰 값이다.

| 앱 | Working Set |
|---|---|
| 기존에 쓰던 WPF 기반 배터리 표시 앱 | 약 298MB |
| Pad Battery Tray | 약 13MB (앱이 직접 할당한 메모리 약 2.5MB) |

패드를 1대 연결하면 XInput이 장치 관련 시스템 모듈을 불러오면서 Working Set이 약 20MB로 늘어난다. 앱이 직접 할당한 메모리는 3.5MB 수준을 유지하고, CPU 사용률은 0%에 가깝다.

## 사용법

`pad-battery-tray.exe`를 실행하면 트레이에 아이콘이 나타난다. 창은 없다. 이미 실행 중이면 새로 띄우지 않는다.

### 아이콘

| 아이콘 | 의미 |
|---|---|
| <img src="docs/icons/none.png" width="32" alt="연결 없음"> | 연결된 패드 없음 |
| <img src="docs/icons/pending.png" width="32" alt="대기"> | 연결 직후 잔량 확인 중 |
| <img src="docs/icons/wired.png" width="32" alt="유선"> | 유선 연결 |
| <img src="docs/icons/empty.png" width="32" alt="비어 있음"> | 비어 있음 |
| <img src="docs/icons/low.png" width="32" alt="낮음"> | 낮음 |
| <img src="docs/icons/medium.png" width="32" alt="보통"> | 보통 |
| <img src="docs/icons/full.png" width="32" alt="가득"> | 가득 |
| <img src="docs/icons/badge-1.png" width="32" alt="P1 가득"> <img src="docs/icons/badge-2.png" width="32" alt="P2 낮음"> | 패드 2대 이상일 때 슬롯 번호 |

- 패드가 2대 이상이면 5초마다 다음 패드로 아이콘이 바뀌고, 오른쪽 아래에 슬롯 번호(1~4)가 붙는다.
- 아이콘에 마우스를 올리면 연결된 패드 전체가 `P1: 보통` 형식으로 나온다.
- 잔량은 Windows가 주는 4단계(비어 있음·낮음·보통·가득)뿐이라 퍼센트는 표시하지 않는다.

### 데모 모드

패드 없이 아이콘과 알림이 제대로 나오는지 확인할 때 쓴다.

```
pad-battery-tray.exe --demo
```

2초마다 모든 아이콘, 2대 연결 시의 번호 순환, 배터리 부족 알림을 차례로 보여 주고 반복한다. 툴팁 첫 줄에 `데모 모드`가 표시된다. 평소 실행과 동시에 띄울 수 있고, 메뉴에서 바꾼 설정은 저장하지 않는다. 우클릭 메뉴의 `종료`로 끝낸다.

### 배터리 부족 알림

잔량이 `낮음`으로 내려가면 Windows 알림을 한 번 띄운다. 같은 패드는 `가득`이 되거나 유선으로 연결한 뒤에만 다시 알린다. 앱을 켤 때 이미 낮은 패드가 있으면 그때 한 번 알린다.

집중 지원이나 게임 모드에서는 Windows가 알림을 숨길 수 있다. 이때도 트레이 아이콘으로 상태를 확인할 수 있다.

### 우클릭 메뉴

| 메뉴 | 동작 |
|---|---|
| Windows 시작 시 실행 | 로그인할 때 자동으로 실행한다. 관리자 권한은 필요 없다 |
| 배터리 부족 알림 | 알림을 켜거나 끈다 |
| 알림 기준 | `낮음부터` 또는 `비어 있을 때만` |
| 종료 | 앱을 끝낸다 |

설정은 `%APPDATA%\PadBatteryTray\settings.ini`에 저장되어 다시 실행해도 유지된다.

exe를 다른 폴더로 옮겼다면 메뉴에서 `Windows 시작 시 실행`을 다시 켠다.

### 제거

1. 우클릭 메뉴에서 `Windows 시작 시 실행`을 끈다.
2. `종료`한다.
3. exe와 `%APPDATA%\PadBatteryTray` 폴더를 지운다.

## 직접 빌드

Windows:

```
cargo build --release
```

Linux에서 크로스 빌드:

```
rustup target add x86_64-pc-windows-gnu
sudo apt-get install mingw-w64
cargo build --release --target x86_64-pc-windows-gnu
```

결과물은 `target/.../release/pad-battery-tray.exe`다.

## 라이선스

MIT
