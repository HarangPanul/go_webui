// Entry point. Desktop/mobile shared logic lives in lib.rs (Tauri mobile 규약)
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    go_webui_lib::run();
}
