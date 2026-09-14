//! haishin-kiroku-kun の Tauri アプリケーションエントリポイント。
//!
//! `run()` が Tauri のビルダーを組み立て、アプリ起動時に SQLite DB を開いて
//! `AppState` として登録した上で、フロントエンドから呼び出される
//! `#[tauri::command]` 群を登録してイベントループを開始します。

mod commands;
mod store;

use std::sync::Mutex;

use commands::session::{AppState, end_session, list_sessions, start_session};
use tauri::Manager;

/// DB ファイル名です。`app_data_dir()` 配下に配置します。
const DB_FILE_NAME: &str = "haishin-kiroku-kun.sqlite3";

/// Tauri アプリケーションを起動します。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("アプリデータディレクトリを解決できること");
            let db_path = data_dir.join(DB_FILE_NAME);
            let conn = store::open_at(&db_path).expect("SQLiteデータベースを開けること");
            app.manage(AppState {
                db: Mutex::new(conn),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_session,
            end_session,
            list_sessions
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
