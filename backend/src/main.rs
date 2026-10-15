// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
mod webview_runtime;

fn main() {
    #[cfg(windows)]
    if !webview_runtime::ensure_available() {
        std::process::exit(1);
    }
    usagepin_lib::run()
}
