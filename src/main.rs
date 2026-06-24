#![windows_subsystem = "windows"]

mod app;
mod config;
mod core;
mod services;
mod ui;

fn main() {
    #[cfg(windows)]
    enable_dpi_awareness();

    if let Err(err) = app::launch() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

#[cfg(windows)]
fn enable_dpi_awareness() {
    use windows_sys::Win32::UI::HiDpi::{SetProcessDpiAwareness, PROCESS_PER_MONITOR_DPI_AWARE};
    unsafe {
        let _ = SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE);
    }
}
