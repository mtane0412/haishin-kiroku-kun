//! ローカル実行の whisper.cpp を使った [`TranscriptionEngine`] 実装です。
//!
//! VADで区切った2〜15秒の発話区間ごとに `whisper-rs` へ渡して文字起こしを行います。
//! モデルファイル（ggml形式）のパスは設定画面で指定する想定で、未設定・不存在の場合は
//! 音声を消費する前に `run` が即座にエラーを返します（Fail-Fast）。

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use super::{TranscribeError, TranscriptEvent, TranscriptionEngine};
use crate::audio::AudioFrame;

/// このエンジンのID。
pub const ENGINE_ID: &str = "whisper-local";

/// ローカルwhisper.cppによる文字起こしエンジンです。
pub struct WhisperLocalEngine {
    /// ggml形式のモデルファイルパス。未設定の場合は `None`。
    model_path: Option<PathBuf>,
}

impl WhisperLocalEngine {
    /// モデルパスを指定して `WhisperLocalEngine` を作成します。
    pub fn new(model_path: Option<PathBuf>) -> Self {
        Self { model_path }
    }

    /// モデルパスの前提条件（設定済み・存在する）を検証します。
    fn validate_model_path(&self) -> Result<&PathBuf, TranscribeError> {
        let path = self
            .model_path
            .as_ref()
            .ok_or_else(|| TranscribeError::ModelNotConfigured(ENGINE_ID.to_string()))?;
        if !path.exists() {
            return Err(TranscribeError::ModelNotFound(path.display().to_string()));
        }
        Ok(path)
    }
}

#[async_trait]
impl TranscriptionEngine for WhisperLocalEngine {
    fn id(&self) -> &'static str {
        ENGINE_ID
    }

    async fn run(
        &self,
        mut audio: broadcast::Receiver<AudioFrame>,
        out: mpsc::Sender<TranscriptEvent>,
        cancel: CancellationToken,
        ready: oneshot::Sender<Result<(), TranscribeError>>,
    ) -> Result<(), TranscribeError> {
        let ctx = match self
            .validate_model_path()
            .cloned()
            .and_then(|model_path| load_context(&model_path))
        {
            Ok(ctx) => Arc::new(ctx),
            Err(e) => {
                let _ = ready.send(Err(e.clone()));
                return Err(e);
            }
        };
        // モデルロードまで完了したので、呼び出し元（EngineRunner::switch_to）へ
        // 初期化成功を通知する。以降は音声受信・文字起こしのメインループに入る。
        let _ = ready.send(Ok(()));

        let mut segmenter = crate::audio::vad::VadSegmenter::new(crate::audio::vad::VadConfig {
            sample_rate: crate::audio::resample::TARGET_SAMPLE_RATE,
            ..Default::default()
        });

        loop {
            tokio::select! {
                _ = cancel.cancelled() => return Ok(()),
                frame = audio.recv() => {
                    let frame = match frame {
                        Ok(frame) => frame,
                        Err(broadcast::error::RecvError::Closed) => return Ok(()),
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    };

                    for segment in segmenter.push(&frame.samples) {
                        // whisper推論はCPUバウンドな同期処理のため、tokioのワーカースレッドを
                        // 占有しないよう spawn_blocking 専用スレッドで実行する。
                        let ctx = Arc::clone(&ctx);
                        let samples = segment.samples;
                        let text = tokio::task::spawn_blocking(move || {
                            transcribe_segment(&ctx, &samples)
                        })
                        .await
                        .map_err(|e| TranscribeError::Runtime(e.to_string()))??;

                        let event = TranscriptEvent {
                            segment_id: uuid::Uuid::new_v4().to_string(),
                            start_ms: segment.start_ms,
                            end_ms: segment.end_ms,
                            text,
                            is_final: true,
                            engine: self.id().to_string(),
                        };
                        if out.send(event).await.is_err() {
                            return Ok(());
                        }
                    }
                }
            }
        }
    }
}

/// whisper.cpp のモデルをロードして推論コンテキストを構築します。
fn load_context(
    model_path: &std::path::Path,
) -> Result<whisper_rs::WhisperContext, TranscribeError> {
    let path_str = model_path.to_str().ok_or_else(|| {
        TranscribeError::InitFailed("モデルパスがUTF-8ではありません".to_string())
    })?;
    whisper_rs::WhisperContext::new_with_params(
        path_str,
        whisper_rs::WhisperContextParameters::default(),
    )
    .map_err(|e| TranscribeError::InitFailed(e.to_string()))
}

