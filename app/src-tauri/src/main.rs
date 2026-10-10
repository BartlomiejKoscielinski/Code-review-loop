// Release builds on Windows start without an extra console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // A release build on Windows has no console, so this message is lost there.
    // P1-29 adds the log file that makes startup failures visible.
    if let Err(error) = lupa_app_lib::run() {
        eprintln!("Lupa failed to start: {error}");
        std::process::exit(1);
    }
}
