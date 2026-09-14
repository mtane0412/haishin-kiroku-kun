/**
 * 配信セッションの型定義です。
 * Rust 側の `src-tauri/src/store/repo.rs` の `Session` 構造体（camelCase で
 * シリアライズされたもの）に対応します。
 */

/** 配信セッションです。 */
export interface Session {
  /** セッションID（UUID v4） */
  id: string;
  /** セッションタイトル */
  title: string;
  /** 開始時刻（epochミリ秒） */
  startedAt: number;
  /** 終了時刻（epochミリ秒）。進行中の場合は null */
  endedAt: number | null;
  /** Twitchチャンネル名。未設定の場合は null */
  twitchChannel: string | null;
  /** 使用する文字起こしエンジン名 */
  engine: string;
}
