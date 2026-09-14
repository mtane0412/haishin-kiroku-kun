//! 入力デバイス列挙を行うモジュールです。
//!
//! `cpal` の共有モード（排他モードは使用しない）でホストのデフォルト入力APIから
//! 列挙するため、OBS等の他アプリケーションが同一デバイスを使用中でも取得できます。

use cpal::traits::{DeviceTrait, HostTrait};
use serde::Serialize;

/// UIに表示する入力デバイス情報です。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputDeviceInfo {
    /// デバイスの一意な識別子（`cpal::DeviceId` の文字列表現）。
    /// `start_audio_capture` コマンドへの指定にはこちらを使用します
    /// （デバイス名は同名の複数デバイスが存在しうるため、識別には使用しません）。
    pub id: String,
    /// UI表示用のデバイス名。
    pub name: String,
    /// OS既定の入力デバイスかどうか。
    pub is_default: bool,
}

/// 利用可能な入力デバイス一覧を取得します。
///
/// デバイス名は `Device` の `Display` 実装（`to_string()`）で取得します。
/// `description()` はデバイスによってはエラーを返しうる一方、名前の取得自体は
/// UI表示に必須のため、確実に取得できる `Display` を使用しています。
pub fn list_input_devices() -> Result<Vec<InputDeviceInfo>, String> {
    let host = cpal::default_host();
    let default_id = host
        .default_input_device()
        .and_then(|d| d.id().ok())
        .map(|id| id.to_string());

    let devices = host.input_devices().map_err(|e| e.to_string())?;
    devices
        .map(|device| {
            let id = device.id().map_err(|e| e.to_string())?.to_string();
            let name = device.to_string();
            let is_default = default_id.as_deref() == Some(id.as_str());
            Ok(InputDeviceInfo { id, name, is_default })
        })
        .collect()
}
