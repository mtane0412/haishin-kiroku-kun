/**
 * 音声デバイス関連の型定義です。
 * Rust 側の `src-tauri/src/audio/devices.rs` の `InputDeviceInfo` 構造体（camelCase で
 * シリアライズされたもの）に対応します。
 */

/** マイク入力デバイスです。 */
export interface InputDeviceInfo {
  /**
   * デバイスの一意な識別子。`start_audio_capture` コマンドの `deviceId` 引数に使用する。
   * デバイス名は同名の複数デバイスが存在しうるため、識別には `id` を使用する。
   */
  id: string;
  /** UI表示用のデバイス名 */
  name: string;
  /** OS既定の入力デバイスかどうか */
  isDefault: boolean;
}
