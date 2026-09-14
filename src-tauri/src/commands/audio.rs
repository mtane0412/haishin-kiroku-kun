//! マイク入力デバイスの列挙とキャプチャの開始・停止を行う Tauri コマンドです。

use std::sync::Mutex;

use tauri::{AppHandle, State};

use crate::audio::devices::{self, InputDeviceInfo};
use crate::audio::AudioCapture;

/// 音声キャプチャの状態を保持するアプリケーション状態です。
pub struct AudioState {
    /// 起動中の `AudioCapture`。単一ユーザーのデスクトップアプリのため
    /// `Mutex` で十分であり、コネクションプールのような仕組みは導入していません。
    pub capture: Mutex<AudioCapture>,
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            capture: Mutex::new(AudioCapture::new()),
        }
    }
}

/// 利用可能な入力デバイス一覧を取得します。
#[tauri::command]
pub fn list_input_devices() -> Result<Vec<InputDeviceInfo>, String> {
    devices::list_input_devices()
}

/// 指定したデバイスからマイク入力のキャプチャを開始します。
#[tauri::command]
pub fn start_audio_capture(
    state: State<'_, AudioState>,
    app_handle: AppHandle,
    device_name: String,
) -> Result<(), String> {
    let mut capture = state.capture.lock().map_err(|e| e.to_string())?;
    capture
        .start(&device_name, app_handle)
        .map_err(|e| e.to_string())
}

/// マイク入力のキャプチャを停止します。
#[tauri::command]
pub fn stop_audio_capture(state: State<'_, AudioState>) -> Result<(), String> {
    let mut capture = state.capture.lock().map_err(|e| e.to_string())?;
    capture.stop();
    Ok(())
}
