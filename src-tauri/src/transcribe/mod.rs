//! 文字起こしエンジンの抽象（trait）とエンジン管理を定義するモジュールです。
//!
//! [`TranscriptionEngine`] はエンジン実装（ローカルwhisper等）が満たすべきインターフェースで、
//! 音声フレームの受信チャンネル・イベント送信チャンネル・停止通知用の [`CancellationToken`]
//! を受け取り非同期に実行されます。[`EngineRunner`] は現在稼働中のエンジンを1つだけ保持し、
//! エンジン切替時には前のエンジンへ停止を通知してその終了を待ってから次のエンジンを起動する
//! ことで、2つのエンジンが同時に同じ音声を消費してしまう事態を防ぎます。

pub mod whisper_local;

use std::sync::Arc;

use async_trait::async_trait;
use serde::Serialize;
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;

use crate::audio::AudioFrame;

/// 文字起こしエンジンが送出する1件の文字起こし結果です。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptEvent {
    /// 発話区間を一意に識別するID。
    pub segment_id: String,
    /// 区間開始時刻（ストリーム先頭からの経過ミリ秒）。
    pub start_ms: i64,
    /// 区間終了時刻（ストリーム先頭からの経過ミリ秒）。
    pub end_ms: i64,
    /// 文字起こし結果のテキスト。
    pub text: String,
    /// 確定結果かどうか（`false` の場合は途中経過）。
    pub is_final: bool,
    /// 生成したエンジンのID（[`TranscriptionEngine::id`]）。
    pub engine: String,
}

/// 文字起こしエンジンのエラーです。
#[derive(Debug, thiserror::Error)]
pub enum TranscribeError {
    /// モデルパスが設定画面等から指定されていない場合のエラーです。
    #[error("モデルが設定されていません: {0}")]
    ModelNotConfigured(String),
    /// 指定されたモデルファイルが存在しない場合のエラーです。
    #[error("モデルファイルが見つかりません: {0}")]
    ModelNotFound(String),
    /// エンジンの初期化（モデルロード等）に失敗した場合のエラーです。
    #[error("文字起こしエンジンの初期化に失敗しました: {0}")]
    InitFailed(String),
    /// 実行中に発生したその他のエラーです。
    #[error("文字起こし処理でエラーが発生しました: {0}")]
    Runtime(String),
}

/// 差し替え可能な文字起こしエンジンの共通インターフェースです。
///
/// 実装は `run` 呼び出し時点でモデル未設定・不存在等の起動に必要な前提条件を検証し、
/// 満たさない場合は音声を消費する前に即座に `Err` を返す必要があります
/// （Fail-Fast、無音へのフォールバックは行いません）。
#[async_trait]
pub trait TranscriptionEngine: Send + Sync {
    /// エンジンを一意に識別するID（例: `"whisper-local"`）。
    fn id(&self) -> &'static str;

    /// 音声フレームを受信しながら文字起こしを行い、結果を `out` へ送出し続けます。
    ///
    /// `cancel` がキャンセルされた場合は速やかに処理を打ち切り `Ok(())` を返します。
    async fn run(
        &self,
        audio: broadcast::Receiver<AudioFrame>,
        out: mpsc::Sender<TranscriptEvent>,
        cancel: CancellationToken,
    ) -> Result<(), TranscribeError>;
}

/// 稼働中のエンジンタスクの実体です。
struct RunningEngine {
    cancel: CancellationToken,
    handle: tokio::task::JoinHandle<Result<(), TranscribeError>>,
}

/// 現在稼働中の文字起こしエンジンを1つだけ保持して管理します。
///
/// `switch_to` は前のエンジンへキャンセルを通知し、そのタスクが終了するまで待った上で
/// 次のエンジンを起動します。これにより、エンジン切替時に2つのエンジンが同時に音声を
/// 消費することはありません。
pub struct EngineRunner {
    current: Option<RunningEngine>,
}

impl EngineRunner {
    /// 何も稼働していない状態の `EngineRunner` を作成します。
    pub fn new() -> Self {
        Self { current: None }
    }

    /// 現在稼働中のエンジンを停止した上で、指定エンジンを起動します。
    pub async fn switch_to(
        &mut self,
        engine: Arc<dyn TranscriptionEngine>,
        audio: broadcast::Receiver<AudioFrame>,
        out: mpsc::Sender<TranscriptEvent>,
    ) {
        self.stop().await;

        let cancel = CancellationToken::new();
        let child_cancel = cancel.clone();
        let handle = tokio::spawn(async move { engine.run(audio, out, child_cancel).await });

        self.current = Some(RunningEngine { cancel, handle });
    }