/// 1つの発話区間（16kHzモノラルf32サンプル列）をwhisperで文字起こしします。
fn transcribe_segment(
    ctx: &whisper_rs::WhisperContext,
    samples: &[f32],
) -> Result<String, TranscribeError> {
    let mut state = ctx
        .create_state()
        .map_err(|e| TranscribeError::Runtime(e.to_string()))?;
    let params = whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
    state
        .full(params, samples)
        .map_err(|e| TranscribeError::Runtime(e.to_string()))?;

    let num_segments = state.full_n_segments();
    let mut text = String::new();
    for i in 0..num_segments {
        let Some(segment) = state.get_segment(i) else {
            continue;
        };
        let segment_text = segment
            .to_str()
            .map_err(|e| TranscribeError::Runtime(e.to_string()))?;
        text.push_str(segment_text);
    }
    Ok(text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn モデル未設定時に明確なエラーが返ること() {
        let engine = WhisperLocalEngine::new(None);
        let (_audio_tx, audio_rx) = broadcast::channel(4);
        let (out_tx, _out_rx) = mpsc::channel(4);
        let cancel = CancellationToken::new();
        let (ready_tx, ready_rx) = oneshot::channel();

        let result = engine.run(audio_rx, out_tx, cancel, ready_tx).await;

        assert!(matches!(
            result,
            Err(TranscribeError::ModelNotConfigured(_))
        ));
        // readyにも同じエラーが即座に通知されること（EngineRunner::switch_toが
        // この通知を待って初期化失敗を呼び出し元へ伝播するため）。
        assert!(matches!(
            ready_rx.await,
            Ok(Err(TranscribeError::ModelNotConfigured(_)))
        ));
    }

    #[tokio::test]
    async fn モデルファイルが存在しない場合に明確なエラーが返ること() {
        let engine = WhisperLocalEngine::new(Some(PathBuf::from("/存在しないパス/model.bin")));
        let (_audio_tx, audio_rx) = broadcast::channel(4);
        let (out_tx, _out_rx) = mpsc::channel(4);
        let cancel = CancellationToken::new();
        let (ready_tx, ready_rx) = oneshot::channel();

        let result = engine.run(audio_rx, out_tx, cancel, ready_tx).await;

        assert!(matches!(result, Err(TranscribeError::ModelNotFound(_))));
        assert!(matches!(
            ready_rx.await,
            Ok(Err(TranscribeError::ModelNotFound(_)))
        ));
    }

    /// 手動確認用: 実モデル・実音声で文字起こしが行われることを確認します。
    ///
    /// `cargo test --ignored 実際の音声で発話が文字起こしされること -- --nocapture` で実行し、
    /// 標準出力に文字起こし結果が表示されることを目視確認してください。
    /// モデル・WAVファイルのパスは環境変数で指定します
    /// （WHISPER_TEST_MODEL / WHISPER_TEST_WAV、16kHzモノラル16bit PCM WAV想定）。
    #[ignore]
    #[tokio::test]
    async fn 実際の音声で発話が文字起こしされること() {
        let model_path =
            std::env::var("WHISPER_TEST_MODEL").expect("WHISPER_TEST_MODEL を設定してください");
        let wav_path =
            std::env::var("WHISPER_TEST_WAV").expect("WHISPER_TEST_WAV を設定してください");

        let samples = read_wav_i16_mono_16k_as_f32(&wav_path);

        let engine = WhisperLocalEngine::new(Some(PathBuf::from(model_path)));
        let (audio_tx, audio_rx) = broadcast::channel(4);
        let (out_tx, mut out_rx) = mpsc::channel(16);
        let cancel = CancellationToken::new();

        let child_cancel = cancel.clone();
        let (ready_tx, ready_rx) = oneshot::channel();
        let handle =
            tokio::spawn(async move { engine.run(audio_rx, out_tx, child_cancel, ready_tx).await });
        ready_rx
            .await
            .expect("readyが送られること")
            .expect("モデル初期化に成功すること");

        audio_tx
            .send(AudioFrame {
                samples,
                captured_at_ms: 0,
            })
            .expect("受信側が存在するため送信できること");

        let event = out_rx.recv().await.expect("イベントを受信できること");

        println!("文字起こし結果: {}", event.text);
        assert!(
            !event.text.trim().is_empty(),
            "文字起こし結果が空でないこと"
        );

        cancel.cancel();
        handle.await.expect("タスクがpanicしないこと").ok();
    }

    /// 16bit PCM モノラルWAVを読み込み、-1.0..=1.0 の f32 サンプル列に変換します
    /// （手動確認テスト専用の簡易パーサ。canonical WAVヘッダ（44バイト）のみ対応）。
    #[cfg(test)]
    fn read_wav_i16_mono_16k_as_f32(path: &str) -> Vec<f32> {
        let bytes = std::fs::read(path).expect("WAVファイルを読み込めること");
        let data = &bytes[44..];
        data.chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / i16::MAX as f32)
            .collect()
    }
}
