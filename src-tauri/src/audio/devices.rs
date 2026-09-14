//! 入力デバイス列挙を行うモジュールです。
//!
//! `cpal` の共有モード（排他モードは使用しない）でホストのデフォルト入力APIから
//! 列挙するため、OBS等の他アプリケーションが同一デバイスを使用中でも取得できます。

use cpal::traits::HostTrait;
use serde::Serialize;

/// UIに表示する入力デバイス情報です。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputDeviceInfo {
    /// デバイス名。`start_audio_capture` コマンドへの指定にそのまま使用します。
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
    let default_name = host.default_input_device().map(|d| d.to_string());

    let devices = host.input_devices().map_err(|e| e.to_string())?;
    Ok(devices
        .map(|device| {
            let name = device.to_string();
            let is_default = default_name.as_deref() == Some(name.as_str());
            InputDeviceInfo { name, is_default }
        })
        .collect())
}
