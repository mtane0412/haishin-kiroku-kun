//! mic-app のバイナリエントリポイント。実処理は `mic_app_lib::run()` に委譲します。

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    mic_app_lib::run()
}
