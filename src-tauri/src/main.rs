//! haishin-kiroku-kun のバイナリエントリポイント。実処理は
//! `haishin_kiroku_kun_lib::run()` に委譲します。

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    haishin_kiroku_kun_lib::run()
}
