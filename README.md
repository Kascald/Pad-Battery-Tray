# Pad Battery Tray

XInput 패드(최대 4대)의 배터리 잔량을 트레이 아이콘으로 보여 주는 단일 exe 앱. 런타임 의존이 없다.

- 아이콘: 연결 없음 · 대기 · 유선 · 비어 있음 · 낮음 · 보통 · 가득
- 2대 이상이면 5초마다 슬롯을 순환하고 아이콘 오른쪽 아래에 슬롯 번호를 찍는다
- 툴팁: 연결된 패드를 `P1: 보통` 형식으로 한 줄씩
- 낮음(또는 비어 있음)으로 내려가면 알림을 한 번 띄운다. 가득 또는 유선 연결 뒤에만 다시 알린다
- 우클릭 메뉴: Windows 시작 시 실행(HKCU `Run`), 배터리 부족 알림, 알림 기준, 종료
- 설정: `%APPDATA%\PadBatteryTray\settings.ini`

지원: Windows 10/11 x64. Xbox 무선 어댑터, USB 유선, Xbox 360 무선 수신기. 블루투스 Series X|S와 비XInput 패드는 제외.

## 빌드

Windows(MSVC):

```
cargo build --release
```

Linux에서 크로스 빌드:

```
rustup target add x86_64-pc-windows-gnu
sudo apt-get install mingw-w64
cargo build --release --target x86_64-pc-windows-gnu
```

테스트(순수 로직, 어느 OS에서나):

```
cargo test
```

아이콘을 바꾸려면 `tools/gen_icons.py`를 고치고 `python3 tools/gen_icons.py`로 `assets/icons/*.ico`를 다시 만든다(Pillow 8.1+).

## 구조

| 모듈 | 책임 |
|---|---|
| `battery` | `XInputGetBatteryInformation` 호출. `BatterySource` 트레이트 뒤에 있어 테스트에서 가짜 구현을 쓴다 |
| `state` | 원시값 → 표시 상태, 알림 판단, 슬롯 순환, 툴팁 문자열. 순수 로직 |
| `badge` | 아이콘 픽셀에 슬롯 번호를 찍는다. 순수 로직 |
| `settings` | 설정 파일과 자동 시작 레지스트리 |
| `tray` | `Shell_NotifyIconW`, 아이콘 로드·캐시 |
| `app` | 숨김 최상위 창, 타이머, 메시지 처리, 메뉴 |

## 클린룸

기존 XB1ControllerBatteryIndicator(GPL-2.0)의 소스·리소스·문자열을 참조하지 않고, 아래 공개 문서만으로 작성했다. 아이콘은 `tools/gen_icons.py`로 새로 그렸다.

- Microsoft Learn: `XInputGetBatteryInformation`, `XINPUT_BATTERY_INFORMATION`
- Microsoft Learn: `Shell_NotifyIconW`, `NOTIFYICONDATAW`, `LoadIconWithScaleDown`, `WM_DEVICECHANGE`
- `windows` crate 문서

## 라이선스

MIT
