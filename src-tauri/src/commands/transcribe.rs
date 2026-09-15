//! 文字起こしエンジンの起動・停止・モデルパス設定を行う Tauri コマンドです。
//!
//! `start_transcription` はマイクキャプチャ（[`AudioState`]）から音声フレームを購読し、
//! ローカルwhisper.cppエンジン（[`WhisperLocalEngine`]）で文字起こしを行いながら、
//! 結果を [`TRANSCRIPT_EVENT`] イベントとしてフロントエンドへ送出し続けます。

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;

use crate::commands::audio::AudioState;
use crate::transcribe::whisper_local::WhisperLocalEngine;
use crate::transcribe::EngineRunner;

/// 文字起こし結果をフロントエンドへ送出する際のイベント名です。
pub const TRANSCRIPT_EVENT: &str = "transcript://event";

/// 送出する [`TranscriptEvent`](crate::transcribe::TranscriptEvent) を溜めておくチャンネルの容量です。
const TRANSCRIPT_CHANNEL_CAPACITY: usize = 32;

/// 文字起こしエンジンの稼働状態とモデルパス設定を保持するアプリケーション状態です。
pub struct TranscribeState {
    /// 現在稼働中のエンジンを管理します。`switch_to`/`stop` 中に awaitを跨いで
    /// ロックを保持するため `tokio::sync::Mutex` を使用しています。
    runner: tokio::sync::Mutex<EngineRunner>,
    /// ローカルwhisperのモデルファイル（ggml形式）パス。設定画面から指定される想定です。
    model_path: Mutex<Option<PathBuf>>,
}

impl Default for TranscribeState {
    fn default() -> Self {
        Self {
            runner: tokio::sync::Mutex::new(EngineRunner::new()),
            model_path: Mutex::new(None),
        }
    }
}

/// ローカルwhisperのモデルファイルパスを設定します。
#[tauri::command]
pub fn set_whisper_model_path(
    state: State<'_, TranscribeState>,
    path: String,
) -> Result<(), String> {
    let mut model_path = state.model_path.lock().map_err(|e| e.to_string())?;
    *model_path = Some(PathBuf::from(path));
    Ok(())
}

/// マイク入力の文字起こしを開始します。
///
/// モデルパスが未設定・不存在の場合はエンジンの初期化に失敗し、その旨のエラーを
/// そのまま返します（Fail-Fast、無音へのフォールバックは行いません）。
#[tauri::command]
pub async fn start_transcription(
    transcribe_state: State<'_, TranscribeState>,
    audio_state: State<'_, AudioState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let model_path = transcribe_state
        .model_path
        .lock()
        .map_err(|e| e.to_string())?
        .clone();

    let audio_rx = {
        let capture = audio_state.capture.lock().map_err(|e| e.to_string())?;
        capture.subscribe()
    };

    let engine = Arc::new(WhisperLocalEngine::new(model_path));
    let (out_tx, mut out_rx) = mpsc::channel(TRANSCRIPT_CHANNEL_CAPACITY);

    {
        let mut runner = transcribe_state.runner.lock().await;
        runner.switch_to(engine, audio_rx, out_tx).await;
    }

    tokio::spawn(async move {
        while let Some(event) = out_rx.recv().await {
            // イベント送出先（フロントエンド）が存在しない場合でもエンジン自体は継続する
            // 必要があるため、送信エラーは無視する。
            let _ = app_handle.emit(TRANSCRIPT_EVENT, event);
        }
    });

    Ok(())
}

/// マイク入力の文字起こしを停止します。
#[tauri::command]
pub async fn stop_transcription(
    transcribe_state: State<'_, TranscribeState>,
) -> Result<(), String> {
    let mut runner = transcribe_state.runner.lock().await;
    runner.stop().await;
    Ok(())
}
