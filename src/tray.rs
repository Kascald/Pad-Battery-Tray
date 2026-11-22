//! 트레이 아이콘·툴팁·알림. 같은 화면이면 `Shell_NotifyIconW`를 다시 부르지 않는다.

use crate::badge;
use crate::state::{Display, TrayView};
use std::collections::HashMap;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::Controls::LoadIconWithScaleDown;
use windows::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub const WM_TRAY: u32 = WM_APP + 1;
const ICON_ID: u32 = 1;

fn icon_resource_id(d: Display) -> u16 {
    match d {
        Display::NoConnection => 101,
        Display::Pending => 102,
        Display::Wired => 103,
        Display::Empty => 104,
        Display::Low => 105,
        Display::Medium => 106,
        Display::Full => 107,
    }
}

fn copy_wstr(dst: &mut [u16], s: &str) {
    let (mut n, max) = (0, dst.len() - 1);
    for (d, c) in dst.iter_mut().zip(s.encode_utf16()).take(max) {
        *d = c;
        n += 1;
    }
    dst[n] = 0;
}

type IconKey = (Display, Option<u8>);

pub struct Tray {
    hwnd: HWND,
    hinst: HINSTANCE,
    size: i32,
    icons: HashMap<IconKey, HICON>,
    added: bool,
    applied: Option<TrayView>,
}

impl Tray {
    pub fn new(hwnd: HWND, hinst: HINSTANCE) -> Self {
        let mut tray = Self { hwnd, hinst, size: 16, icons: HashMap::new(), added: false, applied: None };
        tray.size = tray.current_icon_size();
        tray
    }

    fn current_icon_size(&self) -> i32 {
        unsafe { GetSystemMetricsForDpi(SM_CXSMICON, GetDpiForWindow(self.hwnd)) }.max(16)
    }

    fn data(&self) -> NOTIFYICONDATAW {
        NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: ICON_ID,
            ..Default::default()
        }
    }

    pub fn show(&mut self, view: &TrayView) {
        if self.added && self.applied.as_ref() == Some(view) {
            return;
        }
        let Some(icon) = self.icon((view.icon, view.badge)) else { return };

        let mut nid = self.data();
        nid.uFlags = NIF_ICON | NIF_TIP | NIF_SHOWTIP | NIF_MESSAGE;
        nid.uCallbackMessage = WM_TRAY;
        nid.hIcon = icon;
        copy_wstr(&mut nid.szTip, &view.tooltip);

        unsafe {
            if self.added && !Shell_NotifyIconW(NIM_MODIFY, &nid).as_bool() {
                self.added = false;
            }
            if !self.added {
                // Explorer가 아직 준비되지 않았으면 실패한다. 다음 갱신이나 TaskbarCreated에서 다시 시도한다
                if !Shell_NotifyIconW(NIM_ADD, &nid).as_bool() {
                    self.applied = None;
                    return;
                }
                nid.Anonymous.uVersion = NOTIFYICON_VERSION_4;
                let _ = Shell_NotifyIconW(NIM_SETVERSION, &nid);
                self.added = true;
            }
        }
        self.applied = Some(view.clone());
    }

    /// Explorer 재시작으로 아이콘이 사라졌을 때. 다음 `show`가 NIM_ADD로 다시 등록한다.
    pub fn forget(&mut self) {
        self.added = false;
        self.applied = None;
    }

    /// DPI·디스플레이·설정 변경 시 아이콘 크기가 바뀌었으면 캐시를 버린다. 바뀌었으면 참을 돌려준다.
    pub fn reset_icons(&mut self) -> bool {
        let size = self.current_icon_size();
        if size == self.size {
            return false;
        }
        self.size = size;
        self.destroy_icons();
        self.applied = None;
        true
    }

    pub fn notify(&self, title: &str, body: &str) {
        if !self.added {
            return;
        }
        let mut nid = self.data();
        nid.uFlags = NIF_INFO;
        nid.dwInfoFlags = NIIF_WARNING | NIIF_RESPECT_QUIET_TIME;
        copy_wstr(&mut nid.szInfoTitle, title);
        copy_wstr(&mut nid.szInfo, body);
        unsafe {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
        }
    }

    pub fn remove(&mut self) {
        if self.added {
            unsafe {
                let _ = Shell_NotifyIconW(NIM_DELETE, &self.data());
            }
            self.added = false;
        }
    }

    fn icon(&mut self, key: IconKey) -> Option<HICON> {
        if let Some(&h) = self.icons.get(&key) {
            return Some(h);
        }
        let base = match self.icons.get(&(key.0, None)) {
            Some(&h) => h,
            None => {
                let id = PCWSTR(icon_resource_id(key.0) as usize as *const u16);
                let h = unsafe { LoadIconWithScaleDown(Some(self.hinst), id, self.size, self.size) }.ok()?;
                self.icons.insert((key.0, None), h);
                h
            }
        };
        let Some(digit) = key.1 else { return Some(base) };
        // 번호를 못 찍으면 번호 없는 아이콘으로 대신한다
        let h = with_badge(base, self.size, digit).unwrap_or(base);
        if h != base {
            self.icons.insert(key, h);
        }
        Some(h)
    }

    fn destroy_icons(&mut self) {
        for (_, h) in self.icons.drain() {
            unsafe {
                let _ = DestroyIcon(h);
            }
        }
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        self.remove();
        self.destroy_icons();
    }
}

fn bitmap_info(size: i32) -> BITMAPINFO {
    BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size,
            biHeight: -size,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    }
}

/// 기본 아이콘의 32bpp 픽셀을 꺼내 번호를 찍고 새 아이콘을 만든다.
fn with_badge(base: HICON, size: i32, digit: u8) -> Option<HICON> {
    let n = size as usize;
    let mut px = vec![0u32; n * n];
    unsafe {
        let mut info = ICONINFO::default();
        GetIconInfo(base, &mut info).ok()?;
        let mut bmi = bitmap_info(size);
        let hdc = GetDC(None);
        let lines = GetDIBits(hdc, info.hbmColor, 0, size as u32, Some(px.as_mut_ptr().cast()), &mut bmi, DIB_RGB_COLORS);
        ReleaseDC(None, hdc);
        let _ = DeleteObject(info.hbmColor.into());
        let _ = DeleteObject(info.hbmMask.into());
        // 알파 채널이 없는 아이콘이면 합성 결과가 깨지므로 포기한다
        if lines != size || px.iter().all(|p| p >> 24 == 0) {
            return None;
        }

        badge::stamp(&mut px, n, digit);

        let mut bits = std::ptr::null_mut();
        let color = CreateDIBSection(None, &bitmap_info(size), DIB_RGB_COLORS, &mut bits, None, 0).ok()?;
        std::ptr::copy_nonoverlapping(px.as_ptr(), bits.cast::<u32>(), px.len());
        let mask_bits = vec![0u8; n.div_ceil(16) * 2 * n];
        let mask = CreateBitmap(size, size, 1, 1, Some(mask_bits.as_ptr().cast()));
        let icon = CreateIconIndirect(&ICONINFO { fIcon: true.into(), hbmMask: mask, hbmColor: color, ..Default::default() });
        let _ = DeleteObject(color.into());
        let _ = DeleteObject(mask.into());
        icon.ok()
    }
}