    /// 稼働中のエンジンへ停止を通知し、タスクが終了するまで待機します。
    /// 何も稼働していない場合は何もしません。
    pub async fn stop(&mut self) {
        if let Some(running) = self.current.take() {
            running.cancel.cancel();
            let _ = running.handle.await;
        }
    }
}

impl Default for EngineRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// テスト用のモックエンジンです。受信した [`AudioFrame`] ごとに1件の
    /// [`TranscriptEvent`] を送出し、キャンセルされたら終了して `stopped` を立てます。
    struct MockEngine {
        stopped: Arc<AtomicBool>,
    }

    #[async_trait]
    impl TranscriptionEngine for MockEngine {
        fn id(&self) -> &'static str {
            "mock"
        }

        async fn run(
            &self,
            mut audio: broadcast::Receiver<AudioFrame>,
            out: mpsc::Sender<TranscriptEvent>,
            cancel: CancellationToken,
        ) -> Result<(), TranscribeError> {
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        self.stopped.store(true, Ordering::SeqCst);
                        return Ok(());
                    }
                    frame = audio.recv() => {
                        let Ok(frame) = frame else { return Ok(()) };
                        let event = TranscriptEvent {
                            segment_id: "seg-1".to_string(),
                            start_ms: frame.captured_at_ms,
                            end_ms: frame.captured_at_ms + 100,
                            text: "テスト".to_string(),
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

    #[tokio::test]
    async fn イベントが伝播すること() {
        let (audio_tx, audio_rx) = broadcast::channel(4);
        let (out_tx, mut out_rx) = mpsc::channel(4);
        let cancel = CancellationToken::new();
        let engine = MockEngine {
            stopped: Arc::new(AtomicBool::new(false)),
        };

        let child_cancel = cancel.clone();
        let handle = tokio::spawn(async move { engine.run(audio_rx, out_tx, child_cancel).await });

        audio_tx
            .send(AudioFrame {
                samples: vec![0.0],
                captured_at_ms: 1_000,
            })
            .expect("受信側が存在するため送信できること");

        let event = out_rx.recv().await.expect("イベントを受信できること");
        assert_eq!(event.segment_id, "seg-1");
        assert_eq!(event.start_ms, 1_000);
        assert_eq!(event.end_ms, 1_100);
        assert_eq!(event.text, "テスト");
        assert!(event.is_final);
        assert_eq!(event.engine, "mock");

        cancel.cancel();
        let result = handle.await.expect("タスクがpanicしないこと");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn キャンセルで停止すること() {
        let (_audio_tx, audio_rx) = broadcast::channel(4);
        let (out_tx, _out_rx) = mpsc::channel(4);
        let cancel = CancellationToken::new();
        let stopped = Arc::new(AtomicBool::new(false));
        let engine = MockEngine {
            stopped: stopped.clone(),
        };

        let child_cancel = cancel.clone();
        let handle = tokio::spawn(async move { engine.run(audio_rx, out_tx, child_cancel).await });

        cancel.cancel();
        let result = handle.await.expect("タスクがpanicしないこと");
        assert!(result.is_ok());
        assert!(stopped.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn エンジン切替時に前のエンジンが確実に停止すること() {
        let (_audio_tx1, audio_rx1) = broadcast::channel(4);
        let (out_tx1, _out_rx1) = mpsc::channel(4);
        let stopped1 = Arc::new(AtomicBool::new(false));
        let engine1 = Arc::new(MockEngine {
            stopped: stopped1.clone(),
        });

        let mut runner = EngineRunner::new();
        runner.switch_to(engine1, audio_rx1, out_tx1).await;

        let (_audio_tx2, audio_rx2) = broadcast::channel(4);
        let (out_tx2, _out_rx2) = mpsc::channel(4);
        let stopped2 = Arc::new(AtomicBool::new(false));
        let engine2 = Arc::new(MockEngine {
            stopped: stopped2.clone(),
        });

        // switch_to は内部で前のエンジンの停止を待ってから次のエンジンを起動するため、
        // このawaitが完了した時点でengine1は確実に停止している。
        runner.switch_to(engine2, audio_rx2, out_tx2).await;

        assert!(
            stopped1.load(Ordering::SeqCst),
            "前のエンジンが停止していること"
        );
        assert!(
            !stopped2.load(Ordering::SeqCst),
            "新しいエンジンはまだ停止していないこと"
        );

        runner.stop().await;
        assert!(
            stopped2.load(Ordering::SeqCst),
            "stop呼び出し後は新しいエンジンも停止すること"
        );
    }
}
