//! セッション・文字起こし・チャットメッセージの CRUD を提供するリポジトリ層です。
//!
//! ここに定義する関数はすべて `&Connection` を第一引数に取る自由関数とし、
//! Tauri のコマンド機構に依存しない純粋なドメインロジックとして実装しています。
//! 時刻（epoch ミリ秒）は呼び出し側から引数として渡すため、テストでは固定値を
//! 使って決定的に検証できます。
//!
//! `is_final=false` の暫定文字起こしは永続化の対象外です。呼び出し側（フェーズ3以降の
//! 文字起こしエンジン連携層）が `is_final=true` になった結果のみを
//! [`insert_transcript`] に渡してください。

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use super::{Result, StoreError};

/// 配信セッションです。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub title: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub twitch_channel: Option<String>,
    pub engine: String,
}

/// 文字起こしを新規追加する際の入力です。
///
/// フェーズ3（文字起こしエンジン連携）で `#[tauri::command]` から使用される予定のため、
/// 現時点ではリポジトリ層のテストからのみ使用しており未使用警告を抑制しています。
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct NewTranscript {
    pub session_id: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
    pub engine: String,
}

/// 永続化済みの文字起こしです。
///
/// フェーズ3で使用予定のため、現時点ではテストからのみ使用しており未使用警告を抑制しています。
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transcript {
    pub id: i64,
    pub session_id: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
    pub engine: String,
    pub created_at: i64,
}

/// チャットメッセージを新規追加する際の入力です。
///
/// フェーズ5（Twitchチャット連携）で使用予定のため、現時点ではテストからのみ
/// 使用しており未使用警告を抑制しています。
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct NewChatMessage {
    pub session_id: String,
    pub msg_id: Option<String>,
    pub channel: String,
    pub user_login: String,
    pub display_name: String,
    pub color: Option<String>,
    pub badges: Option<String>,
    pub bits: Option<i64>,
    pub reply_to: Option<String>,
    pub text: String,
    pub sent_at: i64,
}

/// 永続化済みのチャットメッセージです。
///
/// フェーズ5で使用予定のため、現時点ではテストからのみ使用しており未使用警告を抑制しています。
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: i64,
    pub session_id: String,
    pub msg_id: Option<String>,
    pub channel: String,
    pub user_login: String,
    pub display_name: String,
    pub color: Option<String>,
    pub badges: Option<String>,
    pub bits: Option<i64>,
    pub reply_to: Option<String>,
    pub text: String,
    pub sent_at: i64,
}

/// 新しいセッションを作成します。ID は UUID v4 で採番します。
pub fn create_session(
    conn: &Connection,
    title: &str,
    twitch_channel: Option<&str>,
    engine: &str,
    started_at: i64,
) -> Result<Session> {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO sessions (id, title, started_at, ended_at, twitch_channel, engine)
         VALUES (?1, ?2, ?3, NULL, ?4, ?5)",
        params![id, title, started_at, twitch_channel, engine],
    )?;
    Ok(Session {
        id,
        title: title.to_string(),
        started_at,
        ended_at: None,
        twitch_channel: twitch_channel.map(str::to_string),
        engine: engine.to_string(),
    })
}

/// セッションを ID で取得します。存在しない場合は `Ok(None)` を返します。
pub fn get_session(conn: &Connection, id: &str) -> Result<Option<Session>> {
    conn.query_row(
        "SELECT id, title, started_at, ended_at, twitch_channel, engine
         FROM sessions WHERE id = ?1",
        params![id],
        map_session_row,
    )
    .optional()
    .map_err(StoreError::from)
}

/// セッションを終了状態にします。
///
/// 対象が存在しない場合は [`StoreError::SessionNotFound`]、
/// 既に終了済みの場合は [`StoreError::SessionAlreadyEnded`] を返します（Fail-Fast）。
pub fn end_session(conn: &Connection, id: &str, ended_at: i64) -> Result<Session> {
    let session = get_session(conn, id)?.ok_or_else(|| StoreError::SessionNotFound(id.to_string()))?;
    if session.ended_at.is_some() {
        return Err(StoreError::SessionAlreadyEnded(id.to_string()));
    }

    conn.execute(
        "UPDATE sessions SET ended_at = ?1 WHERE id = ?2",
        params![ended_at, id],
    )?;

    Ok(Session {
        ended_at: Some(ended_at),
        ..session
    })
}

/// セッション一覧を開始時刻の降順で取得します。
pub fn list_sessions(conn: &Connection, limit: u32) -> Result<Vec<Session>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, started_at, ended_at, twitch_channel, engine
         FROM sessions ORDER BY started_at DESC, id DESC LIMIT ?1",
    )?;
    let rows = stmt
        .query_map(params![limit], map_session_row)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn map_session_row(row: &rusqlite::Row) -> rusqlite::Result<Session> {
    Ok(Session {
        id: row.get(0)?,
        title: row.get(1)?,
        started_at: row.get(2)?,
        ended_at: row.get(3)?,
        twitch_channel: row.get(4)?,
        engine: row.get(5)?,
    })
}

