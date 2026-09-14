//! SQLite への接続確立とマイグレーションを扱うモジュールです。
//!
//! 本アプリは配信セッション・文字起こし・チャットログを SQLite に永続化します。
//! 実行時は [`open_at`] でアプリデータディレクトリ配下のファイルを WAL モードで開き、
//! テスト時は [`open_in_memory`] でインメモリ DB を開きます。いずれも [`migrate`] を
//! 呼び出してスキーマを適用します。

use std::fs;
use std::path::Path;

use rusqlite::Connection;

pub mod repo;

/// ストア層で発生しうるエラーです。
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// SQLite 操作に起因するエラーです。
    #[error("SQLiteエラー: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// データディレクトリの作成に失敗した場合のエラーです。
    #[error("データディレクトリの作成に失敗しました: {0}")]
    DataDir(#[source] std::io::Error),
    /// 指定したセッションが見つからなかった場合のエラーです。
    #[error("セッションが見つかりません: {0}")]
    SessionNotFound(String),
    /// 既に終了しているセッションを再度終了しようとした場合のエラーです。
    #[error("セッションは既に終了しています: {0}")]
    SessionAlreadyEnded(String),
    /// セッションタイトルが空、または空白文字のみだった場合のエラーです。
    #[error("セッションタイトルを入力してください")]
    BlankTitle,
}

/// ストア層の `Result` エイリアスです。
pub type Result<T> = std::result::Result<T, StoreError>;

/// 指定したパスの SQLite ファイルを WAL モードで開き、マイグレーションを適用します。
///
/// 親ディレクトリが存在しない場合は作成します。作成に失敗した場合はアプリの起動を
/// 止めるため、暗黙のフォールバックはせずエラーを返します（Fail-Fast）。
pub fn open_at(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(StoreError::DataDir)?;
    }
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(conn)
}

/// テスト用にインメモリ SQLite を開き、マイグレーションを適用します。
///
/// 現時点では `#[cfg(test)]` テストからのみ使用されるため、通常ビルドでは
/// 未使用警告が出ます。`repo.rs` の単体テストで使用しているため許容します。
#[allow(dead_code)]
pub fn open_in_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(conn)
}

/// 現在のスキーマバージョンです。`PRAGMA user_version` と比較し、
/// 適用済みであればマイグレーションをスキップすることで冪等性を担保します。
const SCHEMA_VERSION: i64 = 1;

/// スキーマを適用します。複数回呼び出してもエラーにならず、
/// 既に最新バージョンが適用済みであれば何もしません。
pub fn migrate(conn: &Connection) -> Result<()> {
    let current_version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if current_version >= SCHEMA_VERSION {
        return Ok(());
    }

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            started_at INTEGER NOT NULL,
            ended_at INTEGER,
            twitch_channel TEXT,
            engine TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS transcripts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL REFERENCES sessions(id),
            start_ms INTEGER NOT NULL,
            end_ms INTEGER NOT NULL,
            text TEXT NOT NULL,
            engine TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS chat_messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL REFERENCES sessions(id),
            msg_id TEXT,
            channel TEXT NOT NULL,
            user_login TEXT NOT NULL,
            display_name TEXT NOT NULL,
            color TEXT,
            badges TEXT,
            bits INTEGER,
            reply_to TEXT,
            text TEXT NOT NULL,
            sent_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_transcripts_session ON transcripts(session_id, id);
        CREATE INDEX IF NOT EXISTS idx_chat_session ON chat_messages(session_id, id);
        ",
    )?;

    conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn マイグレーションを複数回適用してもエラーにならない() {
        let conn = open_in_memory().expect("インメモリDBを開けること");
        migrate(&conn).expect("2回目の適用でもエラーにならないこと");
        migrate(&conn).expect("3回目の適用でもエラーにならないこと");

        let table_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN ('sessions', 'transcripts', 'chat_messages')",
                [],
                |row| row.get(0),
            )
            .expect("テーブル数を取得できること");
        assert_eq!(table_count, 3);
    }
}
