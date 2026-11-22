#![cfg_attr(windows, windows_subsystem = "windows")]

mod badge;
mod battery;
mod settings;
mod state;

#[cfg(windows)]
mod app;
#[cfg(windows)]
mod tray;

#[cfg(windows)]
fn main() {
    if let Err(e) = app::run() {
        app::fatal(&e.to_string());
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("Windows 전용 프로그램이다. `cargo test`로 순수 로직만 검증할 수 있다.");
}
