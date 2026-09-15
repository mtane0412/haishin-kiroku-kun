/**
 * 文字起こしイベントの型定義です。
 * Rust 側の `src-tauri/src/transcribe/mod.rs` の `TranscriptEvent` 構造体
 * （camelCase でシリアライズされたもの）に対応します。
 */

/** 文字起こしエンジンが送出する1件の文字起こし結果です。 */
export interface TranscriptEvent {
  /** 発話区間を一意に識別するID */
  segmentId: string;
  /** 区間開始時刻（ストリーム先頭からの経過ミリ秒） */
  startMs: number;
  /** 区間終了時刻（ストリーム先頭からの経過ミリ秒） */
  endMs: number;
  /** 文字起こし結果のテキスト */
  text: string;
  /** 確定結果かどうか（false の場合は途中経過） */
  isFinal: boolean;
  /** 生成したエンジンのID */
  engine: string;
}
