//! `%APPDATA%\PadBatteryTray\settings.ini` 읽기·쓰기와 HKCU `Run` 키 자동 시작 등록.

use crate::state::Threshold;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const APP_DIR: &str = "PadBatteryTray";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub notify: bool,
    pub threshold: Threshold,
}

impl Default for Settings {
    fn default() -> Self {
        Self { notify: true, threshold: Threshold::Low }
    }
}

impl Settings {
    /// 모르는 키나 잘못된 값은 무시하고 기본값을 쓴다.
    pub fn parse(text: &str) -> Self {
        let mut s = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else { continue };
            match (key.trim(), value.trim().to_ascii_lowercase().as_str()) {
                ("notify", "1" | "true") => s.notify = true,
                ("notify", "0" | "false") => s.notify = false,
                ("threshold", "low") => s.threshold = Threshold::Low,
                ("threshold", "empty") => s.threshold = Threshold::Empty,
                _ => {}
            }
        }
        s
    }

    pub fn serialize(&self) -> String {
        let threshold = match self.threshold {
            Threshold::Low => "low",
            Threshold::Empty => "empty",
        };
        format!("notify={}\nthreshold={threshold}\n", u8::from(self.notify))
    }

    pub fn load(path: &Path) -> Self {
        fs::read_to_string(path).map(|t| Self::parse(&t)).unwrap_or_default()
    }

    /// 임시 파일에 쓴 뒤 교체해 중간에 끊겨도 기존 파일이 깨지지 않게 한다.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("ini.tmp");
        fs::write(&tmp, self.serialize())?;
        fs::rename(&tmp, path)
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
pub fn default_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|dir| PathBuf::from(dir).join(APP_DIR).join("settings.ini"))
}

#[cfg(windows)]
pub mod autostart {
    use windows::core::{w, HSTRING, PCWSTR};
    use windows::Win32::System::Registry::*;

    const RUN_KEY: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");
    const VALUE: PCWSTR = w!("PadBatteryTray");

    fn command() -> Option<String> {
        let exe = std::env::current_exe().ok()?;
        Some(format!("\"{}\"", exe.display()))
    }

    /// 값이 있고 현재 exe를 가리킬 때만 켜진 것으로 본다. exe를 옮겼으면 꺼진 것으로 보여 다시 켜게 한다.
    pub fn is_enabled() -> bool {
        let Some(expected) = command() else { return false };
        let mut buf = [0u16; 1024];
        let mut len = (buf.len() * 2) as u32;
        let rc = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                RUN_KEY,
                VALUE,
                RRF_RT_REG_SZ,
                None,
                Some(buf.as_mut_ptr().cast()),
                Some(&mut len),
            )
        };
        if rc.is_err() {
            return false;
        }
        let chars = (len as usize / 2).saturating_sub(1);
        String::from_utf16_lossy(&buf[..chars]).eq_ignore_ascii_case(&expected)
    }

    pub fn set(enabled: bool) -> windows::core::Result<()> {
        unsafe {
            if enabled {
                let cmd = HSTRING::from(command().unwrap_or_default());
                let bytes = (cmd.len() + 1) * 2;
                RegSetKeyValueW(
                    HKEY_CURRENT_USER,
                    RUN_KEY,
                    VALUE,
                    REG_SZ.0,
                    Some(cmd.as_ptr().cast()),
                    bytes as u32,
                )
                .ok()
            } else {
                match RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, VALUE) {
                    rc if rc == windows::Win32::Foundation::ERROR_FILE_NOT_FOUND => Ok(()),
                    rc => rc.ok(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let s = Settings { notify: false, threshold: Threshold::Empty };
        assert_eq!(Settings::parse(&s.serialize()), s);
    }

    #[test]
    fn parse_tolerates_garbage() {
        let s = Settings::parse("junk\nnotify = 0\r\nthreshold=bogus\nother=1");
        assert_eq!(s, Settings { notify: false, threshold: Threshold::Low });
    }

    #[test]
    fn save_and_load_in_temp_dir() {
        let dir = std::env::temp_dir().join(format!("pbt-test-{}", std::process::id()));
        let path = dir.join(APP_DIR).join("settings.ini");
        assert_eq!(Settings::load(&path), Settings::default());

        let s = Settings { notify: false, threshold: Threshold::Empty };
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);
        assert!(!path.with_extension("ini.tmp").exists());

        fs::remove_dir_all(dir).unwrap();
    }
}