/// 文字起こしを1件追加します。
///
/// フェーズ3で `#[tauri::command]` から呼び出される予定のため、
/// 現時点ではテストからのみ使用しており未使用警告を抑制しています。
#[allow(dead_code)]
pub fn insert_transcript(
    conn: &Connection,
    new_transcript: &NewTranscript,
    created_at: i64,
) -> Result<Transcript> {
    conn.execute(
        "INSERT INTO transcripts (session_id, start_ms, end_ms, text, engine, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            new_transcript.session_id,
            new_transcript.start_ms,
            new_transcript.end_ms,
            new_transcript.text,
            new_transcript.engine,
            created_at,
        ],
    )?;
    let id = conn.last_insert_rowid();
    Ok(Transcript {
        id,
        session_id: new_transcript.session_id.clone(),
        start_ms: new_transcript.start_ms,
        end_ms: new_transcript.end_ms,
        text: new_transcript.text.clone(),
        engine: new_transcript.engine.clone(),
        created_at,
    })
}

/// 指定セッションの文字起こしを挿入順（`id` 昇順）で取得します。
///
/// フェーズ3で使用予定のため、現時点ではテストからのみ使用しており未使用警告を抑制しています。
#[allow(dead_code)]
pub fn list_transcripts(conn: &Connection, session_id: &str) -> Result<Vec<Transcript>> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, start_ms, end_ms, text, engine, created_at
         FROM transcripts WHERE session_id = ?1 ORDER BY id ASC",
    )?;
    let rows = stmt
        .query_map(params![session_id], |row| {
            Ok(Transcript {
                id: row.get(0)?,
                session_id: row.get(1)?,
                start_ms: row.get(2)?,
                end_ms: row.get(3)?,
                text: row.get(4)?,
                engine: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// チャットメッセージを1件追加します。
///
/// フェーズ5で `#[tauri::command]` から呼び出される予定のため、
/// 現時点ではテストからのみ使用しており未使用警告を抑制しています。
#[allow(dead_code)]
pub fn insert_chat_message(conn: &Connection, new_message: &NewChatMessage) -> Result<ChatMessage> {
    conn.execute(
        "INSERT INTO chat_messages
            (session_id, msg_id, channel, user_login, display_name, color, badges, bits, reply_to, text, sent_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            new_message.session_id,
            new_message.msg_id,
            new_message.channel,
            new_message.user_login,
            new_message.display_name,
            new_message.color,
            new_message.badges,
            new_message.bits,
            new_message.reply_to,
            new_message.text,
            new_message.sent_at,
        ],
    )?;
    let id = conn.last_insert_rowid();
    Ok(ChatMessage {
        id,
        session_id: new_message.session_id.clone(),
        msg_id: new_message.msg_id.clone(),
        channel: new_message.channel.clone(),
        user_login: new_message.user_login.clone(),
        display_name: new_message.display_name.clone(),
        color: new_message.color.clone(),
        badges: new_message.badges.clone(),
        bits: new_message.bits,
        reply_to: new_message.reply_to.clone(),
        text: new_message.text.clone(),
        sent_at: new_message.sent_at,
    })
}

/// 指定セッションのチャットメッセージを挿入順（`id` 昇順）で取得します。
///
/// フェーズ5で使用予定のため、現時点ではテストからのみ使用しており未使用警告を抑制しています。
#[allow(dead_code)]
pub fn list_chat_messages(conn: &Connection, session_id: &str) -> Result<Vec<ChatMessage>> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, msg_id, channel, user_login, display_name, color, badges, bits, reply_to, text, sent_at
         FROM chat_messages WHERE session_id = ?1 ORDER BY id ASC",
    )?;
    let rows = stmt
        .query_map(params![session_id], |row| {
            Ok(ChatMessage {
                id: row.get(0)?,
                session_id: row.get(1)?,
                msg_id: row.get(2)?,
                channel: row.get(3)?,
                user_login: row.get(4)?,
                display_name: row.get(5)?,
                color: row.get(6)?,
                badges: row.get(7)?,
                bits: row.get(8)?,
                reply_to: row.get(9)?,
                text: row.get(10)?,
                sent_at: row.get(11)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::open_in_memory;

    #[test]
    fn セッション作成後に同一内容を取得できる() {
        let conn = open_in_memory().unwrap();
        let created = create_session(&conn, "9月14日 ゲーム配信", None, "whisper-local", 1_000).unwrap();

        let fetched = get_session(&conn, &created.id).unwrap();

        assert_eq!(fetched, Some(created));
    }

    #[test]
    fn 存在しないセッションの取得は未取得を表す() {
        let conn = open_in_memory().unwrap();
        let fetched = get_session(&conn, "存在しないID").unwrap();
        assert_eq!(fetched, None);
    }

    #[test]
    fn セッション終了で終了時刻が設定される() {
        let conn = open_in_memory().unwrap();
        let created = create_session(&conn, "雑談配信", None, "whisper-local", 1_000).unwrap();

        let ended = end_session(&conn, &created.id, 5_000).unwrap();

        assert_eq!(ended.ended_at, Some(5_000));
    }

    #[test]
    fn 存在しないセッションの終了はエラーになる() {
        let conn = open_in_memory().unwrap();
        let result = end_session(&conn, "存在しないID", 5_000);
        assert!(matches!(result, Err(StoreError::SessionNotFound(_))));
    }

    #[test]
    fn 終了済みセッションの再終了はエラーになる() {
        let conn = open_in_memory().unwrap();
        let created = create_session(&conn, "雑談配信", None, "whisper-local", 1_000).unwrap();
        end_session(&conn, &created.id, 5_000).unwrap();

        let result = end_session(&conn, &created.id, 6_000);

        assert!(matches!(result, Err(StoreError::SessionAlreadyEnded(_))));
    }

    #[test]
    fn セッション一覧は開始時刻の降順で返る() {
        let conn = open_in_memory().unwrap();
        let older = create_session(&conn, "古い配信", None, "whisper-local", 1_000).unwrap();
        let newer = create_session(&conn, "新しい配信", None, "whisper-local", 2_000).unwrap();

        let sessions = list_sessions(&conn, 10).unwrap();

        assert_eq!(sessions, vec![newer, older]);
    }

    #[test]
    fn 文字起こしは挿入順で一覧取得できる() {
        let conn = open_in_memory().unwrap();
        let session = create_session(&conn, "雑談配信", None, "whisper-local", 1_000).unwrap();

        let first = insert_transcript(
            &conn,
            &NewTranscript {
                session_id: session.id.clone(),
                start_ms: 0,
                end_ms: 1_000,
                text: "今日はよろしくお願いします".to_string(),
                engine: "whisper-local".to_string(),
            },
            1_100,
        )
        .unwrap();
        let second = insert_transcript(
            &conn,
            &NewTranscript {
                session_id: session.id.clone(),
                start_ms: 1_000,
                end_ms: 2_000,
                text: "今日はゲームの続きをやります".to_string(),
                engine: "whisper-local".to_string(),
            },
            2_100,
        )
        .unwrap();

        let transcripts = list_transcripts(&conn, &session.id).unwrap();

        assert_eq!(transcripts, vec![first, second]);
    }

    #[test]
    fn 存在しないセッションへの文字起こし挿入は外部キー制約で失敗する() {
        let conn = open_in_memory().unwrap();

        let result = insert_transcript(
            &conn,
            &NewTranscript {
                session_id: "存在しないセッション".to_string(),
                start_ms: 0,
                end_ms: 1_000,
                text: "テスト発話".to_string(),
                engine: "whisper-local".to_string(),
            },
            1_000,
        );

        assert!(matches!(result, Err(StoreError::Sqlite(_))));
    }

    #[test]
    fn チャットメッセージの追加と一覧取得ができる() {
        let conn = open_in_memory().unwrap();
        let session = create_session(&conn, "雑談配信", None, "whisper-local", 1_000).unwrap();

        let with_all_fields = insert_chat_message(
            &conn,
            &NewChatMessage {
                session_id: session.id.clone(),
                msg_id: Some("msg-1".to_string()),
                channel: "#example_channel".to_string(),
                user_login: "viewer_taro".to_string(),
                display_name: "視聴者太郎".to_string(),
                color: Some("#FF0000".to_string()),
                badges: Some("subscriber/1".to_string()),
                bits: Some(100),
                reply_to: None,
                text: "配信お疲れ様です".to_string(),
                sent_at: 1_500,
            },
        )
        .unwrap();

        let with_nulls = insert_chat_message(
            &conn,
            &NewChatMessage {
                session_id: session.id.clone(),
                msg_id: None,
                channel: "#example_channel".to_string(),
                user_login: "viewer_hanako".to_string(),
                display_name: "視聴者花子".to_string(),
                color: None,
                badges: None,
                bits: None,
                reply_to: None,
                text: "こんにちは".to_string(),
                sent_at: 1_600,
            },
        )
        .unwrap();

        let messages = list_chat_messages(&conn, &session.id).unwrap();

        assert_eq!(messages, vec![with_all_fields, with_nulls]);
    }
}
