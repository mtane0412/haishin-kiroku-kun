//! セッションの開始・終了・一覧取得を行う Tauri コマンドです。
//!
//! フェーズ1時点では文字起こしエンジンの選択 UI がまだ無いため、`engine` は
//! [`DEFAULT_ENGINE`] 固定、`twitch_channel` は未指定（`None`）としています。
//! これらはフェーズ3以降でエンジン選択・Twitch 連携が実装され次第、
//! コマンドの引数として受け取れるように拡張します。

use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;
use tauri::State;

use crate::store::repo::{self, Session};

/// フェーズ1時点で使用する固定の文字起こしエンジン名です。
const DEFAULT_ENGINE: &str = "whisper-local";

/// DB 接続を保持するアプリケーション状態です。単一ユーザーのデスクトップアプリのため
/// `Mutex<Connection>` で十分であり、コネクションプールは導入していません。
pub struct AppState {
    pub db: Mutex<Connection>,
}

/// 現在時刻を epoch ミリ秒で取得します。
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("システム時刻がUNIXエポックより前になることはない")
        .as_millis() as i64
}

/// 新しい配信セッションを開始します。
#[tauri::command]
pub fn start_session(state: State<'_, AppState>, title: String) -> Result<Session, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::create_session(&conn, &title, None, DEFAULT_ENGINE, now_ms()).map_err(|e| e.to_string())
}

/// 配信セッションを終了します。
#[tauri::command]
pub fn end_session(state: State<'_, AppState>, session_id: String) -> Result<Session, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::end_session(&conn, &session_id, now_ms()).map_err(|e| e.to_string())
}

/// 配信セッション一覧を開始時刻の降順で取得します。
#[tauri::command]
pub fn list_sessions(state: State<'_, AppState>, limit: u32) -> Result<Vec<Session>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::list_sessions(&conn, limit).map_err(|e| e.to_string())
}
