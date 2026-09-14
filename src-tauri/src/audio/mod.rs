//! 音声キャプチャパイプラインのエントリポイントです。
//!
//! [`AudioCapture`] が `cpal` で選択デバイスからの入力ストリームを開始し、届いた生サンプルを
//! [`resample::downmix_and_resample_to_16k_mono`] で16kHzモノラルに正規化した上で
//! [`AudioFrame`] として `tokio::sync::broadcast` チャンネルへ配信します。併せてRMSレベルを
//! Tauriイベントでフロントエンドへ送出し、レベルメータ表示に使います。
//!
//! macOS・WindowsともにOBSと同一デバイスを共有キャプチャできるよう、`cpal` の共有モード
//! （排他モードは使用しない）のデフォルト入力設定のみを使用しています。

pub mod devices;
pub mod resample;
pub mod vad;

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::broadcast;

/// 16kHz モノラル f32 に正規化済みの音声フレームです。
///
/// フェーズ3（文字起こしエンジン連携）で [`AudioCapture::subscribe`] の購読側から
/// 使用される予定のため、現時点ではフィールドの読み取り側が無く未使用警告が出ます。
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AudioFrame {
    /// 16kHz mono, -1.0..=1.0 に正規化済みのサンプル列。
    pub samples: Vec<f32>,
    /// キャプチャ時刻（epochミリ秒）。
    pub captured_at_ms: i64,
}

/// レベルメータ表示用にフロントエンドへ送出するイベント名です。
pub const AUDIO_LEVEL_EVENT: &str = "audio://level";

/// `broadcast` チャンネルの容量です。エンジン側の消費が一時的に遅れても直近のフレームを
/// 保持できるよう、フレーム長（数十ms）に対して十分な余裕を持たせています。
const BROADCAST_CAPACITY: usize = 64;

/// [`AUDIO_LEVEL_EVENT`] のペイロードです。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioLevelPayload {
    /// 直近フレームのRMSレベル（0.0以上）。
    pub rms: f32,
}

/// 音声キャプチャのエラーです。
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    /// 指定した名前の入力デバイスが見つからなかった場合のエラーです。
    #[error("入力デバイスが見つかりません: {0}")]
    DeviceNotFound(String),
    /// `cpal` の操作（デバイス列挙・設定取得・ストリーム構築/開始等）に起因するエラーです。
    #[error("音声デバイス操作でエラーが発生しました: {0}")]
    Cpal(#[from] cpal::Error),
    /// `f32` 以外のサンプルフォーマットが返された場合のエラーです（現状未対応）。
    #[error("サポートされていないサンプルフォーマットです: {0:?}")]
    UnsupportedSampleFormat(SampleFormat),
}

/// マイク入力のキャプチャを管理します。
///
/// `start` で指定デバイスからのストリームを開始し、正規化済みの [`AudioFrame`] を
/// `tokio::sync::broadcast` チャンネルへ配信し続けます。`stop`（または `Drop`）で
/// ストリームを停止します。
pub struct AudioCapture {
    stream: Option<Stream>,
    sender: broadcast::Sender<AudioFrame>,
}

impl AudioCapture {
    /// 未開始状態の `AudioCapture` を作成します。
    pub fn new() -> Self {
        let (sender, _receiver) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            stream: None,
            sender,
        }
    }

    /// 配信される [`AudioFrame`] を購読するレシーバを取得します。
    ///
    /// フェーズ3で文字起こしエンジン側から呼び出される予定のため、
    /// 現時点では未使用警告を抑制しています。
    #[allow(dead_code)]
    pub fn subscribe(&self) -> broadcast::Receiver<AudioFrame> {
        self.sender.subscribe()
    }

    /// 指定したIDの入力デバイスからキャプチャを開始します。既に開始済みの場合は
    /// 一旦停止してから再開します。
    ///
    /// デバイス名ではなく `cpal::DeviceId`（の文字列表現）で指定します。同名の
    /// デバイスが複数存在する環境でも一意にデバイスを特定するためです。
    pub fn start(&mut self, device_id: &str, app_handle: AppHandle) -> Result<(), AudioError> {
        self.stop();

        let host = cpal::default_host();
        let parsed_id: cpal::DeviceId = device_id
            .parse()
            .map_err(|_| AudioError::DeviceNotFound(device_id.to_string()))?;
        let device = host
            .device_by_id(&parsed_id)
            .ok_or_else(|| AudioError::DeviceNotFound(device_id.to_string()))?;

        let config = device.default_input_config()?;
        let sample_format = config.sample_format();
        if sample_format != SampleFormat::F32 {
            return Err(AudioError::UnsupportedSampleFormat(sample_format));
        }
        let stream_config: cpal::StreamConfig = config.into();
        let channels = stream_config.channels;
        let input_rate = stream_config.sample_rate;

        let sender = self.sender.clone();
        let app_handle = Arc::new(app_handle);
        let err_fn = |err| eprintln!("入力ストリームでエラーが発生しました: {err}");

        let stream = device.build_input_stream(
            stream_config,
            move |data: &[f32], _| {
                handle_input_data(data, channels, input_rate, &sender, &app_handle);
            },
            err_fn,
            None,
        )?;
        stream.play()?;

        self.stream = Some(stream);
        Ok(())
    }

    /// キャプチャを停止します。開始していない場合は何もしません。
    pub fn stop(&mut self) {
        // Stream の Drop でホスト側のストリームが停止される
        self.stream = None;
    }
}

impl Default for AudioCapture {
    fn default() -> Self {
        Self::new()
    }
}

/// 入力コールバックから届いた生サンプルを正規化し、配信・UI通知を行います。
fn handle_input_data(
    data: &[f32],
    channels: u16,
    input_rate: u32,
    sender: &broadcast::Sender<AudioFrame>,
    app_handle: &AppHandle,
) {
    let samples = resample::downmix_and_resample_to_16k_mono(data, channels, input_rate);
    if samples.is_empty() {
        return;
    }

    let level = resample::rms(&samples);
    // 受信側（エンジン）が存在しない、またはチャンネルが満杯の場合でもキャプチャ自体は
    // 継続する必要があるため、送信エラーは無視する（Fail-Fastの対象外: リアルタイム
    // ストリームの一時的な受信者不在は異常ではない）。
    let _ = sender.send(AudioFrame {
        samples,
        captured_at_ms: now_ms(),
    });
    let _ = app_handle.emit(AUDIO_LEVEL_EVENT, AudioLevelPayload { rms: level });
}

/// 現在時刻を epoch ミリ秒で取得します。
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("システム時刻がUNIXエポックより前になることはない")
        .as_millis() as i64
}
