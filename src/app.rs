//! 숨김 최상위 창, 메시지 루프, 우클릭 메뉴.
//!
//! `TaskbarCreated`와 `WM_DEVICECHANGE`(DBT_DEVNODES_CHANGED)는 최상위 창에만 브로드캐스트되므로
//! `HWND_MESSAGE` 창 대신 표시하지 않는 `WS_EX_TOOLWINDOW` 최상위 창을 쓴다.

use crate::battery::{BatterySource, XInput};
use crate::settings::{self, autostart, Settings};
use crate::state::{alert_message, Monitor, Threshold, TrayView};
use crate::tray::{Tray, WM_TRAY};
use std::cell::RefCell;
use std::path::PathBuf;
use windows::core::{w, Result, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::*;

const POLL_TIMER: usize = 1;
const POLL_MS: u32 = 5_000;
const DBT_DEVNODES_CHANGED: usize = 0x0007;

const CMD_AUTOSTART: u32 = 1;
const CMD_NOTIFY: u32 = 2;
const CMD_THRESHOLD_LOW: u32 = 3;
const CMD_THRESHOLD_EMPTY: u32 = 4;
const CMD_EXIT: u32 = 9;

struct App {
    tray: Tray,
    monitor: Monitor,
    source: XInput,
    settings: Settings,
    settings_path: Option<PathBuf>,
    view: Option<TrayView>,
}

impl App {
    fn refresh(&mut self, advance: bool) {
        let (view, alerts) = self.monitor.refresh(self.source.read_all(), self.settings.threshold, advance);
        self.tray.show(&view);
        self.view = Some(view);
        if self.settings.notify {
            if let Some((title, body)) = alert_message(&alerts) {
                self.tray.notify(&title, &body);
            }
        }
    }

    fn redraw(&mut self) {
        if let Some(view) = self.view.clone() {
            self.tray.show(&view);
        }
    }

    fn update_settings(&mut self, f: impl FnOnce(&mut Settings)) {
        f(&mut self.settings);
        if let Some(path) = &self.settings_path {
            if self.settings.save(path).is_err() {
                self.tray.notify("설정 저장 실패", &format!("{} 에 쓸 수 없습니다.", path.display()));
            }
        }
    }
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    static TASKBAR_CREATED: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// 메뉴의 모달 루프 안에서도 WndProc가 다시 불리므로, 빌림은 짧게 잡고 겹치면 건너뛴다.
fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|cell| cell.try_borrow_mut().ok()?.as_mut().map(f))
}

pub fn fatal(msg: &str) {
    let text = windows::core::HSTRING::from(msg);
    unsafe {
        MessageBoxW(None, &text, w!("Pad Battery Tray"), MB_ICONERROR | MB_OK);
    }
}

