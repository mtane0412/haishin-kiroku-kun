//! haishin-kiroku-kun の Tauri アプリケーションエントリポイント。
//!
//! `run()` が Tauri のビルダーを組み立て、アプリ起動時に SQLite DB を開いて
//! `AppState` として登録した上で、フロントエンドから呼び出される
//! `#[tauri::command]` 群を登録してイベントループを開始します。

mod audio;
mod commands;
mod store;
mod transcribe;

use std::sync::Mutex;

use commands::audio::{list_input_devices, start_audio_capture, stop_audio_capture, AudioState};
use commands::session::{end_session, list_sessions, start_session, AppState};
use commands::transcribe::{
    set_whisper_model_path, start_transcription, stop_transcription, TranscribeState,
};
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
            app.manage(AudioState::default());
            app.manage(TranscribeState::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_session,
            end_session,
            list_sessions,
            list_input_devices,
            start_audio_capture,
            stop_audio_capture,
            set_whisper_model_path,
            start_transcription,
            stop_transcription
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
