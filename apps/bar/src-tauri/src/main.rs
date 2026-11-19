// Hides the extra console window in release builds on Windows. Do not remove.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // The uninstaller calls this; see `uninstall_cleanup`.
    if std::env::args().any(|a| a == "--uninstall-cleanup") {
        barista_lib::uninstall_cleanup();
        return;
    }
    barista_lib::run();
}
