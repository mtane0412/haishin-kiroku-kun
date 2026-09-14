/**
 * 音声デバイス関連の型定義です。
 * Rust 側の `src-tauri/src/audio/devices.rs` の `InputDeviceInfo` 構造体（camelCase で
 * シリアライズされたもの）に対応します。
 */

/** マイク入力デバイスです。 */
export interface InputDeviceInfo {
  /** デバイス名。`start_audio_capture` コマンドの `deviceName` 引数にそのまま使用する */
  name: string;
  /** OS既定の入力デバイスかどうか */
  isDefault: boolean;
}