pub fn run() -> Result<()> {
    unsafe {
        let _mutex = CreateMutexW(None, true, w!(r"Local\PadBatteryTray.Instance"))?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return Ok(());
        }

        let hinst: HINSTANCE = GetModuleHandleW(None)?.into();
        let class = w!("PadBatteryTrayWindow");
        let wc = WNDCLASSW { lpfnWndProc: Some(wnd_proc), hInstance: hinst, lpszClassName: class, ..Default::default() };
        if RegisterClassW(&wc) == 0 {
            return Err(windows::core::Error::from_thread());
        }
        TASKBAR_CREATED.set(RegisterWindowMessageW(w!("TaskbarCreated")));

        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class,
            w!("Pad Battery Tray"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinst),
            None,
        )?;

        let settings_path = settings::default_path();
        let settings = settings_path.as_deref().map(Settings::load).unwrap_or_default();
        APP.set(Some(App {
            tray: Tray::new(hwnd, hinst),
            monitor: Monitor::default(),
            source: XInput,
            settings,
            settings_path,
            view: None,
        }));
        with_app(|app| app.refresh(false));
        SetTimer(Some(hwnd), POLL_TIMER, POLL_MS, None);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        // 트레이 아이콘 제거와 아이콘 핸들 정리는 Drop에서 한다
        APP.take();
    }
    Ok(())
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_TIMER if wparam.0 == POLL_TIMER => {
            with_app(|app| app.refresh(true));
        }
        WM_DEVICECHANGE if wparam.0 == DBT_DEVNODES_CHANGED => {
            with_app(|app| app.refresh(false));
        }
        WM_DPICHANGED | WM_DISPLAYCHANGE | WM_SETTINGCHANGE => {
            with_app(|app| {
                if app.tray.reset_icons() {
                    app.redraw();
                }
            });
            return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
        }
        WM_TRAY => {
            // NOTIFYICON_VERSION_4: lParam 하위 워드가 이벤트, wParam이 메뉴 기준 좌표다
            if (lparam.0 & 0xFFFF) as u32 == WM_CONTEXTMENU {
                let x = (wparam.0 & 0xFFFF) as i16 as i32;
                let y = ((wparam.0 >> 16) & 0xFFFF) as i16 as i32;
                unsafe { show_menu(hwnd, x, y) };
            }
        }
        WM_DESTROY => unsafe { PostQuitMessage(0) },
        m if m != 0 && m == TASKBAR_CREATED.get() => {
            with_app(|app| {
                app.tray.forget();
                app.redraw();
            });
        }
        _ => return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
    LRESULT(0)
}

unsafe fn show_menu(hwnd: HWND, x: i32, y: i32) {
    let Some(settings) = with_app(|app| app.settings) else { return };
    let auto = autostart::is_enabled();

    let check = |on: bool| if on { MF_CHECKED } else { MF_UNCHECKED };
    let cmd = unsafe {
        let Ok(menu) = CreatePopupMenu() else { return };
        let Ok(sub) = CreatePopupMenu() else {
            let _ = DestroyMenu(menu);
            return;
        };
        let item = |m, flags, id: u32, text: PCWSTR| {
            let _ = AppendMenuW(m, flags, id as usize, text);
        };
        item(menu, MF_STRING | check(auto), CMD_AUTOSTART, w!("Windows 시작 시 실행"));
        item(menu, MF_STRING | check(settings.notify), CMD_NOTIFY, w!("배터리 부족 알림"));
        item(sub, MF_STRING | check(settings.threshold == Threshold::Low), CMD_THRESHOLD_LOW, w!("낮음부터"));
        item(sub, MF_STRING | check(settings.threshold == Threshold::Empty), CMD_THRESHOLD_EMPTY, w!("비어 있을 때만"));
        let sub_flags = if settings.notify { MF_POPUP } else { MF_POPUP | MF_GRAYED };
        let _ = AppendMenuW(menu, sub_flags, sub.0 as usize, w!("알림 기준"));
        item(menu, MF_SEPARATOR, 0, PCWSTR::null());
        item(menu, MF_STRING, CMD_EXIT, w!("종료"));

        // 포그라운드가 아니면 메뉴 밖을 눌러도 닫히지 않는다
        let _ = SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenuEx(menu, (TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON).0, x, y, hwnd, None);
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
        cmd.0 as u32
    };

    match cmd {
        CMD_AUTOSTART => {
            if autostart::set(!auto).is_err() {
                with_app(|app| app.tray.notify("자동 시작 설정 실패", "레지스트리에 쓸 수 없습니다."));
            }
        }
        CMD_NOTIFY => {
            with_app(|app| app.update_settings(|s| s.notify = !s.notify));
        }
        CMD_THRESHOLD_LOW => {
            with_app(|app| app.update_settings(|s| s.threshold = Threshold::Low));
        }
        CMD_THRESHOLD_EMPTY => {
            with_app(|app| app.update_settings(|s| s.threshold = Threshold::Empty));
        }
        CMD_EXIT => {
            with_app(|app| app.tray.remove());
            let _ = unsafe { DestroyWindow(hwnd) };
        }
        _ => {}
    }
}
