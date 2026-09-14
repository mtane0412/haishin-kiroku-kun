//! mic-app の Tauri アプリケーションエントリポイント。
//!
//! `run()` が Tauri のビルダーを組み立て、フロントエンドから呼び出される
//! `#[tauri::command]` 群を登録した上でイベントループを開始します。
//! 現時点ではスキャフォールド直後のため、動作確認用の `greet` コマンドのみを
//! 登録しています。以降のフェーズで音声・文字起こし・Twitch・永続化・API の
//! 各モジュールをここに接続していきます。

/// 動作確認用の挨拶コマンドです。フロントエンドから `invoke("greet", { name })`
/// で呼び出せます。実装が進み次第、本来のコマンド群に置き換えます。
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Tauri アプリケーションを起動します。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
